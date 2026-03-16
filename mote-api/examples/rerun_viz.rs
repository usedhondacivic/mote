//! Example usage of mote-sansio-driver
//! This example publishes sensor data to rerun for visualization

use color_space::{Hsv, Rgb};
use rerun::external::glam;
use std::net::UdpSocket;

use mote_api::MoteLink;
use mote_api::messages::{host_to_mote, mote_to_host};

fn main() {
    // Create the rerun instance
    let rec = rerun::RecordingStreamBuilder::new("mote_rerun_example")
        .serve_grpc()
        .unwrap();

    // Setup rerun to launch the web view
    rerun::serve_web_viewer(rerun::web_viewer::WebViewerConfig {
        connect_to: Vec::from(["localhost/proxy".to_owned()]),
        ..Default::default()
    })
    .unwrap()
    .detach();

    // Both commands and data use the same UDP socket
    'socket_error: loop {
        if let Ok(socket) = UdpSocket::bind("0.0.0.0:0") {
            if let Err(_) = socket.connect("192.168.0.78:7475") {
                continue;
            }

            let mut link = MoteLink::new();

            // Ping the robot
            println!("Pinging Mote");
            link.send(host_to_mote::Message::Ping).unwrap();

            loop {
                // Retrieve and transmit all messages queued to be sent
                while let Some(transmit) = link.poll_transmit() {
                    if let Err(_) = socket.send(&transmit.payload) {
                        continue 'socket_error;
                    }
                    continue;
                }

                // Read a message from the socket
                let mut buf = vec![0u8; 2000];
                let num_read = socket.recv(&mut buf).unwrap();
                link.handle_receive(&mut buf[..num_read]);

                while let Ok(Some(message)) = link.poll_receive() {
                    match message {
                        mote_to_host::Message::Pong => {
                            println!("Got pong from Mote.");
                        }
                        mote_to_host::Message::Ping => {
                            println!("Mote pinged host.");
                            link.send(host_to_mote::Message::Pong).unwrap();
                        }
                        mote_to_host::Message::Scan(scan_data) => {
                            // We got a LiDAR scan message, lets push the points to rerun for visualization
                            let points: Vec<glam::Vec2> = scan_data
                                .iter()
                                .map(|point| {
                                    glam::Vec2::from_angle(point.angle_rads) * point.distance_mm
                                })
                                .collect();

                            let colors: Vec<rerun::Color> = scan_data
                                .iter()
                                .map(|point| {
                                    let rgb = Rgb::from(Hsv::new(
                                        point.distance_mm as f64 / 20.0,
                                        1.0,
                                        1.0,
                                    ));
                                    rerun::Color::from_rgb(rgb.r as u8, rgb.g as u8, rgb.b as u8)
                                })
                                .collect();

                            rec.log(
                                "lidar_scan",
                                &rerun::Points2D::new(points)
                                    .with_colors(colors)
                                    .with_radii([10.0]),
                            )
                            .unwrap();
                        }
                        mote_to_host::Message::State(_) => todo!(),
                    }
                }
            }
        }
    }
}
