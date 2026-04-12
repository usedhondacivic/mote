mod led_driver;

use embassy_executor::Spawner;
use embassy_rp::peripherals::PIO2;
use embassy_rp::pio::Pio;
use embassy_rp::pio_programs::ws2812::PioWs2812Program;
use embassy_time::{Duration, Timer};
use led_driver::{LedDriver, colors};
use mote_api::messages::mote_to_host::{BITList, BITResult};
// use smart_leds::brightness;
use static_cell::StaticCell;

use super::{Irqs, StatusLedResources};
use crate::tasks::CONFIGURATION_STATE;

// ---------------------------------------------------------------------------
// LED indices
// ---------------------------------------------------------------------------
const NUM_LEDS: usize = 3;
// const LED_POWER: usize = 0;
// const LED_WIFI:  usize = 1;
// const LED_IMU:   usize = 2;

// ---------------------------------------------------------------------------
// Determine LED state
// ---------------------------------------------------------------------------
#[derive(PartialEq, Clone, Copy)]
enum LedState {
    Uninitialised,
    Pass,
    Waiting,
    Fail,
}

fn worst_result(results: &BITList) -> LedState {
    if results.is_empty() {
        return LedState::Uninitialised;
    }
    let mut state = LedState::Pass;
    for bit in results.iter() {
        match bit.result {
            BITResult::Fail => return LedState::Fail,
            BITResult::Waiting => state = LedState::Waiting,
            BITResult::Pass => {}
        }
    }
    state
}

// ---------------------------------------------------------------------------
// init
// ---------------------------------------------------------------------------
pub async fn init(spawner: Spawner, r: StatusLedResources) {
    static PROGRAM: StaticCell<PioWs2812Program<PIO2>> = StaticCell::new();

    let Pio { mut common, sm0, .. } = Pio::new(r.pio, Irqs);
    let program = PROGRAM.init(PioWs2812Program::new(&mut common));

    let driver: LedDriver<PIO2, 0, NUM_LEDS> = LedDriver::new(
        &mut common,
        sm0,
        r.dma, // Peri<'_, DMA_CH1>
        Irqs,  // satisfies the Binding<DMA_CH1::Interrupt, InterruptHandler<DMA_CH1>> bound
        r.tx,  // Peri<'_, PIN_19>
        program,
    );

    spawner.spawn(led_status_task(driver).expect("led_status_task already spawned"));
}

// ---------------------------------------------------------------------------
// Status task
// ---------------------------------------------------------------------------
#[embassy_executor::task]
async fn led_status_task(mut driver: LedDriver<'static, PIO2, 0, NUM_LEDS>) {
    driver.set_colors(&[colors::BLUE; NUM_LEDS]).await;
    let mut flash_state = false;

    loop {
        let (power_state, wifi_state, imu_state, lidar_state) = {
            let state = CONFIGURATION_STATE.lock().await;
            let bit = &state.built_in_test;
            (
                worst_result(&bit.power),
                worst_result(&bit.wifi),
                worst_result(&bit.imu),
                worst_result(&bit.lidar),
            )
        };

        flash_state = !flash_state;

        let color_for = |led_state: LedState| match led_state {
            LedState::Uninitialised => colors::BLUE,
            LedState::Pass => colors::GREEN,
            LedState::Waiting => colors::YELLOW,
            LedState::Fail => colors::RED,
        };

        let sensor_led = if flash_state {
            // On even ticks — show IMU color
            match imu_state {
                LedState::Uninitialised => colors::CYAN,
                LedState::Pass => colors::GREEN,
                LedState::Waiting => colors::ORANGE,
                LedState::Fail => colors::MAROON,
            }
        } else {
            // On odd ticks — show lidar color
            match lidar_state {
                LedState::Uninitialised => colors::OCEAN,
                LedState::Pass => colors::GREEN,
                LedState::Waiting => colors::PURPLE,
                LedState::Fail => colors::MAGENTA,
            }
        };

        // let colors_to_set = [color_for(power_state), color_for(wifi_state),
        // sensor_led];

        // let mut color_iter = colors_to_set.into_iter();

        // let colors_dimmeed = brightness(color_iter, 100);

        driver
            .set_colors(&[
                color_for(power_state), // LED 0
                color_for(wifi_state),  // LED 1
                sensor_led,             // LED 2
            ])
            .await;

        Timer::after(Duration::from_millis(1000)).await;
    }
}
