use alloc::collections::vec_deque::VecDeque;
use alloc::vec::Vec;
use core::{fmt::Debug, marker::PhantomData};
use serde::de::DeserializeOwned;

use crate::messages::{configuration, runtime};
use crate::{from_bytes, to_slice};
use serde::{Deserialize, Serialize};

use crate::Error;

// Sets the capacity for the deserialization ringbuffer
const MAX_MESSAGE_LENGTH: usize = 5000;

/// Data structure representing a transmission
/// Returned by sansio driver for the application to transmit
#[derive(Debug)]
pub struct Transmit<const MTU: usize> {
    pub payload: Vec<u8>,
}

/// Bidirectional SansIO communication link betweek mote and a host.
///
/// You probably do not want to directly construct this. Instead, use the type aliases:
/// HostRuntimeCommandLink
/// HostRuntimeDataOffloadLink
/// HostConfigurationLink
/// MoteRuntimeCommandLink
/// MoteRuntimeDataOffloadLink
/// MoteConfigurationLink
pub struct MoteComms<const MTU: usize, I, O>
where
    I: DeserializeOwned, // Input type
    O: Serialize,        // Output type
{
    buffered_transmits: VecDeque<Transmit<MTU>>,
    deserialization_buffer: VecDeque<u8>,

    in_type: PhantomData<I>,
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
    pub fn handle_receive(&mut self, packet: Vec<u8>) {
        // Push the recieved bytes into the serialization buffer, potentially dropping the first
        // value if the buffer is full
        packet.into_iter().for_each(|byte| {
            self.deserialization_buffer.push_back(byte);
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
            // Shortcut for truncated messages, don't need to try to decode
            return Err(Error::CobsError(corncobs::CobsError::Truncated));
        }
    }
}

/// Used by the host to send commands to mote during runtime

pub type HostRuntimeCommandLink = MoteComms<
    1460, // TCP MSS
    runtime::mote_to_host::command::Message,
    runtime::host_to_mote::Message,
>;

pub type HostRuntimeDataOffloadLink = MoteComms<
    1460,
    runtime::mote_to_host::data_offload::Message,
    runtime::host_to_mote::data_offload::DataOffloadSubscribeRequest,
>;

/// Used by mote to respond to control commands during runtime
pub type MoteRuntimeCommandLink =
    MoteComms<1460, runtime::host_to_mote::Message, runtime::mote_to_host::command::Message>;

/// Used by mote to offload sensor data
pub type MoteRuntimeDataOffloadLink = MoteComms<
    1460,
    runtime::host_to_mote::data_offload::DataOffloadSubscribeRequest,
    runtime::mote_to_host::data_offload::Message,
>;

/// Used by the host to talk to mote during configuration
pub type HostConfigurationLink = MoteComms<
    64, // Serial MTU
    configuration::mote_to_host::Message,
    configuration::host_to_mote::Message,
>;

/// Used by mote to talk to the host during configuration
pub type MoteConfigurationLink =
    MoteComms<64, configuration::host_to_mote::Message, configuration::mote_to_host::Message>;
