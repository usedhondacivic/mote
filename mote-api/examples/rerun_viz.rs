//! Example usage of mote-sansio-driver
//! This example publishes sensor data to rerun for visualization

use color_space::{Hsv, Rgb};
use rerun::external::glam;
use std::thread;
use std::{
    io::{Read, Write},
    net::{TcpStream, UdpSocket},
};

use mote_api::MoteLink;
use mote_api::messages::{host_to_mote, mote_to_host};

// This thread starts a TCP socket for sending commands to Mote
// Right now all we do is ping the robot
fn command_thread() {
    'socket_error: loop {
        if let Ok(mut socket) = TcpStream::connect("192.168.0.78:7465") {
            let mut command_link = MoteLink::new();

            // Ping the robot
            println!("Pinging Mote");
            command_link.send(host_to_mote::Message::Ping).unwrap();

            loop {
                // Retrieve and transmit all messages queued to be sent
                while let Some(transmit) = command_link.poll_transmit() {
                    if let Err(_) = socket.write_all(&transmit.payload) {
                        continue 'socket_error;
                    }
                    continue;
                }

                // Read a message from the socket
                let mut buf = vec![0u8; 2000];
                let num_read = socket.read(&mut buf).unwrap();
                command_link.handle_receive(&mut buf[..num_read]);

                while let Ok(Some(message)) = command_link.poll_receive() {
                    // Check what kind of message we got
                    match message {
                        mote_to_host::Message::Pong => {
                            println!("Got pong from Mote.");
                        }
                        mote_to_host::Message::Ping => {
                            println!("Mote pinged host.");
                            command_link.send(host_to_mote::Message::Pong).unwrap();
                        }
                        mote_to_host::Message::Scan(_) => todo!(),
                        mote_to_host::Message::State(_) => todo!(),
                    }
                }
            }
        }
    }
}

// This thread creates a UDP socket and subscribes to sensor data from Mote
// When data is received, it is processed and published to rerun for visualization
fn data_offload_thread() {
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

    'socket_error: loop {
        if let Ok(socket) = UdpSocket::bind("0.0.0.0:0") {
            if let Err(_) = socket.connect("192.168.0.78:7475") {
                continue;
            }

            let mut data_link = MoteLink::new();

            // Subscribe to sensor data
            data_link.send(host_to_mote::Message::Ping).unwrap();

            loop {
                // Retrieve and transmit all messages queued to be sent
                while let Some(transmit) = data_link.poll_transmit() {
                    if let Err(_) = socket.send(&transmit.payload) {
                        continue 'socket_error;
                    }
                    continue;
                }

                // Read a message from the socket
                let mut buf = vec![0u8; 2000];
                let num_read = socket.recv(&mut buf).unwrap();
                data_link.handle_receive(&mut buf[..num_read]);

                while let Ok(Some(message)) = data_link.poll_receive() {
                    // Check what kind of message we got
                    match message {
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
                                    return rerun::Color::from_rgb(
                                        rgb.r as u8,
                                        rgb.g as u8,
                                        rgb.b as u8,
                                    );
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
                        mote_to_host::Message::Ping => todo!(),
                        mote_to_host::Message::Pong => todo!(),
                        mote_to_host::Message::State(_) => todo!(),
                    }
                }
            }
        }
    }
}

fn main() {
    // Start the command thread
    let command_thread_handle = thread::spawn(|| {
        command_thread();
    });

    // Start the data offload thread
    data_offload_thread();

    command_thread_handle.join().unwrap();
}
