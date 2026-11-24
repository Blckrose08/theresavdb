use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use theresav_auth::AuthManager;
use theresav_backup::{BackupManager, BackupManifest};
use theresav_binder::Binder;
use theresav_catalog::Catalog;
use theresav_error::TheresavError;
use theresav_executor::{ExecutionContext, Executor, QueryResult};
use theresav_index::BTreeIndex;
use theresav_lock_manager::LockManager;
use theresav_optimizer::Optimizer;
use theresav_parser::Parser;
use theresav_planner::Planner;
use theresav_storage::DatabaseProcessLock;
use theresav_transaction::{IsolationLevel, Transaction, TransactionManager};
use theresav_types::Value;
use theresav_wal::WalWriter;

pub struct TheresavDB {
    _lock: DatabaseProcessLock,
    data_dir: PathBuf,
    catalog: Arc<Catalog>,
    tables: RwLock<HashMap<String, Vec<Vec<Value>>>>,
    indexes: RwLock<HashMap<String, BTreeIndex>>,
    wal_writer: Arc<WalWriter>,
    lock_manager: Arc<LockManager>,
    tx_manager: Arc<TransactionManager>,
    auth_manager: Arc<AuthManager>,
}

impl TheresavDB {
    pub fn open<P: AsRef<Path>>(data_dir: P) -> Result<Self, TheresavError> {
        let dir = data_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&dir)?;

        let lock = DatabaseProcessLock::acquire(&dir)?;

        let wal_path = dir.join("theresav.wal");
        let wal_writer = Arc::new(WalWriter::open(&wal_path)?);

        let catalog = Arc::new(Catalog::new());
        let lock_manager = Arc::new(LockManager::new());
        let tx_manager = Arc::new(TransactionManager::new(
            wal_writer.clone(),
            lock_manager.clone(),
        ));
        let auth_manager = Arc::new(AuthManager::new());

        Ok(Self {
            _lock: lock,
            data_dir: dir,
            catalog,
            tables: RwLock::new(HashMap::new()),
            indexes: RwLock::new(HashMap::new()),
            wal_writer,
            lock_manager,
            tx_manager,
            auth_manager,
        })
    }

    pub fn recover_heap(
        &self,
        heap: &mut theresav_storage::HeapFile,
    ) -> Result<theresav_recovery::RecoveryStats, TheresavError> {
        let wal_path = self.data_dir.join("theresav.wal");
        theresav_recovery::RecoveryManager::recover_from_wal(&wal_path, heap)
            .map_err(TheresavError::Wal)
    }

    pub fn execute(&self, sql: &str) -> Result<QueryResult, TheresavError> {
        let stmt = Parser::parse_sql(sql).map_err(TheresavError::Type)?;
        let bound = Binder::new(&self.catalog).bind_statement(stmt)?;
        let logical = Planner::create_logical_plan(bound);
        let physical = Optimizer::optimize(logical, &self.catalog);

        let ctx = ExecutionContext {
            catalog: &self.catalog,
            tables: &self.tables,
            indexes: &self.indexes,
        };

        let result = Executor::execute(physical, &ctx)?;
        Ok(result)
    }

    pub fn query(&self, sql: &str) -> Result<QueryResult, TheresavError> {
        self.execute(sql)
    }

    pub fn begin(&self) -> Result<Transaction, TheresavError> {
        self.begin_with_isolation(IsolationLevel::RepeatableRead)
    }

    pub fn begin_with_isolation(
        &self,
        isolation: IsolationLevel,
    ) -> Result<Transaction, TheresavError> {
        let tx = self.tx_manager.begin(isolation)?;
        Ok(tx)
    }

    pub fn commit(&self, tx: &mut Transaction) -> Result<(), TheresavError> {
        self.tx_manager.commit(tx)?;
        Ok(())
    }

    pub fn abort(&self, tx: &mut Transaction) -> Result<(), TheresavError> {
        self.tx_manager.abort(tx)?;
        Ok(())
    }

    pub fn create_backup<P: AsRef<Path>>(
        &self,
        target_dir: P,
    ) -> Result<BackupManifest, TheresavError> {
        let manifest = BackupManager::create_backup(&self.data_dir, target_dir)?;
        Ok(manifest)
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn tx_manager(&self) -> &TransactionManager {
        &self.tx_manager
    }

    pub fn auth_manager(&self) -> &AuthManager {
        &self.auth_manager
    }

    pub fn lock_manager(&self) -> &LockManager {
        &self.lock_manager
    }

    pub fn wal_writer(&self) -> &WalWriter {
        &self.wal_writer
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_embedded_db_full_pipeline() {
        let dir = tempdir().unwrap();
        let db = TheresavDB::open(dir.path()).expect("DB open should succeed");

        db.execute("CREATE TABLE products (id INTEGER PRIMARY KEY, title TEXT, price DECIMAL);")
            .unwrap();

        db.execute("INSERT INTO products (id, title, price) VALUES (101, 'Mechanical Keyboard', 129.99), (102, 'Gaming Mouse', 59.95);")
            .unwrap();

        let res = db
            .query("SELECT id, title, price FROM products WHERE id = 101;")
            .unwrap();
        assert_eq!(res.rows.len(), 1);
        assert_eq!(res.rows[0][0], Value::BigInt(101));
        assert_eq!(res.rows[0][1], Value::Text("Mechanical Keyboard".into()));

        let mut tx = db.begin().unwrap();
        db.commit(&mut tx).unwrap();
    }
}
