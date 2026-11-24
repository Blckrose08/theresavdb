//! Structured domain error hierarchy for TheresavDB.

use theresav_common::{PageId, SlotId, TxId};
use thiserror::Error;

/// Master error type for all TheresavDB operations.
#[derive(Debug, Error)]
pub enum TheresavError {
    #[error("Page error: {0}")]
    Page(#[from] PageError),

    #[error("Type error: {0}")]
    Type(#[from] TypeError),

    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("Transaction error: {0}")]
    Transaction(#[from] TransactionError),

    #[error("WAL error: {0}")]
    Wal(#[from] WalError),

    #[error("Catalog error: {0}")]
    Catalog(#[from] CatalogError),

    #[error("Protocol error: {0}")]
    Protocol(#[from] ProtocolError),

    #[error("Authentication error: {0}")]
    Auth(#[from] AuthError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Errors related to physical 16 KiB binary page layout and validation.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum PageError {
    #[error("Invalid magic bytes: expected b\"THDB\", found {0:?}")]
    InvalidMagic([u8; 4]),

    #[error("Unsupported binary page format version: {0}")]
    UnsupportedVersion(u16),

    #[error("Page checksum mismatch: stored={expected:#010x}, calculated={calculated:#010x}")]
    ChecksumMismatch { expected: u32, calculated: u32 },

    #[error("Invalid page boundaries: lower={lower}, upper={upper}, page_size={page_size}")]
    InvalidBoundary {
        lower: u16,
        upper: u16,
        page_size: usize,
    },

    #[error("Slot index {slot} out of bounds (slot count: {count})")]
    SlotOutOfBounds { slot: u16, count: u16 },

    #[error("Invalid slot {slot_id}: {reason}")]
    InvalidSlot {
        slot_id: SlotId,
        reason: &'static str,
    },

    #[error("Page out of space: requested {required} bytes, available {available} bytes")]
    PageFull { required: usize, available: usize },

    #[error("Tuple corrupted: {0}")]
    TupleCorrupted(String),

    #[error("Page {0} corrupted: {1}")]
    Corrupted(PageId, String),

    #[error("Invalid page type code: {0}")]
    InvalidPageType(u16),
}

/// Errors occurring during data type conversion, arithmetic, or serialization.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum TypeError {
    #[error("Type mismatch: expected {expected}, found {found}")]
    TypeMismatch {
        expected: &'static str,
        found: &'static str,
    },

    #[error("Numeric overflow during arithmetic or conversion: {0}")]
    Overflow(String),

    #[error("Division by zero")]
    DivisionByZero,

    #[error("Invalid binary data encoding: {0}")]
    InvalidEncoding(String),

    #[error("Value parsing error: {0}")]
    ParseError(String),

    #[error("Invalid JSON or JSONB payload: {0}")]
    InvalidJson(String),
}

/// Errors occurring in the storage engine, heap file manager, or buffer pool.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum StorageError {
    #[error("Page not found: {0}")]
    PageNotFound(PageId),

    #[error("Storage file corrupted: {0}")]
    CorruptedFile(String),

    #[error("Disk full: cannot allocate additional pages")]
    DiskFull,

    #[error("Database directory lock failure: {0}")]
    LockFailure(String),
}

/// Errors in transaction lifecycle, concurrency control, or isolation.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum TransactionError {
    #[error("Transaction {0} is aborted")]
    Aborted(TxId),

    #[error("Transaction {0} already finished")]
    AlreadyFinished(TxId),

    #[error("Write-write conflict on tuple in tx {0}")]
    WriteConflict(TxId),

    #[error("Serialization failure under SSI")]
    SerializationFailure,

    #[error("Deadlock detected among transactions")]
    Deadlock,

    #[error("Transaction timed out")]
    Timeout,
}

/// Errors during Write-Ahead Logging or crash recovery.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum WalError {
    #[error("WAL record corrupted: {0}")]
    CorruptedRecord(String),

    #[error("WAL checksum mismatch: expected {expected:#010x}, calculated {calculated:#010x}")]
    ChecksumMismatch { expected: u32, calculated: u32 },

    #[error("Unexpected end of WAL segment")]
    UnexpectedEof,

    #[error("Recovery failed: {0}")]
    RecoveryFailed(String),
}

/// Errors interacting with system catalogs or metadata objects.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum CatalogError {
    #[error("Table '{0}' does not exist")]
    TableNotFound(String),

    #[error("Table '{0}' already exists")]
    TableAlreadyExists(String),

    #[error("Column '{0}' does not exist")]
    ColumnNotFound(String),

    #[error("Index '{0}' does not exist")]
    IndexNotFound(String),

    #[error("Schema version mismatch: expected {expected}, found {found}")]
    SchemaVersionMismatch { expected: u32, found: u32 },
}

/// Errors in client-server wire protocol framing or dispatch.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum ProtocolError {
    #[error("Invalid protocol magic bytes: expected b\"THRP\", found {0:?}")]
    InvalidMagic([u8; 4]),

    #[error("Unsupported protocol version: {0}")]
    UnsupportedVersion(u16),

    #[error("Frame size {size} exceeds maximum limit of {max_limit} bytes")]
    FrameTooLarge { size: u32, max_limit: u32 },

    #[error("Unknown or invalid message type opcode: {0:#06x}")]
    UnknownMessageType(u16),

    #[error("Protocol stream terminated unexpectedly")]
    UnexpectedEof,

    #[error("Protocol error: {0}")]
    Other(String),
}

/// Authentication and authorization errors.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum AuthError {
    #[error("Authentication failed: invalid username or password")]
    InvalidCredentials,

    #[error("User '{0}' does not exist")]
    UserNotFound(String),

    #[error("Permission denied: role requires {permission} on {resource}")]
    PermissionDenied {
        permission: String,
        resource: String,
    },

    #[error("Authentication mechanism '{0}' not supported")]
    UnsupportedMechanism(String),
}

/// Global Result type alias for TheresavDB.
pub type Result<T, E = TheresavError> = std::result::Result<T, E>;
