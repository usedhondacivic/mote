use embassy_rp::flash::{Blocking, Flash, ERASE_SIZE};
use embassy_rp::peripherals::FLASH;
use embassy_rp::Peri;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use mote_api::messages::host_to_mote::SetNetworkConnectionConfig;
use serde::{Deserialize, Serialize};

const FLASH_SIZE: usize = 2 * 1024 * 1024;
const CONFIG_OFFSET: u32 = (FLASH_SIZE - ERASE_SIZE) as u32;
const MAGIC: u32 = 0xC0_FF_EE_42;
/// Header: 4 bytes magic + 2 bytes data_len
const HEADER_SIZE: usize = 6;
/// Must fit encoded StoredConfig. WiFi credentials are ~100 bytes encoded;
/// 256 gives headroom for future fields.
const SCRATCH_SIZE: usize = 256;

/// All data persisted to flash. Add fields here (with `#[serde(default)]`) as new
/// configuration categories are introduced (e.g. IMU biases).
#[derive(Serialize, Deserialize)]
struct StoredConfig {
    wifi: Option<SetNetworkConnectionConfig>,
}

struct FlashConfig {
    flash: Flash<'static, FLASH, Blocking, FLASH_SIZE>,
}

static FLASH_CONFIG: Mutex<CriticalSectionRawMutex, Option<FlashConfig>> = Mutex::new(None);

/// Initialize the flash config singleton. Must be called once before any load/save calls.
pub async fn init(flash: Peri<'static, FLASH>) {
    *FLASH_CONFIG.lock().await = Some(FlashConfig {
        flash: Flash::new_blocking(flash),
    });
}

/// Load the saved WiFi credentials from flash, if any.
pub async fn load_wifi() -> Option<SetNetworkConnectionConfig> {
    FLASH_CONFIG.lock().await.as_mut()?.load_wifi()
}

/// Save WiFi credentials to flash, preserving any other stored config fields.
pub async fn save_wifi(wifi: SetNetworkConnectionConfig) {
    if let Some(config) = FLASH_CONFIG.lock().await.as_mut() {
        config.save_wifi(wifi);
    } else {
        defmt::error!("flash_config::save_wifi called before init");
    }
}

impl FlashConfig {
    fn load(&mut self) -> Option<StoredConfig> {
        let mut header = [0u8; HEADER_SIZE];
        self.flash.blocking_read(CONFIG_OFFSET, &mut header).ok()?;

        let magic = u32::from_le_bytes(header[..4].try_into().unwrap());
        if magic != MAGIC {
            return None;
        }

        let data_len = u16::from_le_bytes(header[4..6].try_into().unwrap()) as usize;
        if data_len > SCRATCH_SIZE - HEADER_SIZE {
            return None;
        }

        let mut buf = [0u8; SCRATCH_SIZE];
        self.flash
            .blocking_read(CONFIG_OFFSET + HEADER_SIZE as u32, &mut buf[..data_len])
            .ok()?;

        bitcode::deserialize(&buf[..data_len]).ok()
    }

    fn save(&mut self, config: StoredConfig) {
        let encoded = bitcode::serialize(&config).expect("StoredConfig serialization failed");

        if encoded.len() > SCRATCH_SIZE - HEADER_SIZE {
            defmt::error!("flash_config: encoded size {} exceeds scratch buffer", encoded.len());
            return;
        }

        let mut buf = [0u8; SCRATCH_SIZE];
        buf[..4].copy_from_slice(&MAGIC.to_le_bytes());
        buf[4..6].copy_from_slice(&(encoded.len() as u16).to_le_bytes());
        buf[HEADER_SIZE..HEADER_SIZE + encoded.len()].copy_from_slice(&encoded);

        if let Err(_) = self.flash.blocking_erase(CONFIG_OFFSET, CONFIG_OFFSET + ERASE_SIZE as u32) {
            defmt::error!("flash_config: erase failed");
            return;
        }
        if let Err(_) = self.flash.blocking_write(CONFIG_OFFSET, &buf) {
            defmt::error!("flash_config: write failed");
        }
    }

    fn load_wifi(&mut self) -> Option<SetNetworkConnectionConfig> {
        self.load()?.wifi
    }

    fn save_wifi(&mut self, wifi: SetNetworkConnectionConfig) {
        // Load first to preserve any other stored fields
        let mut config = self.load().unwrap_or(StoredConfig { wifi: None });
        config.wifi = Some(wifi);
        self.save(config);
    }
}
