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
    pub fn is_compatible(&self, other: &LockMode) -> bool {
        matches!(
            (self, other),
            (LockMode::Shared, LockMode::Shared)
                | (LockMode::Shared, LockMode::IntentShared)
                | (LockMode::IntentShared, LockMode::Shared)
                | (LockMode::IntentShared, LockMode::IntentShared)
                | (LockMode::IntentShared, LockMode::IntentExclusive)
                | (LockMode::IntentExclusive, LockMode::IntentShared)
                | (LockMode::IntentExclusive, LockMode::IntentExclusive)
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceId {
    Table(Oid),
    Row(TupleId),
}

pub struct LockManager {
    locks: RwLock<HashMap<ResourceId, Vec<(TxId, LockMode)>>>,
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

    pub fn acquire(
        &self,
        tx_id: TxId,
        resource: ResourceId,
        mode: LockMode,
    ) -> Result<(), TransactionError> {
        let mut locks = self.locks.write().unwrap();
        let current_locks = locks.entry(resource).or_default();

        if let Some((_, current_mode)) = current_locks.iter().find(|(t, _)| *t == tx_id) {
            if *current_mode == mode
                || (*current_mode == LockMode::Exclusive && mode == LockMode::Shared)
            {
                return Ok(());
            }
        }

        let has_conflict = current_locks
            .iter()
            .any(|(t, m)| *t != tx_id && !m.is_compatible(&mode));

        if has_conflict {
            let mut wfg = self.wait_for.lock().unwrap();
            let waiting_set = wfg.entry(tx_id).or_default();

            for (holder, _) in current_locks.iter() {
                if *holder != tx_id {
                    waiting_set.insert(*holder);
                }
            }

            if self.detect_deadlock_cycle(&wfg, tx_id) {
                return Err(TransactionError::Deadlock);
            }

            return Err(TransactionError::WriteConflict(tx_id));
        }

        current_locks.push((tx_id, mode));
        Ok(())
    }

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
                        return true;
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

        lm.acquire(t1, res1, LockMode::Exclusive).unwrap();

        assert!(lm.acquire(t2, res1, LockMode::Exclusive).is_err());

        lm.acquire(t2, res2, LockMode::Exclusive).unwrap();

        lm.release_all(t1);

        assert!(lm.acquire(t2, res1, LockMode::Exclusive).is_ok());
    }
}
