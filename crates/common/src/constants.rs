pub const PAGE_SIZE: usize = 16 * 1024;

pub const PAGE_HEADER_SIZE: usize = 64;

pub const SLOT_SIZE: usize = 4;

pub const PAGE_MAGIC: [u8; 4] = *b"THDB";

pub const FORMAT_VERSION_V1: u16 = 1;

pub const MAX_PAGE_PAYLOAD: usize = PAGE_SIZE - PAGE_HEADER_SIZE;

pub const MAX_SLOTS_PER_PAGE: usize = MAX_PAGE_PAYLOAD / SLOT_SIZE;

pub const PROTOCOL_MAGIC: [u8; 4] = *b"THRP";

pub const PROTOCOL_VERSION_V1: u16 = 1;

pub const DEFAULT_SERVER_PORT: u16 = 7432;

pub const DEFAULT_MAX_FRAME_SIZE: u32 = 16 * 1024 * 1024;
