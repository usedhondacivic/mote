use core::net::Ipv4Addr;

use defmt::*;
use embassy_futures::select::{Either, select};
use embassy_net::tcp::TcpSocket;
use embassy_net::{IpAddress, Stack};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embedded_io_async::Write;
use mote_messages::configuration::mote_to_host::BITResult;
use mote_messages::runtime::{host_to_mote, mote_to_host};
use mote_sansio_driver::MoteRuntimeCommandLink;
use {defmt_rtt as _, panic_probe as _};

use crate::helpers::update_bit_result;
use crate::tasks::CONFIGURATION_STATE;

pub const TCP_SERVER_PORT: u16 = 7465;
pub static MOTE_TO_HOST_COMMAND: Channel<CriticalSectionRawMutex, mote_to_host::command::Message, 32> = Channel::new();

async fn parse_command_message<'a>(
    rx_message: &host_to_mote::Message,
    endpoint_addr: Ipv4Addr,
    link: &mut MoteRuntimeCommandLink,
) -> Result<(), embassy_net::tcp::Error> {
    match rx_message {
        host_to_mote::Message::Ping => {
            info!("Parsed ping request, responding.");
            let _ = link.send(endpoint_addr, mote_to_host::command::Message::PingResponse);
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
pub async fn tcp_server_task(stack: Stack<'static>) -> ! {
    loop {
        let mut rx_buffer = [0; 4096];
        let mut tx_buffer = [0; 4096];

        loop {
            // Update client connection status
            {
                let mut configuration_state = CONFIGURATION_STATE.lock().await;
                update_bit_result(
                    &mut configuration_state.built_in_test.wifi,
                    "Client Connected",
                    BITResult::Waiting,
                );
            }

            let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);

            if let Err(e) = socket.accept(TCP_SERVER_PORT).await {
                warn!("bind error: {:?}", e);
                continue;
            }

            info!("Received connection from {:?}", socket.remote_endpoint());

            {
                let mut configuration_state = CONFIGURATION_STATE.lock().await;
                update_bit_result(
                    &mut configuration_state.built_in_test.wifi,
                    "Client Connected",
                    BITResult::Pass,
                );
            }

            let mut message_buffer = [0; 4096];

            let mut link = MoteRuntimeCommandLink::new();

            loop {
                match select(socket.read(&mut message_buffer), MOTE_TO_HOST_COMMAND.receive()).await {
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
                        while let Ok(Some(message)) = link.poll_receive() {
                            if let Some(endpoint) = socket.remote_endpoint() {
                                if let IpAddress::Ipv4(ip) = endpoint.addr {
                                    parse_command_message(&message, ip, &mut link).await.unwrap();
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

                while let Some(transmit) = link.poll_transmit() {
                    if let Err(error) = socket.write_all(&transmit.payload).await {
                        error!("TX message failed, got {:?}", error);
                        break;
                    }
                }
            }
        }
    }
}
