use defmt::{error, info};
use embassy_executor::Spawner;
use embassy_futures::select::select4;
use embassy_rp::pio::{Instance, Pio};
use embassy_rp::pwm::SetDutyCycle;
use embassy_rp::{gpio, pwm};
use embassy_time::{Duration, Instant, Ticker, Timer};
use mote_api::messages::mote_to_host::{DriveBaseState, Message, WheelJointState};
use pid::Pid;

use crate::tasks::drive_base::encoder::PioEncoder;
use crate::tasks::drive_base::hbridge::PwmBridge;
use crate::tasks::wifi::{DATA_OFFLOAD_CHANNEL, MOTOR_COMMAND_CHANNEL};
use crate::tasks::{
    DRV8833Resources, EncoderDriverResources, Irqs, LeftEncoderResources, RightEncoderResources, power_gate,
};

mod encoder;
mod hbridge;

/// Number of encoder pulses recorded per rotation.
/// This is specific to each model of TT motor, right now we use:
/// https://category.yahboom.net/products/encoder-tt-motor?srsltid=AfmBOooIQxn2e-jyW08981J7uv54Ng7IHo0c9PmTH5pefo1hFxmwZq3i
/// We may want to make this a configurable value to support different motors.
const ENCODER_PULSES_PER_ROTATION: u16 = 2340;
/// Stiction prevents commands lower than this % from causing motion.
const MOTOR_DEADBAND_PERCENT: u8 = 60;
/// PDI commands lower than this % are filtered to prevent chattering due to
/// gearbox hysteresis.
const CONTROL_DEADBAND_PERCENT: f32 = 2.;
/// ms per iteration of the PID control loop.
const PID_CONTROL_LOOP_PERIOD_MS: u64 = 20;
/// ms per joint state telemetry value.
const TELEMETRY_LOOP_PERIOD_MS: u64 = 100;
/// Seconds of not receiving a command before deactivating the drive base.
const WATCH_DOG_TIMEOUT: u64 = 1;

/// Convert encoder pulses into radians
fn encoder_pulses_to_rad(pulses: i32) -> f32 {
    (pulses as f32 / ENCODER_PULSES_PER_ROTATION as f32) * 2. * core::f32::consts::PI
}

/// Represents a motor with an hbridge driver and quadrature encoder.
struct Motor<'d, T: SetDutyCycle, P: Instance, const SM: usize> {
    bridge: PwmBridge<T>,
    pio_encoder: PioEncoder<'d, P, SM>,
    pid: Pid<f32>,

    encoder_value: i32,
    pub joint_state: WheelJointState,
}

impl<'d, T: SetDutyCycle, P: Instance, const SM: usize> Motor<'d, T, P, SM> {
    fn new(bridge: PwmBridge<T>, encoder: PioEncoder<'d, P, SM>) -> Self {
        let ouput_limit = 100. - MOTOR_DEADBAND_PERCENT as f32;
        let pid = *Pid::<f32>::new(0., ouput_limit)
            .p(10.0, ouput_limit)
            .i(1.0, ouput_limit);

        Self {
            bridge,
            pio_encoder: encoder,
            pid,
            encoder_value: 0,
            joint_state: WheelJointState {
                effort_percent: 0.0,
                velocity_rad_per_s: 0.0,
                postition_rad: 0.0,
            },
        }
    }

    /// Set the target velocity in radians/second
    fn set_setpoint_rad_per_s(&mut self, setpoint: f32) {
        self.pid.setpoint(setpoint);
    }

