// Adapted from https://github.com/milewski/drv8833-driver, which requires std :(

use embedded_hal::digital::OutputPin;
use embedded_hal::pwm::SetDutyCycle;

pub fn remap(value: u8, min: u16, max: u16) -> u16 {
    let percentage = value as f32 / 100.0;
    let min = min as f32;
    let max = max as f32;

    (percentage * (max - min) + min) as u16
}

#[derive(Debug)]
pub enum MotorDriverError {
    /// Returned when fail to set pin low/high.
    GpioError,
    /// Returned when fail to set duty value.
    UnableToSetDuty,
    /// Returned when we are unable to acquire mutex lock.
    PwmLocked,
    /// Returned when in PWM mode and a duty value is not within 0-100 range.
    InvalidRange,
}

pub struct PwmBridge<T: SetDutyCycle> {
    bridge: Bridge<T>,
    min_duty: u16,
}

/// Holds the reference to each pin used to drive the motor forward or reverse.
struct Bridge<T: SetDutyCycle> {
    a1: T,
    a2: T,
}

impl<T: SetDutyCycle> Bridge<T> {
    fn new(a1: T, a2: T) -> Self {
        Self { a1, a2 }
    }
}

impl<T: SetDutyCycle> PwmBridge<T> {
    pub fn new(a1: T, a2: T, min_duty: u16) -> Self {
        Self {
            bridge: Bridge::new(a1, a2),
            min_duty,
        }
    }

    fn set_min_duty(&mut self, duty: u16) {
        self.min_duty = duty;
    }
}

impl<T: SetDutyCycle> PwmBridge<T> {
    pub fn forward(&mut self, percent: u8) -> Result<(), MotorDriverError> {
        let percent = remap(percent, self.min_duty, self.bridge.a1.max_duty_cycle());

        self.bridge
            .a1
            .set_duty_cycle(percent)
            .map_err(|_| MotorDriverError::UnableToSetDuty)?;

        self.bridge
            .a2
            .set_duty_cycle_fully_off()
            .map_err(|_| MotorDriverError::UnableToSetDuty)?;

        Ok(())
    }

    pub fn reverse(&mut self, percent: u8) -> Result<(), MotorDriverError> {
        let percent = remap(percent, self.min_duty, self.bridge.a2.max_duty_cycle());

        self.bridge
            .a1
            .set_duty_cycle_fully_off()
            .map_err(|_| MotorDriverError::UnableToSetDuty)?;

        self.bridge
            .a2
            .set_duty_cycle(percent)
            .map_err(|_| MotorDriverError::UnableToSetDuty)?;

        Ok(())
    }

    pub fn coast(&mut self) -> Result<(), MotorDriverError> {
        self.bridge
            .a1
            .set_duty_cycle_fully_off()
            .map_err(|_| MotorDriverError::GpioError)?;

        self.bridge
            .a2
            .set_duty_cycle_fully_off()
            .map_err(|_| MotorDriverError::GpioError)?;

        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), MotorDriverError> {
        self.bridge
            .a1
            .set_duty_cycle_fully_on()
            .map_err(|_| MotorDriverError::GpioError)?;
        self.bridge
            .a2
            .set_duty_cycle_fully_on()
            .map_err(|_| MotorDriverError::GpioError)?;

        Ok(())
    }
}
