use std::error::Error;
use strum::Display;

const SEQUENCE_NUMBER_SIZE: usize = std::mem::size_of::<u64>();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketHeader {
    pub sequence_number: u64,
}

impl PacketHeader {
    fn parse(bytes: &[u8]) -> Self {
        let sequence_bytes: [u8; SEQUENCE_NUMBER_SIZE] = bytes
            .try_into()
            .expect("header size is fixed by the caller");

        Self {
            sequence_number: u64::from_be_bytes(sequence_bytes),
        }
    }

    fn write_to(self, bytes: &mut [u8]) {
        bytes.copy_from_slice(&self.sequence_number.to_be_bytes());
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Packet<'a> {
    pub header: PacketHeader,
    pub payload: &'a [u8],
    bytes: &'a [u8],
}

impl<'a> Packet<'a> {
    pub const MINIMUM_SIZE: usize = SEQUENCE_NUMBER_SIZE;

    pub fn parse(bytes: &'a [u8]) -> Result<Self, PacketError> {
        if bytes.len() < Self::MINIMUM_SIZE {
            return Err(PacketError::PacketTooSmall {
                actual_size: bytes.len(),
                minimum_size: Self::MINIMUM_SIZE,
            });
        }

        let (header_bytes, payload) = bytes.split_at(Self::MINIMUM_SIZE);

        Ok(Self {
            header: PacketHeader::parse(header_bytes),
            payload,
            bytes,
        })
    }

    pub fn sequence_number(&self) -> u64 {
        self.header.sequence_number
    }

    pub fn payload(&self) -> &'a [u8] {
        self.payload
    }

    pub fn as_bytes(&self) -> &'a [u8] {
        self.bytes
    }
}

#[derive(Debug)]
pub struct PacketMut<'a> {
    pub header: PacketHeader,
    bytes: &'a mut [u8],
}

impl<'a> PacketMut<'a> {
    pub fn new(bytes: &'a mut [u8]) -> Result<Self, PacketError> {
        if bytes.len() < Packet::MINIMUM_SIZE {
            return Err(PacketError::PacketTooSmall {
                actual_size: bytes.len(),
                minimum_size: Packet::MINIMUM_SIZE,
            });
        }

        let header = PacketHeader::parse(&bytes[..Packet::MINIMUM_SIZE]);

        Ok(Self { header, bytes })
    }

    pub fn set_sequence_number(&mut self, sequence_number: u64) {
        self.header.sequence_number = sequence_number;
    }

    pub fn sequence_number(&self) -> u64 {
        self.header.sequence_number
    }

    pub fn payload(&self) -> &[u8] {
        &self.bytes[SEQUENCE_NUMBER_SIZE..]
    }

    pub fn payload_mut(&mut self) -> &mut [u8] {
        &mut self.bytes[SEQUENCE_NUMBER_SIZE..]
    }

    pub fn as_bytes(&mut self) -> &[u8] {
        self.header
            .write_to(&mut self.bytes[..Packet::MINIMUM_SIZE]);
        self.bytes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display)]
pub enum PacketError {
    #[strum(
        to_string = "packet size {actual_size} is smaller than the minimum {minimum_size} bytes"
    )]
    PacketTooSmall {
        actual_size: usize,
        minimum_size: usize,
    },
}

impl Error for PacketError {}
