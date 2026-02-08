//!  Command messages sent to Mote

use alloc::{boxed::Box, string::String};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

trait HostToMoteMessage: Serialize + DeserializeOwned {}

// RUNTIME MESSAGES

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Ping {
    Ping,
    Pong,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SetNetworkConnectionConfig {
    pub ssid: String,
    pub password: String,
}

pub type UID = String;
#[derive(Serialize, Deserialize, Debug)]
pub struct SetUID {
    pub uid: UID,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RequestNetworkScan;

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

// Everything that can be sent from the host to Mote
impl HostToMoteMessage for Ping {}
impl HostToMoteMessage for SetNetworkConnectionConfig {}
impl HostToMoteMessage for SetUID {}

pub type Message = Box<dyn HostToMoteMessage>;
