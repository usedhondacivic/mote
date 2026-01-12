use defmt::{error, info};
use embassy_executor::Spawner;
use embassy_rp::pio::{Instance, Pio};
use embassy_rp::pwm::SetDutyCycle;
use embassy_rp::{gpio, pwm};
use embassy_time::{Duration, Ticker};
use pid::Pid;

use crate::tasks::drive_base::encoder::PioEncoder;
use crate::tasks::drive_base::hbridge::PwmBridge;
use crate::tasks::{DRV8833Resources, EncoderDriverResources, Irqs, LeftEncoderResources, RightEncoderResources};

mod encoder;
mod hbridge;

const ENCODER_PULSES_PER_ROTATION: u16 = 2340;
const MOTOR_DEADBAND: u8 = 60;

fn encoder_pulses_to_rad_per_second(pulses: i32) -> f32 {
    return (pulses as f32 / ENCODER_PULSES_PER_ROTATION as f32) * 2. * 3.14159;
}

struct Motor<'d, T: SetDutyCycle, P: Instance, const SM: usize> {
    bridge: PwmBridge<T>,
    pio_encoder: PioEncoder<'d, P, SM>,
    pid: Pid<f32>,

    encoder_value: i32,
}

impl<'d, T: SetDutyCycle, P: Instance, const SM: usize> Motor<'d, T, P, SM> {
    fn new(bridge: PwmBridge<T>, encoder: PioEncoder<'d, P, SM>) -> Self {
        let ouput_limit = 100. - MOTOR_DEADBAND as f32;
        let pid = *Pid::<f32>::new(0., ouput_limit)
            .p(10.0, ouput_limit)
            .i(1.0, ouput_limit);

        Self {
            bridge,
            pio_encoder: encoder,
            pid,
            encoder_value: 0,
        }
    }

    fn set_setpoint(&mut self, setpoint: f32) {
        self.pid.setpoint(setpoint);
    }

    async fn step(&mut self, dt_ms: u32) {
        let dt = dt_ms as f32 / 1000.;

        let last_encoder_read = self.encoder_value;
        self.encoder_value = self.pio_encoder.read().await;

        let measurement = encoder_pulses_to_rad_per_second(self.encoder_value - last_encoder_read);

        let control_output = self.pid.next_control_output(measurement / dt).output;
        let deadband_adjusted_output = if control_output > 0. {
            (control_output + MOTOR_DEADBAND as f32)
        } else {
            (control_output - MOTOR_DEADBAND as f32)
        };

        info!(
            "Measurement {} | Rad/s {} | Control Output {}",
            measurement,
            measurement / dt,
            control_output
        );

        if control_output > 2.0 {
            self.bridge.forward(deadband_adjusted_output as u8).unwrap();
        } else if control_output < -2.0 {
            self.bridge.reverse(-deadband_adjusted_output as u8).unwrap();
        } else {
            self.bridge.stop().unwrap();
        }
    }
}

#[embassy_executor::task]
async fn motor_task(
    encoder_driver_r: EncoderDriverResources,
    left_encoder_r: LeftEncoderResources,
    right_encoder_r: RightEncoderResources,
    motor_driver_r: DRV8833Resources,
) {
    // Setup PWM
    let desired_freq_hz = 25_000;
    let clock_freq_hz = embassy_rp::clocks::clk_sys_freq();
    let divider = 16u8;
    let period = (clock_freq_hz / (desired_freq_hz * divider as u32)) as u16 - 1;
    let mut pwm_config = pwm::Config::default();
    pwm_config.top = period;
    pwm_config.divider = divider.into();

    // Setup PIO
    let Pio {
        common: mut encoder_common,
        sm0: encoder_sm0,
        sm1: encoder_sm1,
        ..
    } = Pio::new(encoder_driver_r.pio, Irqs);

    // Left wheel
    let left_encoder = PioEncoder::new(
        &mut encoder_common,
        encoder_sm0,
        left_encoder_r.phase_a,
        left_encoder_r.phase_b,
    );

    let left_pwm = pwm::Pwm::new_output_ab(
        motor_driver_r.left_pwm,
        motor_driver_r.left_a,
        motor_driver_r.left_b,
        pwm_config.clone(),
    );
    let (Some(left_a), Some(left_b)) = left_pwm.split() else {
        error!("Unable to init drive base PWM. Drive-base disabled.");
        return;
    };
    let left_pwm_bridge = PwmBridge::new(left_a, left_b, 0);
    let mut left_motor = Motor::new(left_pwm_bridge, left_encoder);

    // Right wheel
    let right_encoder = PioEncoder::new(
        &mut encoder_common,
        encoder_sm1,
        right_encoder_r.phase_a,
        right_encoder_r.phase_b,
    );

    let right_pwm = pwm::Pwm::new_output_ab(
        motor_driver_r.right_pwm,
        motor_driver_r.right_a,
        motor_driver_r.right_b,
        pwm_config.clone(),
    );
    let (Some(right_a), Some(right_b)) = right_pwm.split() else {
        error!("Unable to init drive base PWM. Drive-base disabled.");
        return;
    };
    let right_pwm_bridge = PwmBridge::new(right_a, right_b, 0);
    let mut right_motor = Motor::new(right_pwm_bridge, right_encoder);

    let sleep = gpio::Output::new(motor_driver_r.sleep, gpio::Level::High);

    // Run PID at 50Hz
    let mut ticker = Ticker::every(Duration::from_millis(20));

    // left_motor.set_setpoint(5.0);

    loop {
        left_motor.step(20).await;
        right_motor.step(20).await;

        ticker.next().await;
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
        .spawn(motor_task(
            encoder_driver_r,
            left_encoder_r,
            right_encoder_r,
            motor_driver_r,
        ))
        .unwrap();
}
