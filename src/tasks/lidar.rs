use core::array;

use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::uart::{Config, DataBits, Parity, StopBits, Uart};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use super::{Irqs, RplidarC1Resources};

#[derive(Serialize, Deserialize)]
pub struct Point {
    quality: u8,
    angle: u16,
    distance: u16,
}

#[serde_as]
#[derive(Serialize, Deserialize)]
pub struct Scan {
    #[serde_as(as = "[_; 1000]")]
    points: [Point; 1000],
}

#[embassy_executor::task]
async fn lidar_state_machine_task(r: RplidarC1Resources) {
    let mut config = Config::default();
    config.baudrate = 460800;
    config.stop_bits = StopBits::STOP1;
    config.data_bits = DataBits::DataBits8;
    config.parity = Parity::ParityNone;

    let mut uart = Uart::new(r.uart, r.tx, r.rx, Irqs, r.tx_dma, r.rx_dma, config);

    // Start the LiDAR
    let data = [0xA5u8, 0x20];
    uart.write(&data).await.unwrap();

    // Seek start of message
    let mut start_id_buf = [0; 2];
    while start_id_buf[0] != 0xA5 || start_id_buf[1] != 0x5A {
        start_id_buf[0] = start_id_buf[1];
        match uart.read(array::from_mut(&mut start_id_buf[1])).await {
            Ok(_) => {
                debug!("RX {:#x}", start_id_buf);
            }
            Err(error) => warn!("Failed to read: {:?}", error),
        }
    }

    // Read header
    let mut header_buf = [0; 5];
    match uart.read(&mut header_buf).await {
        Ok(length) => {
            debug!("Read to length: {:?}", length)
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

        // TODO: Serialize and send to wifi task

        // debug!("RX {:#x}", data_buf);
    }
}

pub async fn init(spawner: Spawner, r: RplidarC1Resources) {
    unwrap!(spawner.spawn(lidar_state_machine_task(r)));
}
