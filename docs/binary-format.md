# TheresavDB Binary Storage Format v1

## 1. Physical File Organization

TheresavDB stores relational tables and indexes in binary files partitioned into fixed-size **Pages**.

- **Default Page Size**: 16,384 bytes (16 KiB).
- **Endianness**: Little-endian across all numeric fields and disk structures.
- **File Alignment**: Every page begins at an exact multiple of 16,384 bytes from the start of the file: `FileOffset = PageId * 16384`.

---

## 2. Binary Page Layout (Slotted Page Architecture)

Each 16 KiB page is structured as follows:

```
+--------------------------------------------------------------------+
|                       Page Header (64 Bytes)                       |
+--------------------------------------------------------------------+
| Slot Directory Entry 0 (offset: u16, len: u16) (4 Bytes)           |
| Slot Directory Entry 1 (offset: u16, len: u16) (4 Bytes)           |
| ...                                                                |
| Slot Directory Entry N-1                                           |
|----------------- lower_boundary (points here) ---------------------|
|                                                                    |
|                       Unallocated Free Space                       |
|                                                                    |
|----------------- upper_boundary (points here) ---------------------|
| Tuple Data N-1                                                     |
| ...                                                                |
| Tuple Data 1                                                       |
| Tuple Data 0                                                       |
+--------------------------------------------------------------------+
|                     Special Area (Optional Tail)                   |
+--------------------------------------------------------------------+
```

- **Slot directory** grows downwards (from byte offset 64 towards higher offsets).
- **Tuple storage** grows upwards (from byte offset 16384 - special_size towards lower offsets).
- **Free space** exists strictly between `lower_boundary` and `upper_boundary`.
- When free space is fragmented due to deletions or updates, an in-place **compaction** routine defragments tuples without altering slot indices.

---

## 3. Page Header Specification (Exact 64 Bytes)

The page header layout is strictly deterministic:

| Field Name | Offset (Bytes) | Size (Bytes) | Type | Description |
|---|---|---|---|---|
| `magic` | 0 | 4 | `[u8; 4]` | Magic constant `b"THDB"` (0x42444854) |
| `format_version` | 4 | 2 | `u16` | Binary format version (`1` for v1) |
| `page_type` | 6 | 2 | `u16` | Page Type enum (see below) |
| `page_id` | 8 | 8 | `u64` | Monotonically allocated 64-bit page ID |
| `lsn` | 16 | 8 | `u64` | Log Sequence Number of the last WAL record affecting this page |
| `checksum` | 24 | 4 | `u32` | CRC32C checksum of the page (with checksum field zeroed during computation) |
| `flags` | 28 | 2 | `u16` | Bit flags: `0x01` Dirty, `0x02` Compacted, `0x04` Overflow |
| `slot_count` | 30 | 2 | `u16` | Number of slot directory entries |
| `lower_boundary` | 32 | 2 | `u16` | Byte offset where the next slot directory entry will be placed |
| `upper_boundary` | 34 | 2 | `u16` | Byte offset of the latest tuple data allocated |
| `special_area` | 36 | 2 | `u16` | Size of special area reserved at end of page (e.g. for B-Tree sibling pointers) |
| `reserved` | 38 | 26 | `[u8; 26]` | Reserved for future expansion, MUST be zeroed |
| **Total** | | **64** | | |

### Page Types
- `0x0001`: `SlottedData` (Standard Heap Table Page)
- `0x0002`: `BTreeInternal` (B-Tree Inner Router Page)
- `0x0003`: `BTreeLeaf` (B-Tree Leaf Data/Tuple Pointer Page)
- `0x0004`: `Overflow` (Large Object / LOB Continuation Page)
- `0x0005`: `FreeMap` (Free Space Bitmaps / Extent Descriptors)
- `0x0006`: `Meta` (File / Relation Master Metadata Page)

---

## 4. Slot Directory Entry Format

Each slot entry occupies 4 bytes:

```rust
#[repr(C, packed)]
pub struct PageSlot {
    pub offset: u16, // Byte offset from start of page to tuple start
    pub length: u16, // Length of tuple data in bytes (0 indicates dead/deleted slot)
}
```

- A slot with `offset == 0 && length == 0` is an unused/tombstoned slot.
- Slot indices are stable identifiers (ItemPointer / `Tid` = `{ page_id, slot_id }`). Foreign keys and indexes reference tuples using this stable 10-byte identifier (`PageId` 8 bytes + `SlotId` 2 bytes).

---

## 5. Tuple Binary Format

Tuples are packed binary records supporting MVCC and nullable columns:

```
+-----------------------------------------------------------------------------------+
| xmin (8B) | xmax (8B) | flags (2B) | schema_ver (4B) | null_bitmap (NB) | values... |
+-----------------------------------------------------------------------------------+
```

1. **`xmin` (8 Bytes, `u64`)**: Transaction ID that inserted this tuple.
2. **`xmax` (8 Bytes, `u64`)**: Transaction ID that deleted or created a newer version of this tuple (0 if active/unmodified).
3. **`flags` (2 Bytes, `u16`)**:
   - `0x0001`: Tuple has NULL values.
   - `0x0002`: Tuple has external overflow pointers (TOAST/LOB).
   - `0x0004`: Tuple updated in-place (hot update).
4. **`schema_version` (4 Bytes, `u32`)**: Catalog schema version at the time of tuple creation.
5. **`null_bitmap` (`ceil(num_columns / 8)` Bytes)**: 1 bit per column (1 = NULL, 0 = Present). Omitted if table has no nullable columns or null flag is clear.
6. **Column Values**:
   - Fixed-length columns serialized at fixed byte sizes.
   - Variable-length columns (`Text`, `Bytea`, `Jsonb`) prepended by a 2-byte or 4-byte length prefix.

---

## 6. Integrity and Bounds Validation

When reading any page from disk:
1. `magic == b"THDB"` must match exactly.
2. `format_version == 1`.
3. `lower_boundary >= 64` and `lower_boundary <= upper_boundary`.
4. `upper_boundary <= 16384 - special_area`.
5. For every slot `i < slot_count`:
   - If `length > 0`, `offset >= upper_boundary` and `offset + length <= 16384 - special_area`.
6. Calculated CRC32C over the full 16,384 bytes (with checksum bytes zeroed) must match the header `checksum`.
