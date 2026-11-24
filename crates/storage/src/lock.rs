use std::fs::{File, OpenOptions};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use theresav_error::StorageError;

pub struct DatabaseProcessLock {
    _file: File,
    lock_path: PathBuf,
}

impl DatabaseProcessLock {
    pub fn acquire<P: AsRef<Path>>(db_dir: P) -> Result<Self, StorageError> {
        let dir = db_dir.as_ref();
        if !dir.exists() {
            std::fs::create_dir_all(dir).map_err(|e| {
                StorageError::LockFailure(format!("Failed to create data directory: {}", e))
            })?;
        }

        let lock_path = dir.join("theresav.lock");
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(|e| StorageError::LockFailure(format!("Failed to open lock file: {}", e)))?;

        let fd = file.as_raw_fd();
        let ret = unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) };
        if ret != 0 {
            return Err(StorageError::LockFailure(format!(
                "Database directory '{}' is already locked by another process",
                dir.display()
            )));
        }

        Ok(Self {
            _file: file,
            lock_path,
        })
    }

    pub fn lock_path(&self) -> &Path {
        &self.lock_path
    }
}

impl Drop for DatabaseProcessLock {
    fn drop(&mut self) {
        let fd = self._file.as_raw_fd();
        let _ = unsafe { libc::flock(fd, libc::LOCK_UN) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_process_lock_exclusivity() {
        let dir = tempdir().unwrap();
        let lock1 = DatabaseProcessLock::acquire(dir.path()).expect("First lock should succeed");

        let lock2_res = DatabaseProcessLock::acquire(dir.path());
        assert!(lock2_res.is_err());

        drop(lock1);
        let lock2 = DatabaseProcessLock::acquire(dir.path());
        assert!(lock2.is_ok());
    }
}
