//! Multi-Version Concurrency Control (MVCC) snapshot and visibility engine.

use std::collections::HashSet;
use theresav_common::TxId;
use theresav_page::Tuple;

/// Transaction snapshot representing the state of active transactions at a point in time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    /// Oldest active transaction ID when snapshot was created.
    pub xmin: TxId,
    /// Highest transaction ID assigned + 1 when snapshot was created.
    pub xmax: TxId,
    /// Set of active (uncommitted) transactions at the time of snapshot creation.
    pub active_txids: HashSet<TxId>,
}

impl Snapshot {
    pub fn new(xmin: TxId, xmax: TxId, active_txids: HashSet<TxId>) -> Self {
        Self {
            xmin,
            xmax,
            active_txids,
        }
    }
}

/// Evaluates MVCC visibility of a tuple relative to a transaction snapshot.
pub struct VisibilityEngine;

impl VisibilityEngine {
    pub fn is_visible(
        tuple: &Tuple,
        snapshot: &Snapshot,
        current_tx: TxId,
        committed_txids: &HashSet<TxId>,
    ) -> bool {
        let xmin = tuple.xmin;
        let xmax = tuple.xmax;

        // 1. Check insertion visibility (xmin)
        if xmin == current_tx {
            // Created by self: visible unless deleted by self
            return xmax != current_tx;
        }

        // If xmin is uncommitted or in active set of snapshot -> invisible
        if xmin >= snapshot.xmax
            || snapshot.active_txids.contains(&xmin)
            || !committed_txids.contains(&xmin)
        {
            return false;
        }

        // 2. Check deletion visibility (xmax)
        if xmax == TxId::INVALID || xmax.0 == 0 {
            // Not deleted -> visible
            return true;
        }

        if xmax == current_tx {
            // Deleted by self -> invisible
            return false;
        }

        // If xmax is still active or not yet committed -> deletion not yet visible
        if xmax >= snapshot.xmax
            || snapshot.active_txids.contains(&xmax)
            || !committed_txids.contains(&xmax)
        {
            return true;
        }

        // xmax is committed and visible -> invisible (deleted)
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mvcc_visibility() {
        let mut committed = HashSet::new();
        committed.insert(TxId::new(1));
        committed.insert(TxId::new(2));

        let mut active = HashSet::new();
        active.insert(TxId::new(3));

        let snap = Snapshot::new(TxId::new(2), TxId::new(4), active);

        // Tuple inserted by Tx 1, never deleted
        let t1 = Tuple::new(TxId::new(1), TxId::INVALID, 0, 1, vec![], vec![]);
        assert!(VisibilityEngine::is_visible(
            &t1,
            &snap,
            TxId::new(5),
            &committed
        ));

        // Tuple inserted by Tx 3 (active uncommitted)
        let t2 = Tuple::new(TxId::new(3), TxId::INVALID, 0, 1, vec![], vec![]);
        assert!(!VisibilityEngine::is_visible(
            &t2,
            &snap,
            TxId::new(5),
            &committed
        ));

        // Tuple inserted by Tx 1, deleted by Tx 2 (committed)
        let t3 = Tuple::new(TxId::new(1), TxId::new(2), 0, 1, vec![], vec![]);
        assert!(!VisibilityEngine::is_visible(
            &t3,
            &snap,
            TxId::new(5),
            &committed
        ));
    }
}
