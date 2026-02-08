//!  Sensor and state data telemetered to the host

use alloc::{boxed::Box, string::String, vec::Vec};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

trait MoteToHostMessage: Serialize + DeserializeOwned {}

// RUNTIME MESSAGES

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Ping {
    Ping,
    Pong,
}

// Lidar Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Point {
    pub quality: u8,
    pub angle_rads: f32,
    pub distance_mm: f32,
}

// CONFIGURATION MESSAGES

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NetworkConnection {
    pub ssid: String,
    pub strength: u8, // rssi
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum BITResult {
    Waiting,
    Pass,
    Fail,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BIT {
    pub name: String,
    pub result: BITResult,
}
pub type BITList = Vec<BIT>;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BITCollection {
    pub wifi: BITList,
    pub lidar: BITList,
    pub imu: BITList,
    pub encoders: BITList,
}

pub type UID = String;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct State {
    pub uid: UID,
    pub ip: Option<String>,
    pub current_network_connection: Option<String>,
    pub available_network_connections: Vec<NetworkConnection>,
    pub built_in_test: BITCollection,
}

// Everything that can be sent from Mote to the host
impl MoteToHostMessage for Point {}
impl MoteToHostMessage for State {}

pub type Message = Box<dyn MoteToHostMessage>;
