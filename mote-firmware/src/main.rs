#![no_std]
#![no_main]
#![allow(async_fn_in_trait)]
#![feature(impl_trait_in_assoc_type)]
// Used minimally in the LiDAR driver, still very unstable
#![feature(generic_const_exprs)]
#![feature(core_intrinsics)]

use defmt::info;
use embassy_executor::{Executor, Spawner};
use embassy_rp::clocks::{ClockConfig, CoreVoltage, clk_sys_freq};
use embassy_rp::config::Config;
use embassy_rp::multicore::{Stack, spawn_core1};
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

use crate::tasks::{
    AssignedResources, CONFIGURATION_STATE, Cyw43Resources, DRV8833Resources, EncoderDriverResources, ImuResources,
    LeftEncoderResources, RightEncoderResources, RplidarC1Resources, StatusLedResources, UsbSerialResources,
    drive_base, lidar, usb_serial, wifi,
};

mod helpers;
mod tasks;

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

    wifi::init(spawner, r).await;
    info!("Wifi INIT complete");
}

#[embassy_executor::task]
async fn core1_task(
    spawner: Spawner,
    r_usb: UsbSerialResources,
    r_lidar: RplidarC1Resources,
    encoder_driver_r: EncoderDriverResources,
    left_encoder_r: LeftEncoderResources,
    right_encoder_r: RightEncoderResources,
    motor_driver_r: DRV8833Resources,
) {
    info!("Core 1 spawned");

    /* Set initial configuration state */
    {
        let mut configuration_state = CONFIGURATION_STATE.lock().await;
        (*configuration_state).uid = heapless::String::try_from("mote-:3").expect("Failed to assign to uid.");

        // TODO: read / write wifi configuration to flash, then use it to update
        // config
    }

    usb_serial::init(spawner, r_usb).await;
    info!("USB Serial INIT complete");

    lidar::init(spawner, r_lidar).await;
    info!("LiDAR INIT complete");

    drive_base::init(
        spawner,
        motor_driver_r,
        encoder_driver_r,
        left_encoder_r,
        right_encoder_r,
    )
    .await;
    // info!("Drive base INIT complete");
}
