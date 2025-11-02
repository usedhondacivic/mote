#![no_std]

// Messages used by Mote for firmware <--> host communication

// Messages used during nominal operation
pub mod runtime {
    // Sensor and state data telemetered to the host
    pub mod mote_to_host {
        // Command responses
        pub mod command {
            use serde::{Deserialize, Serialize};

            #[derive(Serialize, Deserialize, Debug, defmt::Format, Clone)]
            pub enum Message {
                Ping,
                PingResponse,
            }
        }

        // Sensor data offload messages
        pub mod data_offload {
            use serde::{Deserialize, Serialize};

            pub const MAX_POINTS_PER_SCAN_MESSAGE: usize = 250;

            // Lidar Data
            #[derive(Serialize, Deserialize, Debug, defmt::Format, Clone)]
            pub struct Point {
                pub quality: u8,
                // Actual heading = angle / 64.0 degrees
                pub angle: u16,
                // Actual distance = distance / 4.0 mm
                pub distance: u16,
            }

            #[derive(Serialize, Deserialize, Debug, defmt::Format, Clone)]
            pub enum Message {
                Scan(heapless::Vec<Point, MAX_POINTS_PER_SCAN_MESSAGE>),
            }
        }
    }

    // Commands sent to mote
    pub mod host_to_mote {
        use serde::{Deserialize, Serialize};

        #[derive(Serialize, Deserialize, Debug, defmt::Format)]
        pub enum Subsystem {
            Lidar,
            Imu,
            DriveBase,
        }

        #[derive(Serialize, Deserialize, Debug, defmt::Format)]
        pub enum Message {
            Ping,
            PingResponse,
            Enable(Subsystem),
            Disable(Subsystem),
            SoftReset,
        }
    }
}

// Messages used for configuration / status / built in test
pub mod configuration {
    pub mod mote_to_host {

        use serde::{Deserialize, Serialize};

        #[derive(Serialize, Deserialize, Debug, defmt::Format, Clone)]
        pub struct NetworkConnection {
            pub ssid: heapless::String<32>,
            pub strength: u8, // rssi
        }
        #[derive(Serialize, Deserialize, Debug, defmt::Format, Clone)]
        pub enum BITResult {
            Waiting,
            Pass,
            Fail,
        }

        #[derive(Serialize, Deserialize, Debug, defmt::Format, Clone)]
        pub struct BIT {
            pub name: heapless::String<20>,
            pub result: BITResult,
        }

        pub type BITList = heapless::Vec<BIT, 5>;

        #[derive(Serialize, Deserialize, Debug, defmt::Format, Clone)]
        pub struct BITCollection {
            pub wifi: BITList,
            pub lidar: BITList,
            pub imu: BITList,
            pub encoders: BITList,
        }

        pub type UID = heapless::String<25>;

        #[derive(Serialize, Deserialize, Debug, defmt::Format, Clone)]
        pub struct State {
            pub uid: UID,
            pub ip: Option<heapless::String<20>>,
            pub current_network_connection: Option<heapless::String<32>>,
            pub available_network_connections: heapless::Vec<NetworkConnection, 10>,
            pub built_in_test: BITCollection,
        }

        #[derive(Serialize, Deserialize, Debug, defmt::Format)]
        pub enum Message {
            State(State),
        }
    }

    pub mod host_to_mote {
        use serde::{Deserialize, Serialize};

        use crate::configuration::mote_to_host::UID;

        #[derive(Serialize, Deserialize, Debug, defmt::Format)]
        pub struct SetNetworkConnectionConfig {
            pub ssid: heapless::String<32>,
            pub password: heapless::String<64>,
        }

        #[derive(Serialize, Deserialize, Debug, defmt::Format)]
        pub struct SetUID {
            pub uid: UID,
        }

        #[derive(Serialize, Deserialize, Debug, defmt::Format)]
        pub enum Message {
            SetNetworkConnectionConfig(SetNetworkConnectionConfig),
            SetUID(SetUID),
            RequestNetworkScan,
        }
    }
}
