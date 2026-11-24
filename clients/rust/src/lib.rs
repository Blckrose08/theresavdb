use byteorder::{ByteOrder, LittleEndian};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub const PROTOCOL_MAGIC: [u8; 4] = *b"THRP";
pub const PROTOCOL_VERSION_V1: u16 = 1;
pub const FRAME_HEADER_SIZE: usize = 20;

#[derive(Debug, Error)]
pub enum TheresavClientError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Protocol error: {0}")]
    Protocol(String),
    #[error("Server error: [{code}] {message}")]
    Server { code: String, message: String },
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum MessageType {
    Startup = 0,
    Auth = 1,
    Query = 2,
    RowDescription = 3,
    DataRow = 4,
    CommandComplete = 5,
    Error = 6,
    Ping = 7,
    Pong = 8,
    Close = 9,
}

impl MessageType {
    pub fn from_u16(val: u16) -> Result<Self, TheresavClientError> {
        match val {
            0 => Ok(Self::Startup),
            1 => Ok(Self::Auth),
            2 => Ok(Self::Query),
            3 => Ok(Self::RowDescription),
            4 => Ok(Self::DataRow),
            5 => Ok(Self::CommandComplete),
            6 => Ok(Self::Error),
            7 => Ok(Self::Ping),
            8 => Ok(Self::Pong),
            9 => Ok(Self::Close),
            other => Err(TheresavClientError::Protocol(format!(
                "Unknown message type code: {}",
                other
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub msg_type: MessageType,
    pub request_id: u64,
    pub payload: Vec<u8>,
}

impl Frame {
    pub fn new(msg_type: MessageType, request_id: u64, payload: Vec<u8>) -> Self {
        Self {
            msg_type,
            request_id,
            payload,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let total_size = FRAME_HEADER_SIZE + self.payload.len();
        let mut buf = vec![0u8; total_size];

        buf[0..4].copy_from_slice(&PROTOCOL_MAGIC);
        LittleEndian::write_u16(&mut buf[4..6], PROTOCOL_VERSION_V1);
        LittleEndian::write_u16(&mut buf[6..8], self.msg_type as u16);
        LittleEndian::write_u32(&mut buf[8..12], self.payload.len() as u32);
        LittleEndian::write_u64(&mut buf[12..20], self.request_id);

        if !self.payload.is_empty() {
            buf[20..].copy_from_slice(&self.payload);
        }

        buf
    }

    pub fn decode(bytes: &[u8]) -> Result<(Self, usize), TheresavClientError> {
        if bytes.len() < FRAME_HEADER_SIZE {
            return Err(TheresavClientError::Protocol("Unexpected EOF".to_string()));
        }

        if bytes[0..4] != PROTOCOL_MAGIC {
            return Err(TheresavClientError::Protocol(format!(
                "Invalid protocol magic: {:?}",
                &bytes[0..4]
            )));
        }

        let version = LittleEndian::read_u16(&bytes[4..6]);
        if version != PROTOCOL_VERSION_V1 {
            return Err(TheresavClientError::Protocol(format!(
                "Unsupported protocol version: {}",
                version
            )));
        }

        let msg_type = MessageType::from_u16(LittleEndian::read_u16(&bytes[6..8]))?;
        let payload_len = LittleEndian::read_u32(&bytes[8..12]) as usize;
        let request_id = LittleEndian::read_u64(&bytes[12..20]);

        let frame_size = FRAME_HEADER_SIZE + payload_len;
        if bytes.len() < frame_size {
            return Err(TheresavClientError::Protocol(
                "Incomplete payload in stream buffer".to_string(),
            ));
        }

        let payload = bytes[FRAME_HEADER_SIZE..frame_size].to_vec();
        Ok((
            Self {
                msg_type,
                request_id,
                payload,
            },
            frame_size,
        ))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct QueryPayload {
    sql: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RowDescriptionPayload {
    columns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DataRowPayload {
    values: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CommandCompletePayload {
    rows_affected: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ErrorPayload {
    code: String,
    message: String,
}

#[derive(Debug, Clone)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
    pub rows_affected: u64,
}

pub struct TheresavClient {
    stream: TcpStream,
    next_req_id: AtomicU64,
}

impl TheresavClient {
    pub async fn connect(addr: &str) -> Result<Self, TheresavClientError> {
        let stream = TcpStream::connect(addr).await?;
        let client = Self {
            stream,
            next_req_id: AtomicU64::new(1),
        };
        Ok(client)
    }

    pub async fn execute(&mut self, sql: &str) -> Result<u64, TheresavClientError> {
        let result = self.query(sql).await?;
        Ok(result.rows_affected)
    }

    pub async fn query(&mut self, sql: &str) -> Result<QueryResult, TheresavClientError> {
        let req_id = self.next_req_id.fetch_add(1, Ordering::SeqCst);
        let payload = serde_json::to_vec(&QueryPayload {
            sql: sql.to_string(),
        })?;
        let query_frame = Frame::new(MessageType::Query, req_id, payload);

        self.stream.write_all(&query_frame.encode()).await?;

        let mut columns = Vec::new();
        let mut rows = Vec::new();
        let mut rows_affected = 0;

        let mut stream_buf = Vec::new();
        let mut temp = [0u8; 4096];

        loop {
            let n = self.stream.read(&mut temp).await?;
            if n == 0 {
                break;
            }
            stream_buf.extend_from_slice(&temp[..n]);

            let mut finished = false;

            while !stream_buf.is_empty() {
                match Frame::decode(&stream_buf) {
                    Ok((frame, bytes_read)) => {
                        stream_buf.drain(..bytes_read);

                        match frame.msg_type {
                            MessageType::RowDescription => {
                                let desc: RowDescriptionPayload =
                                    serde_json::from_slice(&frame.payload)?;
                                columns = desc.columns;
                            }
                            MessageType::DataRow => {
                                let row_msg: DataRowPayload =
                                    serde_json::from_slice(&frame.payload)?;
                                rows.push(row_msg.values);
                            }
                            MessageType::CommandComplete => {
                                let comp: CommandCompletePayload =
                                    serde_json::from_slice(&frame.payload)?;
                                rows_affected = comp.rows_affected;
                                finished = true;
                                break;
                            }
                            MessageType::Error => {
                                let err_msg: ErrorPayload = serde_json::from_slice(&frame.payload)?;
                                return Err(TheresavClientError::Server {
                                    code: err_msg.code,
                                    message: err_msg.message,
                                });
                            }
                            _ => {}
                        }
                    }
                    Err(_) => break,
                }
            }

            if finished {
                break;
            }
        }

        Ok(QueryResult {
            columns,
            rows,
            rows_affected,
        })
    }
}
