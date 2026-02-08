#![no_std]

//! Messages used by Mote for firmware <--> host communication

// I'd prefer to move away from alloc, but it's here for now.
extern crate alloc;
use alloc::vec::Vec;

use serde::{Serialize, de::DeserializeOwned};
use thiserror::Error;

pub mod comms;
pub mod messages;

/// Error type
#[derive(Error, Debug)]
pub enum Error {
    #[error("Bitcode ser/de failed")]
    BitCodeError(#[from] bitcode::Error),
    #[error("Cobs pack/unpack failed")]
    CobsError(corncobs::CobsError),
}

impl From<corncobs::CobsError> for Error {
    fn from(value: corncobs::CobsError) -> Self {
        Self::CobsError(value)
    }
}

/// Implements encoding of message types.
///
/// This implementation is the coanonnical encoding scheme used onboard Mote.
///
/// You probably don't want to use the method directly. Try the MoteComms crate instead.
pub fn to_slice<M>(message: &M) -> Result<Vec<u8>, Error>
where
    M: Serialize + ?Sized,
{
    let ser_buff = bitcode::serialize(message)?;
    let mut cobs_buff: Vec<u8> = Vec::new();
    corncobs::encode_buf(&ser_buff, &mut cobs_buff);

    return Ok(cobs_buff);
}

/// Implements decoding of message types.
/// This implementation is the coanonnical decoding scheme used onboard Mote.
///
/// Returns the decoded message (or an error).
///
/// You probably don't want to use the method directly. Try the MoteComms crate instead.
pub fn from_bytes<M>(bytes: &Vec<u8>) -> Result<M, Error>
where
    M: DeserializeOwned + ?Sized,
{
    let mut cobs_buff: Vec<u8> = Vec::new();
    corncobs::decode_buf(bytes, &mut cobs_buff)?;
    Ok(bitcode::deserialize::<M>(&cobs_buff)?)
}

/// Conditionally enable bindings for python and web assembly
#[cfg(any(feature = "python_bindings", feature = "wasm_bindings"))]
extern crate std;

#[cfg(feature = "python_bindings")]
pub mod python_interface;

#[cfg(feature = "wasm_bindings")]
pub mod wasm_interface;
