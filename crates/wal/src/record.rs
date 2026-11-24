//! WAL record definitions and binary serialization following ARIES guidelines.

use byteorder::{ByteOrder, LittleEndian};
use theresav_common::{Lsn, TxId};
use theresav_error::WalError;

/// Types of WAL log records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum WalRecordType {
    Begin = 1,
    Insert = 2,
    Update = 3,
    Delete = 4,
    PageUpdate = 5,
    Commit = 6,
    Abort = 7,
    Checkpoint = 8,
    Clr = 9, // Compensation Log Record for ARIES Undo
}

impl WalRecordType {
    pub fn from_u16(val: u16) -> Result<Self, WalError> {
        match val {
            1 => Ok(Self::Begin),
            2 => Ok(Self::Insert),
            3 => Ok(Self::Update),
            4 => Ok(Self::Delete),
            5 => Ok(Self::PageUpdate),
            6 => Ok(Self::Commit),
            7 => Ok(Self::Abort),
            8 => Ok(Self::Checkpoint),
            9 => Ok(Self::Clr),
            other => Err(WalError::CorruptedRecord(format!(
                "Invalid WAL record type: {}",
                other
            ))),
        }
    }

    #[inline]
    pub const fn to_u16(self) -> u16 {
        self as u16
    }
}

/// A complete, validated WAL record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalRecord {
    pub lsn: Lsn,
    pub prev_lsn: Lsn,
    pub tx_id: TxId,
    pub record_type: WalRecordType,
    pub payload: Vec<u8>,
}

/// Fixed header size:
/// - record_len: 4
/// - lsn: 8
/// - prev_lsn: 8
/// - tx_id: 8
/// - record_type: 2
/// - checksum: 4
///
/// Total = 34 bytes
pub const WAL_RECORD_HEADER_SIZE: usize = 34;

impl WalRecord {
    pub fn new(
        lsn: Lsn,
        prev_lsn: Lsn,
        tx_id: TxId,
        record_type: WalRecordType,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            lsn,
            prev_lsn,
            tx_id,
            record_type,
            payload,
        }
    }

    /// Encode WAL record to binary buffer with computed CRC32C checksum.
    pub fn encode(&self) -> Vec<u8> {
        let total_len = WAL_RECORD_HEADER_SIZE + self.payload.len();
        let mut buf = vec![0u8; total_len];

        LittleEndian::write_u32(&mut buf[0..4], total_len as u32);
        LittleEndian::write_u64(&mut buf[4..12], self.lsn.0);
        LittleEndian::write_u64(&mut buf[12..20], self.prev_lsn.0);
        LittleEndian::write_u64(&mut buf[20..28], self.tx_id.0);
        LittleEndian::write_u16(&mut buf[28..30], self.record_type.to_u16());
        // checksum at 30..34 will be written below

        // Payload
        buf[WAL_RECORD_HEADER_SIZE..total_len].copy_from_slice(&self.payload);

        // Compute CRC32C over everything except the checksum field itself
        let mut chk = crc32c::crc32c(&buf[0..30]);
        chk = crc32c::crc32c_append(chk, &buf[34..total_len]);
        LittleEndian::write_u32(&mut buf[30..34], chk);

        buf
    }

    /// Decode a WAL record from bytes with strict checksum and boundary checks.
    pub fn decode(bytes: &[u8]) -> Result<(Self, usize), WalError> {
        if bytes.len() < WAL_RECORD_HEADER_SIZE {
            return Err(WalError::UnexpectedEof);
        }

        let total_len = LittleEndian::read_u32(&bytes[0..4]) as usize;
        if total_len < WAL_RECORD_HEADER_SIZE || total_len > bytes.len() {
            return Err(WalError::UnexpectedEof);
        }

        let lsn = Lsn::new(LittleEndian::read_u64(&bytes[4..12]));
        let prev_lsn = Lsn::new(LittleEndian::read_u64(&bytes[12..20]));
        let tx_id = TxId::new(LittleEndian::read_u64(&bytes[20..28]));
        let record_type = WalRecordType::from_u16(LittleEndian::read_u16(&bytes[28..30]))?;
        let stored_checksum = LittleEndian::read_u32(&bytes[30..34]);

        // Verify checksum
        let mut calculated_checksum = crc32c::crc32c(&bytes[0..30]);
        calculated_checksum = crc32c::crc32c_append(calculated_checksum, &bytes[34..total_len]);

        if stored_checksum != calculated_checksum {
            return Err(WalError::ChecksumMismatch {
                expected: stored_checksum,
                calculated: calculated_checksum,
            });
        }

        let payload = bytes[WAL_RECORD_HEADER_SIZE..total_len].to_vec();

        Ok((
            Self {
                lsn,
                prev_lsn,
                tx_id,
                record_type,
                payload,
            },
            total_len,
        ))
    }
}
