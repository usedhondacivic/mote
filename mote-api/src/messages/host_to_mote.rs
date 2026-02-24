//!  Command messages sent to Mote

use alloc::string::String;
use serde::{Deserialize, Serialize};

// RUNTIME MESSAGES

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SetNetworkConnectionConfig {
    pub ssid: String,
    pub password: String,
}

pub type UID = String;
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SetUID {
    pub uid: UID,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Subsystem {
    Lidar,
    Imu,
    DriveBase,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SetEnabled {
    pub subsystem: Subsystem,
    pub enable: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SoftReset;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Message {
    Ping,
    Pong,
    RequestNetworkScan,
    SetNetworkConnectionConfig(SetNetworkConnectionConfig),
    SetUID(SetUID),
}
