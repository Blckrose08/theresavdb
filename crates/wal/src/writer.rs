use crate::record::{WalRecord, WalRecordType};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use theresav_common::{Lsn, TxId};
use theresav_error::WalError;

pub struct WalWriter {
    _path: PathBuf,
    file: Mutex<File>,
    current_lsn: AtomicU64,
    flushed_lsn: AtomicU64,
}

impl WalWriter {
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

        if record_type == WalRecordType::Commit || record_type == WalRecordType::Abort {
            file.sync_data().map_err(|e| {
                WalError::CorruptedRecord(format!("Failed to sync WAL to disk: {}", e))
            })?;
            self.flushed_lsn.store(new_lsn, Ordering::SeqCst);
        }

        Ok(allocated_lsn)
    }

    pub fn flush(&self) -> Result<(), WalError> {
        let file = self.file.lock().unwrap();
        file.sync_data()
            .map_err(|e| WalError::CorruptedRecord(format!("Failed to sync WAL: {}", e)))?;
        self.flushed_lsn
            .store(self.current_lsn.load(Ordering::SeqCst), Ordering::SeqCst);
        Ok(())
    }

    pub fn current_lsn(&self) -> Lsn {
        Lsn::new(self.current_lsn.load(Ordering::SeqCst))
    }

    pub fn flushed_lsn(&self) -> Lsn {
        Lsn::new(self.flushed_lsn.load(Ordering::SeqCst))
    }
}
