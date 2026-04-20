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

// NUMBER OF MISSED IMU READS IN A ROW BEFORE WE FLAG A BIT FAILURE
const MISSED_READ_THRESHOLD: u8 = 10;
const INVALID_TEMPERATURE: f32 = 25.0; // value returned by get_sensor_data on read failure

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
                INVALID_TEMPERATURE, // invalid temperature to indicate error
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
    let i2c = I2c::new_async(r.i2c, r.scl, r.sda, Irqs, Config::default());
    let mut imu = reset_imu(i2c).await;
    let mut missed_read_count: u8 = 0;

    // Sensor Reading loop
    loop {
        let (temp, measurement) = get_sensor_data(&mut imu).await;
        let _ = DATA_OFFLOAD_CHANNEL.try_send(mote_to_host::Message::IMUMeasurement(measurement));

        // get sensor data errored, update BIT and log, and missed read count
        if temp == INVALID_TEMPERATURE {
            missed_read_count += 1;
            defmt::error!("Failed to read IMU sensor data. Missed reads in a row: {}, Waiting 5 seconds before attempting recovery", missed_read_count);
            if missed_read_count >= MISSED_READ_THRESHOLD
            {
                {
                    let mut configuration_state = CONFIGURATION_STATE.lock().await;
                    update_bit_result(
                        &mut configuration_state.built_in_test.imu,
                        "Reading Values",
                        BITResult::Fail
                    );
                }
                // waiting here for a few seconds to allow the BIT state to be observed as failed before attempting recovery
                embassy_time::Timer::after_secs(5).await;

                // reclaim i2c resources
                let i2c = imu.release();
                imu = reset_imu(i2c).await; // attempt to reset the IMU after hitting the missed read threshold
            }
        } else {
            missed_read_count = 0; // reset missed read count on successful read
        }

        embassy_time::Timer::after_millis(20).await;
    }

}

async fn reset_imu(mut i2c: I2c<'static, I2C1, embassy_rp::i2c::Async>) -> Lsm6ds3TRC<I2c<'static, I2C1, embassy_rp::i2c::Async>> {

    let imu = 
    loop { // attepmt to initialize and configure the IMU, if it fails, log the error and retry after a delay
        defmt::info!("Resetting IMU");
        let (returned_i2c, result) = async {
            // Attempt to create the driver
            match Lsm6ds3TRC::new(i2c, 0x6A).await {
                Ok(mut driver) => {
                    // Try to configure it
                    let config_res = async {
                        driver.set_accelerometer_output(regs::AccelerometerOutput::Rate104).await?;
                        driver.set_accelerometer_scale(regs::AccelerometerScale::G02).await?;
                        driver.set_gyroscope_output(regs::GyroscopeOutput::Rate104).await?;
                        driver.set_gyroscope_scale(regs::GyroscopeFullScale::Dps245).await?;
                        Ok(())
                    }.await;

                    match config_res {
                        Ok(_) => (None, Ok(driver)), // success, return the driver and don't return the i2c bus
                        Err(e) => (Some(driver.release()), Err(e)), // config failed, return i2c bus for next loop and log the error
                    }
                }
                Err((returned_bus, e)) => (Some(returned_bus), Err(e)), // init failed, Return the bus
            }
        }.await;

        match result {
            Ok(driver) => break driver,
            Err(e) => {
                // 1. Reclaim the i2c bus if it was returned by the failed driver init or config attempt
                if let Some(bus) = returned_i2c {
                    i2c = bus;
                }
                else {
                    defmt::error!("IMU initialization failed but no I2C bus was returned. This likely means the I2C bus is in a bad state and the IMU driver failed to reset it. Attempting to continue with the same I2C instance, but subsequent attempts may also fail until the bus is reset.");
                    panic!("IMU initialization failed and no I2C bus was returned. This likely means the I2C bus is in a bad state and the IMU driver failed to reset it. Attempting to continue with the same I2C instance, but subsequent attempts may also fail until the bus is reset.");
                }

                // 2. Log the specific failure
                match e {
                    lib::Error::Communication(_) => defmt::error!("IMU Error: I2C Communication failed"),
                    lib::Error::ChipDetectFailed => defmt::error!("IMU Error: Chip not detected"),
                    _ => defmt::error!("IMU Error: Unknown setup error"),
                }

                // 3. Log the RESTART attempt BEFORE the timer to confirm logic reached here
                defmt::warn!("IMU not initialized. Retrying in 5 seconds...");
                
                // 4. Update BIT state and wait
                {
                    let mut state = CONFIGURATION_STATE.lock().await;
                    update_bit_result(&mut state.built_in_test.imu, "Init", BITResult::Fail);
                }

                embassy_time::Timer::after_secs(5).await;
            }
        }
    };

    defmt::info!("IMU Initialized and Configured");
    {
        let mut configuration_state = CONFIGURATION_STATE.lock().await;
        update_bit_result(&mut configuration_state.built_in_test.imu, "Init", BITResult::Pass);
        update_bit_result(&mut configuration_state.built_in_test.imu, "Reading Values", BITResult::Pass);
    }

    imu
}

pub async fn init(spawner: Spawner, r: ImuResources) {
    // setup bit state for config page
    {
        let mut configuration_state = CONFIGURATION_STATE.lock().await;
        let init_bit = BIT {
            name: "Init".into(),
            result: BITResult::Waiting,
        };
        let health_bit = BIT {
            name: "Reading Values".into(),
            result: BITResult::Waiting,
        };
        configuration_state.built_in_test.imu.push(init_bit);
        configuration_state.built_in_test.imu.push(health_bit);
    }

    spawner.spawn(imu_task(r)).unwrap();

}
