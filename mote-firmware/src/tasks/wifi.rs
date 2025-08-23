use cyw43_pio::{PioSpi, RM2_CLOCK_DIVIDER};
use defmt::*;
use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_net::udp::{PacketMetadata, UdpMetadata, UdpSocket};
use embassy_net::{Config, Stack, StackResources};
use embassy_rp::clocks::RoscRng;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, PIO0};
use embassy_rp::pio::Pio;
use leasehund::DhcpServer;
use mote_messages::runtime::{host_to_mote, mote_to_host};
use postcard::{from_bytes, to_vec};
use rand::RngCore;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

use super::{Cyw43Resources, Irqs};
use crate::tasks::MOTE_TO_HOST;

const SERVER_PORT: u16 = 1738;

async fn parse_message(
    rx_message: &host_to_mote::Message,
    endpoint: UdpMetadata,
    socket: &mut UdpSocket<'static>,
) -> Result<(), embassy_net::udp::SendError> {
    match rx_message {
        host_to_mote::Message::Ping => {
            let buf: heapless_postcard::Vec<u8, 100> = to_vec(&mote_to_host::Message::PingResponse).unwrap();
            info!("Parsed ping request, responding.");
            socket.send_to(&buf, endpoint).await?;
        }
        host_to_mote::Message::PingResponse => {
            info!("Received ping response from host.")
        }
        _ => {
            error!("Received unhandled message type");
        }
    }
    Ok(())
}

#[embassy_executor::task]
async fn tcp_server_task(stack: Stack<'static>) -> ! {
    loop {
        static TX_BUFFER: StaticCell<[u8; 4096]> = StaticCell::new();
        static TX_META: StaticCell<[PacketMetadata; 16]> = StaticCell::new();
        let tx_buffer = &mut TX_BUFFER.init([0; 4096])[..];
        let tx_meta = &mut TX_META.init([PacketMetadata::EMPTY; 16])[..];

        static RX_BUFFER: StaticCell<[u8; 4096]> = StaticCell::new();
        static RX_META: StaticCell<[PacketMetadata; 16]> = StaticCell::new();
        let rx_buffer = &mut RX_BUFFER.init([0; 4096])[..];
        let rx_meta = &mut RX_META.init([PacketMetadata::EMPTY; 16])[..];

        let mut socket = UdpSocket::new(stack, rx_meta, rx_buffer, tx_meta, tx_buffer);

        if let Err(e) = socket.bind(SERVER_PORT) {
            warn!("bind error: {:?}", e);
            continue;
        }

        let mut message_buffer = [0; 4096];
        let mut endpoint: Option<UdpMetadata> = None;
        let mut serialize_buf: heapless_postcard::Vec<u8, 4096> = heapless_postcard::Vec::new();
        loop {
            match select(socket.recv_from(&mut message_buffer), MOTE_TO_HOST.receive()).await {
                Either::First(Ok((bytes_read, ep))) => {
                    info!("Read {} bytes from {}.", bytes_read, ep);
                    let rx_message: host_to_mote::Message = from_bytes(&message_buffer).unwrap();
                    parse_message(&rx_message, ep, &mut socket).await.unwrap();
                    endpoint = Some(ep);
                }
                Either::First(Err(err)) => error!("TCP server received error {}", err),
                Either::Second(tx_message) => {
                    // TODO:
                    // * postcard serialized size is currently experimental and not implemented as
                    // a constant fn. After that feature is stabilized, consider how to rightsize this buffer
                    // * add anyhow and avoid this unwrap
                    serialize_buf = to_vec(&tx_message).unwrap();
                    if let Some(ep) = endpoint {
                        if let Err(error) = socket.send_to(&serialize_buf, ep).await {
                            error!("TX message failed, got {}", error);
                        }
                    }
                }
            }
        }
    }
}

#[embassy_executor::task]
async fn cyw43_task(runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn dhcp_server_task(stack: Stack<'static>) -> ! {
    let mut dhcp_server: DhcpServer<32, 4> = DhcpServer::new_with_dns(
        embassy_net::Ipv4Address::new(192, 168, 1, 1),   // Server IP
        embassy_net::Ipv4Address::new(255, 255, 255, 0), // Subnet mask
        embassy_net::Ipv4Address::new(192, 168, 1, 1),   // Router/Gateway
        embassy_net::Ipv4Address::new(8, 8, 8, 8),       // DNS server
        embassy_net::Ipv4Address::new(192, 168, 1, 100), // IP pool start
        embassy_net::Ipv4Address::new(192, 168, 1, 200), // IP pool end
    );

    dhcp_server.run(stack).await;
}

pub async fn init(spawner: Spawner, r: Cyw43Resources) {
    // let fw = include_bytes!("../cyw43-firmware/43439A0.bin");
    // let clm = include_bytes!("../cyw43-firmware/43439A0_clm.bin");

    // To make flashing faster for development, you may want to flash the firmwares independently
    // at hardcoded addresses, instead of baking them into the program with `include_bytes!`:
    //     probe-rs download ../../cyw43-firmware/43439A0.bin --binary-format bin --chip RP235x --base-address 0x10100000
    //     probe-rs download ../../cyw43-firmware/43439A0_clm.bin --binary-format bin --chip RP235x --base-address 0x10140000
    let fw = unsafe { core::slice::from_raw_parts(0x10100000 as *const u8, 230321) };
    let clm = unsafe { core::slice::from_raw_parts(0x10140000 as *const u8, 4752) };

    let pwr = Output::new(r.pwr, Level::Low);
    let cs = Output::new(r.cs, Level::High);
    let mut pio = Pio::new(r.pio, Irqs);
    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        RM2_CLOCK_DIVIDER,
        pio.irq0,
        cs,
        r.dio,
        r.clk,
        r.dma,
    );

    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;
    unwrap!(spawner.spawn(cyw43_task(runner)));

    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    // Use a link-local address for communication without an external DHCP server
    let config = Config::ipv4_static(embassy_net::StaticConfigV4 {
        address: embassy_net::Ipv4Cidr::new(embassy_net::Ipv4Address::new(192, 168, 1, 1), 16),
        dns_servers: heapless::Vec::new(),
        gateway: None,
    });

    // Generate random seed
    let seed = RoscRng.next_u64();

    // Init network stack
    static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
    let (stack, runner) = embassy_net::new(net_device, config, RESOURCES.init(StackResources::new()), seed);
    unwrap!(spawner.spawn(net_task(runner)));

    // Start a DCHP server
    unwrap!(spawner.spawn(dhcp_server_task(stack)));

    // Open an AP for the client
    control.start_ap_open("mote", 5).await;

    // Start the core tcp server
    unwrap!(spawner.spawn(tcp_server_task(stack)));
}
