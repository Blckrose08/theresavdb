use std::collections::BTreeMap;
use theresav_common::PageId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SpaceCategory {
    Full = 0,
    VeryLow = 1,
    Low = 2,
    Medium = 3,
    High = 4,
    VeryHigh = 5,
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

    pub fn update(&mut self, page_id: PageId, free_bytes: usize) {
        self.page_free_bytes.insert(page_id, free_bytes);
    }

    pub fn find_page_with_space(&self, min_space: usize) -> Option<PageId> {
        for (&page_id, &free_bytes) in &self.page_free_bytes {
            if free_bytes >= min_space {
                return Some(page_id);
            }
        }
        None
    }

    pub fn remove(&mut self, page_id: PageId) {
        self.page_free_bytes.remove(&page_id);
    }
}
