//! Transaction lifecycle manager coordinating ACID transactions, WAL, and locks.

use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use theresav_common::{Lsn, TxId};
use theresav_error::TransactionError;
use theresav_lock_manager::LockManager;
use theresav_mvcc::Snapshot;
use theresav_wal::{WalRecordType, WalWriter};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsolationLevel {
    ReadUncommitted,
    ReadCommitted,
    RepeatableRead,
    Serializable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionState {
    Active,
    Committed,
    Aborted,
}

pub struct Transaction {
    pub tx_id: TxId,
    pub state: TransactionState,
    pub isolation: IsolationLevel,
    pub snapshot: Snapshot,
    pub last_lsn: Lsn,
    pub savepoints: Vec<String>,
}

pub struct TransactionManager {
    next_tx_id: AtomicU64,
    active_txids: RwLock<HashSet<TxId>>,
    committed_txids: RwLock<HashSet<TxId>>,
    wal_writer: Arc<WalWriter>,
    lock_manager: Arc<LockManager>,
}

impl TransactionManager {
    pub fn new(wal_writer: Arc<WalWriter>, lock_manager: Arc<LockManager>) -> Self {
        Self {
            next_tx_id: AtomicU64::new(10),
            active_txids: RwLock::new(HashSet::new()),
            committed_txids: RwLock::new(HashSet::new()),
            wal_writer,
            lock_manager,
        }
    }

    /// Begin a new transaction and log BEGIN record to WAL.
    pub fn begin(&self, isolation: IsolationLevel) -> Result<Transaction, TransactionError> {
        let tx_id = TxId::new(self.next_tx_id.fetch_add(1, Ordering::SeqCst));

        let lsn = self
            .wal_writer
            .append(Lsn::INVALID, tx_id, WalRecordType::Begin, vec![])
            .map_err(|_| TransactionError::Aborted(tx_id))?;

        let mut active = self.active_txids.write().unwrap();
        let oldest = active.iter().cloned().min().unwrap_or(tx_id);
        let snapshot = Snapshot::new(oldest, tx_id.next(), active.clone());
        active.insert(tx_id);

        Ok(Transaction {
            tx_id,
            state: TransactionState::Active,
            isolation,
            snapshot,
            last_lsn: lsn,
            savepoints: Vec::new(),
        })
    }

    /// Commit transaction, flush WAL durable record, and release all locks.
    pub fn commit(&self, tx: &mut Transaction) -> Result<(), TransactionError> {
        if tx.state != TransactionState::Active {
            return Err(TransactionError::AlreadyFinished(tx.tx_id));
        }

        let lsn = self
            .wal_writer
            .append(tx.last_lsn, tx.tx_id, WalRecordType::Commit, vec![])
            .map_err(|_| TransactionError::Aborted(tx.tx_id))?;

        tx.last_lsn = lsn;
        tx.state = TransactionState::Committed;

        {
            let mut active = self.active_txids.write().unwrap();
            active.remove(&tx.tx_id);
        }
        {
            let mut committed = self.committed_txids.write().unwrap();
            committed.insert(tx.tx_id);
        }

        self.lock_manager.release_all(tx.tx_id);
        Ok(())
    }

    /// Rollback transaction, log ABORT record, and release all locks.
    pub fn abort(&self, tx: &mut Transaction) -> Result<(), TransactionError> {
        if tx.state != TransactionState::Active {
            return Err(TransactionError::AlreadyFinished(tx.tx_id));
        }

        let lsn = self
            .wal_writer
            .append(tx.last_lsn, tx.tx_id, WalRecordType::Abort, vec![])
            .map_err(|_| TransactionError::Aborted(tx.tx_id))?;

        tx.last_lsn = lsn;
        tx.state = TransactionState::Aborted;

        {
            let mut active = self.active_txids.write().unwrap();
            active.remove(&tx.tx_id);
        }

        self.lock_manager.release_all(tx.tx_id);
        Ok(())
    }

    pub fn is_committed(&self, tx_id: TxId) -> bool {
        let committed = self.committed_txids.read().unwrap();
        committed.contains(&tx_id)
    }
}
