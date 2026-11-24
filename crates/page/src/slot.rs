use byteorder::{ByteOrder, LittleEndian};
use theresav_common::SLOT_SIZE;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PageSlot {
    pub offset: u16,
    pub length: u16,
}

const _: () = assert!(std::mem::size_of::<PageSlot>() == SLOT_SIZE);

impl PageSlot {
    pub const DEAD: Self = Self {
        offset: 0,
        length: 0,
    };

    #[inline]
    pub const fn new(offset: u16, length: u16) -> Self {
        Self { offset, length }
    }

    #[inline]
    pub const fn is_dead(&self) -> bool {
        self.length == 0
    }

    #[inline]
    pub const fn is_live(&self) -> bool {
        self.length > 0
    }

    #[inline]
    pub fn read_from(slice: &[u8]) -> Self {
        assert!(slice.len() >= SLOT_SIZE);
        let offset = LittleEndian::read_u16(&slice[0..2]);
        let length = LittleEndian::read_u16(&slice[2..4]);
        Self { offset, length }
    }

    #[inline]
    pub fn write_to(&self, slice: &mut [u8]) {
        assert!(slice.len() >= SLOT_SIZE);
        LittleEndian::write_u16(&mut slice[0..2], self.offset);
        LittleEndian::write_u16(&mut slice[2..4], self.length);
    }
}
