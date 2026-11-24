//! Strongly typed 64-bit and 16-bit database identifiers.

use serde::{Deserialize, Serialize};
use std::fmt;

/// 64-bit physical or logical page identifier.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
pub struct PageId(pub u64);

impl PageId {
    pub const INVALID: Self = Self(u64::MAX);

    #[inline]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    #[inline]
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    #[inline]
    pub fn is_valid(self) -> bool {
        self != Self::INVALID
    }

    #[inline]
    pub fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

impl fmt::Display for PageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PageId({})", self.0)
    }
}

/// 64-bit transaction identifier (TxId).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
pub struct TxId(pub u64);

impl TxId {
    pub const INVALID: Self = Self(0);
    pub const BOOTSTRAP: Self = Self(1);
    pub const FIRST_NORMAL: Self = Self(2);

    #[inline]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    #[inline]
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    #[inline]
    pub fn is_valid(self) -> bool {
        self.0 > 0
    }

    #[inline]
    pub fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

impl fmt::Display for TxId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TxId({})", self.0)
    }
}

/// 64-bit Log Sequence Number (LSN) for Write-Ahead Logging (WAL).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
pub struct Lsn(pub u64);

impl Lsn {
    pub const INVALID: Self = Self(0);

    #[inline]
    pub const fn new(lsn: u64) -> Self {
        Self(lsn)
    }

    #[inline]
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    #[inline]
    pub fn is_valid(self) -> bool {
        self.0 > 0
    }
}

impl fmt::Display for Lsn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Lsn(0x{:016X})", self.0)
    }
}

/// 64-bit Object Identifier (OID) for catalog entities (tables, schemas, columns, indexes).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
pub struct Oid(pub u64);

impl Oid {
    pub const INVALID: Self = Self(0);

    #[inline]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    #[inline]
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    #[inline]
    pub fn is_valid(self) -> bool {
        self.0 > 0
    }
}

impl fmt::Display for Oid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Oid({})", self.0)
    }
}

/// 16-bit slot directory index within a page.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
pub struct SlotId(pub u16);

impl SlotId {
    pub const INVALID: Self = Self(u16::MAX);

    #[inline]
    pub const fn new(slot: u16) -> Self {
        Self(slot)
    }

    #[inline]
    pub const fn as_u16(self) -> u16 {
        self.0
    }

    #[inline]
    pub const fn as_usize(self) -> usize {
        self.0 as usize
    }

    #[inline]
    pub fn is_valid(self) -> bool {
        self != Self::INVALID
    }
}

impl fmt::Display for SlotId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SlotId({})", self.0)
    }
}

/// 10-byte stable Tuple Identifier / Item Pointer (`Tid`).
/// Composed of 8-byte PageId and 2-byte SlotId.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
pub struct TupleId {
    pub page_id: PageId,
    pub slot_id: SlotId,
}

impl TupleId {
    pub const INVALID: Self = Self {
        page_id: PageId::INVALID,
        slot_id: SlotId::INVALID,
    };

    #[inline]
    pub const fn new(page_id: PageId, slot_id: SlotId) -> Self {
        Self { page_id, slot_id }
    }

    #[inline]
    pub fn is_valid(self) -> bool {
        self.page_id.is_valid() && self.slot_id.is_valid()
    }
}

impl fmt::Display for TupleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Tid({}, {})", self.page_id.0, self.slot_id.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_types() {
        let p = PageId::new(42);
        assert_eq!(p.as_u64(), 42);
        assert_eq!(p.next(), PageId::new(43));
        assert!(p.is_valid());

        let t = TxId::new(100);
        assert_eq!(t.as_u64(), 100);
        assert!(t.is_valid());

        let l = Lsn::new(0x1A2B3C);
        assert_eq!(l.as_u64(), 0x1A2B3C);
        assert!(l.is_valid());

        let tid = TupleId::new(p, SlotId::new(3));
        assert!(tid.is_valid());
        assert_eq!(tid.to_string(), "Tid(42, 3)");
    }
}
