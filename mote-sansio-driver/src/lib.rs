#![no_std]

// Sans-io message handling library for interacting with Mote
// IO handling should be implemented by the consumer. See ../examples.

use core::{fmt::Debug, marker::PhantomData, net::SocketAddr};

use heapless_postcard::{Deque, Vec};
use mote_messages::{configuration, runtime};
use postcard::{take_from_bytes_cobs, to_vec_cobs};
use serde::{Deserialize, Serialize};

// Data structure representing a transmission
// Returned by sansio driver for the application to transmit
#[derive(Debug)]
pub struct Transmit<T: Debug + Copy, const MTU: usize> {
    pub dst: T,
    pub payload: Vec<u8, MTU>,
}

// You probably do not want to directly construct this. Instead, use the type aliases:
// HostRuntimeLink
// HostConfigurationLink
// MoteRuntimeLink
// MoteConfigurationLink
pub struct SansIo<T, const MTU: usize, const B: usize, I, O>
where
    T: Debug + Copy,              // Transmission type
    I: for<'de> Deserialize<'de>, // Input type
    O: Serialize,                 // Output type
{
    buffered_transmits: Deque<Transmit<T, MTU>, 10>,
    serialization_buffer: Vec<u8, B>,
    in_type: PhantomData<I>,
    out_type: PhantomData<O>,
}

impl<T, const MTU: usize, const B: usize, I, O> SansIo<T, MTU, B, I, O>
where
    T: Debug + Copy,              // Transmission type
    I: for<'de> Deserialize<'de>, // Input type
    O: Serialize,                 // Output type
{
    // Generate a new link
    pub fn new() -> Self {
        Self {
            buffered_transmits: Deque::new(),
            serialization_buffer: Vec::new(),
            in_type: PhantomData,
            out_type: PhantomData,
        }
    }

    // Queue a message to be sent
    pub fn send(&mut self, dst: T, message: O) -> Result<(), postcard::Error> {
        let encoded_bytes: Vec<u8, B> = to_vec_cobs(&message)?;

        // Break message into packets given the MTU
        for chunk in encoded_bytes.chunks(MTU) {
            let _ = self.buffered_transmits.push_back(Transmit {
                dst: dst.clone(),
                payload: Vec::from_slice(chunk).unwrap(),
            });
        }

        return Ok(());
    }

    // Get the next message to be sent
    pub fn poll_transmit(&mut self) -> Option<Transmit<T, MTU>> {
        self.buffered_transmits.pop_front()
    }

    // Receive a message from raw bytes
    pub fn handle_receive(&mut self, packet: &mut [u8]) -> Result<Option<I>, postcard::Error> {
        // Push the recieved bytes into the serialization buffer
        if let Err(_) = self.serialization_buffer.extend_from_slice(packet) {
            // We can't add to the buffer because it is full
            // Clear it and append
            self.serialization_buffer.clear();
            self.serialization_buffer.extend_from_slice(packet).unwrap();
        }

        // Check if the buffer contains the COBS delimiter (zero)
        while let Some(end) = self.serialization_buffer.iter().position(|&x| x == 0) {
            // If it does, attempt to deserialize
            // If deserialization fails, try again ignoring an additional byte from the start of
            // the buffer
            // This allows us to discard malformed packets
            let mut idx = 0;
            loop {
                match take_from_bytes_cobs::<I>(&mut self.serialization_buffer[idx..end + 1]) {
                    Ok((msg, remainder)) => {
                        self.serialization_buffer = Vec::from_slice(remainder).unwrap();
                        return Ok(Some(msg));
                    }
                    Err(postcard::Error::DeserializeBadEncoding) => {
                        idx += 1;
                    }
                    Err(err) => {
                        self.serialization_buffer.clear();
                        return Err(err);
                    }
                }
            }
        }
        Ok(None)
    }
}

// Used by the host to talk to mote during runtime
pub type HostRuntimeLink = SansIo<
    SocketAddr,
    1460, // TCP MSS
    2000,
    runtime::mote_to_host::Message,
    runtime::host_to_mote::Message,
>;

// Used by mote to talk to the host during runtime
pub type MoteRuntimeLink = SansIo<
    SocketAddr,
    250, // The ser/de buffer and MSS are smaller on Mote due to memory constraints
    250,
    runtime::host_to_mote::Message,
    runtime::mote_to_host::Message,
>;

// Currently we do not disambiguate between messages sent to different serial ports
#[derive(Debug, Clone, Copy)]
pub struct SerialEndpoint;

// Used by the host to talk to mote during configuration
pub type HostConfigurationLink = SansIo<
    SerialEndpoint,
    64, // Serial MTU
    2000,
    configuration::mote_to_host::Message,
    configuration::host_to_mote::Message,
>;

// Used by mote to talk to the host during configuration
pub type MoteConfigurationLink = SansIo<
    SerialEndpoint,
    64,
    1000,
    configuration::host_to_mote::Message,
    configuration::mote_to_host::Message,
>;
