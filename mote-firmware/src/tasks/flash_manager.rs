use alloc::string::String;

use defmt::info;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;

use crate::flash_config;
use crate::tasks::{CONFIGURATION_STATE, FlashResources};

pub enum FlashSaveRequest {
    Uid(String),
}

/// Send flash save requests here from any core. The flash_manager_task drains
/// this channel on Core 0, where flash erase/write is safe.
pub static FLASH_SAVE_CHANNEL: Channel<CriticalSectionRawMutex, FlashSaveRequest, 4> = Channel::new();

#[embassy_executor::task]
pub async fn flash_manager_task(flash_r: FlashResources) -> ! {
    flash_config::init(flash_r.flash).await;

    {
        // No saved UID — derive one from the RP2350 OTP chip ID so it is
        // stable across reboots and unique per device.
        let uid = flash_config::load_uid().await.unwrap_or_else(|| {
            let default_uid = embassy_rp::otp::get_chipid()
                .map(|id| alloc::format!("mote-{:016x}", id))
                .unwrap_or("mote-unknown".into());
            info!("No saved UID, using chip-derived default: {}", default_uid.as_str());
            default_uid
        });
        CONFIGURATION_STATE.lock().await.uid = uid;
    }

    loop {
        match FLASH_SAVE_CHANNEL.receive().await {
            FlashSaveRequest::Uid(uid) => {
                flash_config::save_uid(uid).await;
            }
        }
    }
}
