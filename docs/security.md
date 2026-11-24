# TheresavDB Security Architecture

## 1. Threat Model & Security Principles

TheresavDB treats security as a core architectural constraint.

### Principles:
1. **Zero-Trust Input Processing**: All inputs—from network TCP packets, client query strings, to bytes stored on disk—are untrusted and must be rigorously validated before ingestion.
2. **Defense in Depth**: Security controls are enforced at multiple independent layers: network transport, protocol framing, authentication, catalog authorization (RBAC), query binding, and physical storage.
3. **Least Privilege**: Default configurations are locked down: local-only binding (`127.0.0.1`), mandatory authentication, and restrictive file permissions (`0700` for data directories).

---

## 2. Authentication & Credential Storage

### Local Password Storage
- Passwords are never stored in plaintext or with reversible encryption.
- Hashing uses **Argon2id** with memory-hard parameters:
  - Memory cost: 64 MiB
  - Iterations: 3
  - Parallelism: 4
  - Unique cryptographically secure 128-bit salt per user.

### Network Authentication
- Supported Mechanisms:
  - **SCRAM-SHA-256** (Salted Challenge Response Authentication Mechanism) for standard client-server authentication.
  - **Mutual TLS (mTLS)** for cryptographically authenticated node-to-node or service-to-database connections.
  - **Local Peer Authentication** via UNIX domain sockets using OS credentials (`SO_PEERCRED`).
- Constant-time memory comparison (`subtle::ConstantTimeEq`) is mandated for all secret and hash comparisons to eliminate timing attack vectors.

---

## 3. Role-Based Access Control (RBAC)

Hierarchical roles control fine-grained permissions:

### Built-in Roles:
- `superuser`: Complete access to all engine internals, system catalogs, and administrative commands.
- `db_admin`: Manages databases, schemas, users, and grants within assigned databases.
- `schema_admin`: Creates, modifies, and drops tables, views, and indexes within designated schemas.
- `developer`: Read and write data (`SELECT`, `INSERT`, `UPDATE`, `DELETE`) on application tables.
- `readonly`: Query execution (`SELECT`) only.
- `backup_operator`: Executes full and incremental backups and inspects replication status.

### Permission Hierarchy:
Permissions can be granted at:
1. Database level.
2. Schema level.
3. Table level.
4. Column level.

All authorization checks occur inside the **Binder** before query execution begins, guaranteeing that unprivileged users cannot observe table structure or column data.

---

## 4. SQL Injection Protection

- Input parameterization is enforced via the binary wire protocol (`PREPARE`, `BIND`, `EXECUTE`).
- User input cannot modify the Abstract Syntax Tree (AST).
- Prepared statements parse and analyze SQL strictly once; subsequent parameter bindings are verified against explicit data type constraints.
- Identifiers are validated against strict alphanumeric and character set rules.

---

## 5. Storage & Memory Safety

- **100% Safe Rust by Default**: No unbounded pointer arithmetic or transmutes across unverified memory blocks.
- **Bounds Checking**: Every page decode checks `lower_boundary`, `upper_boundary`, slot offsets, and tuple boundaries.
- **Encryption at Rest**:
  - File pages encrypted using **AES-256-GCM** or **XChaCha20-Poly1305** authenticated encryption.
  - Page checksums are verified before cryptographic decryption.
  - External key management with key rotation support.
