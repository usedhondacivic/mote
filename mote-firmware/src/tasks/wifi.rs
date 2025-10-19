use core::cmp::min;
use core::net::{Ipv4Addr, Ipv6Addr};

use cyw43::JoinOptions;
use cyw43_pio::{PioSpi, RM2_CLOCK_DIVIDER};
use defmt::*;
use edge_mdns::HostAnswersMdnsHandler;
use edge_mdns::buf::VecBufAccess;
use edge_mdns::domain::base::Ttl;
use edge_mdns::host::{Host, Service, ServiceAnswers};
use edge_mdns::io::{self, IPV4_DEFAULT_SOCKET};
use edge_nal::UdpSplit;
use edge_nal_embassy::{Udp, UdpBuffers};
use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_net::tcp::TcpSocket;
use embassy_net::{Config, IpAddress, Stack, StackResources};
use embassy_rp::clocks::RoscRng;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, PIO0};
use embassy_rp::pio::Pio;
use embassy_sync::blocking_mutex::raw::{CriticalSectionRawMutex, NoopRawMutex};
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Ticker};
use embedded_io_async::Write;
use mote_messages::configuration::host_to_mote::SetNetworkConnectionConfig;
use mote_messages::configuration::mote_to_host::{BIT, BITResult, NetworkConnection};
use mote_messages::runtime::{host_to_mote, mote_to_host};
use mote_sansio_driver::MoteRuntimeLink;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

use super::{Cyw43Resources, Irqs};
use crate::helpers::update_bit_result;
use crate::tasks::CONFIGURATION_STATE;

const SERVER_PORT: u16 = 1738;

pub static MOTE_TO_HOST: Channel<CriticalSectionRawMutex, mote_messages::runtime::mote_to_host::Message, 32> =
    Channel::new();
pub static WIFI_REQUEST_CONNECT: Channel<
    CriticalSectionRawMutex,
    mote_messages::configuration::host_to_mote::SetNetworkConnectionConfig,
    1,
> = Channel::new();

pub static WIFI_REQUEST_RESCAN: Signal<CriticalSectionRawMutex, ()> = Signal::new();

async fn attempt_join_network<'a>(
    control: &mut cyw43::Control<'a>,
    config: mote_messages::configuration::host_to_mote::SetNetworkConnectionConfig,
) {
    // TODO: Read network ssid and password from flash

    for attempt in 1..6 {
        if let Err(err) = control.join("hi", JoinOptions::new("whats up hello".as_bytes())).await {
            info!("join failed with status={}, attempt {} / 5", err.status, attempt);
        } else {
            let mut configuration_state = CONFIGURATION_STATE.lock().await;
            configuration_state.current_network_connection = Some(
                heapless::String::try_from(
                    "TODO:
    placeholder",
                )
                .expect("Failed to create string from network ID"),
            );
            update_bit_result(
                &mut configuration_state.built_in_test.wifi,
                "Connected to Network",
                BITResult::Pass,
            );
            return;
        }
    }
    {
        let mut configuration_state = CONFIGURATION_STATE.lock().await;
        configuration_state.current_network_connection =
            Some(heapless::String::try_from("").expect("Failed to create string from network ID"));
        update_bit_result(
            &mut configuration_state.built_in_test.wifi,
            "Connected to Network",
            BITResult::Fail,
        );
    }
}

