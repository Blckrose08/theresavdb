//! Concurrency Lock Manager with hierarchical locking and deadlock detection.

use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, RwLock};
use theresav_common::{Oid, TupleId, TxId};
use theresav_error::TransactionError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LockMode {
    Shared,
    Exclusive,
    IntentShared,
    IntentExclusive,
    SharedIntentExclusive,
}

impl LockMode {
    /// Lock compatibility matrix.
    pub fn is_compatible(&self, other: &LockMode) -> bool {
        match (self, other) {
            (LockMode::Shared, LockMode::Shared) => true,
            (LockMode::Shared, LockMode::IntentShared) => true,
            (LockMode::IntentShared, LockMode::Shared) => true,
            (LockMode::IntentShared, LockMode::IntentShared) => true,
            (LockMode::IntentShared, LockMode::IntentExclusive) => true,
            (LockMode::IntentExclusive, LockMode::IntentShared) => true,
            (LockMode::IntentExclusive, LockMode::IntentExclusive) => true,
            _ => false,
        }
    }
}

/// Resource being locked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceId {
    Table(Oid),
    Row(TupleId),
}

pub struct LockManager {
    // Resource -> List of (TxId, LockMode)
    locks: RwLock<HashMap<ResourceId, Vec<(TxId, LockMode)>>>,
    // Wait-For Graph: TxId waiting for TxId
    wait_for: Mutex<HashMap<TxId, HashSet<TxId>>>,
}

impl Default for LockManager {
    fn default() -> Self {
        Self::new()
    }
}

impl LockManager {
    pub fn new() -> Self {
        Self {
            locks: RwLock::new(HashMap::new()),
            wait_for: Mutex::new(HashMap::new()),
        }
    }

    /// Acquire a lock on a table or row.
    pub fn acquire(&self, tx_id: TxId, resource: ResourceId, mode: LockMode) -> Result<(), TransactionError> {
        let mut locks = self.locks.write().unwrap();
        let current_locks = locks.entry(resource).or_default();

        // Check if transaction already holds this lock or compatible
        if let Some((_, current_mode)) = current_locks.iter().find(|(t, _)| *t == tx_id) {
            if *current_mode == mode || (*current_mode == LockMode::Exclusive && mode == LockMode::Shared) {
                return Ok(());
            }
        }

        // Check compatibility with existing locks
        let has_conflict = current_locks
            .iter()
            .any(|(t, m)| *t != tx_id && !m.is_compatible(&mode));

        if has_conflict {
            // Record dependencies in Wait-For Graph and check for deadlocks
            let mut wfg = self.wait_for.lock().unwrap();
            let waiting_set = wfg.entry(tx_id).or_default();

            for (holder, _) in current_locks.iter() {
                if *holder != tx_id {
                    waiting_set.insert(*holder);
                }
            }

            // Check deadlock (cycle detection via DFS)
            if self.detect_deadlock_cycle(&wfg, tx_id) {
                return Err(TransactionError::Deadlock);
            }

            return Err(TransactionError::WriteConflict(tx_id));
        }

        current_locks.push((tx_id, mode));
        Ok(())
    }

    /// Release all locks held by a transaction upon commit or abort.
    pub fn release_all(&self, tx_id: TxId) {
        let mut locks = self.locks.write().unwrap();
        for list in locks.values_mut() {
            list.retain(|(t, _)| *t != tx_id);
        }

        let mut wfg = self.wait_for.lock().unwrap();
        wfg.remove(&tx_id);
        for waiting_set in wfg.values_mut() {
            waiting_set.remove(&tx_id);
        }
    }

    fn detect_deadlock_cycle(&self, wfg: &HashMap<TxId, HashSet<TxId>>, start: TxId) -> bool {
        let mut visited = HashSet::new();
        let mut stack = HashSet::new();

        fn dfs(
            node: TxId,
            wfg: &HashMap<TxId, HashSet<TxId>>,
            visited: &mut HashSet<TxId>,
            stack: &mut HashSet<TxId>,
        ) -> bool {
            visited.insert(node);
            stack.insert(node);

            if let Some(neighbors) = wfg.get(&node) {
                for &neighbor in neighbors {
                    if !visited.contains(&neighbor) {
                        if dfs(neighbor, wfg, visited, stack) {
                            return true;
                        }
                    } else if stack.contains(&neighbor) {
                        return true; // Cycle found
                    }
                }
            }

            stack.remove(&node);
            false
        }

        dfs(start, wfg, &mut visited, &mut stack)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_compatibility_and_deadlock() {
        let lm = LockManager::new();
        let t1 = TxId::new(1);
        let t2 = TxId::new(2);
        let res1 = ResourceId::Table(Oid::new(10));
        let res2 = ResourceId::Table(Oid::new(20));

        // T1 takes Exclusive on Res1
        lm.acquire(t1, res1, LockMode::Exclusive).unwrap();

        // T2 taking Exclusive on Res1 must conflict
        assert!(lm.acquire(t2, res1, LockMode::Exclusive).is_err());

        // T2 takes Exclusive on Res2
        lm.acquire(t2, res2, LockMode::Exclusive).unwrap();

        // Release T1
        lm.release_all(t1);

        // Now T2 can take Res1
        assert!(lm.acquire(t2, res1, LockMode::Exclusive).is_ok());
    }
}
