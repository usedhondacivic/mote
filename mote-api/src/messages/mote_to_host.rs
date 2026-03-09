//!  Sensor and state data telemetered to the host

use alloc::{string::String, vec::Vec};
use serde::{Deserialize, Serialize};

#[cfg(feature = "schemars")]
use schemars::JsonSchema;

// RUNTIME MESSEGES

// Lidar Data
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Point {
    pub quality: u8,
    pub angle_rads: f32,
    pub distance_mm: f32,
}

// CONFIGURATION MESSAGES

#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct NetworkConnection {
    pub ssid: String,
    pub strength: u8, // rssi
}

#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum BITResult {
    Waiting,
    Pass,
    Fail,
}

#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct BIT {
    pub name: String,
    pub result: BITResult,
}
pub type BITList = Vec<BIT>;

#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct BITCollection {
    pub power: BITList,
    pub wifi: BITList,
    pub lidar: BITList,
    pub imu: BITList,
    pub encoders: BITList,
}

pub type UID = String;

#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct State {
    pub uid: UID,
    pub ip: Option<String>,
    pub current_network_connection: Option<String>,
    pub available_network_connections: Vec<NetworkConnection>,
    pub built_in_test: BITCollection,
}

#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Message {
    Ping,
    Pong,
    Scan(Vec<Point>),
    State(State),
}
