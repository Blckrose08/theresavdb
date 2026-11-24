use byteorder::{ByteOrder, LittleEndian};
use theresav_common::{Lsn, PageId, FORMAT_VERSION_V1, PAGE_HEADER_SIZE, PAGE_MAGIC};
use theresav_error::PageError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum PageType {
    SlottedData = 0x0001,
    BTreeInternal = 0x0002,
    BTreeLeaf = 0x0003,
    Overflow = 0x0004,
    FreeMap = 0x0005,
    Meta = 0x0006,
}

impl PageType {
    pub fn from_u16(val: u16) -> Result<Self, PageError> {
        match val {
            0x0001 => Ok(Self::SlottedData),
            0x0002 => Ok(Self::BTreeInternal),
            0x0003 => Ok(Self::BTreeLeaf),
            0x0004 => Ok(Self::Overflow),
            0x0005 => Ok(Self::FreeMap),
            0x0006 => Ok(Self::Meta),
            other => Err(PageError::InvalidPageType(other)),
        }
    }

    #[inline]
    pub const fn to_u16(self) -> u16 {
        self as u16
    }
}

#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageHeader {
    pub magic: [u8; 4],
    pub format_version: u16,
    pub page_type: PageType,
    pub page_id: PageId,
    pub lsn: Lsn,
    pub checksum: u32,
    pub flags: u16,
    pub slot_count: u16,
    pub lower_boundary: u16,
    pub upper_boundary: u16,
    pub special_area: u16,
    pub reserved: [u8; 26],
}

const _: () = assert!(std::mem::size_of::<PageHeader>() == PAGE_HEADER_SIZE);

impl PageHeader {
    pub fn new(page_id: PageId, page_type: PageType, page_size: u16) -> Self {
        Self {
            magic: PAGE_MAGIC,
            format_version: FORMAT_VERSION_V1,
            page_type,
            page_id,
            lsn: Lsn::INVALID,
            checksum: 0,
            flags: 0,
            slot_count: 0,
            lower_boundary: PAGE_HEADER_SIZE as u16,
            upper_boundary: page_size,
            special_area: 0,
            reserved: [0u8; 26],
        }
    }

    pub fn encode(&self, out: &mut [u8]) {
        assert!(out.len() >= PAGE_HEADER_SIZE);
        out[0..4].copy_from_slice(&self.magic);
        LittleEndian::write_u16(&mut out[4..6], self.format_version);
        LittleEndian::write_u16(&mut out[6..8], self.page_type.to_u16());
        LittleEndian::write_u64(&mut out[8..16], self.page_id.0);
        LittleEndian::write_u64(&mut out[16..24], self.lsn.0);
        LittleEndian::write_u32(&mut out[24..28], self.checksum);
        LittleEndian::write_u16(&mut out[28..30], self.flags);
        LittleEndian::write_u16(&mut out[30..32], self.slot_count);
        LittleEndian::write_u16(&mut out[32..34], self.lower_boundary);
        LittleEndian::write_u16(&mut out[34..36], self.upper_boundary);
        LittleEndian::write_u16(&mut out[36..38], self.special_area);
        out[38..64].copy_from_slice(&self.reserved);
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, PageError> {
        if bytes.len() < PAGE_HEADER_SIZE {
            return Err(PageError::Corrupted(
                PageId::INVALID,
                "Buffer smaller than 64-byte page header".into(),
            ));
        }

        let mut magic = [0u8; 4];
        magic.copy_from_slice(&bytes[0..4]);
        if magic != PAGE_MAGIC {
            return Err(PageError::InvalidMagic(magic));
        }

        let format_version = LittleEndian::read_u16(&bytes[4..6]);
        if format_version != FORMAT_VERSION_V1 {
            return Err(PageError::UnsupportedVersion(format_version));
        }

        let page_type_raw = LittleEndian::read_u16(&bytes[6..8]);
        let page_type = PageType::from_u16(page_type_raw)?;

        let page_id = PageId::new(LittleEndian::read_u64(&bytes[8..16]));
        let lsn = Lsn::new(LittleEndian::read_u64(&bytes[16..24]));
        let checksum = LittleEndian::read_u32(&bytes[24..28]);
        let flags = LittleEndian::read_u16(&bytes[28..30]);
        let slot_count = LittleEndian::read_u16(&bytes[30..32]);
        let lower_boundary = LittleEndian::read_u16(&bytes[32..34]);
        let upper_boundary = LittleEndian::read_u16(&bytes[34..36]);
        let special_area = LittleEndian::read_u16(&bytes[36..38]);

        let mut reserved = [0u8; 26];
        reserved.copy_from_slice(&bytes[38..64]);

        Ok(Self {
            magic,
            format_version,
            page_type,
            page_id,
            lsn,
            checksum,
            flags,
            slot_count,
            lower_boundary,
            upper_boundary,
            special_area,
            reserved,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_header_size_and_roundtrip() {
        let original = PageHeader::new(PageId::new(77), PageType::SlottedData, 16384);
        let mut buf = [0u8; 64];
        original.encode(&mut buf);

        let decoded = PageHeader::decode(&buf).expect("Decode should succeed");
        assert_eq!(original, decoded);
        assert_eq!(buf.len(), 64);
    }
}
