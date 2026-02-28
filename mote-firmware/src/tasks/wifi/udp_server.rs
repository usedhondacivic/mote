use alloc::vec::Vec;

use defmt::{error, info, warn};
use embassy_futures::select::select;
use embassy_net::Stack;
use embassy_net::udp::{PacketMetadata, SendError, UdpMetadata, UdpSocket};
use mote_api::HostLink;

use crate::tasks::wifi::MOTE_TO_HOST_DATA_OFFLOAD;

const UDP_SERVER_PORT: u16 = 7475;

#[embassy_executor::task]
pub async fn udp_server_task(stack: Stack<'static>) -> ! {
    let mut data_offload_subscribers: Vec<UdpMetadata> = Vec::new();
    let mut dead_connections: Vec<usize> = Vec::new();

    loop {
        let mut rx_buffer = [0; 4096];
        let mut tx_buffer = [0; 4096];
        let mut rx_meta = [PacketMetadata::EMPTY; 16];
        let mut tx_meta = [PacketMetadata::EMPTY; 16];
        let mut socket = UdpSocket::new(stack, &mut rx_meta, &mut rx_buffer, &mut tx_meta, &mut tx_buffer);

        if let Err(e) = socket.bind(UDP_SERVER_PORT) {
            warn!("bind error: {:?}", e);
            continue;
        }

        let mut link = HostLink::new();

        let mut message_buffer = [0; 4096];

        loop {
            match select(
                socket.recv_from(&mut message_buffer),
                MOTE_TO_HOST_DATA_OFFLOAD.receive(),
            )
            .await
            {
                embassy_futures::select::Either::First(Ok((_, ep))) => {
                    info!("Registering data offload subscriber {}", ep);

                    if !data_offload_subscribers.iter().any(|&i| ep == i) {
                        data_offload_subscribers.push(ep)
                    }
                }
                embassy_futures::select::Either::First(Err(err)) => {
                    error!("Udp server got error: {}", err);
                }
                embassy_futures::select::Either::Second(message) => {
                    link.send(message).unwrap();
                }
            }

            while let Some(transmit) = link.poll_transmit() {
                for (idx, ep) in data_offload_subscribers.iter().enumerate() {
                    match socket.send_to(&transmit.payload, *ep).await {
                        Ok(_) => continue,
                        Err(SendError::NoRoute) => {
                            dead_connections.push(idx);
                            info!("Removing data offload subscriber {}", ep)
                        }
                        Err(err) => error!("UDP server got error: {}", err),
                    }
                }
                for idx in &dead_connections {
                    data_offload_subscribers.remove(*idx);
                }
                dead_connections.clear();
            }
        }
    }
}
