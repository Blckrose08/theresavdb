//! WAL append-only log writer with group commit and durable sync.

use crate::record::{WalRecord, WalRecordType};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use theresav_common::{Lsn, TxId};
use theresav_error::WalError;

/// WAL Writer managing durable write-ahead logging to disk.
pub struct WalWriter {
    _path: PathBuf,
    file: Mutex<File>,
    current_lsn: AtomicU64,
    flushed_lsn: AtomicU64,
}

impl WalWriter {
    /// Open or create the WAL segment file.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, WalError> {
        let p = path.as_ref().to_path_buf();
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| WalError::CorruptedRecord(e.to_string()))?;
        }

        let file = OpenOptions::new()
            .read(true)
            .create(true)
            .append(true)
            .open(&p)
            .map_err(|e| WalError::CorruptedRecord(e.to_string()))?;

        let file_len = file
            .metadata()
            .map_err(|e| WalError::CorruptedRecord(e.to_string()))?
            .len();

        Ok(Self {
            _path: p,
            file: Mutex::new(file),
            current_lsn: AtomicU64::new(file_len),
            flushed_lsn: AtomicU64::new(file_len),
        })
    }

    /// Append a WAL record and return its allocated LSN.
    pub fn append(
        &self,
        prev_lsn: Lsn,
        tx_id: TxId,
        record_type: WalRecordType,
        payload: Vec<u8>,
    ) -> Result<Lsn, WalError> {
        let mut file = self.file.lock().unwrap();
        let allocated_lsn = Lsn::new(self.current_lsn.load(Ordering::SeqCst));

        let record = WalRecord::new(allocated_lsn, prev_lsn, tx_id, record_type, payload);
        let bytes = record.encode();

        file.write_all(&bytes)
            .map_err(|e| WalError::CorruptedRecord(format!("Failed to write WAL record: {}", e)))?;

        let new_lsn = allocated_lsn.0 + bytes.len() as u64;
        self.current_lsn.store(new_lsn, Ordering::SeqCst);

        // If transaction commits or aborts, flush immediately (group commit / durability guarantee)
        if record_type == WalRecordType::Commit || record_type == WalRecordType::Abort {
            file.sync_data().map_err(|e| {
                WalError::CorruptedRecord(format!("Failed to sync WAL to disk: {}", e))
            })?;
            self.flushed_lsn.store(new_lsn, Ordering::SeqCst);
        }

        Ok(allocated_lsn)
    }

    /// Explicitly flush and sync all pending records to disk.
    pub fn flush(&self) -> Result<(), WalError> {
        let file = self.file.lock().unwrap();
        file.sync_data()
            .map_err(|e| WalError::CorruptedRecord(format!("Failed to sync WAL: {}", e)))?;
        self.flushed_lsn
            .store(self.current_lsn.load(Ordering::SeqCst), Ordering::SeqCst);
        Ok(())
    }

    /// Latest allocated LSN.
    pub fn current_lsn(&self) -> Lsn {
        Lsn::new(self.current_lsn.load(Ordering::SeqCst))
    }

    /// Latest durable LSN synced to physical storage.
    pub fn flushed_lsn(&self) -> Lsn {
        Lsn::new(self.flushed_lsn.load(Ordering::SeqCst))
    }
}
