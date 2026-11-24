//! Buffer frame representing an in-memory page cache slot.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use theresav_common::PageId;
use theresav_page::Page;

/// A frame holding an in-memory Page, pin count, and dirty state.
pub struct Frame {
    pub page: Page,
    pub page_id: PageId,
    pub pin_count: AtomicU32,
    pub is_dirty: AtomicBool,
}

impl Frame {
    pub fn new(page_id: PageId, page: Page) -> Self {
        Self {
            page,
            page_id,
            pin_count: AtomicU32::new(1),
            is_dirty: AtomicBool::new(false),
        }
    }

    pub fn empty() -> Self {
        Self {
            page: Page::new(PageId::INVALID, theresav_page::PageType::SlottedData),
            page_id: PageId::INVALID,
            pin_count: AtomicU32::new(0),
            is_dirty: AtomicBool::new(false),
        }
    }

    pub fn pin(&self) {
        self.pin_count.fetch_add(1, Ordering::SeqCst);
    }

    pub fn unpin(&self, is_dirty: bool) -> u32 {
        if is_dirty {
            self.is_dirty.store(true, Ordering::SeqCst);
        }
        let prev = self.pin_count.fetch_sub(1, Ordering::SeqCst);
        prev.saturating_sub(1)
    }

    pub fn is_pinned(&self) -> bool {
        self.pin_count.load(Ordering::SeqCst) > 0
    }

    pub fn is_dirty(&self) -> bool {
        self.is_dirty.load(Ordering::SeqCst)
    }

    pub fn clear_dirty(&self) {
        self.is_dirty.store(false, Ordering::SeqCst);
    }
}
