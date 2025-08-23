// Sans-io message handling library for interacting with Mote
// IO handling should be implemented by the consumer. See ../examples.

use mote_messages::runtime::{host_to_mote, mote_to_host};

use postcard::{from_bytes, to_allocvec};
use std::{collections::VecDeque, net::SocketAddr};

pub struct Transmit {
    pub dst: SocketAddr,
    pub payload: Vec<u8>,
}

pub struct MoteCommunication {
    buffered_transmits: VecDeque<Transmit>,
}

impl MoteCommunication {
    pub fn new() -> Self {
        Self {
            buffered_transmits: VecDeque::new(),
        }
    }

    pub fn send(
        &mut self,
        dst: SocketAddr,
        message: host_to_mote::Message,
    ) -> Result<(), postcard::Error> {
        self.buffered_transmits.push_back(Transmit {
            dst: dst,
            payload: to_allocvec(&message)?,
        });

        return Ok(());
    }

    pub fn poll_transmit(&mut self) -> Option<Transmit> {
        self.buffered_transmits.pop_front()
    }

    pub fn handle_recieve(packet: &[u8]) -> Result<mote_to_host::Message, postcard::Error> {
        Ok(from_bytes(packet)?)
    }
}
