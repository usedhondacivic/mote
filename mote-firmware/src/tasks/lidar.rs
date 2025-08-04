use cortex_m::prelude::_embedded_hal_blocking_serial_Write;
use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::uart::{Config, DataBits, Parity, StopBits, Uart};
use embassy_time::{Duration, TimeoutError, Timer, with_timeout};
use mote_messages::{MAX_POINTS_PER_SCAN_MESSAGE, Point};

use super::{Irqs, RplidarC1Resources};
use crate::tasks::MOTE_TO_HOST;

enum LidarState {
    Start,
    Reset,
    CheckHealth,
    ScanRequest,
    ReceiveSample,
    ReceiveTimeout,
    ProcessSample,
    Stop,
}

#[embassy_executor::task]
async fn lidar_state_machine_task(r: RplidarC1Resources) {
    let mut config = Config::default();
    config.baudrate = 460800;
    config.stop_bits = StopBits::STOP1;
    config.data_bits = DataBits::DataBits8;
    config.parity = Parity::ParityNone;

    let mut uart = Uart::new(r.uart, r.tx, r.rx, Irqs, r.tx_dma, r.rx_dma, config);

    let mut state = LidarState::Reset;

    let mut scan_points = heapless::Vec::<Point, MAX_POINTS_PER_SCAN_MESSAGE>::new();

    loop {
        state = match state {
            LidarState::Start => LidarState::Reset,
            LidarState::Reset => {
                let mut next_state = LidarState::Reset;
                let data = [0xA5, 0x40];
                let mut resp = [0; 1];
                match uart.write(&data).await {
                    Ok(()) => {
                        // Delay to give the LiDAR time to reboot
                        Timer::after_millis(1000).await;
                        // Clear the UART buffer
                        while !with_timeout(Duration::from_millis(100), uart.read(&mut resp))
                            .await
                            .is_err()
                        {}
                        Timer::after_millis(1000).await;
                        next_state = LidarState::CheckHealth;
                    }
                    Err(err) => {
                        // Otherwise we have an error, attempt to reset again after a short delay
                        error!("Failed to send RESET command to LiDAR ({}), retrying...", err);
                        Timer::after_millis(1000).await;
                    }
                }
                next_state
            }
            LidarState::CheckHealth => {
                let mut next_state = LidarState::Reset;
                let data = [0xA5u8, 0x52];
                let mut resp = [0; 10];
                match uart.write(&data).await {
                    Ok(()) => match uart.read(&mut resp).await {
                        Ok(()) => {
                            if resp[0..7] == [0xA5, 0x5A, 0x03, 0x00, 0x00, 0x00, 0x06] {
                                match resp[7] {
                                    0x00 => next_state = LidarState::ScanRequest,
                                    status => {
                                        error!(
                                            "LiDAR GET_HEALTH returned status code {} and error code {}",
                                            status,
                                            u16::from_le_bytes(resp[7..10].try_into().unwrap_or([0x00, 0x00]))
                                        );
                                    }
                                }
                            } else {
                                error!(
                                    "LiDAR returned incorrect response to GET_HEALTH message ({:#x}), reseting...",
                                    resp
                                );
                            }
                        }
                        Err(err) => {
                            error!("Failed to read GET_HEALTH response from LiDAR ({}), reseting...", err);
                        }
                    },
                    Err(err) => {
                        error!("Failed to send GET_HEALTH command to LiDAR ({}), reseting...", err);
                    }
                }
                next_state
            }
            LidarState::ScanRequest => {
                let mut next_state = LidarState::Reset;
                let data = [0xA5, 0x20];
                let mut resp = [0; 7];
                match uart.write(&data).await {
                    Ok(()) => match uart.read(&mut resp).await {
                        Ok(()) => {
                            if resp == [0xA5, 0x5A, 0x05, 0x00, 0x00, 0x40, 0x81] {
                                next_state = LidarState::ReceiveSample
                            } else {
                                error!(
                                    "LiDAR returned incorrect response to START_SCAN message ({:#x}), reseting...",
                                    resp
                                );
                            }
                        }
                        Err(err) => {
                            error!("Failed to read START_SCAN response from LiDAR ({}), reseting...", err);
                        }
                    },
                    Err(err) => {
                        error!("Failed to send START_SCAN command to LiDAR ({}), reseting...", err);
                    }
                }
                next_state
            }
            LidarState::ReceiveSample => {
                let mut next_state = LidarState::Reset;
                let mut resp = [0; 5];
                match with_timeout(Duration::from_millis(5000), uart.read(&mut resp)).await {
                    Ok(Ok(())) => {
                        if resp[0] & 0b01 == resp[0] & 0b10 {
                            error!("Start flag data check failed for LiDAR data message. reseting...");
                        } else if resp[1] & 0b1 != 1 {
                            error!("Check bit data check failed for LiDAR data message. reseting...");
                        } else {
                            let angle = ((resp[2] as u16) << 7) | ((resp[1] as u16 & 0xFE) >> 1);
                            if let Ok(()) = scan_points.push(Point {
                                quality: (resp[0] & !0b11) >> 2,
                                angle: angle,
                                distance: u16::from_le_bytes(resp[3..5].try_into().unwrap_or([0x00, 0x00])),
                            }) {
                                next_state = LidarState::ReceiveSample
                            } else {
                                next_state = LidarState::ProcessSample
                            }
                        }
                    }
                    Ok(Err(err)) => {
                        error!("Failed to read START_SCAN response from LiDAR ({}), reseting...", err);
                    }
                    Err(TimeoutError) => next_state = LidarState::ReceiveTimeout,
                }
                next_state
            }
            LidarState::ReceiveTimeout => {
                error!("Timeout while receiving data from LiDAR. Checking status...");
                LidarState::CheckHealth
            }
            LidarState::ProcessSample => {
                MOTE_TO_HOST
                    .send(mote_messages::MoteToHostMessage::Scan(scan_points.clone()))
                    .await;
                scan_points.clear();
                LidarState::ReceiveSample
            }
            LidarState::Stop => LidarState::Reset,
        }
    }
}

pub async fn init(spawner: Spawner, r: RplidarC1Resources) {
    unwrap!(spawner.spawn(lidar_state_machine_task(r)));
}
