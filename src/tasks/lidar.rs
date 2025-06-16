use core::array;

use super::{Irqs, RplidarC1Resources};
use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::uart::{Config, DataBits, Parity, StopBits, Uart};
use embassy_time::Timer;

#[embassy_executor::task]
async fn lidar_state_machine_task(r: RplidarC1Resources) {
    let mut config = Config::default();
    config.baudrate = 460800;
    config.stop_bits = StopBits::STOP1;
    config.data_bits = DataBits::DataBits8;
    config.parity = Parity::ParityNone;

    let mut uart = Uart::new(r.uart, r.tx, r.rx, Irqs, r.tx_dma, r.rx_dma, config);

    // // Reset the LiDAR
    // let data = [0xA5u8, 0x40];
    // uart.write(&data).await.unwrap();
    //
    // Timer::after_millis(500).await;
    //
    // // Stop the LiDAR
    // let data = [0xA5u8, 0x25];
    // uart.write(&data).await.unwrap();
    //
    // Timer::after_millis(10).await;

    // Start the LiDAR
    let data = [0xA5u8, 0x20];
    uart.write(&data).await.unwrap();

    // Seek start of message
    let mut start_id_buf = [0; 2];
    while start_id_buf[0] != 0xA5 || start_id_buf[1] != 0x5A {
        start_id_buf[0] = start_id_buf[1];
        match uart.read(array::from_mut(&mut start_id_buf[1])).await {
            Ok(_) => {
                info!("RX {:#x}", start_id_buf);
            }
            Err(error) => warn!("Failed to read: {:?}", error),
        }
    }

    // Read header
    let mut header_buf = [0; 5];
    match uart.read(&mut header_buf).await {
        Ok(length) => {
            println!("Read to length: {:?}", length)
        }
        Err(error) => warn!("Failed to read: {:?}", error),
    }

    // Read data
    let mut data_buf = [0; 100];
    loop {
        match uart.read(&mut data_buf).await {
            Ok(_) => {}
            Err(error) => warn!("Failed to read: {:?}", error),
        }
        info!("RX {:#x}", data_buf);
    }
}

pub async fn init(spawner: Spawner, r: RplidarC1Resources) {
    unwrap!(spawner.spawn(lidar_state_machine_task(r)));
}
