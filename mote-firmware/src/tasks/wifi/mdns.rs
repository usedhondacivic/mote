use alloc::string::{String, ToString};
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
use embassy_rp::pac;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Ticker};
use mote_api::messages::mote_to_host::BITResult;
use {defmt_rtt as _, panic_probe as _};

use crate::helpers::update_bit_result;
use crate::tasks::CONFIGURATION_STATE;
use crate::tasks::wifi::udp_server::UDP_SERVER_PORT;

// TODO: Replace with embassy_rp::clocks::RoscRng once embassy-rp releases to
// include  https://github.com/embassy-rs/embassy/commit/d75598c7ebd25777f7a690e5e6c7ae5b17993139
struct RoscRnd;

impl RoscRnd {
    fn random_byte() -> u8 {
        let random_reg = pac::ROSC.randombit();
        let mut acc: u8 = 0;
        for _ in 0..8 {
            acc = (acc << 1) | random_reg.read().randombit() as u8;
        }
        acc
    }
}

impl rand_core::TryRng for RoscRnd {
    type Error = core::convert::Infallible;

    fn try_next_u32(&mut self) -> Result<u32, Self::Error> {
        let b = [
            Self::random_byte(),
            Self::random_byte(),
            Self::random_byte(),
            Self::random_byte(),
        ];
        Ok(u32::from_le_bytes(b))
    }

    fn try_next_u64(&mut self) -> Result<u64, Self::Error> {
        let lo = self.try_next_u32()? as u64;
        let hi = self.try_next_u32()? as u64;
        Ok((hi << 32) | lo)
    }

    fn try_fill_bytes(&mut self, dst: &mut [u8]) -> Result<(), Self::Error> {
        for byte in dst.iter_mut() {
            *byte = Self::random_byte();
        }
        Ok(())
    }
}

#[embassy_executor::task]
pub async fn mdns_task(stack: Stack<'static>) -> ! {
    // Wait for IPV4 to come up
    stack.wait_link_up().await;
    stack.wait_config_up().await;
    let ip = stack.config_v4().unwrap().address.address();
    let mut hostname: String;
    info!("Got ip: {}", ip);
    {
        let mut configuration_state = CONFIGURATION_STATE.lock().await;
        update_bit_result(&mut configuration_state.built_in_test.wifi, "IPV4 UP", BITResult::Pass);
        hostname = configuration_state.uid.clone();
        configuration_state.ip = Some(ip.to_string());
    }

    info!(
        "Running mDNS responder. It will be addressable using {}.local, so try to `ping {}.local`.",
        hostname, hostname
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

    let command_service = Service {
        name: "Mote Server",
        priority: 1,
        weight: 5,
        service: "_mote-api",
        protocol: "_udp",
        port: UDP_SERVER_PORT,
        service_subtypes: &[],
        txt_kvs: &[],
    };

    let signal: Signal<NoopRawMutex, ()> = Signal::new();

    let mdns = io::Mdns::new(
        Some(Ipv4Addr::UNSPECIFIED),
        None,
        recv,
        send,
        recv_buf,
        send_buf,
        RoscRnd,
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
        let host = Host {
            hostname: &hostname,
            ipv4: ip,
            ipv6: Ipv6Addr::UNSPECIFIED,
            ttl: Ttl::from_secs(60),
        };

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
                hostname = CONFIGURATION_STATE.lock().await.uid.clone();
                signal.signal(());
            }
        }
    }
}
