use tempfile::tempdir;
use theresav_buffer_pool::BufferPoolManager;
use theresav_common::{Lsn, PageId, TxId};
use theresav_index::{BTreeIndex, IndexKey};
use theresav_page::{PageType, Tuple};
use theresav_storage::{DatabaseProcessLock, DiskManager, HeapFile};
use theresav_types::Value;
use theresav_wal::{WalReader, WalRecordType, WalWriter};

#[test]
fn test_end_to_end_storage_wal_and_index_pipeline() {
    let dir = tempdir().unwrap();
    let db_path = dir.path();

    let lock = DatabaseProcessLock::acquire(db_path).expect("Must acquire directory lock");
    assert!(lock.lock_path().exists());

    let wal_path = db_path.join("theresav.wal");
    let wal = WalWriter::open(&wal_path).expect("WAL file creation must succeed");

    let tx_id = TxId::new(42);
    let lsn_begin = wal
        .append(Lsn::INVALID, tx_id, WalRecordType::Begin, vec![])
        .unwrap();

    let table_path = db_path.join("users.thdb");
    let mut heap = HeapFile::open(&table_path).expect("Table heap must open cleanly");

    let mut index = BTreeIndex::new(PageId::new(0), true);

    let mut inserted_tids = Vec::new();
    let mut prev_lsn = lsn_begin;

    for id in 1..=10 {
        let tuple = Tuple::new(
            tx_id,
            TxId::INVALID,
            0,
            1,
            vec![],
            format!("User Record Name_{}", id).into_bytes(),
        );
        let serialized_tuple = tuple.serialize();

        let lsn_insert = wal
            .append(
                prev_lsn,
                tx_id,
                WalRecordType::Insert,
                serialized_tuple.clone(),
            )
            .unwrap();
        prev_lsn = lsn_insert;

        let tid = heap.insert_tuple(&serialized_tuple).unwrap();

        let key = IndexKey::single(Value::Integer(id));
        index.insert(key, tid).unwrap();

        inserted_tids.push((id, tid, tuple));
    }

    let lsn_commit = wal
        .append(prev_lsn, tx_id, WalRecordType::Commit, vec![])
        .unwrap();
    assert!(wal.flushed_lsn() >= lsn_commit);

    let search_key = IndexKey::single(Value::Integer(5));
    let found_tids = index
        .search(&search_key)
        .expect("Key 5 must exist in index");
    assert_eq!(found_tids.len(), 1);
    let raw = heap.get_tuple(found_tids[0]).unwrap();
    let decoded = Tuple::deserialize(&raw).unwrap();
    assert_eq!(decoded.xmin, tx_id);
    assert_eq!(decoded.payload, b"User Record Name_5");

    let low = IndexKey::single(Value::Integer(3));
    let high = IndexKey::single(Value::Integer(7));
    let range_results = index.scan_range(Some(&low), Some(&high));
    assert_eq!(range_results.len(), 5);

    let data_file = DiskManager::open_file(&table_path).unwrap();
    let bpm = BufferPoolManager::new(4, data_file);

    let (new_page_id, frame) = bpm.new_page(PageType::SlottedData).unwrap();
    {
        let mut guard = frame.write().unwrap();
        guard.page.insert_tuple(b"BufferPoolCachedPayload").unwrap();
    }
    bpm.unpin_page(new_page_id, true).unwrap();
    bpm.flush_all().unwrap();

    let mut reader = WalReader::open(&wal_path).unwrap();
    let mut record_count = 0;
    while let Some(record) = reader.next_record().unwrap() {
        record_count += 1;
        assert_eq!(record.tx_id, tx_id);
    }
    assert_eq!(record_count, 12);
}
