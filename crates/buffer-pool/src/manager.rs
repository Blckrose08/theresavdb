use crate::frame::Frame;
use crate::replacer::ClockReplacer;
use std::collections::HashMap;
use std::fs::File;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex, RwLock};
use theresav_common::PageId;
use theresav_error::StorageError;
use theresav_page::PageType;
use theresav_storage::DiskManager;

pub struct BufferPoolManager {
    pub pool_size: usize,
    frames: Vec<Arc<RwLock<Frame>>>,
    page_table: Mutex<HashMap<PageId, usize>>,
    free_list: Mutex<Vec<usize>>,
    replacer: Mutex<ClockReplacer>,
    file: Mutex<File>,
}

impl BufferPoolManager {
    pub fn new(pool_size: usize, file: File) -> Self {
        let mut free_list = Vec::with_capacity(pool_size);
        let mut frames = Vec::with_capacity(pool_size);
        for i in 0..pool_size {
            free_list.push(i);
            frames.push(Arc::new(RwLock::new(Frame::empty())));
        }

        Self {
            pool_size,
            frames,
            page_table: Mutex::new(HashMap::new()),
            free_list: Mutex::new(free_list),
            replacer: Mutex::new(ClockReplacer::new(pool_size)),
            file: Mutex::new(file),
        }
    }

    pub fn fetch_page(&self, page_id: PageId) -> Result<Arc<RwLock<Frame>>, StorageError> {
        let mut page_table = self.page_table.lock().unwrap();

        if let Some(&frame_id) = page_table.get(&page_id) {
            let frame = self.frames[frame_id].clone();
            frame.read().unwrap().pin();
            self.replacer.lock().unwrap().record_access(frame_id);
            self.replacer.lock().unwrap().set_evictable(frame_id, false);
            return Ok(frame);
        }

        let frame_id = self.find_available_frame(&mut page_table)?;

        let page = {
            let mut file = self.file.lock().unwrap();
            DiskManager::read_page(&mut file, page_id)?
        };

        let frame = self.frames[frame_id].clone();
        {
            let mut guard = frame.write().unwrap();
            guard.page = page;
            guard.page_id = page_id;
            guard.pin_count.store(1, Ordering::SeqCst);
            guard.is_dirty.store(false, Ordering::SeqCst);
        }

        page_table.insert(page_id, frame_id);
        self.replacer.lock().unwrap().record_access(frame_id);
        self.replacer.lock().unwrap().set_evictable(frame_id, false);

        Ok(frame)
    }

    pub fn new_page(
        &self,
        page_type: PageType,
    ) -> Result<(PageId, Arc<RwLock<Frame>>), StorageError> {
        let mut page_table = self.page_table.lock().unwrap();

        let (page_id, page) = {
            let mut file = self.file.lock().unwrap();
            DiskManager::allocate_page(&mut file, page_type)?
        };

        let frame_id = self.find_available_frame(&mut page_table)?;

        let frame = self.frames[frame_id].clone();
        {
            let mut guard = frame.write().unwrap();
            guard.page = page;
            guard.page_id = page_id;
            guard.pin_count.store(1, Ordering::SeqCst);
            guard.is_dirty.store(false, Ordering::SeqCst);
        }

        page_table.insert(page_id, frame_id);
        self.replacer.lock().unwrap().record_access(frame_id);
        self.replacer.lock().unwrap().set_evictable(frame_id, false);

        Ok((page_id, frame))
    }

    pub fn unpin_page(&self, page_id: PageId, is_dirty: bool) -> Result<(), StorageError> {
        let page_table = self.page_table.lock().unwrap();
        if let Some(&frame_id) = page_table.get(&page_id) {
            let frame = &self.frames[frame_id];
            let remaining = frame.read().unwrap().unpin(is_dirty);
            if remaining == 0 {
                self.replacer.lock().unwrap().set_evictable(frame_id, true);
            }
            Ok(())
        } else {
            Err(StorageError::PageNotFound(page_id))
        }
    }

    pub fn flush_page(&self, page_id: PageId) -> Result<(), StorageError> {
        let page_table = self.page_table.lock().unwrap();
        if let Some(&frame_id) = page_table.get(&page_id) {
            let frame = &self.frames[frame_id];
            let mut frame_guard = frame.write().unwrap();
            if frame_guard.is_dirty() {
                let mut file = self.file.lock().unwrap();
                DiskManager::write_page(&mut file, page_id, &mut frame_guard.page)?;
                frame_guard.clear_dirty();
            }
            Ok(())
        } else {
            Err(StorageError::PageNotFound(page_id))
        }
    }

    pub fn flush_all(&self) -> Result<(), StorageError> {
        let page_table = self.page_table.lock().unwrap();
        let mut file = self.file.lock().unwrap();

        for (&page_id, &frame_id) in page_table.iter() {
            let frame = &self.frames[frame_id];
            let mut frame_guard = frame.write().unwrap();
            if frame_guard.is_dirty() {
                DiskManager::write_page(&mut file, page_id, &mut frame_guard.page)?;
                frame_guard.clear_dirty();
            }
        }

        DiskManager::sync(&file)
    }

    fn find_available_frame(
        &self,
        page_table: &mut HashMap<PageId, usize>,
    ) -> Result<usize, StorageError> {
        let mut free_list = self.free_list.lock().unwrap();
        if let Some(fid) = free_list.pop() {
            return Ok(fid);
        }

        let victim_fid = self
            .replacer
            .lock()
            .unwrap()
            .victim()
            .ok_or(StorageError::DiskFull)?;

        let victim_frame = &self.frames[victim_fid];
        let mut victim_guard = victim_frame.write().unwrap();

        if victim_guard.is_dirty() {
            let mut file = self.file.lock().unwrap();
            DiskManager::write_page(&mut file, victim_guard.page_id, &mut victim_guard.page)?;
            victim_guard.clear_dirty();
        }

        page_table.remove(&victim_guard.page_id);
        Ok(victim_fid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_buffer_pool_lifecycle() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("bpm_test.thdb");
        let file = DiskManager::open_file(&file_path).unwrap();

        let bpm = BufferPoolManager::new(3, file);

        let (p0, f0) = bpm.new_page(PageType::SlottedData).unwrap();
        let (p1, _f1) = bpm.new_page(PageType::SlottedData).unwrap();
        let (p2, _f2) = bpm.new_page(PageType::SlottedData).unwrap();

        {
            let mut g0 = f0.write().unwrap();
            g0.page.insert_tuple(b"data-on-page-0").unwrap();
        }

        bpm.unpin_page(p0, true).unwrap();
        bpm.unpin_page(p1, false).unwrap();
        bpm.unpin_page(p2, false).unwrap();

        let (p3, _) = bpm.new_page(PageType::SlottedData).unwrap();
        bpm.unpin_page(p3, false).unwrap();

        let f0_reloaded = bpm.fetch_page(p0).unwrap();
        {
            let g0 = f0_reloaded.read().unwrap();
            let tuple = g0.page.get_tuple(theresav_common::SlotId::new(0)).unwrap();
            assert_eq!(tuple, b"data-on-page-0");
        }
        bpm.unpin_page(p0, false).unwrap();
    }
}
