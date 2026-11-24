//! Comprehensive performance benchmarks for TheresavDB storage engine, WAL, Index, and Embedded query engine.

use std::time::Instant;
use tempfile::tempdir;
use theresav_buffer_pool::BufferPoolManager;
use theresav_common::{Lsn, PageId, SlotId, TupleId, TxId};
use theresav_embedded::TheresavDB;
use theresav_index::{BTreeIndex, IndexKey};
use theresav_page::{Page, PageType, Tuple};
use theresav_storage::DiskManager;
use theresav_types::Value;
use theresav_wal::{WalRecordType, WalWriter};

fn main() {
    println!("============================================================");
    println!("             THERESAVDB PERFORMANCE BENCHMARKS              ");
    println!("============================================================");

    bench_page_operations();
    bench_wal_throughput();
    bench_btree_index();
    bench_buffer_pool();
    bench_embedded_sql();

    println!("\nAll benchmarks completed successfully.");
}

fn bench_page_operations() {
    println!("\n--- [1] 16 KiB Slotted Page Insertion & Compaction ---");
    let mut page = Page::new(PageId(1), PageType::SlottedData);
    let payload = vec![0xABu8; 128]; // 128-byte tuple payload
    let tuple = Tuple::new(TxId(100), TxId(0), 0, 1, vec![0], payload);
    let tuple_bytes = tuple.serialize();

    let iterations = 100;
    let start = Instant::now();
    for _ in 0..iterations {
        page.insert_tuple(&tuple_bytes).expect("insert tuple");
    }
    let duration = start.elapsed();
    let free_space = page.contiguous_free_space();
    let ops_per_sec = (iterations as f64) / duration.as_secs_f64();
    println!(
        "Inserted {} tuples in {:?} ({:.2} ops/sec)",
        iterations, duration, ops_per_sec
    );
    println!(
        "Remaining contiguous page free space: {} bytes / 16384 bytes",
        free_space
    );

    // Benchmark in-place compaction
    let start = Instant::now();
    page.delete_tuple(SlotId(5)).unwrap();
    page.delete_tuple(SlotId(10)).unwrap();
    page.delete_tuple(SlotId(15)).unwrap();
    page.compact().unwrap();
    let compaction_time = start.elapsed();
    println!("Defragmentation & compaction time: {:?}", compaction_time);
}

fn bench_wal_throughput() {
    println!("\n--- [2] WAL Sequential Write & Durability Flush ---");
    let dir = tempdir().unwrap();
    let wal_path = dir.path().join("bench.wal");
    let wal = WalWriter::open(&wal_path).unwrap();

    let iterations = 10_000;
    let payload = vec![0x42u8; 64];
    let start = Instant::now();

    for i in 1..=iterations {
        wal.append(
            Lsn::INVALID,
            TxId(i),
            WalRecordType::Insert,
            payload.clone(),
        )
        .unwrap();
    }
    wal.flush().unwrap();
    let duration = start.elapsed();

    let total_bytes = iterations * (34 + 64); // 34-byte header + 64 payload
    let mb = (total_bytes as f64) / (1024.0 * 1024.0);
    let throughput_mb_s = mb / duration.as_secs_f64();
    let ops_sec = (iterations as f64) / duration.as_secs_f64();

    println!(
        "Appended & flushed {} WAL records in {:?} ({:.2} records/sec, {:.2} MB/s)",
        iterations, duration, ops_sec, throughput_mb_s
    );
}

fn bench_btree_index() {
    println!("\n--- [3] B-Tree Index (Unique & Composite Keys) ---");
    let mut btree = BTreeIndex::new(PageId(1), true); // unique
    let iterations = 5_000;

    let start = Instant::now();
    for i in 0..iterations {
        let key = IndexKey::single(Value::BigInt(i));
        btree
            .insert(
                key,
                TupleId {
                    page_id: PageId(1),
                    slot_id: SlotId(i as u16),
                },
            )
            .unwrap();
    }
    let insert_dur = start.elapsed();
    let insert_ops = (iterations as f64) / insert_dur.as_secs_f64();
    println!(
        "Inserted {} keys in {:?} ({:.2} ops/sec)",
        iterations, insert_dur, insert_ops
    );

    let start = Instant::now();
    for i in 0..iterations {
        let key = IndexKey::single(Value::BigInt(i));
        let res = btree.search(&key);
        assert!(res.is_some(), "Key must be present");
    }
    let lookup_dur = start.elapsed();
    let lookup_ops = (iterations as f64) / lookup_dur.as_secs_f64();
    let latency_us = (lookup_dur.as_micros() as f64) / (iterations as f64);
    println!(
        "Looked up {} keys in {:?} ({:.2} ops/sec, {:.3} µs/lookup)",
        iterations, lookup_dur, lookup_ops, latency_us
    );
}

fn bench_buffer_pool() {
    println!("\n--- [4] Buffer Pool Manager (Clock Eviction & Page Cache) ---");
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("bench.db");
    let file = DiskManager::open_file(&db_path).unwrap();

    // 64-frame buffer pool
    let bpm = BufferPoolManager::new(64, file);

    let iterations = 200;
    let payload = vec![0x33u8; 32];
    let tuple = Tuple::new(TxId(1), TxId(0), 0, 1, vec![0], payload);
    let tuple_bytes = tuple.serialize();

    let start = Instant::now();
    for _ in 0..iterations {
        let (page_id, frame_arc) = bpm.new_page(PageType::SlottedData).unwrap();
        {
            let mut frame = frame_arc.write().unwrap();
            frame.page.insert_tuple(&tuple_bytes).unwrap();
        }
        bpm.unpin_page(page_id, true).unwrap();

        // Fetch back and verify
        let fetched = bpm.fetch_page(page_id).unwrap();
        {
            let guard = fetched.read().unwrap();
            assert_eq!(guard.page.header().page_id, page_id);
        }
        bpm.unpin_page(page_id, false).unwrap();
    }
    bpm.flush_all().unwrap();
    let duration = start.elapsed();
    let ops = (iterations as f64) / duration.as_secs_f64();
    println!(
        "Allocated, written, and flushed {} pages in {:?} ({:.2} pages/sec)",
        iterations, duration, ops
    );
}

fn bench_embedded_sql() {
    println!("\n--- [5] Embedded Engine SQL Query Execution ---");
    let dir = tempdir().unwrap();
    let db = TheresavDB::open(dir.path()).unwrap();

    db.execute("CREATE TABLE users (id BIGINT, name TEXT, balance DECIMAL);")
        .unwrap();

    let iterations = 200;
    let start = Instant::now();
    for i in 0..iterations {
        let query = format!("INSERT INTO users VALUES ({}, 'User{}', 1500.50);", i, i);
        db.execute(&query).unwrap();
    }
    let insert_dur = start.elapsed();
    let insert_ops = (iterations as f64) / insert_dur.as_secs_f64();
    println!(
        "Executed {} SQL INSERTs in {:?} ({:.2} inserts/sec)",
        iterations, insert_dur, insert_ops
    );

    let start = Instant::now();
    let result = db.query("SELECT * FROM users;").unwrap();
    let select_dur = start.elapsed();
    println!(
        "Executed SELECT query returning {} rows in {:?}",
        result.rows.len(),
        select_dur
    );
    assert_eq!(result.rows.len(), iterations);
}
