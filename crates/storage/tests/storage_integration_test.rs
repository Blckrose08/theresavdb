use tempfile::tempdir;
use theresav_common::{Lsn, PageId, TxId};
use theresav_index::{BTreeIndex, IndexKey};
use theresav_storage::{DatabaseProcessLock, HeapFile};
use theresav_types::Value;
use theresav_wal::{WalReader, WalRecordType, WalWriter};

#[test]
fn test_end_to_end_storage_wal_and_index_pipeline() {
    let dir = tempdir().unwrap();
    let db_path = dir.path();

    // 1. Acquire exclusive directory process lock
    let _lock = DatabaseProcessLock::acquire(db_path).expect("Directory lock must succeed");

    // 2. Open Heap table file
    let table_path = db_path.join("users.thdb");
    let mut heap = HeapFile::open(&table_path).expect("Heap file creation must succeed");

    // 3. Open Write-Ahead Log
    let wal_path = db_path.join("theresav.wal");
    let wal_writer = WalWriter::open(&wal_path).expect("WAL creation must succeed");

    // 4. Initialize B-Tree unique index on user_id (column 0)
    let mut user_id_index = BTreeIndex::new(PageId::new(0), true);

    let tx_id = TxId::new(101);
    let lsn_begin = wal_writer
        .append(Lsn::INVALID, tx_id, WalRecordType::Begin, vec![])
        .unwrap();

    // 5. Insert records, log to WAL, and populate index
    let user_ids = vec![1001, 1002, 1003, 1004, 1005];
    let mut prev_lsn = lsn_begin;

    for &uid in &user_ids {
        let payload = format!("User Record #{}", uid).into_bytes();
        let tid = heap
            .insert_tuple(&payload)
            .expect("Tuple insert must succeed");

        // Write WAL insert record
        prev_lsn = wal_writer
            .append(prev_lsn, tx_id, WalRecordType::Insert, payload)
            .expect("WAL append must succeed");

        // Index tuple
        let key = IndexKey::single(Value::BigInt(uid));
        user_id_index
            .insert(key, tid)
            .expect("Unique index insert must succeed");
    }

    // 6. Commit transaction in WAL
    let lsn_commit = wal_writer
        .append(prev_lsn, tx_id, WalRecordType::Commit, vec![])
        .expect("WAL commit must succeed");

    assert!(wal_writer.flushed_lsn() >= lsn_commit);

    // 7. Verify index lookup retrieves exact tuple from heap
    let target_key = IndexKey::single(Value::BigInt(1003));
    let matching_tids = user_id_index
        .search(&target_key)
        .expect("Search must find key");
    assert_eq!(matching_tids.len(), 1);

    let retrieved_tuple = heap.get_tuple(matching_tids[0]).expect("Tuple must exist");
    assert_eq!(retrieved_tuple, b"User Record #1003");

    // 8. Test crash recovery: verify WAL sequentially replays all operations
    let mut wal_reader = WalReader::open(&wal_path).expect("WAL reader must open");
    let mut replayed_records = Vec::new();
    while let Some(record) = wal_reader.next_record().expect("Record must decode") {
        replayed_records.push(record);
    }

    assert_eq!(replayed_records.len(), 1 + user_ids.len() + 1); // Begin + 5 inserts + Commit
    assert_eq!(replayed_records[0].record_type, WalRecordType::Begin);
    assert_eq!(
        replayed_records.last().unwrap().record_type,
        WalRecordType::Commit
    );
}
