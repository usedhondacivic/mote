//!  Command messages sent to Mote

use alloc::string::String;
use serde::{Deserialize, Serialize};

#[cfg(feature = "schemars")]
use schemars::JsonSchema;

// CONFIGURATION MESSAGES

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

// RUNTIME MESSAGES

#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SetDriveBaseVelocity {
    pub left_velocity_rad: f32,
    pub right_velocity_rad: f32,
}

#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Message {
    Ping,
    Pong,
    RequestNetworkScan,
    SetNetworkConnectionConfig(SetNetworkConnectionConfig),
    SetUID(SetUID),
    DriveBaseCommand(SetDriveBaseVelocity),
}
