// Example usage of mote-sansio-driver

// This example publishes sensor data to rerun for visualization using a single thread
// For more complicated real integrations you'd most likely want to use a async runtime
// (tokio, smol) to handle socket io concurrently

use color_space::{Hsv, Rgb};
use rerun::external::glam;
use std::{
    io::{Read, Write},
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream},
    time::Duration,
};

use mote_sansio_driver::HostRuntimeLink;

use mote_messages::runtime::{host_to_mote, mote_to_host};

#[async_std::main]
async fn main() -> anyhow::Result<()> {
    // By default this example will use mDNS to discover your robot.
    // If mDNS is not available (most public networks), use the configuration page to find your robot's
    // IP and insert it here
    // let mote_ip = Ipv4Addr::UNSPECIFIED;
    // let port = 1738;
    //
    // if mote_ip == Ipv4Addr::UNSPECIFIED {
    //     println!(
    //         "{:?}",
    //         mdns::resolve::one(
    //             "_mote._tcp.local.",
    //             "mote._mote._tcp.local.",
    //             Duration::from_secs(15)
    //         )
    //         .await
    //     );
    //     // if let Ok(Some(response)) = mdns::resolve::one(
    //     //     "_mote._tcp.local",
    //     //     "mote._mote._tcp.local",
    //     //     Duration::from_secs(15),
    //     // )
    //     // .await
    //     // {
    //     //     println!("{:?}", response);
    //     // }
    // }

    // let socket_addr = SocketAddr::new(IpAddr::V4(mote_ip), port);

    let mut socket = TcpStream::connect("192.168.7.64:1738").unwrap();
    // socket.set_read_timeout(Some(Duration::from_millis(2500)))?;

    let mut comms = HostRuntimeLink::new();

    // Send one ping command
    comms.send("192.168.7.64".parse().unwrap(), host_to_mote::Message::Ping)?;

    // Create the rerun instance
    let rec = rerun::RecordingStreamBuilder::new("mote_rerun_example").serve_grpc()?;

    // Setup rerun to launch the web view
    rerun::serve_web_viewer(rerun::web_viewer::WebViewerConfig {
        connect_to: Vec::from(["localhost/proxy".to_owned()]),
        ..Default::default()
    })?
    .detach();

    loop {
        // Retrieve and transmit all messages queued to be sent
        while let Some(transmit) = comms.poll_transmit() {
            socket.write_all(&transmit.payload)?;
            continue;
        }

        // Read a message from the socket
        let mut buf = vec![0u8; 2000];
        let num_read = socket.read(&mut buf)?;
        comms.handle_receive(&mut buf[..num_read]);

        while let Ok(Some(message)) = comms.poll_receive() {
            // Check what kind of message we got
            match message {
                mote_to_host::Message::PingResponse => {
                    println!("Got ping response from Mote.");
                }
                mote_to_host::Message::Ping => {
                    println!("Mote pinged PC.");
                    comms.send(
                        "192.168.0.78".parse().unwrap(),
                        host_to_mote::Message::PingResponse,
                    )?;
                }
                mote_to_host::Message::Scan(scan_data) => {
                    // We got a LiDAR scan message, lets push the points to rerun for visualization
                    let points: heapless::Vec<
                        glam::Vec2,
                        { mote_messages::runtime::mote_to_host::MAX_POINTS_PER_SCAN_MESSAGE },
                    > = scan_data
                        .iter()
                        .map(|point| {
                            glam::Vec2::from_angle((point.angle as f32 / 64.0).to_radians())
                                * (point.distance as f32 * 4.0)
                                / 100.0
                        })
                        .collect();

                    let colors: Vec<rerun::Color> = scan_data
                        .iter()
                        .map(|point| {
                            let rgb = Rgb::from(Hsv::new((point.distance as f64) / 40.0, 1.0, 1.0));
                            return rerun::Color::from_rgb(rgb.r as u8, rgb.g as u8, rgb.b as u8);
                        })
                        .collect();

                    rec.log(
                        "lidar_scan",
                        &rerun::Points2D::new(points)
                            .with_colors(colors)
                            .with_radii([1.0]),
                    )?;
                }
            }
        }
    }
}
