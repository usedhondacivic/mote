use core::cmp::{max, min};

use embassy_executor::Spawner;
use embassy_rp::adc::{Adc, Channel, Config};
use embassy_rp::gpio::Pull;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::watch::Watch;
use embassy_time::Timer;

use crate::tasks::{Irqs, UsbPowerDetectionResources};

#[derive(Clone, PartialEq)]
enum PowerState {
    Invalid,
    Disconnected,
    Max500ma,
    Max1_5a,
    Max3a,
}

impl From<u16> for PowerState {
    fn from(value: u16) -> Self {
        let v = (value as f32 * 3.3) / 4096.0;
        match v {
            v if v > 0.0 && v < 0.15 => Self::Disconnected,
            v if v > 0.25 && v < 0.61 => Self::Max500ma,
            v if v > 0.70 && v < 1.16 => Self::Max1_5a,
            v if v > 1.31 => Self::Max3a,
            _ => Self::Invalid,
        }
    }
}

static POWER_GATE_WATCH: Watch<CriticalSectionRawMutex, PowerState, 2> = Watch::new();

#[embassy_executor::task]
async fn power_gate_task(r: UsbPowerDetectionResources) -> ! {
    let mut adc = Adc::new(r.adc, Irqs, Config::default());
    let mut cc1 = Channel::new_pin(r.cc1, Pull::Down);
    let mut cc2 = Channel::new_pin(r.cc2, Pull::Down);

    let sender = POWER_GATE_WATCH.sender();

    loop {
        let cc1_reading = adc.read(&mut cc1).await.unwrap();
        let cc2_reading = adc.read(&mut cc2).await.unwrap();

        let lower = PowerState::from(min(cc1_reading, cc2_reading));
        let upper = PowerState::from(max(cc1_reading, cc2_reading));

        // https://global.discourse-cdn.com/digikey/original/3X/c/9/c9109631c71df719fc2dd3c426ccf3c69949f388.png
        let state = match (lower, upper) {
            (PowerState::Invalid, _) => PowerState::Invalid,
            (_, PowerState::Invalid) => PowerState::Invalid,
            (PowerState::Disconnected, PowerState::Disconnected) => PowerState::Disconnected,
            (PowerState::Disconnected, PowerState::Max500ma) => PowerState::Max500ma,
            (PowerState::Disconnected, PowerState::Max1_5a) => PowerState::Max1_5a,
            (PowerState::Disconnected, PowerState::Max3a) => PowerState::Max3a,
            _ => PowerState::Invalid,
        };

        sender.send(state);

        Timer::after_secs(1).await;
    }
}

pub async fn init(spawner: Spawner, r: UsbPowerDetectionResources) {
    spawner.spawn(power_gate_task(r)).unwrap();
}

// Block thread until 1.5A capability is advertised on USB cc1
// and cc2 pins
pub async fn gate_1_5_amp() {
    POWER_GATE_WATCH
        .receiver()
        .unwrap()
        .get_and(|v| *v == PowerState::Max1_5a || *v == PowerState::Max3a)
        .await;
}

// Block thread until 3A capability is advertised on USB cc1 and
// cc2 pins
pub async fn gate_3_amp() {
    POWER_GATE_WATCH
        .receiver()
        .unwrap()
        .get_and(|v| *v == PowerState::Max1_5a || *v == PowerState::Max3a)
        .await;
}
