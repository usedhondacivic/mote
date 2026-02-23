#![no_std]

//! Messages used by Mote for firmware <--> host communication

// I'd prefer to move away from alloc, but it's here for now.
extern crate alloc;
use core::marker::PhantomData;

use alloc::{collections::vec_deque::VecDeque, vec::Vec};

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use thiserror::Error;

use crate::messages::{host_to_mote, mote_to_host};

pub mod messages;

// Conditionally enable bindings for python and web assembly
#[cfg(any(feature = "python_ffi", feature = "wasm_ffi"))]
extern crate std;
#[cfg(any(feature = "python_ffi", feature = "wasm_ffi"))]
pub mod ffi;

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
fn to_slice<M>(message: &M) -> Result<Vec<u8>, Error>
where
    M: Serialize + ?Sized,
{
    let ser_buff = bitcode::serialize(message)?;
    let encoded_size = corncobs::max_encoded_len(ser_buff.len());
    let mut cobs_buff: Vec<u8> = Vec::with_capacity(encoded_size);
    cobs_buff.resize(encoded_size, 0);
    corncobs::encode_buf(&ser_buff, &mut cobs_buff);

    return Ok(cobs_buff);
}

/// Implements decoding of message types.
fn from_bytes<M>(bytes: &Vec<u8>) -> Result<M, Error>
where
    M: DeserializeOwned + ?Sized,
{
    let encoded_size = corncobs::max_encoded_len(bytes.len());
    let mut cobs_buff: Vec<u8> = Vec::with_capacity(encoded_size);
    cobs_buff.resize(encoded_size, 0);
    corncobs::decode_buf(bytes, &mut cobs_buff)?;
    Ok(bitcode::deserialize::<M>(&cobs_buff)?)
}

// Sets the capacity for the deserialization ringbuffer
const MAX_MESSAGE_LENGTH: usize = 5000;

/// Data structure representing a transmission
/// Returned by sansio driver for the application to transmit
#[derive(Debug, Serialize)]
pub struct Transmit<const MTU: usize> {
    pub payload: Vec<u8>,
}

/// Bidirectional SansIO communication link betweek mote and the host.
///
/// You probably do not want to directly construct this. Instead, use the type aliases:
/// MoteLink (use on host)
/// HostLink (use on mote)
/// MoteConfigLink
/// HostConfigLink
pub struct MoteComms<const MTU: usize, I, O>
where
    I: DeserializeOwned, // Input type
    O: Serialize,        // Output type
{
    buffered_transmits: VecDeque<Transmit<MTU>>,
    deserialization_buffer: VecDeque<u8>,

    pub in_type: PhantomData<I>,
    out_type: PhantomData<O>,
}

impl<const MTU: usize, I, O> MoteComms<MTU, I, O>
where
    I: for<'de> Deserialize<'de>, // Input type
    O: Serialize,                 // Output type
{
    /// Generate a new link
    pub fn new() -> Self {
        Self {
            buffered_transmits: VecDeque::new(),
            deserialization_buffer: VecDeque::new(),
            in_type: PhantomData,
            out_type: PhantomData,
        }
    }

    /// Queue a message to be sent
    pub fn send(&mut self, message: O) -> Result<(), Error> {
        let encoded_bytes: Vec<u8> = to_slice(&message)?;

        // Break message into packets given the MTU
        for chunk in encoded_bytes.chunks(MTU) {
            let _ = self.buffered_transmits.push_back(Transmit {
                payload: Vec::from(chunk),
            });
        }

        return Ok(());
    }

    /// Get the next message to be sent
    pub fn poll_transmit(&mut self) -> Option<Transmit<MTU>> {
        self.buffered_transmits.pop_front()
    }

    /// Receive a message from raw bytes
    pub fn handle_receive(&mut self, packet: &[u8]) {
        // Push the recieved bytes into the serialization buffer, potentially dropping the first
        // value if the buffer is full
        packet.into_iter().for_each(|byte| {
            self.deserialization_buffer.push_back(byte.clone());
            if self.deserialization_buffer.len() > MAX_MESSAGE_LENGTH {
                self.deserialization_buffer.pop_front();
            }
        });
    }

    /// Poll for new messages in the recv buffer
    pub fn poll_receive(&mut self) -> Result<Option<I>, Error> {
        if let Some(end) = self.deserialization_buffer.iter().position(|&x| x == 0) {
            let mut linear_buf = self.deserialization_buffer.drain(0..=end).collect();
            match from_bytes::<I>(&mut linear_buf) {
                Ok(msg) => Ok(Some(msg)),
                Err(Error::BitCodeError(err)) => Err(err.into()),
                Err(Error::CobsError(corncobs::CobsError::Corrupt)) => {
                    Err(Error::CobsError(corncobs::CobsError::Corrupt))
                }
                Err(Error::CobsError(corncobs::CobsError::Truncated)) => {
                    // We checked for this in the if above, so it shouldn't happen.
                    // But it isn't an error.
                    Ok(None)
                }
            }
        } else {
            // No end byte = no message
            return Ok(None);
        }
    }
}

/// Used by the host to send commands to and receive data from Mote
pub type MoteLink = MoteComms<
    1400, // UDP MTU(ish)
    mote_to_host::Message,
    host_to_mote::Message,
>;

/// Used by Mote to send data to and receive commands from the host
pub type HostLink = MoteComms<
    1400, // UDP MTU(ish)
    host_to_mote::Message,
    mote_to_host::Message,
>;

/// Used by the host to send commands to and receive data from Mote
pub type MoteConfigLink = MoteComms<
    64, // Serial MTU
    mote_to_host::Message,
    host_to_mote::Message,
>;

/// Used by Mote to send data to and receive commands from the host
pub type HostConfigLink = MoteComms<
    64, // Serial MTU
    host_to_mote::Message,
    mote_to_host::Message,
>;
