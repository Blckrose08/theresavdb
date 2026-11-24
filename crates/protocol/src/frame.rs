//! Binary wire protocol framing for TheresavDB.

use byteorder::{ByteOrder, LittleEndian};
use theresav_common::{DEFAULT_MAX_FRAME_SIZE, PROTOCOL_MAGIC, PROTOCOL_VERSION_V1};
use theresav_error::ProtocolError;

pub const FRAME_HEADER_SIZE: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum MessageType {
    Startup = 0x0001,
    AuthChallenge = 0x0002,
    AuthResponse = 0x0003,
    AuthOk = 0x0004,
    Close = 0x000E,
    Query = 0x0010,
    RowDescription = 0x0020,
    DataRow = 0x0021,
    CommandComplete = 0x0022,
    Commit = 0x0030,
    Rollback = 0x0031,
    Pong = 0x00FD,
    Ping = 0x00FE,
    Error = 0x00FF,
}

impl MessageType {
    pub fn from_u16(val: u16) -> Result<Self, ProtocolError> {
        match val {
            0x0001 => Ok(Self::Startup),
            0x0002 => Ok(Self::AuthChallenge),
            0x0003 => Ok(Self::AuthResponse),
            0x0004 => Ok(Self::AuthOk),
            0x000E => Ok(Self::Close),
            0x0010 => Ok(Self::Query),
            0x0020 => Ok(Self::RowDescription),
            0x0021 => Ok(Self::DataRow),
            0x0022 => Ok(Self::CommandComplete),
            0x0030 => Ok(Self::Commit),
            0x0031 => Ok(Self::Rollback),
            0x00FD => Ok(Self::Pong),
            0x00FE => Ok(Self::Ping),
            0x00FF => Ok(Self::Error),
            other => Err(ProtocolError::UnknownMessageType(other)),
        }
    }

    #[inline]
    pub const fn to_u16(self) -> u16 {
        self as u16
    }
}

/// 20-byte Frame Header:
/// - magic: 4 bytes (b"THRP")
/// - version: 2 bytes (1)
/// - message_type: 2 bytes
/// - payload_length: 4 bytes
/// - request_id: 8 bytes
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameHeader {
    pub message_type: MessageType,
    pub payload_length: u32,
    pub request_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub header: FrameHeader,
    pub payload: Vec<u8>,
}

impl Frame {
    pub fn new(message_type: MessageType, request_id: u64, payload: Vec<u8>) -> Self {
        Self {
            header: FrameHeader {
                message_type,
                payload_length: payload.len() as u32,
                request_id,
            },
            payload,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let total_size = FRAME_HEADER_SIZE + self.payload.len();
        let mut buf = vec![0u8; total_size];

        buf[0..4].copy_from_slice(&PROTOCOL_MAGIC);
        LittleEndian::write_u16(&mut buf[4..6], PROTOCOL_VERSION_V1);
        LittleEndian::write_u16(&mut buf[6..8], self.header.message_type.to_u16());
        LittleEndian::write_u32(&mut buf[8..12], self.header.payload_length);
        LittleEndian::write_u64(&mut buf[12..20], self.header.request_id);

        buf[FRAME_HEADER_SIZE..total_size].copy_from_slice(&self.payload);
        buf
    }

    pub fn decode(bytes: &[u8]) -> Result<(Self, usize), ProtocolError> {
        if bytes.len() < FRAME_HEADER_SIZE {
            return Err(ProtocolError::UnexpectedEof);
        }

        if bytes[0..4] != PROTOCOL_MAGIC {
            let mut magic = [0u8; 4];
            magic.copy_from_slice(&bytes[0..4]);
            return Err(ProtocolError::InvalidMagic(magic));
        }

        let version = LittleEndian::read_u16(&bytes[4..6]);
        if version != PROTOCOL_VERSION_V1 {
            return Err(ProtocolError::UnsupportedVersion(version));
        }

        let msg_type = MessageType::from_u16(LittleEndian::read_u16(&bytes[6..8]))?;
        let payload_len = LittleEndian::read_u32(&bytes[8..12]);
        let request_id = LittleEndian::read_u64(&bytes[12..20]);

        if payload_len > DEFAULT_MAX_FRAME_SIZE {
            return Err(ProtocolError::FrameTooLarge {
                size: payload_len,
                max_limit: DEFAULT_MAX_FRAME_SIZE,
            });
        }

        let total_len = FRAME_HEADER_SIZE + payload_len as usize;
        if bytes.len() < total_len {
            return Err(ProtocolError::UnexpectedEof);
        }

        let payload = bytes[FRAME_HEADER_SIZE..total_len].to_vec();

        Ok((
            Self {
                header: FrameHeader {
                    message_type: msg_type,
                    payload_length: payload_len,
                    request_id,
                },
                payload,
            },
            total_len,
        ))
    }
}
