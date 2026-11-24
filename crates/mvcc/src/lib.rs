use std::collections::HashSet;
use theresav_common::TxId;
use theresav_page::Tuple;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub xmin: TxId,
    pub xmax: TxId,
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

        if xmin == current_tx {
            return xmax != current_tx;
        }

        if xmin >= snapshot.xmax
            || snapshot.active_txids.contains(&xmin)
            || !committed_txids.contains(&xmin)
        {
            return false;
        }

        if xmax == TxId::INVALID || xmax.0 == 0 {
            return true;
        }

        if xmax == current_tx {
            return false;
        }

        if xmax >= snapshot.xmax
            || snapshot.active_txids.contains(&xmax)
            || !committed_txids.contains(&xmax)
        {
            return true;
        }

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

        let t1 = Tuple::new(TxId::new(1), TxId::INVALID, 0, 1, vec![], vec![]);
        assert!(VisibilityEngine::is_visible(
            &t1,
            &snap,
            TxId::new(5),
            &committed
        ));

        let t2 = Tuple::new(TxId::new(3), TxId::INVALID, 0, 1, vec![], vec![]);
        assert!(!VisibilityEngine::is_visible(
            &t2,
            &snap,
            TxId::new(5),
            &committed
        ));

        let t3 = Tuple::new(TxId::new(1), TxId::new(2), 0, 1, vec![], vec![]);
        assert!(!VisibilityEngine::is_visible(
            &t3,
            &snap,
            TxId::new(5),
            &committed
        ));
    }
}
