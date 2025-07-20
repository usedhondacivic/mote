// Example usage of mote-host-driver

// This example publishes sensor data to rerun for visualization using a single thread
// For more complicated real integrations you'd most likely want to use a async runtime
// (tokio, smol) to handle socket io concurrently

use std::{
    net::{ToSocketAddrs, UdpSocket},
    time::Duration,
};

use rerun::{demo_util::grid, external::glam};

use anyhow::Context;
use mote_host_driver::MoteCommunication;

fn main() -> anyhow::Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:34254").unwrap();
    socket
        .set_read_timeout(Some(Duration::from_millis(500)))
        .unwrap();

    let mut comms = MoteCommunication::new();

    // Send one ping command
    let socket_addr = "192.168.1.1:1738"
        .to_socket_addrs()?
        .next()
        .context("Failed to create socket addr")?;
    comms.send(socket_addr, mote_messages::HostToMoteMessage::Ping)?;

    // Create the rerun instance
    let rec = rerun::RecordingStreamBuilder::new("mote_rerun_example").serve_grpc()?;

    // Setup rerun to launch the web view
    rerun::serve_web_viewer(rerun::web_viewer::WebViewerConfig {
        connect_to: Some("localhost/proxy".to_owned()),
        ..Default::default()
    })?
    .detach();

    loop {
        // Retrieve and transmit all messages queued to be sent
        while let Some(transmit) = comms.poll_transmit() {
            socket.send_to(&transmit.payload, transmit.dst)?;
            continue;
        }

        // Read a message from the socket
        let mut buf = vec![0u8; 1500];
        let (num_read, source) = socket.recv_from(&mut buf)?;
        println!("Received {} bytes from {}", num_read, source);

        let message = MoteCommunication::handle_recieve(&buf[..num_read]).unwrap();
        // Check what kind of message we got
        match message {
            mote_messages::MoteToHostMessage::PingResponse => {
                println!("Got ping response from Mote.");
            }
            mote_messages::MoteToHostMessage::Ping => {
                println!("Mote pinged PC.");
                comms.send(socket_addr, mote_messages::HostToMoteMessage::PingResponse)?;
            }
            mote_messages::MoteToHostMessage::Scan(scan_data) => {
                // We got a LiDAR scan message, lets push the points to rerun for visualization
                let points: heapless::Vec<
                    glam::Vec2,
                    { mote_messages::MAX_POINTS_PER_SCAN_MESSAGE },
                > = scan_data
                    .iter()
                    .map(|point| {
                        glam::Vec2::from_angle((point.angle as f32 / 64.0).to_radians())
                            * (point.distance as f32 * 4.0)
                            / 100.0
                    })
                    .collect();

                // let points = grid(glam::Vec3::splat(-10.0), glam::Vec3::splat(10.0), 10);
                let colors: heapless::Vec<
                    rerun::Color,
                    { mote_messages::MAX_POINTS_PER_SCAN_MESSAGE },
                > = points.iter().map(|point| rerun::Color::WHITE).collect();

                // rerun::Color::from_rgb(0, 255, 255));

                rec.log(
                    "my_points",
                    &rerun::Points2D::new(points)
                        .with_colors(colors)
                        .with_radii([1.0]),
                )?;
            }
        }
    }
}
