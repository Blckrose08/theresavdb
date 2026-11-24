# TheresavDB Phased Development Roadmap

## Phase Overview

### v0.1 — Foundation (Current Target)
- [x] Workspace initialization and toolchain configuration.
- [x] Architecture, binary format, security, dialect, protocol documentation.
- [x] ADR-0001: Initial Architecture and Binary Storage Format.
- [ ] Core primitives crate (`crates/common`): PageId, TxId, Lsn, Oid, constants.
- [ ] Error hierarchy crate (`crates/error`): Domain-specific typed error model.
- [ ] Types crate (`crates/types`): 15 standard database types with binary serialization.
- [ ] Slotted page crate (`crates/page`): 16 KiB page format, 64-byte header, slot management, CRC32C, compaction, bounds validation.
- [ ] Unit test suite and property-based test foundations.

### v0.2 — Storage Engine & WAL
- Heap file manager and disk page allocator.
- Buffer pool manager with frame table and clock/LRU replacement.
- Free-space map (FSM) tracking.
- B-Tree index engine (unique and composite indexes).
- Write-Ahead Logging (WAL) record serialization and flush coordinator.

### v0.3 — SQL Engine Foundation
- Lexer and recursive descent parser for TheresavSQL.
- Abstract Syntax Tree (AST) definition.
- Semantic Binder and Catalog resolution.
- Logical and Physical Planner.
- Basic execution operators: `SeqScan`, `Filter`, `Projection`, `Limit`.
- DDL (`CREATE TABLE`, `DROP TABLE`) and DML (`INSERT`, `SELECT`).

### v0.4 — Transaction Engine & MVCC
- Transaction Manager with active transaction table.
- MVCC snapshot isolation and tuple visibility engine (`xmin`/`xmax`).
- Hierarchical Lock Manager with 2PL and deadlock detection.
- ARIES-style crash recovery (Analysis, Redo, Undo with CLRs).

### v0.5 — Server, Embedded & Client APIs
- Async Tokio TCP server binding to `127.0.0.1:7432`.
- Binary wire protocol framing (`THRP`).
- SCRAM-SHA-256 and Argon2id authentication.
- Embedded Rust API (`TheresavDB::open("./data")`).
- Interactive CLI client (`theresav-cli`).

### v0.6 — Advanced Query Execution
- Join operators: `NestedLoopJoin`, `HashJoin`.
- Aggregates and `GROUP BY` execution.
- Subqueries and CTEs.
- Rule-based and cost-based query optimizer.
- Rich JSONB operators and indexing.

### v0.7 — Multi-Language Ecosystem
- Node.js / Bun client (`@theresav/db`) via pure wire protocol (no Rust compile step required).
- Python client (`theresavdb`) on PyPI.
- Docker multi-stage container and release pipeline.

### v1.0 — Production Release
- Comprehensive security audit.
- Full & incremental backup verification.
- Reproducible Criterion benchmarks against SQLite and PostgreSQL.
- Formal release on crates.io and npm.
