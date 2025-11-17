use defmt::info;
use embassy_executor::Spawner;
use embassy_rp::pac::pio::regs::Irq;
use embassy_rp::pio::Pio;
use embassy_time::Timer;
use pid::Pid;

use crate::tasks::drive_base::encoder::PioEncoder;
use crate::tasks::{DriveBaseResources, Irqs, LeftEncoderResources, RightEncoderResources};
use crate::wifi::HOST_TO_MOTE_COMMAND;

mod encoder;
mod motor;

#[embassy_executor::task]
async fn drive_base_task(
    drive_base_r: DriveBaseResources,
    left_encoder_r: LeftEncoderResources,
    right_encoder_r: RightEncoderResources,
) {
    // Left wheel
    let Pio {
        common: mut left_encoder_common,
        sm0: left_encoder_sm0,
        ..
    } = Pio::new(left_encoder_r.pio, Irqs);
    let mut left_encoder = PioEncoder::new(
        &mut left_encoder_common,
        left_encoder_sm0,
        left_encoder_r.phase_a,
        left_encoder_r.phase_b,
    );

    // Right wheel
    let Pio {
        common: mut right_encoder_common,
        sm0: right_encoder_sm0,
        ..
    } = Pio::new(right_encoder_r.pio, Irqs);
    let mut right_encoder = PioEncoder::new(
        &mut right_encoder_common,
        right_encoder_sm0,
        right_encoder_r.phase_a,
        right_encoder_r.phase_b,
    );

    loop {
        info!(
            "Left: {} | Right: {}",
            left_encoder.read().await,
            right_encoder.read().await
        );
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
