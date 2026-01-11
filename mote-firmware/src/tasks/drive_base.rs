use core::intrinsics::{cosf16, cosf32};

use defmt::{error, info};
use embassy_executor::Spawner;
use embassy_rp::pio::Pio;
use embassy_rp::{gpio, pwm};
use embassy_time::Timer;

use crate::tasks::drive_base::drv8833::{DRV8833Driver, MotorChannel, MotorDirection};
use crate::tasks::drive_base::encoder::PioEncoder;
use crate::tasks::{DRV8833Resources, EncoderDriverResources, Irqs, LeftEncoderResources, RightEncoderResources};

mod drv8833;
mod encoder;

#[embassy_executor::task]
async fn drive_base_task(
    motor_driver_r: DRV8833Resources,
    encoder_driver_r: EncoderDriverResources,
    left_encoder_r: LeftEncoderResources,
    right_encoder_r: RightEncoderResources,
) {
    // Setup PIO
    let Pio {
        common: mut encoder_common,
        sm0: encoder_sm0,
        sm1: encoder_sm1,
        ..
    } = Pio::new(encoder_driver_r.pio, Irqs);

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

    let desired_freq_hz = 25_000;
    let clock_freq_hz = embassy_rp::clocks::clk_sys_freq();
    let divider = 16u8;
    let period = (clock_freq_hz / (desired_freq_hz * divider as u32)) as u16 - 1;
    let mut pwm_config = pwm::Config::default();
    pwm_config.top = period;
    pwm_config.divider = divider.into();

    let mut left_pwm = pwm::Pwm::new_output_ab(
        motor_driver_r.left_pwm,
        motor_driver_r.left_a,
        motor_driver_r.left_b,
        pwm_config.clone(),
    );
    let (Some(left_a), Some(left_b)) = left_pwm.split_by_ref() else {
        error!("Unable to init drive base PWM. Drive-base disabled.");
        return;
    };

    let mut right_pwm = pwm::Pwm::new_output_ab(
        motor_driver_r.right_pwm,
        motor_driver_r.right_a,
        motor_driver_r.right_b,
        pwm_config.clone(),
    );
    let (Some(right_a), Some(right_b)) = right_pwm.split_by_ref() else {
        error!("Unable to init drive base PWM. Drive-base disabled.");
        return;
    };

    let sleep = gpio::Output::new(motor_driver_r.sleep, gpio::Level::Low);

    let mut motor_driver = DRV8833Driver::new(left_a, left_b, right_a, right_b, sleep);

    motor_driver.wakeup().unwrap_or_else(|_| {
        error!("Error while waking up drive base. Drive-base disabled.");
        return;
    });

    let mut t: f32 = 0.;
    loop {
        t += 0.1;
        let c = cosf32(t) * 100.;
        if c > 0. {
            motor_driver
                .set(MotorChannel::ChannelA, c as u8, MotorDirection::Forward)
                .unwrap_or_else(|_| {
                    error!("Error while waking up drive base. Drive-base disabled.");
                    return;
                });
        } else {
            motor_driver
                .set(MotorChannel::ChannelA, -c as u8, MotorDirection::Reverse)
                .unwrap_or_else(|_| {
                    error!("Error while waking up drive base. Drive-base disabled.");
                    return;
                });
        }

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
    motor_driver_r: DRV8833Resources,
    encoder_driver_r: EncoderDriverResources,
    left_encoder_r: LeftEncoderResources,
    right_encoder_r: RightEncoderResources,
) {
    spawner
        .spawn(drive_base_task(
            motor_driver_r,
            encoder_driver_r,
            left_encoder_r,
            right_encoder_r,
        ))
        .unwrap();
}