async fn run_network_scan<'a>(control: &mut cyw43::Control<'a>) {
    // Clear previous scan
    {
        let mut configuration_state = CONFIGURATION_STATE.lock().await;
        configuration_state.available_network_connections.clear();
    }

    let mut scanner = control.scan(Default::default()).await;
    while let Some(bss) = scanner.next().await {
        if let Ok(ssid_str) = str::from_utf8(&bss.ssid) {
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
}

async fn parse_message<'a>(
    rx_message: &host_to_mote::Message,
    endpoint_addr: Ipv4Addr,
    link: &mut MoteRuntimeLink,
) -> Result<(), embassy_net::tcp::Error> {
    match rx_message {
        host_to_mote::Message::Ping => {
            info!("Parsed ping request, responding.");
            let _ = link.send(endpoint_addr, mote_to_host::Message::PingResponse);
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
        let mut rx_buffer = [0; 4096];
        let mut tx_buffer = [0; 4096];

        loop {
            let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);

            if let Err(e) = socket.accept(SERVER_PORT).await {
                warn!("bind error: {:?}", e);
                continue;
            }

            info!("Received connection from {:?}", socket.remote_endpoint());

            // Update client connection status
            {
                let mut configuration_state = CONFIGURATION_STATE.lock().await;
                update_bit_result(
                    &mut configuration_state.built_in_test.wifi,
                    "Client Connected",
                    BITResult::Pass,
                );
            }

            let mut message_buffer = [0; 4096];

            let mut link = MoteRuntimeLink::new();

            loop {
                match select(socket.read(&mut message_buffer), MOTE_TO_HOST.receive()).await {
                    Either::First(Ok(0)) => {
                        info!("Socket connection closed");
                        // Update client connection status
                        {
                            let mut configuration_state = CONFIGURATION_STATE.lock().await;
                            update_bit_result(
                                &mut configuration_state.built_in_test.wifi,
                                "Client Connected",
                                BITResult::Waiting,
                            );
                        }
                        break;
                    }
                    Either::First(Ok(bytes_read)) => {
                        link.handle_receive(&mut message_buffer[..bytes_read]);
                        if let Ok(Some(message)) = link.poll_receive() {
                            if let Some(endpoint) = socket.remote_endpoint() {
                                if let IpAddress::Ipv4(ip) = endpoint.addr {
                                    parse_message(&message, ip, &mut link).await.unwrap();
                                }
                            }
                        }
                    }
                    Either::First(Err(embassy_net::tcp::Error::ConnectionReset)) => {
                        break;
                    }
                    Either::Second(tx_message) => {
                        if let Some(endpoint) = socket.remote_endpoint() {
                            if let IpAddress::Ipv4(ip) = endpoint.addr {
                                link.send(ip, tx_message).unwrap();
                            }
                        }
                    }
                }

                if let Some(transmit) = link.poll_transmit() {
                    if let Err(error) = socket.write_all(&transmit.payload).await {
                        error!("TX message failed, got {:?}", error);
                        break;
                    }
                }
            }
        }
    }
}

#[embassy_executor::task]
async fn mdns_task(stack: Stack<'static>) -> ! {
    // Wait for IPV4 to come up
    stack.wait_config_up().await;
    {
        let mut configuration_state = CONFIGURATION_STATE.lock().await;
        update_bit_result(&mut configuration_state.built_in_test.wifi, "IPV4 UP", BITResult::Pass);
    }

    let ip = stack.config_v4().unwrap().address.address();
    info!("Got ip: {}", ip);

    info!(
        "Running mDNS responder. It will be addressable using {}.local, so try to `ping {}.local`.",
        "mote", "mote"
    );

    let udp_buffers = UdpBuffers::<4, 1500, 1500, 2>::new();
    let udp_stack = Udp::new(stack, &udp_buffers);

    let mut socket = io::bind(&udp_stack, IPV4_DEFAULT_SOCKET, Some(Ipv4Addr::UNSPECIFIED), None)
        .await
        .unwrap();

    let (recv_buf, send_buf) = (
        VecBufAccess::<NoopRawMutex, 1500>::new(),
        VecBufAccess::<NoopRawMutex, 1500>::new(),
    );

    let (recv, send) = socket.split();

    let host = Host {
        hostname: "mote",
        ipv4: ip,
        ipv6: Ipv6Addr::UNSPECIFIED,
        ttl: Ttl::from_secs(60),
    };

    let service = Service {
        name: "Mote Telemetry Stream",
        priority: 1,
        weight: 5,
        service: "_mote",
        protocol: "_tcp",
        port: SERVER_PORT,
        service_subtypes: &[],
        txt_kvs: &[],
    };

    let signal = Signal::new();

    let mdns = io::Mdns::<NoopRawMutex, _, _, _, _>::new(
        Some(Ipv4Addr::UNSPECIFIED),
        None,
        recv,
        send,
        recv_buf,
        send_buf,
        |buf| RoscRng.fill_bytes(buf),
        &signal,
    );

    // Periodic timer for refreshing
    let mut ticker = Ticker::every(Duration::from_secs(15));

    // Update mdns status
    {
        let mut configuration_state = CONFIGURATION_STATE.lock().await;
        update_bit_result(&mut configuration_state.built_in_test.wifi, "mDNS UP", BITResult::Pass);
    }

    loop {
        match select(
            mdns.run(HostAnswersMdnsHandler::new(ServiceAnswers::new(&host, &service))),
            ticker.next(),
        )
        .await
        {
            Either::First(Ok(())) => {}
            Either::First(Err(e)) => {
                warn!("mDNS exited with error: {:?}", e);
            }
            Either::Second(_) => {
                signal.signal(());
            }
        }
    }
}

#[embassy_executor::task]
async fn connection_manager_task(mut control: cyw43::Control<'static>) -> ! {
    // Populate network scan state
    run_network_scan(&mut control).await;

    // Attempt to join whatever network is saved in flash
    // TODO: Load config from flash, then attempt connect
    // attempt_join_network(&mut control).await;

    loop {
        match select(WIFI_REQUEST_CONNECT.receive(), WIFI_REQUEST_RESCAN.wait()).await {
            Either::First(config) => {
                info!("Got join request {}, {}", config.ssid, config.password);
                attempt_join_network(&mut control, config).await;
            }
            Either::Second(_) => run_network_scan(&mut control).await,
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
        let ip_v4 = BIT {
            name: heapless::String::try_from("IPV4 UP").expect("Failed to assign name to BIT"),
            result: BITResult::Waiting,
        };
        let multicast = BIT {
            name: heapless::String::try_from("mDNS UP").expect("Failed to assign name to BIT"),
            result: BITResult::Waiting,
        };
        let client = BIT {
            name: heapless::String::try_from("Client Connected").expect("Failed to assign name to BIT"),
            result: BITResult::Waiting,
        };
        for test in [init, connection, ip_v4, multicast, client] {
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

    // Update init state
    {
        let mut configuration_state = CONFIGURATION_STATE.lock().await;
        update_bit_result(&mut configuration_state.built_in_test.wifi, "Init", BITResult::Pass);
    }

    let config = Config::dhcpv4(Default::default());

    // Generate random seed
    let seed = RoscRng.next_u64();

    // Init network stack
    static RESOURCES: StaticCell<StackResources<5>> = StaticCell::new();
    let (stack, runner) = embassy_net::new(net_device, config, RESOURCES.init(StackResources::new()), seed);

    // Start connection manager task
    unwrap!(spawner.spawn(connection_manager_task(control)));

    // Start network task
    unwrap!(spawner.spawn(net_task(runner)));

    // Start the core tcp server
    unwrap!(spawner.spawn(tcp_server_task(stack)));

    // Start mdns responder
    unwrap!(spawner.spawn(mdns_task(stack)));
}
