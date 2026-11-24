use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use theresav_common::{PageId, PAGE_SIZE};
use theresav_error::StorageError;
use theresav_page::{Page, PageType};

#[derive(Debug)]
pub struct DiskManager;

impl DiskManager {
    pub fn open_file<P: AsRef<Path>>(path: P) -> Result<File, StorageError> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path.as_ref())
            .map_err(|e| StorageError::Io(format!("Failed to open database file: {}", e)))?;

        let meta = file
            .metadata()
            .map_err(|e| StorageError::Io(format!("Failed to get file metadata: {}", e)))?;
        if meta.len() % (PAGE_SIZE as u64) != 0 {
            return Err(StorageError::CorruptedFile(format!(
                "File length {} is not a multiple of page size {}",
                meta.len(),
                PAGE_SIZE
            )));
        }

        Ok(file)
    }

    pub fn read_page(file: &mut File, page_id: PageId) -> Result<Page, StorageError> {
        let offset = page_id.0.checked_mul(PAGE_SIZE as u64).ok_or_else(|| {
            StorageError::CorruptedFile(format!("PageId {} overflows file offset", page_id.0))
        })?;

        file.seek(SeekFrom::Start(offset))
            .map_err(|e| StorageError::Io(format!("Seek failure at offset {}: {}", offset, e)))?;

        let mut buf = [0u8; PAGE_SIZE];
        file.read_exact(&mut buf).map_err(|e| {
            StorageError::Io(format!(
                "Failed to read page {} at offset {}: {}",
                page_id.0, offset, e
            ))
        })?;

        Page::from_bytes(&buf).map_err(|e| {
            StorageError::CorruptedFile(format!("Page {} failed verification: {}", page_id.0, e))
        })
    }

    pub fn write_page(
        file: &mut File,
        page_id: PageId,
        page: &mut Page,
    ) -> Result<(), StorageError> {
        let offset = page_id.0.checked_mul(PAGE_SIZE as u64).ok_or_else(|| {
            StorageError::CorruptedFile(format!("PageId {} overflows file offset", page_id.0))
        })?;

        page.compute_checksum();

        file.seek(SeekFrom::Start(offset))
            .map_err(|e| StorageError::Io(format!("Seek failure at offset {}: {}", offset, e)))?;

        file.write_all(page.as_bytes()).map_err(|e| {
            StorageError::Io(format!(
                "Failed to write page {} at offset {}: {}",
                page_id.0, offset, e
            ))
        })?;

        Ok(())
    }

    pub fn allocate_page(
        file: &mut File,
        page_type: PageType,
    ) -> Result<(PageId, Page), StorageError> {
        let len = file
            .metadata()
            .map_err(|e| StorageError::Io(e.to_string()))?
            .len();
        let next_page_id = PageId::new(len / (PAGE_SIZE as u64));

        let mut new_page = Page::new(next_page_id, page_type);
        Self::write_page(file, next_page_id, &mut new_page)?;

        Ok((next_page_id, new_page))
    }

    pub fn page_count(file: &File) -> Result<u64, StorageError> {
        let len = file
            .metadata()
            .map_err(|e| StorageError::Io(e.to_string()))?
            .len();
        Ok(len / (PAGE_SIZE as u64))
    }

    pub fn sync(file: &File) -> Result<(), StorageError> {
        file.sync_data()
            .map_err(|e| StorageError::Io(format!("Failed to sync file: {}", e)))
    }
}
