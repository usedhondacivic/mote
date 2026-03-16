#![no_std]

//! Messages used by Mote for firmware <--> host communication

// I'd prefer to move away from alloc, but it's here for now.
extern crate alloc;
use core::marker::PhantomData;

use alloc::{collections::vec_deque::VecDeque, vec::Vec};

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use thiserror::Error;

pub mod messages;

use crate::messages::{host_to_mote, mote_to_host};

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
    cobs_buff.resize(encoded_size, 10);
    let encoded_size = corncobs::encode_buf(&ser_buff, &mut cobs_buff);
    cobs_buff.truncate(encoded_size);

    Ok(cobs_buff)
}

/// Implements decoding of message types.
fn from_bytes<M>(bytes: &[u8]) -> Result<M, Error>
where
    M: DeserializeOwned,
{
    let mut cobs_buff: Vec<u8> = Vec::with_capacity(bytes.len());
    cobs_buff.resize(bytes.len(), 10);
    let decoded_size = corncobs::decode_buf(bytes, &mut cobs_buff)?;
    cobs_buff.truncate(decoded_size);

    Ok(bitcode::deserialize::<M>(&cobs_buff)?)
}

// Sets the capacity for the deserialization ringbuffer
const MAX_MESSAGE_LENGTH: usize = 5000;

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
    buffered_transmits: VecDeque<Vec<u8>>,
    deserialization_buffer: VecDeque<u8>,

    in_type: PhantomData<I>,
    out_type: PhantomData<O>,
}
impl<const MTU: usize, I, O> Default for MoteComms<MTU, I, O>
where
    I: for<'de> Deserialize<'de>, // Input type
    O: Serialize,
{
    fn default() -> Self {
        Self::new()
    }
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
            self.buffered_transmits.push_back(Vec::from(chunk));
        }

        Ok(())
    }

    /// Get the next packet to be sent
    pub fn poll_transmit(&mut self) -> Option<Vec<u8>> {
        self.buffered_transmits.pop_front()
    }

    /// Receive a message from raw bytes
    pub fn handle_receive(&mut self, packet: &[u8]) {
        // Push the recieved bytes into the serialization buffer, potentially dropping the first
        // value if the buffer is full
        packet.iter().for_each(|byte| {
            self.deserialization_buffer.push_back(*byte);
            if self.deserialization_buffer.len() > MAX_MESSAGE_LENGTH {
                self.deserialization_buffer.pop_front();
            }
        });
    }

    /// Poll for new messages in the recv buffer
    pub fn poll_receive(&mut self) -> Result<Option<I>, Error> {
        if let Some(end) = self.deserialization_buffer.iter().position(|&x| x == 0) {
            let linear_buf: Vec<u8> = self.deserialization_buffer.drain(0..=end).collect();
            match from_bytes::<I>(&linear_buf) {
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
            Ok(None)
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

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::{string::String, vec};

    // Returns all mote_to_host message variants including heap-allocated ones.
    fn all_mote_messages() -> Vec<mote_to_host::Message> {
        vec![
            mote_to_host::Message::Ping,
            mote_to_host::Message::Pong,
            mote_to_host::Message::Scan(vec![
                mote_to_host::Point {
                    quality: 255,
                    angle_rads: 1.5707,
                    distance_mm: 500.0,
                },
                mote_to_host::Point {
                    quality: 0,
                    angle_rads: 0.0,
                    distance_mm: 0.0,
                },
            ]),
            mote_to_host::Message::State(mote_to_host::State {
                uid: String::from("mote-test"),
                ip: Some(String::from("192.168.1.100")),
                current_network_connection: Some(String::from("MyWifi")),
                available_network_connections: vec![mote_to_host::NetworkConnection {
                    ssid: String::from("MyWifi"),
                    strength: 80,
                }],
                built_in_test: mote_to_host::BITCollection {
                    power: vec![mote_to_host::BIT {
                        name: String::from("battery"),
                        result: mote_to_host::BITResult::Pass,
                    }],
                    wifi: vec![],
                    lidar: vec![mote_to_host::BIT {
                        name: String::from("lidar_init"),
                        result: mote_to_host::BITResult::Waiting,
                    }],
                    imu: vec![],
                    encoders: vec![mote_to_host::BIT {
                        name: String::from("left_enc"),
                        result: mote_to_host::BITResult::Fail,
                    }],
                },
            }),
        ]
    }

    // Returns all host_to_mote message variants.
    fn all_host_messages() -> Vec<host_to_mote::Message> {
        vec![
            host_to_mote::Message::Ping,
            host_to_mote::Message::Pong,
            host_to_mote::Message::RequestNetworkScan,
            host_to_mote::Message::SetNetworkConnectionConfig(
                host_to_mote::SetNetworkConnectionConfig {
                    ssid: String::from("MyWifi"),
                    password: String::from("hunter2"),
                },
            ),
            host_to_mote::Message::SetUID(host_to_mote::SetUID {
                uid: String::from("mote-abc"),
            }),
        ]
    }

    // --- encode / decode ---

    #[test]
    fn test_encode_decode_all_variants() -> Result<(), Error> {
        for msg in all_mote_messages() {
            let bytes = to_slice(&msg)?;
            let recv: mote_to_host::Message = from_bytes(&bytes)?;
            assert_eq!(msg, recv);
        }
        for msg in all_host_messages() {
            let bytes = to_slice(&msg)?;
            let recv: host_to_mote::Message = from_bytes(&bytes)?;
            assert_eq!(msg, recv);
        }
        Ok(())
    }

    // --- poll_transmit / poll_receive on empty state ---

    #[test]
    fn test_poll_transmit_empty() {
        let mut link = MoteLink::new();
        assert!(link.poll_transmit().is_none());
    }

    #[test]
    fn test_poll_receive_empty() -> Result<(), Error> {
        let mut link = MoteLink::new();
        assert!(link.poll_receive()?.is_none());
        Ok(())
    }

    // --- Default::default() ---

    #[test]
    fn test_default() -> Result<(), Error> {
        let mut link: MoteLink = Default::default();
        link.send(host_to_mote::Message::Ping)?;
        assert!(link.poll_transmit().is_some());
        Ok(())
    }

    // --- config link round-trips (all variants) ---

    #[test]
    fn test_config_links() -> Result<(), Error> {
        for msg in all_mote_messages() {
            let mut host_l = HostConfigLink::new();
            host_l.send(msg.clone())?;
            let mut mote_l = MoteConfigLink::new();
            while let Some(payload) = host_l.poll_transmit() {
                mote_l.handle_receive(&payload);
            }
            assert_eq!(mote_l.poll_receive()?.unwrap(), msg);
        }

        for msg in all_host_messages() {
            let mut mote_l = MoteConfigLink::new();
            mote_l.send(msg.clone())?;
            let mut host_l = HostConfigLink::new();
            while let Some(payload) = mote_l.poll_transmit() {
                host_l.handle_receive(&payload);
            }
            assert_eq!(host_l.poll_receive()?.unwrap(), msg);
        }
        Ok(())
    }

    // --- UDP link round-trips (all variants) ---

    #[test]
    fn test_udp_links() -> Result<(), Error> {
        for msg in all_mote_messages() {
            let mut host_l = HostLink::new();
            host_l.send(msg.clone())?;
            let mut mote_l = MoteLink::new();
            while let Some(payload) = host_l.poll_transmit() {
                mote_l.handle_receive(&payload);
            }
            assert_eq!(mote_l.poll_receive()?.unwrap(), msg);
        }

        for msg in all_host_messages() {
            let mut mote_l = MoteLink::new();
            mote_l.send(msg.clone())?;
            let mut host_l = HostLink::new();
            while let Some(payload) = mote_l.poll_transmit() {
                host_l.handle_receive(&payload);
            }
            assert_eq!(host_l.poll_receive()?.unwrap(), msg);
        }
        Ok(())
    }

    // --- Fragmentation: large message split across MTU=64 packets ---

    #[test]
    fn test_fragmentation() -> Result<(), Error> {
        let scan = mote_to_host::Message::Scan(
            (0..100u8)
                .map(|i| mote_to_host::Point {
                    quality: i,
                    angle_rads: i as f32 * 0.01,
                    distance_mm: i as f32 * 10.0,
                })
                .collect(),
        );

        let mut host_l = HostConfigLink::new(); // MTU = 64
        host_l.send(scan.clone())?;

        let mut packet_count = 0usize;
        let mut mote_l = MoteConfigLink::new();
        while let Some(payload) = host_l.poll_transmit() {
            assert!(payload.len() <= 64, "packet exceeded MTU");
            mote_l.handle_receive(&payload);
            packet_count += 1;
        }
        assert!(
            packet_count > 1,
            "expected fragmentation into multiple packets"
        );

        assert_eq!(mote_l.poll_receive()?.unwrap(), scan);
        Ok(())
    }

    // --- Multiple messages received in order ---

    #[test]
    fn test_multiple_messages_in_order() -> Result<(), Error> {
        let messages = [
            host_to_mote::Message::Ping,
            host_to_mote::Message::RequestNetworkScan,
            host_to_mote::Message::Pong,
        ];
        let mut mote_l = MoteConfigLink::new();
        let mut host_l = HostConfigLink::new();

        for msg in &messages {
            mote_l.send(msg.clone())?;
        }
        while let Some(payload) = mote_l.poll_transmit() {
            host_l.handle_receive(&payload);
        }
        for expected in &messages {
            assert_eq!(&host_l.poll_receive()?.unwrap(), expected);
        }
        assert!(host_l.poll_receive()?.is_none());
        Ok(())
    }

    // --- Bad data in the receive buffer ---

    #[test]
    fn test_truncated_cobs_produces_no_message() -> Result<(), Error> {
        let mut link = MoteLink::new();
        // First byte 0xFF tells COBS to skip 254 more bytes, but the packet ends
        // after three bytes — corncobs returns Truncated, which maps to Ok(None).
        link.handle_receive(&[0xFF, 0xFE, 0xFD, 0x00]);
        assert!(link.poll_receive()?.is_none());
        Ok(())
    }

    #[test]
    fn test_empty_cobs_payload_returns_error() {
        let mut link = MoteLink::new();
        // [0x01, 0x00] is a valid COBS frame (overhead byte 0x01 = no data, then
        // terminator), but the empty decoded payload cannot be deserialized as a
        // message — bitcode returns an error.
        link.handle_receive(&[0x01, 0x00]);
        assert!(link.poll_receive().is_err());
    }

    // --- Receive buffer is capped at MAX_MESSAGE_LENGTH ---

    #[test]
    fn test_receive_buffer_overflow() -> Result<(), Error> {
        let mut link = MoteLink::new();
        // Feed more bytes than MAX_MESSAGE_LENGTH with no terminator.
        let data = vec![0xABu8; MAX_MESSAGE_LENGTH + 500];
        link.handle_receive(&data);
        // No zero byte in the buffer so poll_receive returns None, not an error.
        assert!(link.poll_receive()?.is_none());
        Ok(())
    }
}
