#![no_std]
#![no_main]
#![allow(async_fn_in_trait)]
#![feature(impl_trait_in_assoc_type)]
// Used minimally in the LiDAR driver, still very unstable
#![feature(generic_const_exprs)]

extern crate alloc;

use defmt::info;
use embassy_executor::{Executor, Spawner};
use embassy_rp::clocks::{ClockConfig, CoreVoltage, clk_sys_freq};
use embassy_rp::config::Config;
use embassy_rp::multicore::{Stack, spawn_core1};
use embedded_alloc::LlffHeap as Heap;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

use crate::tasks::*;

mod helpers;
mod tasks;

#[global_allocator]
static HEAP: Heap = Heap::empty();

const HEAP_SIZE: usize = 100 * 1024;

// Program metadata for `picotool info`.
#[unsafe(link_section = ".bi_entries")]
#[used]
pub static PICOTOOL_ENTRIES: [embassy_rp::binary_info::EntryAddr; 4] = [
    embassy_rp::binary_info::rp_program_name!(c"Mote"),
    embassy_rp::binary_info::rp_program_description!(c"A low cost, high confidence robot for education"),
    embassy_rp::binary_info::rp_cargo_version!(),
    embassy_rp::binary_info::rp_program_build_attribute!(),
];

static mut CORE1_STACK: Stack<10000> = Stack::new();
static EXECUTOR0: StaticCell<Executor> = StaticCell::new();
static EXECUTOR1: StaticCell<Executor> = StaticCell::new();

#[cortex_m_rt::entry]
fn main() -> ! {
    // Set up for clock frequency of 200 MHz, setting all necessary defaults.
    let mut config = Config::new(ClockConfig::system_freq(200_000_000).unwrap());
    config.clocks.core_voltage = CoreVoltage::V1_15;

    let p = embassy_rp::init(config);
    let r = split_resources!(p);

    info!("System clock frequency: {} MHz", clk_sys_freq() / 1_000_000);

    // Init the allocator
    unsafe {
        embedded_alloc::init!(HEAP, HEAP_SIZE);
    }

    spawn_core1(
        p.CORE1,
        unsafe { &mut *core::ptr::addr_of_mut!(CORE1_STACK) },
        move || {
            let executor1 = EXECUTOR1.init(Executor::new());
            executor1.run(|spawner| {
                spawner
                    .spawn(core1_task(
                        spawner,
                        r.usb_serial,
                        r.lidar_uart,
                        r.encoder_driver,
                        r.left_encoder,
                        r.right_encoder,
                        r.drv8833_resources,
                        r.usb_power_detection,
                    ))
                    .unwrap()
            });
        },
    );

    let executor0 = EXECUTOR0.init(Executor::new());

    executor0.run(|spawner| spawner.spawn(core0_task(spawner, r.wifi)).unwrap());
}

#[embassy_executor::task]
async fn core0_task(spawner: Spawner, r: Cyw43Resources) {
    info!("Core 0 spawned");

    info!("Gating on 1.5A capable before starting WIFI");
    power_gate::gate_1_5_amp().await;

    wifi::init(spawner, r).await;
    info!("Wifi INIT complete");
}

#[embassy_executor::task]
async fn core1_task(
    spawner: Spawner,
    usb_r: UsbSerialResources,
    lidar_r: RplidarC1Resources,
    encoder_driver_r: EncoderDriverResources,
    left_encoder_r: LeftEncoderResources,
    right_encoder_r: RightEncoderResources,
    motor_driver_r: DRV8833Resources,
    usb_power_r: UsbPowerDetectionResources,
) {
    info!("Core 1 spawned");

    /* Set initial configuration state */
    {
        let mut configuration_state = CONFIGURATION_STATE.lock().await;
        configuration_state.uid = "mote-:3".into();

        // TODO: read / write wifi configuration to flash, then use it to update
        // config
    }

    usb_serial::init(spawner, usb_r).await;
    info!("USB Serial INIT complete");

    power_gate::init(spawner, usb_power_r).await;
    info!("Power Gate INIT complete");

    info!("Gating on 1.5A capable before starting LiDAR");
    power_gate::gate_1_5_amp().await;
    info!("Power supply is 1.5A capable");

    lidar::init(spawner, lidar_r).await;
    info!("LiDAR INIT complete");

    info!("Gating on 3A capable before starting drive base");
    power_gate::gate_3_amp().await;
    info!("Power supply is 3A capable");

    drive_base::init(
        spawner,
        motor_driver_r,
        encoder_driver_r,
        left_encoder_r,
        right_encoder_r,
    )
    .await;
    info!("Drive base INIT complete");
}
