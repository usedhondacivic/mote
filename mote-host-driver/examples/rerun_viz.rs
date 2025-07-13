// Example usage of mote-host-driver

// This example publishes sensor data to rerun for visualization using a single thread
// For more complicated real integrations you'd most likely want to use a async runtime
// (tokio, smol) to handle socket io concurrently

use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpStream, ToSocketAddrs},
};

use anyhow::Context;
use mote_host_driver::MiteCommunication;

fn main() -> anyhow::Result<()> {
    let mut socket = TcpStream::connect("192.168.1.1:1738")?;

    let mut comms = MiteCommunication::new();

    comms.send(
        "192.168.1.1:1738"
            .to_socket_addrs()?
            .next()
            .context("Failed to create socket addr")?,
        mote_messages::HostToMoteMessage::Ping,
    )?;

    loop {
        if let Some(transmit) = comms.poll_transmit() {
            socket.write(&transmit.payload)?;
            continue;
        }

        let mut buf = vec![0u8; 1500];
        let num_read = socket.read_to_end(&mut buf)?;

        if let Ok(message) = MiteCommunication::handle_recieve(&buf[..num_read]) {
            dbg!("RXd: {:?}", message);
        }
    }
}
