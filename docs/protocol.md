# TheresavDB Binary Wire Protocol v1

## 1. Frame Structure

TheresavDB uses a high-performance binary framing protocol over TCP and TLS 1.3.

All frames begin with a fixed **16-byte Frame Header**:

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                       Magic: b"THRP"                          |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|         Version (u16)         |       Message Type (u16)      |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                      Payload Length (u32)                     |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                                                               |
+                       Request ID (u64)                        +
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                        Payload Data ...                       |
+---------------------------------------------------------------+
```

### Header Fields:
- `magic`: Exactly 4 bytes: `b"THRP"` (`0x50524854`).
- `version`: Protocol version (`1` for v1).
- `message_type`: 16-bit message opcode (see below).
- `payload_length`: 32-bit unsigned integer denoting length of payload following the 16-byte header. Max frame size: 16 MiB (`16,777,216` bytes).
- `request_id`: 64-bit client-specified correlation ID echoed by the server in responses.

---

## 2. Message Types

| Opcode | Name | Direction | Description |
|---|---|---|---|
| `0x0001` | `STARTUP` | Client -> Server | Initial connection handshake, protocol negotiation |
| `0x0002` | `AUTH_CHALLENGE` | Server -> Client | Authentication challenge (SCRAM / Salt) |
| `0x0003` | `AUTH_RESPONSE` | Client -> Server | Authentication response payload |
| `0x0004` | `AUTH_OK` | Server -> Client | Authentication succeeded, session established |
| `0x0010` | `QUERY` | Client -> Server | Simple query text execution |
| `0x0011` | `PREPARE` | Client -> Server | Parse and prepare SQL statement |
| `0x0012` | `BIND` | Client -> Server | Bind parameters to prepared statement |
| `0x0013` | `EXECUTE` | Client -> Server | Execute prepared statement portal |
| `0x0014` | `FETCH` | Client -> Server | Fetch next batch of rows |
| `0x0020` | `ROW_DESCRIPTION`| Server -> Client | Column metadata and types for result rows |
| `0x0021` | `DATA_ROW` | Server -> Client | Serialized row binary values |
| `0x0022` | `COMMAND_COMPLETE`| Server -> Client| Completion status (rows affected, query time) |
| `0x0030` | `COMMIT` | Client -> Server | Commit current transaction |
| `0x0031` | `ROLLBACK` | Client -> Server | Rollback current transaction |
| `0x00FE` | `PING` / `PONG` | Both | Keepalive heartbeats |
| `0x00FF` | `ERROR` | Server -> Client | Error response with structured error code |
