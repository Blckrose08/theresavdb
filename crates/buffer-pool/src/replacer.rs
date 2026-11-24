//! Clock replacement policy for buffer pool page eviction.

use std::collections::HashSet;

/// Clock-based eviction replacer.
#[derive(Debug)]
pub struct ClockReplacer {
    clock_hand: usize,
    capacity: usize,
    ref_bits: Vec<bool>,
    evictable: Vec<bool>,
    evictable_set: HashSet<usize>,
}

impl ClockReplacer {
    pub fn new(capacity: usize) -> Self {
        Self {
            clock_hand: 0,
            capacity,
            ref_bits: vec![false; capacity],
            evictable: vec![false; capacity],
            evictable_set: HashSet::with_capacity(capacity),
        }
    }

    /// Mark a frame as evictable or not (pinned frames are not evictable).
    pub fn set_evictable(&mut self, frame_id: usize, is_evictable: bool) {
        if frame_id >= self.capacity {
            return;
        }

        if is_evictable {
            if !self.evictable[frame_id] {
                self.evictable[frame_id] = true;
                self.evictable_set.insert(frame_id);
            }
        } else if self.evictable[frame_id] {
            self.evictable[frame_id] = false;
            self.evictable_set.remove(&frame_id);
        }
    }

    /// Record an access to a frame (sets the reference bit).
    pub fn record_access(&mut self, frame_id: usize) {
        if frame_id < self.capacity {
            self.ref_bits[frame_id] = true;
        }
    }

    /// Find a victim frame to evict.
    pub fn victim(&mut self) -> Option<usize> {
        if self.evictable_set.is_empty() {
            return None;
        }

        let total = self.capacity;
        let mut scanned = 0;

        while scanned < total * 2 {
            let fid = self.clock_hand;
            self.clock_hand = (self.clock_hand + 1) % self.capacity;

            if self.evictable[fid] {
                if self.ref_bits[fid] {
                    self.ref_bits[fid] = false;
                } else {
                    self.evictable[fid] = false;
                    self.evictable_set.remove(&fid);
                    return Some(fid);
                }
            }
            scanned += 1;
        }

        // Fallback: take any evictable frame
        if let Some(&fid) = self.evictable_set.iter().next() {
            self.evictable[fid] = false;
            self.evictable_set.remove(&fid);
            Some(fid)
        } else {
            None
        }
    }

    /// Number of frames currently evictable.
    pub fn size(&self) -> usize {
        self.evictable_set.len()
    }
}
