# TheresavDB System Architecture

## 1. Overview and Design Philosophy

**TheresavDB** is an independent, high-performance relational database management system (RDBMS) written from scratch in Rust. It is neither a fork, wrapper, nor modification of SQLite or PostgreSQL. While drawing inspiration from proven architectural principles of modern databases, every core component—storage engine, transaction manager, MVCC, write-ahead logging (WAL), SQL planner/executor, and binary wire protocol—is custom-designed.

Key Architectural Tenets:
1. **Correctness and Data Integrity First**: Data persistence, transactional guarantees (ACID), and crash consistency are never compromised for raw synthetic benchmarks.
2. **Unified Core Engine**: The same engine powers both standalone TCP server mode and in-process embedded mode. No redundant or divergent database implementations exist.
3. **Zero-Trust Binary Storage**: Every byte loaded from disk or network is treated as untrusted and subjected to boundary, checksum, and semantic validation.
4. **Strict Modularity**: Layered crate architecture in a Rust workspace with strictly directed dependencies and zero circular dependencies.
5. **Modern Concurrency**: Asynchronous I/O via Tokio for network services combined with fine-grained latching and MVCC snapshot isolation in storage.

---

## 2. Workspace Modular Architecture

```
+-----------------------------------------------------------------------------------+
|                                Client Applications                                 |
|           Rust Client  |  Node.js / Bun Client  |  Python Client  |  CLI          |
+-----------------------------------------------------------------------------------+
                                          |
                      (Custom Binary Protocol / TLS 1.3)
                                          v
+-----------------------------------------------------------------------------------+
|                                 Server Layer                                      |
|   crates/server (Tokio TCP / Session State)  |  crates/embedded (In-Process C API) |
|   crates/protocol (Frame Encoding / Decoding)|  crates/auth (SCRAM / RBAC)        |
+-----------------------------------------------------------------------------------+
                                          |
                                          v
+-----------------------------------------------------------------------------------+
|                                SQL Query Engine                                   |
|   crates/parser    : Lexer, Recursive Descent Parser, TheresavSQL AST              |
|   crates/binder    : Semantic Analysis, Scope Resolution, Catalog Binding         |
|   crates/planner   : Logical Plan Generation                                      |
|   crates/optimizer : Rule-Based / Cost-Based Query Transformations                 |
|   crates/executor  : Volcano / Push-based Operator Pipelines                      |
+-----------------------------------------------------------------------------------+
                                          |
                                          v
+-----------------------------------------------------------------------------------+
|                        Transaction & Concurrency Layer                            |
|   crates/transaction  : Tx Lifecycle, Savepoints, Commit Protocol                |
|   crates/mvcc         : Read Views, Snapshot Isolation, Visibility Engine         |
|   crates/lock-manager : Hierarchical Intent & Row Locks, Deadlock Detector        |
+-----------------------------------------------------------------------------------+
                                          |
                                          v
+-----------------------------------------------------------------------------------+
|                          Storage & Recovery Engine                                |
|   crates/index       : B-Tree Index Engine (Unique, Composite, Range Queries)     |
|   crates/storage     : Heap File Management, Free-Space Map, Table Storage        |
|   crates/buffer-pool : Buffer Pool Manager, Clock / LRU-k Eviction, Dirty Tracking|
|   crates/wal         : Append-Only Write-Ahead Log, Group Commit, LSN Generator   |
|   crates/recovery    : ARIES Redo/Undo, Compensation Log Records (CLR)            |
|   crates/page        : Slotted Page Layout (16 KiB), Checksums, Compaction        |
+-----------------------------------------------------------------------------------+
                                          |
                                          v
+-----------------------------------------------------------------------------------+
|                              Foundational Layer                                   |
|   crates/catalog : System Catalogs, Schema Metadata, Relational Definitions       |
|   crates/types   : Logical Type System, Strict Ordering, Binary Representation    |
|   crates/error   : Unified Domain Error Hierarchy (thiserror)                     |
|   crates/common  : Shared Primitives, LSN, TxId, PageId, Byte Order Constants     |
+-----------------------------------------------------------------------------------+
```

---

## 3. Layer Responsibilities & Data Flow

### 3.1 Foundational Layer
- **`crates/common`**: Defines fundamental scalar identifiers: `PageId` (64-bit), `TxId` (64-bit), `Lsn` (64-bit log sequence number), `Oid` (64-bit object identifier), and page constants (`PAGE_SIZE = 16384`).
- **`crates/error`**: Structured typed errors (`TheresavError`) with zero string-based error propagation.
- **`crates/types`**: Type system enforcing fixed-point arithmetic (`Decimal`), exact binary JSON (`Jsonb`), variable-length primitives (`Text`, `Bytea`), temporal representations (`Timestamp`, `Date`), and UUIDs.
- **`crates/catalog`**: Transactional relational metadata describing databases, schemas, tables, columns, indexes, and user privileges.

### 3.2 Storage Layer
- **`crates/page`**: Slotted page format adhering to a 64-byte header, slot directory growing upward, and tuple data packed downward. Includes CRC32C validation.
- **`crates/buffer-pool`**: Frame table managing in-memory caching of 16 KiB disk pages with concurrent read/write latches.
- **`crates/storage`**: File heap manager coordinating data files, extension, and free-space map (FSM).
- **`crates/wal` & `crates/recovery`**: Strict Write-Ahead Logging following ARIES principles:
  - Changes logged to WAL before dirty pages are flushed.
  - Page LSN tracking to prevent unneeded Redo.
  - Undo phase rolling back active uncommitted transactions using CLRs.

### 3.3 Transaction & Concurrency Layer
- Multi-Version Concurrency Control (MVCC) where tuples store `xmin` (creating transaction) and `xmax` (deleting/updating transaction).
- Concurrency levels: Read Committed, Repeatable Read, and Serializable (using SSI predicate tracking).
- Two-phase locking (2PL) for physical index pages and structural alterations, with wait-for-graph cycle detection for deadlock resolution.

### 3.4 Query Processing Layer
- **Lexer & Parser**: Converts TheresavSQL text into strongly-typed AST nodes.
- **Binder**: Validates table/column existence against Catalog, assigns types, and computes expression signatures.
- **Planner & Optimizer**: Translates AST to Logical Plan, applies rewrite rules (projection pushdown, predicate pushdown), and emits Physical Plan.
- **Executor**: Executes physical iterator trees producing `Tuple` batches.

---

## 4. Architectural & Performance Benchmark Comparison

For a detailed side-by-side engineering and benchmark comparison of TheresavDB against PostgreSQL, MySQL (InnoDB), and MariaDB, refer to [benchmarks-comparison.md](file:///home/ubuntu/database/docs/benchmarks-comparison.md).

