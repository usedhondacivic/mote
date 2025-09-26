use core::cmp::min;
use core::net::{IpAddr, Ipv4Addr, SocketAddr};

use cyw43::JoinOptions;
use cyw43_pio::{PioSpi, RM2_CLOCK_DIVIDER};
use defmt::*;
use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_net::Stack;
use embassy_net::udp::{PacketMetadata, UdpMetadata, UdpSocket};
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, PIO0};
use embassy_rp::pio::Pio;
use mote_messages::configuration::mote_to_host::{BIT, BITResult, NetworkConnection};
use mote_messages::runtime::{host_to_mote, mote_to_host};
use mote_sansio_driver::MoteRuntimeLink;
use postcard::to_vec;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

use super::{Cyw43Resources, Irqs};
use crate::helpers::update_bit_result;
use crate::tasks::{CONFIGURATION_STATE, MOTE_TO_HOST};

const SERVER_PORT: u16 = 1738;

async fn run_network_scan() {}

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

        let mut link = MoteRuntimeLink::new();

        loop {
            match select(socket.recv_from(&mut message_buffer), MOTE_TO_HOST.receive()).await {
                Either::First(Ok((bytes_read, ep))) => {
                    info!("Read {} bytes from {}.", bytes_read, ep);
                    if let Ok(Some(message)) = link.handle_receive(&mut message_buffer[..bytes_read]) {
                        parse_message(&message, ep, &mut socket).await.unwrap();
                        endpoint = Some(ep);
                    }
                }
                Either::First(Err(err)) => error!("TCP server received error {}", err),
                Either::Second(tx_message) => {
                    link.send(
                        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080),
                        tx_message,
                    )
                    .unwrap();
                }
            }

            if let Some(ep) = endpoint {
                if let Some(transmit) = link.poll_transmit() {
                    if let Err(error) = socket.send_to(&transmit.payload, ep).await {
                        error!("TX message failed, got {:?}", error);
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

pub async fn init(spawner: Spawner, r: Cyw43Resources) {
    // Init BIT
    {
        let mut configuration_state = CONFIGURATION_STATE.lock().await;
        let init = BIT {
            name: heapless::String::try_from("Init").expect("Failed to assign name to BIT"),
            result: BITResult::Waiting,
        };
        let connection = BIT {
            name: heapless::String::try_from("Connected to Network").expect("Failed to assign name to BIT"),
            result: BITResult::Waiting,
        };
        let tcp = BIT {
            name: heapless::String::try_from("TCP UP").expect("Failed to assign name to BIT"),
            result: BITResult::Waiting,
        };
        let multicast = BIT {
            name: heapless::String::try_from("UDP Multicast UP").expect("Failed to assign name to BIT"),
            result: BITResult::Waiting,
        };
        let client = BIT {
            name: heapless::String::try_from("Client Connected").expect("Failed to assign name to BIT"),
            result: BITResult::Waiting,
        };
        for test in [init, connection, tcp, multicast, client] {
            configuration_state
                .built_in_test
                .wifi
                .push(test)
                .expect("Failed to add test");
        }
    }

    // let fw = include_bytes!("../cyw43-firmware/43439A0.bin");
    // let clm = include_bytes!("../cyw43-firmware/43439A0_clm.bin");

    // To make flashing faster for development, you may want to flash the firmwares
    // independently at hardcoded addresses, instead of baking them into the
    // program with `include_bytes!`:     probe-rs download
    // ../../cyw43-firmware/43439A0.bin --binary-format bin --chip RP235x
    // --base-address 0x10100000     probe-rs download
    // ../../cyw43-firmware/43439A0_clm.bin --binary-format bin --chip RP235x
    // --base-address 0x10140000
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

    // Update current network
    {
        let mut configuration_state = CONFIGURATION_STATE.lock().await;
        update_bit_result(&mut configuration_state.built_in_test.wifi, "Init", BITResult::Pass);
    }

    while let Err(err) = control
        .join("<network ssid>", JoinOptions::new("<network password>".as_bytes()))
        .await
    {
        info!("join failed with status={}", err.status);
    }

    // Update current network
    {
        let mut configuration_state = CONFIGURATION_STATE.lock().await;
        configuration_state.current_network_connection =
            Some(heapless::String::try_from("pew pew zing balm pop pew splat").expect(""));
        update_bit_result(
            &mut configuration_state.built_in_test.wifi,
            "Connected to Network",
            BITResult::Pass,
        );
    }

    let mut scanner = control.scan(Default::default()).await;
    while let Some(bss) = scanner.next().await {
        if let Ok(ssid_str) = str::from_utf8(&bss.ssid) {
            info!("scanned {} == {:x} -- {}", ssid_str, bss.bssid, bss.ssid);
            if bss.ssid.iter().all(|&n| n == 0) {
                continue;
            }

            // Update available networks
            {
                let mut configuration_state = CONFIGURATION_STATE.lock().await;

                let new_connection = NetworkConnection {
                    ssid: heapless::String::try_from(ssid_str)
                        .expect("Failed to create SSID from the value returned by the scan."),
                    strength: -bss.rssi as u8,
                };

                // Check if this network is already listed
                if let Some(item) = configuration_state
                    .available_network_connections
                    .iter_mut()
                    .find(|i| i.ssid == new_connection.ssid)
                {
                    item.strength = min(item.strength, new_connection.strength);
                    continue;
                }

                // If we've run out of entries, drop the weakest
                if configuration_state.available_network_connections.is_full() {
                    let (weakest_index, weakest) = configuration_state
                        .available_network_connections
                        .iter()
                        .enumerate()
                        .max_by_key(|&(_index, val)| val.strength)
                        .unwrap();

                    if weakest.strength > new_connection.strength {
                        configuration_state.available_network_connections.remove(weakest_index);
                    }
                }

                let _ = configuration_state.available_network_connections.push(new_connection);
            }
        }
    }

    // let config = Config::dhcpv4(Default::default());
    //
    // // Generate random seed
    // let seed = RoscRng.next_u64();
    //
    // // Init network stack
    // static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
    // let (stack, runner) = embassy_net::new(net_device, config,
    // RESOURCES.init(StackResources::new()), seed); unwrap!(spawner.
    // spawn(net_task(runner)));
    //
    // // Start the core tcp server
    // unwrap!(spawner.spawn(tcp_server_task(stack)));
    //
    // // Update init state
    // {
    //     let mut configuration_state = CONFIGURATION_STATE.lock().await;
    //     update_bit_result(&mut configuration_state.built_in_test.wifi,
    // "Init", BITResult::Pass); }
}
