//! Structured message encoding and decoding on top of binary frames.

use crate::frame::{Frame, MessageType};
use serde::{Deserialize, Serialize};
use theresav_error::ProtocolError;
use theresav_types::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartupMessage {
    pub database: String,
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryMessage {
    pub sql: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RowDescriptionMessage {
    pub columns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataRowMessage {
    pub values: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandCompleteMessage {
    pub rows_affected: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMessage {
    pub code: String,
    pub message: String,
}

pub struct MessageCodec;

impl MessageCodec {
    pub fn encode_query(request_id: u64, sql: &str) -> Frame {
        let msg = QueryMessage {
            sql: sql.to_string(),
        };
        let payload = serde_json::to_vec(&msg).unwrap();
        Frame::new(MessageType::Query, request_id, payload)
    }

    pub fn decode_query(frame: &Frame) -> Result<QueryMessage, ProtocolError> {
        serde_json::from_slice(&frame.payload).map_err(|e| ProtocolError::Other(e.to_string()))
    }

    pub fn encode_row_desc(request_id: u64, columns: Vec<String>) -> Frame {
        let msg = RowDescriptionMessage { columns };
        let payload = serde_json::to_vec(&msg).unwrap();
        Frame::new(MessageType::RowDescription, request_id, payload)
    }

    pub fn encode_data_row(request_id: u64, values: Vec<Value>) -> Frame {
        let msg = DataRowMessage { values };
        let payload = serde_json::to_vec(&msg).unwrap();
        Frame::new(MessageType::DataRow, request_id, payload)
    }

    pub fn encode_complete(request_id: u64, rows_affected: u64) -> Frame {
        let msg = CommandCompleteMessage { rows_affected };
        let payload = serde_json::to_vec(&msg).unwrap();
        Frame::new(MessageType::CommandComplete, request_id, payload)
    }

    pub fn encode_error(request_id: u64, code: &str, message: &str) -> Frame {
        let msg = ErrorMessage {
            code: code.to_string(),
            message: message.to_string(),
        };
        let payload = serde_json::to_vec(&msg).unwrap();
        Frame::new(MessageType::Error, request_id, payload)
    }
}
