mod rp_c1_driver;

use embassy_executor::Spawner;
use embassy_rp::uart::{BufferedUart, Config, DataBits, Parity, StopBits};
use mote_messages::configuration::mote_to_host::{BIT, BITResult};
use mote_messages::runtime::mote_to_host;
use static_cell::StaticCell;

use super::{Irqs, RplidarC1Resources};
use crate::helpers::update_bit_result;
use crate::tasks::CONFIGURATION_STATE;
use crate::tasks::lidar::rp_c1_driver::{LidarState, Point, RPLidarC1};
use crate::wifi::MOTE_TO_HOST_DATA_OFFLOAD;

impl From<Point> for mote_to_host::data_offload::Point {
    fn from(value: Point) -> Self {
        mote_to_host::data_offload::Point {
            quality: value.quality,
            // Feels expensive, but the RP2350 has a FPU
            angle_rads: (value.angle as f32 / 64.0).to_radians(),
            distance_mm: value.distance as f32 / 4.0,
        }
    }
}

#[embassy_executor::task]
async fn lidar_state_machine_task(r: RplidarC1Resources) {
    let mut config = Config::default();
    config.baudrate = 460800;
    config.stop_bits = StopBits::STOP1;
    config.data_bits = DataBits::DataBits8;
    config.parity = Parity::ParityNone;

    static TX_BUF: StaticCell<[u8; 64]> = StaticCell::new();
    let tx_buf = &mut TX_BUF.init([0; 64])[..];
    static RX_BUF: StaticCell<[u8; 64]> = StaticCell::new();
    let rx_buf = &mut RX_BUF.init([0; 64])[..];
    let uart = BufferedUart::new(r.uart, r.tx, r.rx, Irqs, tx_buf, rx_buf, config);

    let mut state = LidarState::Reset;

    let mut point_buf: [rp_c1_driver::Point; mote_to_host::data_offload::MAX_POINTS_PER_SCAN_MESSAGE] =
        [rp_c1_driver::Point::default(); _];
    let mut valid_points = 0;

    let mut driver = RPLidarC1::new(uart);

    loop {
        state = match state {
            LidarState::Idle => LidarState::Idle,
            LidarState::Start => LidarState::Reset,
            LidarState::Reset => driver.reset().await,
            LidarState::CheckHealth => {
                let next_state = driver.check_health().await;
                {
                    let mut configuration_state = CONFIGURATION_STATE.lock().await;
                    update_bit_result(
                        &mut configuration_state.built_in_test.lidar,
                        "Check Health",
                        if next_state == LidarState::Reset {
                            BITResult::Fail
                        } else {
                            BITResult::Pass
                        },
                    );
                }
                next_state
            }
            LidarState::ScanRequest => driver.scan_request().await,
            // This could be updated to use zerocopy for a nice performance boost
            LidarState::ReceiveSample => {
                match driver.receive_samples(&mut point_buf).await {
                    Ok(count) => {
                        if count < mote_to_host::data_offload::MAX_POINTS_PER_SCAN_MESSAGE >> 1 {
                            // More than 50% of points were not read correctly
                            LidarState::CheckHealth
                        } else {
                            valid_points = count;
                            LidarState::ProcessSample
                        }
                    }
                    Err(_) => {
                        // Something is wrong, check health and try again
                        LidarState::CheckHealth
                    }
                }
            }
            LidarState::ProcessSample => {
                // We don't care if these packets get lost, so don't block if the channel is
                // full
                let _ = MOTE_TO_HOST_DATA_OFFLOAD.try_send(mote_to_host::data_offload::Message::Scan(
                    point_buf[..valid_points]
                        .into_iter()
                        .map(|&point| point.into())
                        .collect(),
                ));

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
        for test in [init, check_health] {
            configuration_state
                .built_in_test
                .lidar
                .push(test)
                .expect("Failed to add test");
        }
    }

    // Start task
    spawner.spawn(lidar_state_machine_task(r)).unwrap();

    // Update init state
    {
        let mut configuration_state = CONFIGURATION_STATE.lock().await;
        update_bit_result(&mut configuration_state.built_in_test.lidar, "Init", BITResult::Pass);
    }
}
