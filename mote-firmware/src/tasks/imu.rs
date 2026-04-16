pub mod lib;
pub mod regs;
use embassy_executor::Spawner;
use embassy_rp::i2c::{Config, I2c};
use embassy_rp::peripherals::I2C1;
pub use lib::Lsm6ds3TRC;
use mote_api::messages::mote_to_host;
use mote_api::messages::mote_to_host::{BIT, BITResult, IMUAxisTriple, IMUMeasurement};

use super::{ImuResources, Irqs};
use crate::helpers::update_bit_result;
use crate::tasks::CONFIGURATION_STATE;
use crate::wifi::DATA_OFFLOAD_CHANNEL;

// returns temperature and (accel, gyro) IMU measurement
pub async fn get_sensor_data(
    imu: &mut Lsm6ds3TRC<I2c<'static, I2C1, embassy_rp::i2c::Async>>,
) -> (f32, IMUMeasurement) {
    match imu.read_all().await {
        Ok((temperature, gyro_tuple, accel_tuple)) => {
            // Map the accelerometer tuple (f32, f32, f32) to IMUAxisTriple
            let accel = IMUAxisTriple {
                x: accel_tuple.0,
                y: accel_tuple.1,
                z: accel_tuple.2,
            };

            // Map the gyroscope tuple (f32, f32, f32) to IMUAxisTriple
            let gyro = IMUAxisTriple {
                x: gyro_tuple.0,
                y: gyro_tuple.1,
                z: gyro_tuple.2,
            };

            // Return the temperature and the combined measurement
            (temperature, IMUMeasurement { accel, gyro })
        }
        Err(_) => {
            // Default error case
            (
                0.0,
                IMUMeasurement {
                    accel: IMUAxisTriple { x: 0.0, y: 0.0, z: 0.0 },
                    gyro: IMUAxisTriple { x: 0.0, y: 0.0, z: 0.0 },
                },
            )
        }
    }
}

#[embassy_executor::task]
async fn imu_task(r: ImuResources) {
    // init the I2C interface and the IMU driver
    let i2c = I2c::new_async(r.i2c, r.scl, r.sda, Irqs, Config::default());

    // Explicitly handle the Result to help the compiler infer types
    let mut imu = match Lsm6ds3TRC::new(i2c, 0x6A).await {
        Ok(lsm) => lsm,
        // Match specifically on the communication error to see the I2C error
        Err((_i2c, lib::Error::Communication(e))) => {
            defmt::error!("IMU Communication Error: {:?}", e);
            panic!("IMU Init Failed: I2C error");
        }
        // handle the case where the ID doesn't match
        Err((_i2c, lib::Error::ChipDetectFailed)) => {
            defmt::error!("IMU Chip Detect Failed - check wiring or address");
            panic!("IMU Init Failed: Wrong ID");
        }
        // Other Errors
        Err((_i2c, _)) => {
            panic!("IMU Init Failed: Unknown error");
        }
    };

    // --- WAKE UP THE SENSORS ---
    // Set Accelerometer to 104Hz, 2g scale
    imu.set_accelerometer_output(regs::AccelerometerOutput::Rate104)
        .await
        .unwrap();
    imu.set_accelerometer_scale(regs::AccelerometerScale::G02)
        .await
        .unwrap();

    // Set Gyroscope to 104Hz, 245 dps scale
    imu.set_gyroscope_output(regs::GyroscopeOutput::Rate104).await.unwrap();
    imu.set_gyroscope_scale(regs::GyroscopeFullScale::Dps245).await.unwrap();

    loop {
        let (_, measurement) = get_sensor_data(&mut imu).await;
        let _ = DATA_OFFLOAD_CHANNEL.try_send(mote_to_host::Message::IMUMeasurement(measurement));

        embassy_time::Timer::after_millis(20).await;
    }
}

pub async fn init(spawner: Spawner, r: ImuResources) {
    // setup bit state for config page
    {
        let mut configuration_state = CONFIGURATION_STATE.lock().await;
        let init_bit = BIT {
            name: "Init".into(),
            result: BITResult::Waiting,
        };
        configuration_state.built_in_test.imu.push(init_bit);
    }

    spawner.spawn(imu_task(r)).unwrap();

    // update bit to pass
    {
        let mut configuration_state = CONFIGURATION_STATE.lock().await;
        update_bit_result(&mut configuration_state.built_in_test.imu, "Init", BITResult::Pass);
    }
}
