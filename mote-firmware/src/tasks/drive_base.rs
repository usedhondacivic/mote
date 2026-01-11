use defmt::info;
use embassy_executor::Spawner;
use embassy_rp::pio::Pio;
use embassy_time::Timer;

use crate::tasks::drive_base::encoder::PioEncoder;
use crate::tasks::{DriveBaseResources, Irqs, LeftEncoderResources, RightEncoderResources};

mod encoder;
mod motor;

#[embassy_executor::task]
async fn drive_base_task(
    drive_base_r: DriveBaseResources,
    left_encoder_r: LeftEncoderResources,
    right_encoder_r: RightEncoderResources,
) {
    // Setup PIO
    let Pio {
        common: mut encoder_common,
        sm0: encoder_sm0,
        sm1: encoder_sm1,
        ..
    } = Pio::new(drive_base_r.pio, Irqs);

    // Left wheel
    let mut left_encoder = PioEncoder::new(
        &mut encoder_common,
        encoder_sm0,
        left_encoder_r.phase_a,
        left_encoder_r.phase_b,
    );

    // Right wheel
    let mut right_encoder = PioEncoder::new(
        &mut encoder_common,
        encoder_sm1,
        right_encoder_r.phase_a,
        right_encoder_r.phase_b,
    );

    loop {
        // info!(
        //     "Left: {} | Right: {}",
        //     left_encoder.read().await,
        //     right_encoder.read().await
        // );
        Timer::after_millis(100).await;
    }
}

pub async fn init(
    spawner: Spawner,
    drive_base_r: DriveBaseResources,
    left_encoder_r: LeftEncoderResources,
    right_encoder_r: RightEncoderResources,
) {
    spawner.spawn(drive_base_task(drive_base_r, left_encoder_r, right_encoder_r).unwrap());
}
