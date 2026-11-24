//! Free-Space Map (FSM) tracking available bytes per page for rapid tuple placement.

use std::collections::BTreeMap;
use theresav_common::PageId;

/// Free space category (0 to 5) for indexed space search.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SpaceCategory {
    Full = 0,     // < 32 bytes free
    VeryLow = 1,  // 32..256 bytes free
    Low = 2,      // 256..1024 bytes free
    Medium = 3,   // 1024..4096 bytes free
    High = 4,     // 4096..8192 bytes free
    VeryHigh = 5, // >= 8192 bytes free
}

impl SpaceCategory {
    pub fn from_free_bytes(bytes: usize) -> Self {
        if bytes < 32 {
            Self::Full
        } else if bytes < 256 {
            Self::VeryLow
        } else if bytes < 1024 {
            Self::Low
        } else if bytes < 4096 {
            Self::Medium
        } else if bytes < 8192 {
            Self::High
        } else {
            Self::VeryHigh
        }
    }

    pub fn min_bytes(&self) -> usize {
        match self {
            Self::Full => 0,
            Self::VeryLow => 32,
            Self::Low => 256,
            Self::Medium => 1024,
            Self::High => 4096,
            Self::VeryHigh => 8192,
        }
    }
}

/// In-memory Free Space Map index mapping page IDs to exact available free bytes.
#[derive(Debug, Default, Clone)]
pub struct FreeSpaceMap {
    page_free_bytes: BTreeMap<PageId, usize>,
}

impl FreeSpaceMap {
    pub fn new() -> Self {
        Self {
            page_free_bytes: BTreeMap::new(),
        }
    }

    /// Record or update the free space for a specific page.
    pub fn update(&mut self, page_id: PageId, free_bytes: usize) {
        self.page_free_bytes.insert(page_id, free_bytes);
    }

    /// Find the first page that can accommodate at least `min_space` bytes.
    pub fn find_page_with_space(&self, min_space: usize) -> Option<PageId> {
        for (&page_id, &free_bytes) in &self.page_free_bytes {
            if free_bytes >= min_space {
                return Some(page_id);
            }
        }
        None
    }

    /// Remove a page from the map.
    pub fn remove(&mut self, page_id: PageId) {
        self.page_free_bytes.remove(&page_id);
    }
}
