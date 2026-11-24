use crate::header::{PageHeader, PageType};
use crate::slot::PageSlot;
use byteorder::{ByteOrder, LittleEndian};
use std::fmt;
use theresav_common::{
    PageId, SlotId, FORMAT_VERSION_V1, PAGE_HEADER_SIZE, PAGE_MAGIC, PAGE_SIZE, SLOT_SIZE,
};
use theresav_error::PageError;

pub struct Page {
    data: Box<[u8; PAGE_SIZE]>,
}

impl Page {
    pub fn new(page_id: PageId, page_type: PageType) -> Self {
        let mut data = Box::new([0u8; PAGE_SIZE]);
        let header = PageHeader::new(page_id, page_type, PAGE_SIZE as u16);
        header.encode(&mut data[0..PAGE_HEADER_SIZE]);

        let mut page = Self { data };
        page.compute_checksum();
        page
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, PageError> {
        if bytes.len() != PAGE_SIZE {
            return Err(PageError::Corrupted(
                PageId::INVALID,
                format!("Expected page size {}, got {}", PAGE_SIZE, bytes.len()),
            ));
        }

        let mut data = Box::new([0u8; PAGE_SIZE]);
        data.copy_from_slice(bytes);

        if data[0..4] != PAGE_MAGIC {
            let mut magic = [0u8; 4];
            magic.copy_from_slice(&data[0..4]);
            return Err(PageError::InvalidMagic(magic));
        }

        let version = LittleEndian::read_u16(&data[4..6]);
        if version != FORMAT_VERSION_V1 {
            return Err(PageError::UnsupportedVersion(version));
        }

        let stored_checksum = LittleEndian::read_u32(&data[24..28]);
        let page_id = PageId::new(LittleEndian::read_u64(&data[8..16]));

        let calculated_checksum = compute_page_checksum(&data);
        if stored_checksum != calculated_checksum {
            return Err(PageError::ChecksumMismatch {
                expected: stored_checksum,
                calculated: calculated_checksum,
            });
        }

        let header = PageHeader::decode(&data[0..PAGE_HEADER_SIZE])?;
        let lower = header.lower_boundary;
        let upper = header.upper_boundary;
        let max_upper = (PAGE_SIZE - header.special_area as usize) as u16;

        if lower < PAGE_HEADER_SIZE as u16 || lower > upper || upper > max_upper {
            return Err(PageError::InvalidBoundary {
                lower,
                upper,
                page_size: PAGE_SIZE,
            });
        }

        let expected_lower = (PAGE_HEADER_SIZE + header.slot_count as usize * SLOT_SIZE) as u16;
        if lower != expected_lower {
            return Err(PageError::Corrupted(
                page_id,
                format!(
                    "Inconsistent slot directory lower boundary: expected {}, found {}",
                    expected_lower, lower
                ),
            ));
        }

        for i in 0..header.slot_count {
            let slot_offset = PAGE_HEADER_SIZE + (i as usize) * SLOT_SIZE;
            let slot = PageSlot::read_from(&data[slot_offset..slot_offset + SLOT_SIZE]);
            if slot.is_live() && (slot.offset < upper || slot.offset + slot.length > max_upper) {
                return Err(PageError::Corrupted(
                    page_id,
                    format!(
                        "Slot {} points out of bounds: offset={}, length={}, upper={}, max={}",
                        i, slot.offset, slot.length, upper, max_upper
                    ),
                ));
            }
        }

        Ok(Self { data })
    }

    #[inline]
    pub fn as_bytes(&self) -> &[u8; PAGE_SIZE] {
        &self.data
    }

    #[inline]
    pub fn as_bytes_mut(&mut self) -> &mut [u8; PAGE_SIZE] {
        &mut self.data
    }

    pub fn header(&self) -> PageHeader {
        PageHeader::decode(&self.data[0..PAGE_HEADER_SIZE]).expect("Page bytes already validated")
    }

    pub fn set_header(&mut self, header: &PageHeader) {
        header.encode(&mut self.data[0..PAGE_HEADER_SIZE]);
    }

    pub fn compute_checksum(&mut self) {
        let chk = compute_page_checksum(&self.data);
        LittleEndian::write_u32(&mut self.data[24..28], chk);
    }

    pub fn contiguous_free_space(&self) -> usize {
        let lower = LittleEndian::read_u16(&self.data[32..34]) as usize;
        let upper = LittleEndian::read_u16(&self.data[34..36]) as usize;
        upper.saturating_sub(lower)
    }

    pub fn insert_tuple(&mut self, tuple_data: &[u8]) -> Result<SlotId, PageError> {
        let tuple_len = tuple_data.len();
        if tuple_len == 0 {
            return Err(PageError::TupleCorrupted(
                "Cannot insert 0-byte tuple".into(),
            ));
        }

        let slot_count = LittleEndian::read_u16(&self.data[30..32]);
        let mut lower = LittleEndian::read_u16(&self.data[32..34]) as usize;
        let mut upper = LittleEndian::read_u16(&self.data[34..36]) as usize;

        let mut reused_slot_idx: Option<u16> = None;
        for i in 0..slot_count {
            let slot_offset = PAGE_HEADER_SIZE + (i as usize) * SLOT_SIZE;
            let slot = PageSlot::read_from(&self.data[slot_offset..slot_offset + SLOT_SIZE]);
            if slot.is_dead() {
                reused_slot_idx = Some(i);
                break;
            }
        }

        let needed_space = if reused_slot_idx.is_some() {
            tuple_len
        } else {
            tuple_len + SLOT_SIZE
        };

        let free_space = upper.saturating_sub(lower);

        if free_space < needed_space {
            return Err(PageError::PageFull {
                required: needed_space,
                available: free_space,
            });
        }

        upper -= tuple_len;
        self.data[upper..upper + tuple_len].copy_from_slice(tuple_data);

        let target_slot_id = if let Some(idx) = reused_slot_idx {
            let slot_offset = PAGE_HEADER_SIZE + (idx as usize) * SLOT_SIZE;
            let slot = PageSlot::new(upper as u16, tuple_len as u16);
            slot.write_to(&mut self.data[slot_offset..slot_offset + SLOT_SIZE]);
            SlotId::new(idx)
        } else {
            let slot_offset = lower;
            let slot = PageSlot::new(upper as u16, tuple_len as u16);
            slot.write_to(&mut self.data[slot_offset..slot_offset + SLOT_SIZE]);
            lower += SLOT_SIZE;

            LittleEndian::write_u16(&mut self.data[30..32], slot_count + 1);
            LittleEndian::write_u16(&mut self.data[32..34], lower as u16);
            SlotId::new(slot_count)
        };

        LittleEndian::write_u16(&mut self.data[34..36], upper as u16);
        self.compute_checksum();

        Ok(target_slot_id)
    }

    pub fn get_tuple(&self, slot_id: SlotId) -> Result<&[u8], PageError> {
        let slot_count = LittleEndian::read_u16(&self.data[30..32]);
        if slot_id.0 >= slot_count {
            return Err(PageError::SlotOutOfBounds {
                slot: slot_id.0,
                count: slot_count,
            });
        }

        let slot_offset = PAGE_HEADER_SIZE + (slot_id.0 as usize) * SLOT_SIZE;
        let slot = PageSlot::read_from(&self.data[slot_offset..slot_offset + SLOT_SIZE]);

        if slot.is_dead() {
            return Err(PageError::InvalidSlot {
                slot_id,
                reason: "Tuple has been deleted",
            });
        }

        let start = slot.offset as usize;
        let end = start + slot.length as usize;
        if end > PAGE_SIZE {
            return Err(PageError::Corrupted(
                self.header().page_id,
                format!("Slot {} exceeds page size", slot_id.0),
            ));
        }

        Ok(&self.data[start..end])
    }

    pub fn delete_tuple(&mut self, slot_id: SlotId) -> Result<(), PageError> {
        let slot_count = LittleEndian::read_u16(&self.data[30..32]);
        if slot_id.0 >= slot_count {
            return Err(PageError::SlotOutOfBounds {
                slot: slot_id.0,
                count: slot_count,
            });
        }

        let slot_offset = PAGE_HEADER_SIZE + (slot_id.0 as usize) * SLOT_SIZE;
        let dead_slot = PageSlot::DEAD;
        dead_slot.write_to(&mut self.data[slot_offset..slot_offset + SLOT_SIZE]);

        self.compute_checksum();
        Ok(())
    }

    pub fn compact(&mut self) -> Result<(), PageError> {
        let slot_count = LittleEndian::read_u16(&self.data[30..32]) as usize;
        let special_area = LittleEndian::read_u16(&self.data[36..38]) as usize;
        let max_upper = PAGE_SIZE - special_area;

        let mut live_items = Vec::new();
        for i in 0..slot_count {
            let slot_offset = PAGE_HEADER_SIZE + i * SLOT_SIZE;
            let slot = PageSlot::read_from(&self.data[slot_offset..slot_offset + SLOT_SIZE]);
            if slot.is_live() {
                let start = slot.offset as usize;
                let end = start + slot.length as usize;
                let tuple_bytes = self.data[start..end].to_vec();
                live_items.push((i, tuple_bytes));
            }
        }

        let mut new_upper = max_upper;

        for (slot_idx, tuple_bytes) in live_items {
            let len = tuple_bytes.len();
            new_upper -= len;
            self.data[new_upper..new_upper + len].copy_from_slice(&tuple_bytes);

            let slot_offset = PAGE_HEADER_SIZE + slot_idx * SLOT_SIZE;
            let new_slot = PageSlot::new(new_upper as u16, len as u16);
            new_slot.write_to(&mut self.data[slot_offset..slot_offset + SLOT_SIZE]);
        }

        LittleEndian::write_u16(&mut self.data[34..36], new_upper as u16);
        self.compute_checksum();
        Ok(())
    }
}

fn compute_page_checksum(data: &[u8; PAGE_SIZE]) -> u32 {
    let mut part1 = crc32c::crc32c(&data[0..24]);
    let zeros = [0u8; 4];
    part1 = crc32c::crc32c_append(part1, &zeros);
    crc32c::crc32c_append(part1, &data[28..PAGE_SIZE])
}

impl Clone for Page {
    fn clone(&self) -> Self {
        let mut copy = Box::new([0u8; PAGE_SIZE]);
        copy.copy_from_slice(&self.data[..]);
        Self { data: copy }
    }
}

impl fmt::Debug for Page {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Page(id={}, type={:?}, free={})",
            self.header().page_id,
            self.header().page_type,
            self.contiguous_free_space()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_page_lifecycle_and_checksum() {
        let mut page = Page::new(PageId::new(1), PageType::SlottedData);
        assert_eq!(page.header().page_id, PageId::new(1));
        assert_eq!(page.contiguous_free_space(), PAGE_SIZE - PAGE_HEADER_SIZE);

        let t1 = b"Row 1 content in TheresavDB";
        let t2 = b"Row 2 another content";

        let slot1 = page.insert_tuple(t1).expect("Insert t1 should succeed");
        assert_eq!(slot1.0, 0);

        let slot2 = page.insert_tuple(t2).expect("Insert t2 should succeed");
        assert_eq!(slot2.0, 1);

        assert_eq!(page.get_tuple(slot1).unwrap(), t1);
        assert_eq!(page.get_tuple(slot2).unwrap(), t2);

        let serialized = *page.as_bytes();
        let loaded_page =
            Page::from_bytes(&serialized).expect("Deserialization and validation must succeed");
        assert_eq!(loaded_page.get_tuple(slot1).unwrap(), t1);
        assert_eq!(loaded_page.get_tuple(slot2).unwrap(), t2);

        let mut corrupted = serialized;
        corrupted[100] ^= 0xFF;
        assert!(matches!(
            Page::from_bytes(&corrupted),
            Err(PageError::ChecksumMismatch { .. })
        ));
    }

