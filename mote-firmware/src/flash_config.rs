use alloc::string::String;
use alloc::vec::Vec;

use embassy_rp::Peri;
use embassy_rp::flash::{Blocking, ERASE_SIZE, Flash};
use embassy_rp::peripherals::FLASH;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use mote_api::messages::host_to_mote::SetNetworkConnectionConfig;
use serde::{Deserialize, Serialize};

const FLASH_SIZE: usize = 2 * 1024 * 1024;
const CONFIG_OFFSET: u32 = (FLASH_SIZE - ERASE_SIZE) as u32;
const MAGIC: u32 = 0xC0_FF_EE_42;
/// Header: 4 bytes magic + 2 bytes data_len
const HEADER_SIZE: usize = 6;
const SCRATCH_SIZE: usize = ERASE_SIZE;

const MAX_SAVED_WIFI_NETWORKS: usize = 3;

/// All data persisted to flash. Add fields here (with `#[serde(default)]`) as
/// new configuration categories are introduced (e.g. IMU biases).
#[derive(Serialize, Deserialize, Default)]
struct StoredConfig {
    /// Saved WiFi networks, most recently connected first.
    #[serde(default)]
    wifi: Vec<SetNetworkConnectionConfig>,
    /// User-assigned device identifier.
    #[serde(default)]
    uid: Option<String>,
}

struct FlashConfig {
    flash: Flash<'static, FLASH, Blocking, FLASH_SIZE>,
    scratch: [u8; SCRATCH_SIZE],
}

static FLASH_CONFIG: Mutex<CriticalSectionRawMutex, Option<FlashConfig>> = Mutex::new(None);

/// Initialize the flash config singleton. Must be called before any other
/// flash_config methods.
pub async fn init(flash: Peri<'static, FLASH>) {
    *FLASH_CONFIG.lock().await = Some(FlashConfig {
        flash: Flash::new_blocking(flash),
        scratch: [0u8; SCRATCH_SIZE],
    });
}

/// Load saved WiFi networks from flash, most recently connected first.
pub async fn load_wifi() -> Vec<SetNetworkConnectionConfig> {
    FLASH_CONFIG
        .lock()
        .await
        .as_mut()
        .map(|c| c.load_wifi())
        .unwrap_or_default()
}

/// Load the saved UID from flash, if any.
pub async fn load_uid() -> Option<String> {
    FLASH_CONFIG.lock().await.as_mut()?.load_uid()
}

/// Save the UID to flash.
pub async fn save_uid(uid: String) {
    if let Some(config) = FLASH_CONFIG.lock().await.as_mut() {
        config.save_uid(uid);
    } else {
        defmt::error!("flash_config::save_uid called before init");
    }
}

/// Save WiFi credentials to flash.
pub async fn save_wifi(wifi: SetNetworkConnectionConfig) {
    if let Some(config) = FLASH_CONFIG.lock().await.as_mut() {
        config.save_wifi(wifi);
    } else {
        defmt::error!("flash_config::save_wifi called before init");
    }
}

impl FlashConfig {
    fn load(&mut self) -> Option<StoredConfig> {
        self.flash
            .blocking_read(CONFIG_OFFSET, &mut self.scratch[..HEADER_SIZE])
            .ok()?;

        let magic = u32::from_le_bytes(self.scratch[..4].try_into().unwrap());
        if magic != MAGIC {
            return None;
        }

        let data_len = u16::from_le_bytes(self.scratch[4..6].try_into().unwrap()) as usize;
        if data_len > SCRATCH_SIZE - HEADER_SIZE {
            return None;
        }

        self.flash
            .blocking_read(CONFIG_OFFSET + HEADER_SIZE as u32, &mut self.scratch[..data_len])
            .ok()?;

        bitcode::deserialize(&self.scratch[..data_len]).ok()
    }

    fn save(&mut self, config: StoredConfig) {
        let encoded = bitcode::serialize(&config).expect("StoredConfig serialization failed");

        if encoded.len() > SCRATCH_SIZE - HEADER_SIZE {
            defmt::error!("flash_config: encoded size {} exceeds scratch buffer", encoded.len());
            return;
        }

        self.scratch.fill(0);
        self.scratch[..4].copy_from_slice(&MAGIC.to_le_bytes());
        self.scratch[4..6].copy_from_slice(&(encoded.len() as u16).to_le_bytes());
        self.scratch[HEADER_SIZE..HEADER_SIZE + encoded.len()].copy_from_slice(&encoded);

        if self
            .flash
            .blocking_erase(CONFIG_OFFSET, CONFIG_OFFSET + ERASE_SIZE as u32)
            .is_err()
        {
            defmt::error!("flash_config: erase failed");
            return;
        }
        if self.flash.blocking_write(CONFIG_OFFSET, &self.scratch).is_err() {
            defmt::error!("flash_config: write failed");
        }
    }

    fn load_uid(&mut self) -> Option<String> {
        self.load()?.uid
    }

    fn save_uid(&mut self, uid: String) {
        let mut config = self.load().unwrap_or_default();
        config.uid = Some(uid);
        self.save(config);
    }

    fn load_wifi(&mut self) -> Vec<SetNetworkConnectionConfig> {
        self.load().map(|c| c.wifi).unwrap_or_default()
    }

    fn save_wifi(&mut self, wifi: SetNetworkConnectionConfig) {
        let mut config = self.load().unwrap_or_default();
        // Remove any existing entry for this SSID so it doesn't appear twice
        config.wifi.retain(|n| n.ssid != wifi.ssid);
        // Most recently connected goes first
        config.wifi.insert(0, wifi);
        config.wifi.truncate(MAX_SAVED_WIFI_NETWORKS);
        self.save(config);
    }
}
