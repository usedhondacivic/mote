#![no_std]

// Messages used by Mote for firmware <--> host communication

// Messages used during nominal operation
pub mod runtime {

    // Sensor and state data telemetered to the host
    pub mod mote_to_host {
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

        #[derive(Serialize, Deserialize, Debug, Clone)]
        pub enum Message {
            Ping,
            PingResponse,
            Scan(heapless::Vec<Point, MAX_POINTS_PER_SCAN_MESSAGE>),
        }
    }

    // Commands sent to mote
    pub mod host_to_mote {
        use serde::{Deserialize, Serialize};

        #[derive(Serialize, Deserialize, Debug)]
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
        #[derive(Serialize, Deserialize, Debug)]
        pub struct BIT {
            pub lidar: bool,
            pub imu: bool,
            pub wifi: bool,
            pub encoders: bool,
        }

        #[derive(Serialize, Deserialize, Debug)]
        pub struct NetworkConnection {
            pub ssid: heapless::String<32>,
            pub connected: bool,
        }

        #[derive(Serialize, Deserialize, Debug)]
        pub struct State {
            pub built_in_test: BIT,
            pub network_connection: Option<NetworkConnection>,
        }

        #[derive(Serialize, Deserialize, Debug)]
        pub enum Message {
            State(State),
        }
    }

    pub mod host_to_mote {
        use serde::{Deserialize, Serialize};

        #[derive(Serialize, Deserialize, Debug)]
        pub struct SetNetworkConnectionConfig {}

        #[derive(Serialize, Deserialize, Debug)]
        pub struct SetUID {}

        #[derive(Serialize, Deserialize, Debug)]
        pub enum Message {
            SetNetworkConnectionConfig(SetNetworkConnectionConfig),
            SetUID(SetUID),
        }
    }
}
