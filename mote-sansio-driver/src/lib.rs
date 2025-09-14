#![no_std]

// Sans-io message handling library for interacting with Mote
// IO handling should be implemented by the consumer. See ../examples.

use core::{fmt::Debug, marker::PhantomData, net::SocketAddr};

use heapless_postcard::{Deque, Vec};
use mote_messages::{configuration, runtime};
use postcard::{from_bytes_cobs, to_vec_cobs};
use serde::{Deserialize, Serialize};

const MTU: usize = 1500;

// Data structure representing a transmission
// Returned by sansio driver for the application to transmit
#[derive(Debug)]
pub struct Transmit<T: Debug> {
    pub dst: T,
    pub payload: Vec<u8, MTU>,
}

// The driver
// You probably do not want to directly construct this. Instead, use the type aliases:
// HostRuntimeLink
// HostConfigurationLink
// MoteRuntimeLink
// MoteConfigurationLink
pub struct SansIo<T, I, O>
where
    T: Debug,                     // Transmission type
    I: for<'de> Deserialize<'de>, // Input type
    O: Serialize,                 // Output type
{
    buffered_transmits: Deque<Transmit<T>, 10>,
    in_type: PhantomData<I>,
    out_type: PhantomData<O>,
}

impl<T, I, O> SansIo<T, I, O>
where
    T: Debug,                     // Transmission type
    I: for<'de> Deserialize<'de>, // Input type
    O: Serialize,                 // Output type
{
    // Generate a new link
    pub fn new() -> Self {
        Self {
            buffered_transmits: Deque::new(),
            in_type: PhantomData,
            out_type: PhantomData,
        }
    }

    // Queue a message to be sent
    pub fn send(&mut self, dst: T, message: O) -> Result<(), postcard::Error> {
        self.buffered_transmits
            .push_back(Transmit {
                dst: dst,
                payload: to_vec_cobs(&message)?,
            })
            .unwrap();

        return Ok(());
    }

    // Get the next message to be sent
    pub fn poll_transmit(&mut self) -> Option<Transmit<T>> {
        self.buffered_transmits.pop_front()
    }

    // Receive a message from raw bytes
    pub fn handle_receive(&self, packet: &mut [u8]) -> Result<I, postcard::Error> {
        Ok(from_bytes_cobs(packet)?)
    }
}

// Used by the host to talk to mote during runtime
pub type HostRuntimeLink =
    SansIo<SocketAddr, runtime::mote_to_host::Message, runtime::host_to_mote::Message>;

// Used by mote to talk to the host during runtime
pub type MoteRuntimeLink =
    SansIo<SocketAddr, runtime::host_to_mote::Message, runtime::mote_to_host::Message>;

#[derive(Debug)]
pub struct SerialEndpoint;

// Used by the host to talk to mote during configuration
pub type HostConfigurationLink = SansIo<
    SerialEndpoint,
    configuration::mote_to_host::Message,
    configuration::host_to_mote::Message,
>;

// Used by mote to talk to the host during configuration
pub type MoteConfigurationLink = SansIo<
    SerialEndpoint,
    configuration::host_to_mote::Message,
    configuration::mote_to_host::Message,
>;
