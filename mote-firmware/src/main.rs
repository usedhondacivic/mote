#![no_std]
#![no_main]
#![allow(async_fn_in_trait)]
#![feature(impl_trait_in_assoc_type)]

use embassy_executor::Spawner;
use {defmt_rtt as _, panic_probe as _};

use crate::tasks::{
    AssignedResources, CONFIGURATION_STATE, Cyw43Resources, RplidarC1Resources, UsbSerialResources, lidar, usb_serial,
    wifi,
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

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let r = split_resources!(p);

    /* Set initial configuration state */
    {
        let mut configuration_state = CONFIGURATION_STATE.lock().await;
        (*configuration_state).uid = heapless::String::try_from("mote-:3").expect("Failed to assign to uid.");
    }

    usb_serial::init(spawner, r.usb_serial).await;
    wifi::init(spawner, r.wifi).await;
    lidar::init(spawner, r.lidar_uart).await
}
