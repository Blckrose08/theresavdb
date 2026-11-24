//! Write-Ahead Logging (WAL) engine for crash recovery and ACID durability.

pub mod reader;
pub mod record;
pub mod writer;

pub use reader::WalReader;
pub use record::{WalRecord, WalRecordType, WAL_RECORD_HEADER_SIZE};
pub use writer::WalWriter;