    #[test]
    fn test_page_compaction() {
        let mut page = Page::new(PageId::new(2), PageType::SlottedData);
        let s0 = page.insert_tuple(b"data-0").unwrap();
        let s1 = page.insert_tuple(b"data-1-to-be-deleted").unwrap();
        let s2 = page.insert_tuple(b"data-2").unwrap();

        page.delete_tuple(s1).unwrap();
        assert!(page.get_tuple(s1).is_err());

        let space_before = page.contiguous_free_space();
        page.compact().unwrap();
        let space_after = page.contiguous_free_space();

        assert!(space_after > space_before);
        assert_eq!(page.get_tuple(s0).unwrap(), b"data-0");
        assert_eq!(page.get_tuple(s2).unwrap(), b"data-2");
    }

    #[test]
    fn test_page_boundary_validation() {
        let mut page = Page::new(PageId::new(10), PageType::SlottedData);
        let oversized = vec![0xAA; PAGE_SIZE];
        assert!(matches!(
            page.insert_tuple(&oversized),
            Err(PageError::PageFull { .. })
        ));

        assert!(matches!(
            page.get_tuple(SlotId::new(99)),
            Err(PageError::SlotOutOfBounds { .. })
        ));
    }

    #[test]
    fn test_tuple_tombstone_reuse() {
        let mut page = Page::new(PageId::new(5), PageType::SlottedData);
        let s0 = page.insert_tuple(b"first").unwrap();
        let s1 = page.insert_tuple(b"second").unwrap();
        assert_eq!(s0.0, 0);
        assert_eq!(s1.0, 1);

        page.delete_tuple(s0).unwrap();
        assert!(page.get_tuple(s0).is_err());

        let s_reused = page.insert_tuple(b"reused").unwrap();
        assert_eq!(s_reused.0, 0);
        assert_eq!(page.get_tuple(s_reused).unwrap(), b"reused");
    }

    proptest! {
        #[test]
        fn prop_from_bytes_never_panics(raw in proptest::collection::vec(any::<u8>(), PAGE_SIZE..=PAGE_SIZE)) {
            let res = Page::from_bytes(&raw);
            prop_assert!(res.is_ok() || res.is_err());
        }

        #[test]
        fn prop_page_insertion_roundtrip(data in proptest::collection::vec(proptest::collection::vec(1u8..=255, 1..64), 1..20)) {
            let mut page = Page::new(PageId::new(1), PageType::SlottedData);
            let mut slots = Vec::new();

            for item in &data {
                if let Ok(slot) = page.insert_tuple(item) {
                    slots.push((slot, item));
                }
            }

            let bytes = *page.as_bytes();
            let reloaded = Page::from_bytes(&bytes).expect("Valid page must deserialize cleanly");

            for (slot, expected) in slots {
                let actual = reloaded.get_tuple(slot).expect("Slot must be accessible");
                prop_assert_eq!(actual, expected.as_slice());
            }
        }
    }
}
