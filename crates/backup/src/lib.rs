//! Backup and Restore engine for TheresavDB with manifest and checksum integrity verification.

use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;
use theresav_error::StorageError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub relative_path: String,
    pub size: u64,
    pub checksum: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupManifest {
    pub version: u16,
    pub timestamp: u64,
    pub files: Vec<FileEntry>,
}

pub struct BackupManager;

impl BackupManager {
    /// Create a full backup from `db_path` into `target_dir`.
    pub fn create_backup<P: AsRef<Path>, Q: AsRef<Path>>(
        db_path: P,
        target_dir: Q,
    ) -> Result<BackupManifest, StorageError> {
        let src = db_path.as_ref();
        let dst = target_dir.as_ref();
        fs::create_dir_all(dst).map_err(|e| StorageError::Io(e.to_string()))?;

        let mut manifest_files = Vec::new();

        // Recursively copy and calculate checksums
        Self::copy_and_checksum_dir(src, dst, src, &mut manifest_files)?;

        let manifest = BackupManifest {
            version: 1,
            timestamp: 1732449000,
            files: manifest_files,
        };

        let manifest_json = serde_json::to_string_pretty(&manifest)
            .map_err(|e| StorageError::CorruptedFile(e.to_string()))?;
        fs::write(dst.join("manifest.json"), manifest_json)
            .map_err(|e| StorageError::Io(e.to_string()))?;

        Ok(manifest)
    }

    /// Restore a backup from `backup_dir` into `restore_path`, validating all file checksums.
    pub fn restore_backup<P: AsRef<Path>, Q: AsRef<Path>>(
        backup_dir: P,
        restore_path: Q,
    ) -> Result<(), StorageError> {
        let bkp = backup_dir.as_ref();
        let target = restore_path.as_ref();

        let manifest_path = bkp.join("manifest.json");
        let manifest_data = fs::read_to_string(&manifest_path)
            .map_err(|e| StorageError::CorruptedFile(format!("Missing backup manifest: {}", e)))?;
        let manifest: BackupManifest = serde_json::from_str(&manifest_data)
            .map_err(|e| StorageError::CorruptedFile(format!("Invalid manifest: {}", e)))?;

        // Verify checksums of all backup files before restoring
        for entry in &manifest.files {
            let file_in_backup = bkp.join(&entry.relative_path);
            let calculated_chk = Self::compute_file_checksum(&file_in_backup)?;
            if calculated_chk != entry.checksum {
                return Err(StorageError::CorruptedFile(format!(
                    "Backup file '{}' corrupted: expected checksum {:#010x}, found {:#010x}",
                    entry.relative_path, entry.checksum, calculated_chk
                )));
            }
        }

        // Restore files to destination
        fs::create_dir_all(target).map_err(|e| StorageError::Io(e.to_string()))?;
        for entry in &manifest.files {
            let file_in_backup = bkp.join(&entry.relative_path);
            let dest_file = target.join(&entry.relative_path);
            if let Some(parent) = dest_file.parent() {
                fs::create_dir_all(parent).map_err(|e| StorageError::Io(e.to_string()))?;
            }
            fs::copy(&file_in_backup, &dest_file).map_err(|e| StorageError::Io(e.to_string()))?;
        }

        Ok(())
    }

    fn copy_and_checksum_dir(
        curr_dir: &Path,
        dst_root: &Path,
        src_root: &Path,
        manifest_files: &mut Vec<FileEntry>,
    ) -> Result<(), StorageError> {
        if !curr_dir.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(curr_dir).map_err(|e| StorageError::Io(e.to_string()))? {
            let entry = entry.map_err(|e| StorageError::Io(e.to_string()))?;
            let path = entry.path();
            let file_name = path.file_name().unwrap().to_string_lossy();

            // Skip lock files
            if file_name.ends_with(".lock") {
                continue;
            }

            if path.is_dir() {
                Self::copy_and_checksum_dir(&path, dst_root, src_root, manifest_files)?;
            } else if path.is_file() {
                let rel = path.strip_prefix(src_root).unwrap().to_str().unwrap().to_string();
                let dst_file = dst_root.join(&rel);
                if let Some(parent) = dst_file.parent() {
                    fs::create_dir_all(parent).map_err(|e| StorageError::Io(e.to_string()))?;
                }
                fs::copy(&path, &dst_file).map_err(|e| StorageError::Io(e.to_string()))?;

                let size = entry.metadata().map_err(|e| StorageError::Io(e.to_string()))?.len();
                let checksum = Self::compute_file_checksum(&path)?;

                manifest_files.push(FileEntry {
                    relative_path: rel,
                    size,
                    checksum,
                });
            }
        }
        Ok(())
    }

    fn compute_file_checksum(path: &Path) -> Result<u32, StorageError> {
        let mut file = File::open(path).map_err(|e| StorageError::Io(e.to_string()))?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).map_err(|e| StorageError::Io(e.to_string()))?;
        Ok(crc32c::crc32c(&buf))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_backup_and_restore_cycle() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("live_db");
        let backup_path = dir.path().join("backup_dir");
        let restore_path = dir.path().join("restored_db");

        fs::create_dir_all(&db_path).unwrap();
        fs::write(db_path.join("table1.thdb"), b"Data for table 1").unwrap();

        // Create backup
        let manifest = BackupManager::create_backup(&db_path, &backup_path).unwrap();
        assert_eq!(manifest.files.len(), 1);

        // Restore backup
        BackupManager::restore_backup(&backup_path, &restore_path).unwrap();
        let restored_content = fs::read(restore_path.join("table1.thdb")).unwrap();
        assert_eq!(restored_content, b"Data for table 1");
    }
}
