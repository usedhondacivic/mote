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

struct PwmBridge<T: SetDutyCycle> {
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
    fn new(a1: T, a2: T, min_duty: u16) -> Self {
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
    fn forward(&mut self, percent: u8) -> Result<(), MotorDriverError> {
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

    fn reverse(&mut self, percent: u8) -> Result<(), MotorDriverError> {
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

    fn coast(&mut self) -> Result<(), MotorDriverError> {
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

    fn stop(&mut self) -> Result<(), MotorDriverError> {
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

struct PwmSplitDriver<T: SetDutyCycle> {
    a: PwmBridge<T>,
    b: PwmBridge<T>,
}

impl<T: SetDutyCycle> PwmSplitDriver<T> {
    fn new(a1: T, a2: T, b1: T, b2: T) -> Self {
        Self {
            a: PwmBridge::new(a1, a2, 0),
            b: PwmBridge::new(b1, b2, 0),
        }
    }
}

impl<T: SetDutyCycle> PwmSplitDriver<T> {
    fn set_min_duty(&mut self, duty: u16) {
        self.a.set_min_duty(duty);
        self.b.set_min_duty(duty);
    }
}

pub enum MotorChannel {
    ChannelA,
    ChannelB,
}

pub enum MotorDirection {
    Forward,
    Reverse,
}

pub enum MotorBrakeMode {
    Coast,
    Brake,
}

pub struct DRV8833Driver<T: SetDutyCycle, O: OutputPin> {
    driver: PwmSplitDriver<T>,
    sleep: O,
}

impl<T: SetDutyCycle, O: OutputPin> DRV8833Driver<T, O> {
    pub fn new(a1: T, a2: T, b1: T, b2: T, sleep: O) -> Self {
        Self {
            driver: PwmSplitDriver::new(a1, a2, b1, b2),
            sleep: sleep,
        }
    }

    /// Puts the device into a low power sleep state, In this state, the
    /// H-bridges are disabled, the gate drive charge pump is stopped, all
    /// internal logic is reset, and all internal clocks are stopped. All
    /// inputs are ignored until [MotorDriver::wakeup] is called.
    pub fn sleep(&mut self) -> Result<(), MotorDriverError> {
        self.sleep.set_low().map_err(|_| MotorDriverError::GpioError)
    }

    /// Wake up the device from sleep mode.
    pub fn wakeup(&mut self) -> Result<(), MotorDriverError> {
        self.sleep.set_high().map_err(|_| MotorDriverError::GpioError)
    }

    // Set the
    pub fn set(&mut self, channel: MotorChannel, speed: u8, direction: MotorDirection) -> Result<(), MotorDriverError> {
        let chan = match channel {
            MotorChannel::ChannelA => &mut self.driver.a,
            MotorChannel::ChannelB => &mut self.driver.b,
        };

        match direction {
            MotorDirection::Forward => chan.forward(speed),
            MotorDirection::Reverse => chan.reverse(speed),
        }
    }

    pub fn brake(&mut self, channel: MotorChannel, mode: MotorBrakeMode) -> Result<(), MotorDriverError> {
        let chan = match channel {
            MotorChannel::ChannelA => &mut self.driver.a,
            MotorChannel::ChannelB => &mut self.driver.b,
        };

        match mode {
            MotorBrakeMode::Coast => chan.coast(),
            MotorBrakeMode::Brake => chan.stop(),
        }
    }
}
