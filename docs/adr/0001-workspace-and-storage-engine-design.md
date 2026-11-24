# ADR 0001: Workspace Structure and Slotted Binary Page Storage Format

## Status
Accepted

## Context
TheresavDB is designed as an independent RDBMS implemented from scratch in Rust.
To achieve reliability, high performance, and crash resilience without external database dependencies, the foundational layer must define:
1. A modular workspace preventing circular dependencies.
2. A binary storage format with predictable alignment, strong integrity checks, and zero-panic boundary guarantees.
3. Explicit serialization semantics that treat raw disk bytes as untrusted data.

## Decision

### 1. Workspace Organization
The project uses a Cargo workspace with partitioned crates:
- `common`: Scalar identifiers (`PageId`, `TxId`, `Lsn`, `Oid`), byte orders, constants.
- `error`: Structured errors via `thiserror`.
- `types`: Logical and binary value representations.
- `page`: 16 KiB slotted page format, slot directory, and tuple packing.
Subsequent crates (`storage`, `buffer-pool`, `wal`, `mvcc`, `parser`, `server`, etc.) will depend downward without cycles.

### 2. Page Format (16 KiB Slotted Page)
- Page size fixed at **16,384 bytes** (16 KiB), matching modern SSD block sizes and operating system I/O characteristics.
- **64-byte Header**:
  - `magic`: `b"THDB"` (`0x42444854`).
  - `format_version`: `1` (`u16`).
  - `page_type`: 16-bit enum distinguishing Heap, B-Tree, Overflow, and Meta pages.
  - `page_id`: 64-bit monotonically assigned identifier.
  - `lsn`: 64-bit Log Sequence Number for ARIES recovery.
  - `checksum`: 32-bit CRC32C over the full 16 KiB page.
  - `flags`, `slot_count`, `lower_boundary`, `upper_boundary`, `special_area`, `reserved`.
- **Slotted Storage**:
  - Slot directory entries (`offset: u16`, `length: u16`) grow downward from offset 64.
  - Tuples grow upward from offset 16384.
  - Deleted tuples leave a tombstoned slot (`offset: 0, length: 0`) and page compaction defragments data when required.

### 3. Checksums and Validation
- CRC32C is selected for page checksums due to hardware acceleration (SSE4.2 / ARM CRC) and fast verification.
- Every read checks: magic bytes, version, boundary invariants (`64 <= lower_boundary <= upper_boundary <= 16384`), slot offset bounds, and CRC32C. Any mismatch immediately yields a typed `PageError::Corrupted` rather than panicking or executing undefined behavior.

## Consequences
- Clean separation between storage primitives and higher-level database engines.
- Guaranteed memory safety and defensive disk byte parsing.
- Stable tuple identifiers (`PageId`, `SlotId`) usable by indexes and MVCC.
