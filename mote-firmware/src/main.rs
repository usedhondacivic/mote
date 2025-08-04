#![no_std]
#![no_main]
#![allow(async_fn_in_trait)]
#![feature(impl_trait_in_assoc_type)]

use embassy_executor::Spawner;
use {defmt_rtt as _, panic_probe as _};

use crate::tasks::{AssignedResources, Cyw43Resources, RplidarC1Resources, lidar, wifi};

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

    wifi::init(spawner, r.wifi).await;
    lidar::init(spawner, r.lidar_uart).await
}
