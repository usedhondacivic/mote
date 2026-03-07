//!  Command messages sent to Mote

use alloc::string::String;
use serde::{Deserialize, Serialize};

#[cfg(feature = "schemars")]
use schemars::JsonSchema;

// RUNTIME MESSAGES

#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SetNetworkConnectionConfig {
    pub ssid: String,
    pub password: String,
}

#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SetUID {
    pub uid: String,
}

#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[derive(Serialize, Deserialize, Debug)]
pub enum Subsystem {
    Lidar,
    Imu,
    DriveBase,
}

#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[derive(Serialize, Deserialize, Debug)]
pub struct SetEnabled {
    pub subsystem: Subsystem,
    pub enable: bool,
}

#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[derive(Serialize, Deserialize, Debug)]
pub struct SoftReset;

#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Message {
    Ping,
    Pong,
    RequestNetworkScan,
    SetNetworkConnectionConfig(SetNetworkConnectionConfig),
    SetUID(SetUID),
}
