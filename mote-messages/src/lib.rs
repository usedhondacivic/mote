#![no_std]

// Messages used by Mote for firmware <--> host communication

// Messages used during nominal operation
pub mod runtime {

    // Sensor and state data telemetered to the host
    pub mod mote_to_host {
        use serde::{Deserialize, Serialize};

        pub const MAX_POINTS_PER_SCAN_MESSAGE: usize = 200;

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
            Ping,
            PingResponse,
            Scan(heapless::Vec<Point, MAX_POINTS_PER_SCAN_MESSAGE>),
        }
    }

    // Commands sent to mote
    pub mod host_to_mote {
        use serde::{Deserialize, Serialize};

        #[derive(Serialize, Deserialize, Debug, defmt::Format)]
        pub enum Message {
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
    }
}

// Messages used for configuration / status / built in test
pub mod configuration {
    pub mod mote_to_host {
        use serde::{Deserialize, Serialize};

        // TODO: These should have richer types
        #[derive(Serialize, Deserialize, Debug, defmt::Format)]
        pub struct BIT {
            pub lidar: bool,
            pub imu: bool,
            pub wifi: bool,
            pub encoders: bool,
        }

        #[derive(Serialize, Deserialize, Debug, defmt::Format)]
        pub struct NetworkConnection {
            pub ssid: heapless::String<32>,
            pub strength: u8,
            pub connected: bool,
        }

        #[derive(Serialize, Deserialize, Debug, defmt::Format)]
        pub struct State {
            pub built_in_test: BIT,
            pub current_network_connection: Option<NetworkConnection>,
            pub available_network_connections: heapless::Vec<NetworkConnection, 10>,
        }

        #[derive(Serialize, Deserialize, Debug, defmt::Format)]
        pub enum Message {
            State(State),
        }
    }

    pub mod host_to_mote {
        use serde::{Deserialize, Serialize};

        #[derive(Serialize, Deserialize, Debug, defmt::Format)]
        pub struct SetNetworkConnectionConfig {
            pub ssid: heapless::String<32>,
            pub password: heapless::String<64>,
        }

        #[derive(Serialize, Deserialize, Debug, defmt::Format)]
        pub struct SetUID {
            pub uid: heapless::String<10>,
        }

        #[derive(Serialize, Deserialize, Debug, defmt::Format)]
        pub enum Message {
            SetNetworkConnectionConfig(SetNetworkConnectionConfig),
            SetUID(SetUID),
        }
    }
}
