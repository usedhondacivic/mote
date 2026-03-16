use alloc::vec::Vec;

use defmt::{error, info, warn};
use embassy_futures::select::{Either, select};
use embassy_net::Stack;
use embassy_net::udp::{PacketMetadata, SendError, UdpMetadata, UdpSocket};
use mote_api::HostLink;
use mote_api::messages::mote_to_host::BITResult;
use mote_api::messages::{host_to_mote, mote_to_host};

use crate::helpers::update_bit_result;
use crate::tasks::CONFIGURATION_STATE;
use crate::tasks::wifi::MOTE_TO_HOST_DATA_OFFLOAD;

pub const UDP_SERVER_PORT: u16 = 7475;

fn handle_command(rx_message: &host_to_mote::Message, link: &mut HostLink) {
    match rx_message {
        host_to_mote::Message::Ping => {
            info!("Parsed ping request, responding.");
            let _ = link.send(mote_to_host::Message::Pong);
        }
        host_to_mote::Message::Pong => {
            info!("Received ping response from host.");
        }
        _ => {
            error!("Received unhandled message type");
        }
    }
}

#[embassy_executor::task]
pub async fn udp_server_task(stack: Stack<'static>) -> ! {
    let mut rx_meta = [PacketMetadata::EMPTY; 16];
    let mut rx_buffer = [0; 4096];
    let mut tx_meta = [PacketMetadata::EMPTY; 16];
    let mut tx_buffer = [0; 4096];
    let mut socket = UdpSocket::new(stack, &mut rx_meta, &mut rx_buffer, &mut tx_meta, &mut tx_buffer);

    if let Err(e) = socket.bind(UDP_SERVER_PORT) {
        warn!("bind error: {:?}", e);
    }

    let mut link = HostLink::new();
    let mut message_buffer = [0; 4096];
    let mut subscribers: Vec<UdpMetadata> = Vec::new();
    let mut dead_indices: Vec<usize> = Vec::new();

    loop {
        match select(
            socket.recv_from(&mut message_buffer),
            MOTE_TO_HOST_DATA_OFFLOAD.receive(),
        )
        .await
        {
            Either::First(Ok((bytes_read, ep))) => {
                if !subscribers.contains(&ep) {
                    info!("Registering subscriber {}", ep);
                    subscribers.push(ep);
                    let mut configuration_state = CONFIGURATION_STATE.lock().await;
                    update_bit_result(
                        &mut configuration_state.built_in_test.wifi,
                        "Client Connected",
                        BITResult::Pass,
                    );
                }

                link.handle_receive(&message_buffer[..bytes_read]);
                while let Ok(Some(message)) = link.poll_receive() {
                    handle_command(&message, &mut link);
                }

                while let Some(payload) = link.poll_transmit() {
                    if let Err(err) = socket.send_to(&payload, ep).await {
                        error!("UDP send error: {}", err);
                    }
                }
            }
            Either::First(Err(err)) => {
                error!("UDP recv error: {}", err);
            }
            Either::Second(message) => {
                link.send(message).unwrap();

                while let Some(payload) = link.poll_transmit() {
                    for (idx, ep) in subscribers.iter().enumerate() {
                        match socket.send_to(&payload, *ep).await {
                            Ok(_) => {}
                            Err(SendError::NoRoute) => {
                                dead_indices.push(idx);
                                info!("Removing subscriber {}", ep);
                            }
                            Err(err) => error!("UDP send error: {}", err),
                        }
                    }
                }

                for idx in dead_indices.drain(..).rev() {
                    subscribers.remove(idx);
                }
                if subscribers.is_empty() {
                    let mut configuration_state = CONFIGURATION_STATE.lock().await;
                    update_bit_result(
                        &mut configuration_state.built_in_test.wifi,
                        "Client Connected",
                        BITResult::Waiting,
                    );
                }
            }
        }
    }
}
