use alloc::string::String;

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;

use crate::flash_config;

pub enum FlashSaveRequest {
    Uid(String),
}

/// Send flash save requests here from any core. The flash_manager_task drains
/// this channel on Core 0, where flash erase/write is safe.
pub static FLASH_SAVE_CHANNEL: Channel<CriticalSectionRawMutex, FlashSaveRequest, 4> = Channel::new();

#[embassy_executor::task]
pub async fn flash_manager_task() -> ! {
    loop {
        match FLASH_SAVE_CHANNEL.receive().await {
            FlashSaveRequest::Uid(uid) => {
                flash_config::save_uid(uid).await;
            }
        }
    }
}
