#![no_std]

// Messages used by Mote for firmware <--> host communication

use serde::{Deserialize, Serialize};

pub const MAX_POINTS_PER_SCAN_MESSAGE: usize = 200;

// Lidar Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Point {
    pub quality: u8,
    // Actual heading = angle / 64.0 degrees
    pub angle: u16,
    // Actual distance = distance / 4.0 mm
    pub distance: u16,
}

// Collector enums
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MoteToHostMessage {
    Ping,
    PingResponse,
    Scan(heapless::Vec<Point, MAX_POINTS_PER_SCAN_MESSAGE>),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum HostToMoteMessage {
    Ping,
    PingResponse,
    EnableLidar,
    DisableLidar,
    EnableImu,
    DisableImu,
    EnableEncoders,
    DisableEncoders,
    EnableMotors,
    DisableMotors,
    SoftReset,
}
