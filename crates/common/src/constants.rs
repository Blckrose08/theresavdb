//! Core architectural constants for TheresavDB.

/// Default physical page size: 16 KiB (16,384 bytes).
pub const PAGE_SIZE: usize = 16 * 1024;

/// Physical page header size in bytes.
pub const PAGE_HEADER_SIZE: usize = 64;

/// Size of a single slot directory entry in bytes (offset: u16, length: u16).
pub const SLOT_SIZE: usize = 4;

/// Magic bytes identifying a valid TheresavDB binary page ("THDB").
pub const PAGE_MAGIC: [u8; 4] = *b"THDB";

/// Current binary page format version.
pub const FORMAT_VERSION_V1: u16 = 1;

/// Maximum payload available on an empty page.
pub const MAX_PAGE_PAYLOAD: usize = PAGE_SIZE - PAGE_HEADER_SIZE;

/// Absolute maximum number of slots that can theoretically fit on a page.
pub const MAX_SLOTS_PER_PAGE: usize = MAX_PAGE_PAYLOAD / SLOT_SIZE;

/// Magic bytes identifying a TheresavDB binary wire protocol frame ("THRP").
pub const PROTOCOL_MAGIC: [u8; 4] = *b"THRP";

/// Current binary wire protocol version.
pub const PROTOCOL_VERSION_V1: u16 = 1;

/// Default database server TCP port.
pub const DEFAULT_SERVER_PORT: u16 = 7432;

/// Default maximum frame size for protocol transport: 16 MiB.
pub const DEFAULT_MAX_FRAME_SIZE: u32 = 16 * 1024 * 1024;
