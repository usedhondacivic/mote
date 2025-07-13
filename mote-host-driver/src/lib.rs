// Sans-io message handling library for interacting with Mote
// IO handling should be implemented by the consumer. See ../examples.

use mote_messages::{HostToMoteMessage, MoteToHostMessage};

use postcard::{from_bytes, to_allocvec};
use std::{collections::VecDeque, net::SocketAddr};

pub struct Transmit {
    pub dst: SocketAddr,
    pub payload: Vec<u8>,
}

pub struct MiteCommunication {
    buffered_transmits: VecDeque<Transmit>,
}

impl MiteCommunication {
    pub fn new() -> Self {
        Self {
            buffered_transmits: VecDeque::new(),
        }
    }

    pub fn send(
        &mut self,
        dst: SocketAddr,
        message: HostToMoteMessage,
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

    pub fn handle_recieve(packet: &[u8]) -> Result<MoteToHostMessage, postcard::Error> {
        from_bytes(packet)?
    }
}
