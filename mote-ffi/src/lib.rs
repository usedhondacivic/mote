//! Foreign function interfaces for Python and TypeScript (WASM)

use std::marker::PhantomData;
use std::string::String;
use std::vec::Vec;

use mote_api::{Error as MoteCommsError, MoteComms};

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use thiserror::Error;

#[cfg(feature = "python_ffi")]
pub mod python;

#[cfg(feature = "wasm_ffi")]
pub mod wasm;

/// Error type
#[derive(Error, Debug)]
pub enum Error {
    #[error("Internal comms error")]
    MoteCommsError(#[from] MoteCommsError),
    #[error("Serde JSON error")]
    SerdeJson(#[from] serde_json::Error),
}

pub struct MoteCommsFFI<const MTU: usize, I, O>
where
    I: DeserializeOwned, // Input type
    O: Serialize,        // Output type
{
    in_type: PhantomData<I>,
    out_type: PhantomData<O>,

    link: MoteComms<MTU, I, O>,
}

impl<const MTU: usize, I, O> From<MoteComms<MTU, I, O>> for MoteCommsFFI<MTU, I, O>
where
    I: DeserializeOwned, // Input type
    O: Serialize,        // Output type
{
    fn from(link: MoteComms<MTU, I, O>) -> Self {
        Self {
            link,
            in_type: PhantomData,
            out_type: PhantomData,
        }
    }
}

// JSON shim methods for MoteComms.
// These methods erase the underlying message types, instead using their json string representation.
// This makes FFI implementation easier, as they don't need to worry about converting complex native type.
// JSON schemas are generated at build time, from which foreign language implementations may use to generate native type information.
#[allow(dead_code)]
impl<const MTU: usize, I, O> MoteCommsFFI<MTU, I, O>
where
    I: Serialize + for<'de> Deserialize<'de>, // Input type
    O: Serialize + for<'de> Deserialize<'de>, // Output type
{
    fn new(link: MoteComms<MTU, I, O>) -> Self {
        Self {
            link,
            in_type: PhantomData,
            out_type: PhantomData,
        }
    }

    /// Queue a message to be sent
    fn send(&mut self, json: &str) -> Result<(), Error> {
        let msg: O = serde_json::from_str(json)?;
        self.link.send(msg).map_err(|e| e.into())
    }

    fn poll_transmit(&mut self) -> Result<Option<String>, Error> {
        if let Some(v) = self.link.poll_transmit() {
            Ok(Some(serde_json::to_string(&v)?))
        } else {
            Ok(None)
        }
    }

    fn handle_receive(&mut self, json: &str) -> Result<(), Error> {
        let packet: Vec<u8> = serde_json::from_str(json)?;
        self.link.handle_receive(&packet);
        Ok(())
    }

    fn poll_receive(&mut self) -> Result<Option<String>, Error> {
        if let Some(v) = self.link.poll_receive()? {
            Ok(Some(serde_json::to_string(&v)?))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mote_api::{
        HostLink, MoteLink,
        messages::{host_to_mote, mote_to_host},
    };

    type HostFFI = MoteCommsFFI<1400, mote_to_host::Message, host_to_mote::Message>;

    fn make_host_ffi() -> HostFFI {
        MoteCommsFFI::from(MoteLink::new())
    }

    /// Extract payload bytes from a poll_transmit JSON result.
    fn extract_payload(packet_json: &str) -> Vec<u8> {
        let v: serde_json::Value = serde_json::from_str(packet_json).unwrap();
        v["payload"]
            .as_array()
            .unwrap()
            .iter()
            .map(|b| b.as_u64().unwrap() as u8)
            .collect()
    }

    #[test]
    fn test_ffi_send_and_poll_transmit() {
        let mut host = make_host_ffi();
        host.send("\"Ping\"").unwrap();
        let packet_json = host.poll_transmit().unwrap().unwrap();
        let v: serde_json::Value = serde_json::from_str(&packet_json).unwrap();
        assert!(v.get("payload").is_some());
        assert!(host.poll_transmit().unwrap().is_none());
    }

    #[test]
    fn test_ffi_send_invalid_json_errors() {
        let mut host = make_host_ffi();
        assert!(host.send("not valid json").is_err());
    }

    #[test]
    fn test_ffi_poll_receive_empty() {
        let mut host = make_host_ffi();
        assert!(host.poll_receive().unwrap().is_none());
    }

    #[test]
    fn test_ffi_host_to_mote_round_trip() {
        let mut host_ffi = make_host_ffi();
        let mut mote = HostLink::new();

        host_ffi.send("\"Ping\"").unwrap();
        let packet_json = host_ffi.poll_transmit().unwrap().unwrap();
        mote.handle_receive(&extract_payload(&packet_json));

        let received = mote.poll_receive().unwrap().unwrap();
        assert_eq!(received, host_to_mote::Message::Ping);
    }

    #[test]
    fn test_ffi_mote_to_host_round_trip() {
        let mut mote = HostLink::new();
        let mut host_ffi = make_host_ffi();

        mote.send(mote_to_host::Message::Pong).unwrap();
        let transmission = mote.poll_transmit().unwrap();
        let payload_json = serde_json::to_string(&transmission.payload).unwrap();

        host_ffi.handle_receive(&payload_json).unwrap();
        let received_json = host_ffi.poll_receive().unwrap().unwrap();
        let received: mote_to_host::Message = serde_json::from_str(&received_json).unwrap();
        assert_eq!(received, mote_to_host::Message::Pong);
    }

    #[test]
    fn test_ffi_multiple_host_to_mote_messages() {
        let mut host_ffi = make_host_ffi();
        let mut mote = HostLink::new();

        let messages = [
            ("\"Ping\"", host_to_mote::Message::Ping),
            ("\"Pong\"", host_to_mote::Message::Pong),
            (
                "\"RequestNetworkScan\"",
                host_to_mote::Message::RequestNetworkScan,
            ),
        ];

        for (json, expected) in &messages {
            host_ffi.send(json).unwrap();
            let packet_json = host_ffi.poll_transmit().unwrap().unwrap();
            mote.handle_receive(&extract_payload(&packet_json));
            let received = mote.poll_receive().unwrap().unwrap();
            assert_eq!(received, *expected);
        }
    }

    #[test]
    fn test_ffi_set_uid_round_trip() {
        let mut host_ffi = make_host_ffi();
        let mut mote = HostLink::new();

        host_ffi.send(r#"{"SetUID":{"uid":"mote-abc"}}"#).unwrap();
        let packet_json = host_ffi.poll_transmit().unwrap().unwrap();
        mote.handle_receive(&extract_payload(&packet_json));

        let received = mote.poll_receive().unwrap().unwrap();
        assert_eq!(
            received,
            host_to_mote::Message::SetUID(host_to_mote::SetUID {
                uid: "mote-abc".into()
            })
        );
    }

    #[test]
    fn test_ffi_set_network_config_round_trip() {
        let mut host_ffi = make_host_ffi();
        let mut mote = HostLink::new();

        host_ffi
            .send(r#"{"SetNetworkConnectionConfig":{"ssid":"MyWifi","password":"secret"}}"#)
            .unwrap();
        let packet_json = host_ffi.poll_transmit().unwrap().unwrap();
        mote.handle_receive(&extract_payload(&packet_json));

        let received = mote.poll_receive().unwrap().unwrap();
        assert_eq!(
            received,
            host_to_mote::Message::SetNetworkConnectionConfig(
                host_to_mote::SetNetworkConnectionConfig {
                    ssid: "MyWifi".into(),
                    password: "secret".into(),
                }
            )
        );
    }
}
