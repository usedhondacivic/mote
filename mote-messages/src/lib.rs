#![no_std]

// Messages used by Mote for firmware <--> host communication

use serde::{Deserialize, Serialize};
use serde_with::serde_as;

// Lidar Data
#[derive(Serialize, Deserialize, Debug)]
pub struct Point {
    pub quality: u8,
    pub angle: u16,
    pub distance: u16,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Scan<const N: usize> {
    #[serde_as(as = "[_; N]")]
    pub points: [Point; N],
}

// Collector enums

#[derive(Serialize, Deserialize, Debug)]
pub enum MoteToHostMessage {
    Ping,
    PingResponse,
    Scan(Scan<100>),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum HostToMoteMessage {
    Ping,
    PingResponse,
}
