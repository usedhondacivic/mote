use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::uart::{Config, DataBits, Parity, StopBits, Uart};
use embassy_time::{Duration, TimeoutError, Timer, with_timeout};
use mote_messages::configuration::mote_to_host::{BIT, BITResult};
use mote_messages::runtime::mote_to_host;

use super::{Irqs, RplidarC1Resources};
use crate::helpers::update_bit_result;
use crate::tasks::CONFIGURATION_STATE;
use crate::wifi::MOTE_TO_HOST;

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

    let mut scan_points = heapless::Vec::<mote_to_host::Point, { mote_to_host::MAX_POINTS_PER_SCAN_MESSAGE }>::new();

    let mut count = 0;

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
                                    0x00 => {
                                        next_state = LidarState::ScanRequest;
                                        {
                                            let mut configuration_state = CONFIGURATION_STATE.lock().await;
                                            update_bit_result(
                                                &mut configuration_state.built_in_test.lidar,
                                                "Check Health",
                                                BITResult::Pass,
                                            );
                                        }
                                    }
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
                            let _ = scan_points.push(mote_to_host::Point {
                                quality: (resp[0] & !0b11) >> 2,
                                angle: angle,
                                distance: u16::from_le_bytes(resp[3..5].try_into().unwrap_or([0x00, 0x00])),
                            });

                            if scan_points.is_full() {
                                next_state = LidarState::ProcessSample
                            } else {
                                next_state = LidarState::ReceiveSample
                            }
                        }
                    }
                    Ok(Err(err)) => {
                        error!("Failed to read point from LiDAR ({}), reseting...", err);
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
                // We don't care if these packets get lost, so keep going if the channel is full
                let _ = MOTE_TO_HOST.try_send(mote_to_host::Message::Scan(scan_points.clone()));

                scan_points.clear();
                LidarState::ReceiveSample
            }
            LidarState::Stop => LidarState::Reset,
        }
    }
}

pub async fn init(spawner: Spawner, r: RplidarC1Resources) {
    // Init BIT
    {
        let mut configuration_state = CONFIGURATION_STATE.lock().await;
        let init = BIT {
            name: heapless::String::try_from("Init").expect("Failed to assign name to BIT"),
            result: BITResult::Waiting,
        };
        let check_health = BIT {
            name: heapless::String::try_from("Check Health").expect("Failed to assign name to BIT"),
            result: BITResult::Waiting,
        };
        let measurements = BIT {
            name: heapless::String::try_from("Receiving Data").expect("Failed to assign name to BIT"),
            result: BITResult::Waiting,
        };
        for test in [init, check_health, measurements] {
            configuration_state
                .built_in_test
                .lidar
                .push(test)
                .expect("Failed to add test");
        }
    }

    // Start task
    spawner.spawn(lidar_state_machine_task(r).unwrap());

    // Update init state
    {
        let mut configuration_state = CONFIGURATION_STATE.lock().await;
        update_bit_result(&mut configuration_state.built_in_test.lidar, "Init", BITResult::Pass);
    }
}
