//! High-performance async Rust client for TheresavDB over binary wire protocol.

use std::sync::atomic::{AtomicU64, Ordering};
use theresav_error::TheresavError;
use theresav_protocol::{Frame, MessageCodec, MessageType};
use theresav_types::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Value>>,
    pub rows_affected: u64,
}

pub struct TheresavClient {
    stream: TcpStream,
    next_req_id: AtomicU64,
}

impl TheresavClient {
    /// Connect to a running TheresavDB instance over TCP.
    pub async fn connect(addr: &str) -> Result<Self, TheresavError> {
        let stream = TcpStream::connect(addr).await?;
        let client = Self {
            stream,
            next_req_id: AtomicU64::new(1),
        };
        Ok(client)
    }

    /// Execute a SQL statement and receive query results.
    pub async fn query(&mut self, sql: &str) -> Result<QueryResult, TheresavError> {
        let req_id = self.next_req_id.fetch_add(1, Ordering::SeqCst);
        let query_frame = MessageCodec::encode_query(req_id, sql);

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

                        match frame.header.message_type {
                            MessageType::RowDescription => {
                                let desc: theresav_protocol::RowDescriptionMessage =
                                    serde_json::from_slice(&frame.payload).unwrap();
                                columns = desc.columns;
                            }
                            MessageType::DataRow => {
                                let row_msg: theresav_protocol::DataRowMessage =
                                    serde_json::from_slice(&frame.payload).unwrap();
                                rows.push(row_msg.values);
                            }
                            MessageType::CommandComplete => {
                                let comp: theresav_protocol::CommandCompleteMessage =
                                    serde_json::from_slice(&frame.payload).unwrap();
                                rows_affected = comp.rows_affected;
                                finished = true;
                                break;
                            }
                            MessageType::Error => {
                                let err_msg: theresav_protocol::ErrorMessage =
                                    serde_json::from_slice(&frame.payload).unwrap();
                                return Err(TheresavError::Internal(format!(
                                    "[{}] {}",
                                    err_msg.code, err_msg.message
                                )));
                            }
                            _ => {}
                        }
                    }
                    Err(_) => break, // Awaiting more bytes
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
