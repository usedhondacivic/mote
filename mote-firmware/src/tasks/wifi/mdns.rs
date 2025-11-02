use core::net::{Ipv4Addr, Ipv6Addr};

use defmt::*;
use edge_mdns::HostAnswersMdnsHandler;
use edge_mdns::buf::VecBufAccess;
use edge_mdns::domain::base::Ttl;
use edge_mdns::host::{Host, Service, ServiceAnswers};
use edge_mdns::io::{self, IPV4_DEFAULT_SOCKET};
use edge_nal::UdpSplit;
use edge_nal_embassy::{Udp, UdpBuffers};
use embassy_futures::select::{Either, select};
use embassy_net::Stack;
use embassy_rp::clocks::RoscRng;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Ticker};
use mote_messages::configuration::mote_to_host::BITResult;
use {defmt_rtt as _, panic_probe as _};

use crate::helpers::update_bit_result;
use crate::tasks::CONFIGURATION_STATE;
use crate::tasks::wifi::tcp_server::TCP_SERVER_PORT;

#[embassy_executor::task]
pub async fn mdns_task(stack: Stack<'static>) -> ! {
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

    let command_service = Service {
        name: "Mote Command Server",
        priority: 1,
        weight: 5,
        service: "_mote",
        protocol: "_tcp",
        port: TCP_SERVER_PORT,
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
            mdns.run(HostAnswersMdnsHandler::new(ServiceAnswers::new(
                &host,
                &command_service,
            ))),
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
