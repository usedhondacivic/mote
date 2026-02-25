//! Foreign function interfaces for Python and TypeScript (WASM)

use std::string::String;
use std::vec::Vec;

use crate::{Error as MoteCommsError, MoteComms};

use serde::{Deserialize, Serialize};
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

// Add JSON shim methods to MoteComms.
// These methods erase the underlying message types, instead using their json string representation.
// This makes FFI implementation easier, as they don't need to worry about converting complex native type.
// JSON schemas are generated at build time, from which foreign language implementations may generate native type information.
#[allow(dead_code)]
impl<const MTU: usize, I, O> MoteComms<MTU, I, O>
where
    I: Serialize + for<'de> Deserialize<'de>, // Input type
    O: Serialize + for<'de> Deserialize<'de>, // Output type
{
    /// Queue a message to be sent
    fn send_json(&mut self, json: &str) -> Result<(), Error> {
        let msg: O = serde_json::from_str(json)?;
        self.send(msg).map_err(|e| e.into())
    }

    fn poll_transmit_json(&mut self) -> Result<Option<String>, Error> {
        if let Some(v) = self.poll_transmit() {
            Ok(Some(serde_json::to_string(&v)?))
        } else {
            Ok(None)
        }
    }

    fn handle_receive_json(&mut self, json: &str) -> Result<(), Error> {
        let packet: Vec<u8> = serde_json::from_str(json)?;
        self.handle_receive(&packet);
        Ok(())
    }

    fn poll_receive_json(&mut self) -> Result<Option<String>, Error> {
        if let Some(v) = self.poll_receive()? {
            Ok(Some(serde_json::to_string(&v)?))
        } else {
            Ok(None)
        }
    }
}