    /// Step the motor's PDI loop, targeting the latest setpoint commanded by
    /// set_setpoint
    async fn step(&mut self, dt_ms: u64) {
        let dt = dt_ms as f32 / 1000.;

        // Calculate rotation delta as pulses per second
        let last_encoder_read = self.encoder_value;
        self.encoder_value = self.pio_encoder.read().await;
        let measurement = encoder_pulses_to_rad(self.encoder_value - last_encoder_read);

        // Get the PID output accounting for the motor deadband
        let control_output = self.pid.next_control_output(measurement / dt).output;
        let deadband_adjusted_output = if control_output > 0. {
            control_output + MOTOR_DEADBAND_PERCENT as f32
        } else {
            control_output - MOTOR_DEADBAND_PERCENT as f32
        };

        // If the command exceeds the control deadband, forward it to the hbridge with
        // the correct polarity
        if control_output > CONTROL_DEADBAND_PERCENT {
            self.bridge.forward(deadband_adjusted_output as u8).unwrap();
        } else if control_output < -CONTROL_DEADBAND_PERCENT {
            self.bridge.reverse(-deadband_adjusted_output as u8).unwrap();
        } else {
            self.bridge.stop().unwrap();
        }

        // Update the joint state
        self.joint_state.postition_rad = encoder_pulses_to_rad(self.encoder_value);
        self.joint_state.velocity_rad_per_s = measurement / dt;
        self.joint_state.effort_percent = deadband_adjusted_output;
    }
}

#[embassy_executor::task]
async fn motor_task(
    encoder_driver_r: EncoderDriverResources,
    left_encoder_r: LeftEncoderResources,
    right_encoder_r: RightEncoderResources,
    motor_driver_r: DRV8833Resources,
) {
    info!("Gating on 3A capable before starting drive base");
    power_gate::gate_3_amp().await;
    info!("Power supply is 3A capable");

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

    // Configure left wheel
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

    // Configure right wheel
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

    // Init sleep pin
    let mut sleep = gpio::Output::new(motor_driver_r.sleep, gpio::Level::High);

    // PID, telem and watchdog timers
    let mut pid_ticker = Ticker::every(Duration::from_millis(PID_CONTROL_LOOP_PERIOD_MS));
    let mut telemetry_ticker = Ticker::every(Duration::from_millis(TELEMETRY_LOOP_PERIOD_MS));
    let mut watchdog_deadline = Instant::now() + Duration::from_secs(WATCH_DOG_TIMEOUT);

    // Motors start with 0 velocity
    left_motor.set_setpoint_rad_per_s(0.0);
    right_motor.set_setpoint_rad_per_s(0.0);

    loop {
        match select4(
            pid_ticker.next(),
            telemetry_ticker.next(),
            Timer::at(watchdog_deadline),
            MOTOR_COMMAND_CHANNEL.receive(),
        )
        .await
        {
            embassy_futures::select::Either4::First(_) => {
                // Run PID update
                left_motor.step(PID_CONTROL_LOOP_PERIOD_MS).await;
                right_motor.step(PID_CONTROL_LOOP_PERIOD_MS).await;
            }
            embassy_futures::select::Either4::Second(_) => {
                // Send a value to the data offload link
                let _ = DATA_OFFLOAD_CHANNEL.try_send(Message::DriveBaseState(DriveBaseState {
                    left: left_motor.joint_state.clone(),
                    right: right_motor.joint_state.clone(),
                }));
            }
            embassy_futures::select::Either4::Third(_) => {
                // Watchdog timeout, stop the motors and sleep the drive base
                sleep.set_low();
                left_motor.set_setpoint_rad_per_s(0.0);
                right_motor.set_setpoint_rad_per_s(0.0);
                // Push deadline far into the future so it doesn't re-fire immediately
                watchdog_deadline = Instant::now() + Duration::from_secs(10000);
            }
            embassy_futures::select::Either4::Fourth(command) => {
                // Command received, feed the watchdog
                watchdog_deadline = Instant::now() + Duration::from_secs(WATCH_DOG_TIMEOUT);
                // Handle the command
                sleep.set_high();
                left_motor.set_setpoint_rad_per_s(command.left_velocity_rad);
                right_motor.set_setpoint_rad_per_s(command.right_velocity_rad);
            }
        }
    }
}

pub async fn init(
    spawner: Spawner,
    motor_driver_r: DRV8833Resources,
    encoder_driver_r: EncoderDriverResources,
    left_encoder_r: LeftEncoderResources,
    right_encoder_r: RightEncoderResources,
) {
    spawner.spawn(motor_task(encoder_driver_r, left_encoder_r, right_encoder_r, motor_driver_r).unwrap());
}
