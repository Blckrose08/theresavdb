use crate::disk::DiskManager;
use crate::fsm::FreeSpaceMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use theresav_common::{PageId, TupleId, PAGE_HEADER_SIZE, PAGE_SIZE};
use theresav_error::StorageError;
use theresav_page::PageType;

pub struct HeapFile {
    _path: PathBuf,
    file: File,
    fsm: FreeSpaceMap,
}

impl HeapFile {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, StorageError> {
        let p = path.as_ref().to_path_buf();
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).map_err(|e| StorageError::Io(e.to_string()))?;
        }

        let mut file = DiskManager::open_file(&p)?;
        let mut fsm = FreeSpaceMap::new();

        let count = DiskManager::page_count(&file)?;
        for i in 0..count {
            let pid = PageId::new(i);
            if let Ok(page) = DiskManager::read_page(&mut file, pid) {
                fsm.update(pid, page.contiguous_free_space());
            }
        }

        Ok(Self {
            _path: p,
            file,
            fsm,
        })
    }

    pub fn insert_tuple(&mut self, tuple_data: &[u8]) -> Result<TupleId, StorageError> {
        let needed_space = tuple_data.len() + 4;
        if needed_space > PAGE_SIZE - PAGE_HEADER_SIZE {
            return Err(StorageError::CorruptedFile(format!(
                "Tuple size {} exceeds maximum page capacity",
                tuple_data.len()
            )));
        }

        let target_page_id = match self.fsm.find_page_with_space(needed_space) {
            Some(pid) => pid,
            None => {
                let (new_pid, _) =
                    DiskManager::allocate_page(&mut self.file, PageType::SlottedData)?;
                self.fsm.update(new_pid, PAGE_SIZE - PAGE_HEADER_SIZE);
                new_pid
            }
        };

        let mut page = DiskManager::read_page(&mut self.file, target_page_id)?;
        let slot_id = page.insert_tuple(tuple_data).map_err(|e| {
            StorageError::CorruptedFile(format!(
                "Insert failed on page {}: {}",
                target_page_id.0, e
            ))
        })?;

        DiskManager::write_page(&mut self.file, target_page_id, &mut page)?;
        self.fsm
            .update(target_page_id, page.contiguous_free_space());

        Ok(TupleId::new(target_page_id, slot_id))
    }

    pub fn get_tuple(&mut self, tid: TupleId) -> Result<Vec<u8>, StorageError> {
        let page = DiskManager::read_page(&mut self.file, tid.page_id)?;
        let tuple_bytes = page.get_tuple(tid.slot_id).map_err(|e| {
            StorageError::CorruptedFile(format!("Failed to get tuple {}: {}", tid, e))
        })?;
        Ok(tuple_bytes.to_vec())
    }

    pub fn delete_tuple(&mut self, tid: TupleId) -> Result<(), StorageError> {
        let mut page = DiskManager::read_page(&mut self.file, tid.page_id)?;
        page.delete_tuple(tid.slot_id).map_err(|e| {
            StorageError::CorruptedFile(format!("Failed to delete tuple {}: {}", tid, e))
        })?;

        DiskManager::write_page(&mut self.file, tid.page_id, &mut page)?;
        self.fsm.update(tid.page_id, page.contiguous_free_space());
        Ok(())
    }

    pub fn page_count(&self) -> Result<u64, StorageError> {
        DiskManager::page_count(&self.file)
    }

    pub fn sync(&self) -> Result<(), StorageError> {
        DiskManager::sync(&self.file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_heap_file_insert_and_retrieval() {
        let dir = tempdir().unwrap();
        let table_path = dir.path().join("users.thdb");

        let mut heap = HeapFile::open(&table_path).expect("Opening heap file should succeed");

        let t1 = b"Row 1: Alice in Wonderland";
        let t2 = b"Row 2: Bob the Builder";

        let tid1 = heap.insert_tuple(t1).expect("Insert t1 must succeed");
        let tid2 = heap.insert_tuple(t2).expect("Insert t2 must succeed");

        assert_eq!(heap.get_tuple(tid1).unwrap(), t1);
        assert_eq!(heap.get_tuple(tid2).unwrap(), t2);

        heap.delete_tuple(tid1).expect("Delete must succeed");
        assert!(heap.get_tuple(tid1).is_err());

        assert_eq!(heap.get_tuple(tid2).unwrap(), t2);

        drop(heap);
        let mut reopened = HeapFile::open(&table_path).expect("Reopen must succeed");
        assert_eq!(reopened.get_tuple(tid2).unwrap(), t2);
    }
}
