//! WAL log reader for crash recovery and replication stream ingestion.

use crate::record::WalRecord;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use theresav_error::WalError;

/// Sequential WAL reader scanning log records from disk.
pub struct WalReader {
    buffer: Vec<u8>,
    cursor: usize,
}

impl WalReader {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, WalError> {
        let mut file = File::open(path.as_ref())
            .map_err(|e| WalError::CorruptedRecord(format!("Failed to open WAL file: {}", e)))?;

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .map_err(|e| WalError::CorruptedRecord(format!("Failed to read WAL file: {}", e)))?;

        Ok(Self { buffer, cursor: 0 })
    }

    /// Read next valid record from the WAL stream.
    pub fn next_record(&mut self) -> Result<Option<WalRecord>, WalError> {
        if self.cursor >= self.buffer.len() {
            return Ok(None);
        }

        let slice = &self.buffer[self.cursor..];
        match WalRecord::decode(slice) {
            Ok((record, bytes_read)) => {
                self.cursor += bytes_read;
                Ok(Some(record))
            }
            Err(WalError::UnexpectedEof) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::WalRecordType;
    use crate::writer::WalWriter;
    use tempfile::tempdir;
    use theresav_common::{Lsn, TxId};

    #[test]
    fn test_wal_roundtrip_and_durability() {
        let dir = tempdir().unwrap();
        let wal_path = dir.path().join("theresav.wal");

        let writer = WalWriter::open(&wal_path).unwrap();

        let lsn1 = writer
            .append(Lsn::INVALID, TxId::new(1), WalRecordType::Begin, vec![])
            .unwrap();

        let lsn2 = writer
            .append(
                lsn1,
                TxId::new(1),
                WalRecordType::Insert,
                b"Inserted tuple".to_vec(),
            )
            .unwrap();

        let lsn3 = writer
            .append(lsn2, TxId::new(1), WalRecordType::Commit, vec![])
            .unwrap();

        assert!(writer.flushed_lsn() >= lsn3);

        // Read records back using WalReader
        let mut reader = WalReader::open(&wal_path).unwrap();

        let r1 = reader.next_record().unwrap().unwrap();
        assert_eq!(r1.lsn, lsn1);
        assert_eq!(r1.record_type, WalRecordType::Begin);

        let r2 = reader.next_record().unwrap().unwrap();
        assert_eq!(r2.lsn, lsn2);
        assert_eq!(r2.record_type, WalRecordType::Insert);
        assert_eq!(r2.payload, b"Inserted tuple");

        let r3 = reader.next_record().unwrap().unwrap();
        assert_eq!(r3.lsn, lsn3);
        assert_eq!(r3.record_type, WalRecordType::Commit);

        let r4 = reader.next_record().unwrap();
        assert!(r4.is_none());
    }
}
