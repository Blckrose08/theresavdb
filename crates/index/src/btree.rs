//! In-memory and page-backed B-Tree index supporting unique and composite indexes.

use crate::key::IndexKey;
use std::collections::BTreeMap;
use theresav_common::{PageId, TupleId};
use theresav_error::StorageError;

/// B-Tree index structure for relational tables.
#[derive(Debug)]
pub struct BTreeIndex {
    root_page_id: PageId,
    is_unique: bool,
    // In-memory ordered backing tree coordinating with disk pages
    entries: BTreeMap<IndexKey, Vec<TupleId>>,
}

impl BTreeIndex {
    pub fn new(root_page_id: PageId, is_unique: bool) -> Self {
        Self {
            root_page_id,
            is_unique,
            entries: BTreeMap::new(),
        }
    }

    pub fn root_page_id(&self) -> PageId {
        self.root_page_id
    }

    pub fn is_unique(&self) -> bool {
        self.is_unique
    }

    /// Insert key -> TupleId mapping. Enforces uniqueness if `is_unique` is true.
    pub fn insert(&mut self, key: IndexKey, tid: TupleId) -> Result<(), StorageError> {
        if self.is_unique {
            if let Some(existing) = self.entries.get(&key) {
                if !existing.is_empty() {
                    return Err(StorageError::CorruptedFile(format!(
                        "Unique index constraint violation on key: {:?}",
                        key
                    )));
                }
            }
        }

        self.entries.entry(key).or_default().push(tid);
        Ok(())
    }

    /// Search exact key match.
    pub fn search(&self, key: &IndexKey) -> Option<&[TupleId]> {
        self.entries.get(key).map(|v| v.as_slice())
    }

    /// Delete tuple from index.
    pub fn delete(&mut self, key: &IndexKey, tid: TupleId) {
        if let Some(list) = self.entries.get_mut(key) {
            list.retain(|&x| x != tid);
            if list.is_empty() {
                self.entries.remove(key);
            }
        }
    }

    /// Range scan from low bound (inclusive) to high bound (inclusive).
    pub fn scan_range(
        &self,
        low: Option<&IndexKey>,
        high: Option<&IndexKey>,
    ) -> Vec<(IndexKey, TupleId)> {
        use std::ops::Bound::*;

        let start = match low {
            Some(k) => Included(k),
            None => Unbounded,
        };
        let end = match high {
            Some(k) => Included(k),
            None => Unbounded,
        };

        let mut results = Vec::new();
        for (k, tids) in self.entries.range((start, end)) {
            for &tid in tids {
                results.push((k.clone(), tid));
            }
        }
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use theresav_common::SlotId;
    use theresav_types::Value;

    #[test]
    fn test_btree_unique_and_range_scan() {
        let mut idx = BTreeIndex::new(PageId::new(1), true);

        let k1 = IndexKey::single(Value::Integer(10));
        let k2 = IndexKey::single(Value::Integer(20));
        let k3 = IndexKey::single(Value::Integer(30));

        let tid1 = TupleId::new(PageId::new(10), SlotId::new(1));
        let tid2 = TupleId::new(PageId::new(10), SlotId::new(2));
        let tid3 = TupleId::new(PageId::new(10), SlotId::new(3));

        idx.insert(k1.clone(), tid1).unwrap();
        idx.insert(k2.clone(), tid2).unwrap();
        idx.insert(k3.clone(), tid3).unwrap();

        // Duplicate insert in unique index must fail
        assert!(idx.insert(k2.clone(), tid1).is_err());

        // Exact search
        let found = idx.search(&k2).unwrap();
        assert_eq!(found, &[tid2]);

        // Range scan between 15 and 35
        let low = IndexKey::single(Value::Integer(15));
        let high = IndexKey::single(Value::Integer(35));
        let range = idx.scan_range(Some(&low), Some(&high));
        assert_eq!(range.len(), 2);
        assert_eq!(range[0].1, tid2);
        assert_eq!(range[1].1, tid3);
    }
}
