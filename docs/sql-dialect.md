# TheresavSQL Dialect Specification

## 1. Overview

**TheresavSQL** is the native SQL dialect for TheresavDB. It provides an intuitive, standard-compliant SQL syntax with strict typing, first-class JSONB manipulation, and explicit transactional semantics.

---

## 2. Supported Data Types

| SQL Type | Rust Type | Description |
|---|---|---|
| `BOOLEAN` / `BOOL` | `bool` | True or False |
| `SMALLINT` / `INT2` | `i16` | Signed 16-bit integer |
| `INTEGER` / `INT` / `INT4` | `i32` | Signed 32-bit integer |
| `BIGINT` / `INT8` | `i64` | Signed 64-bit integer |
| `REAL` / `FLOAT4` | `f32` | 32-bit IEEE 754 floating point |
| `DOUBLE PRECISION` / `FLOAT8` | `f64` | 64-bit IEEE 754 floating point |
| `DECIMAL(p, s)` / `NUMERIC` | `Decimal` | Exact fixed-point numeric representation |
| `TEXT` | `String` | Variable-length UTF-8 text |
| `VARCHAR(n)` | `String` | Length-constrained UTF-8 text |
| `BYTEA` | `Vec<u8>` | Variable-length raw binary octets |
| `UUID` | `uuid::Uuid` | 128-bit Universally Unique Identifier |
| `DATE` | `NaiveDate` | Calendar date (year, month, day) |
| `TIMESTAMP` | `NaiveDateTime` | Date and time without timezone |
| `TIMESTAMPTZ` | `DateTime<Utc>` | Date and time with UTC timezone |
| `JSON` | `String` | Text-validated JSON representation |
| `JSONB` | `BinaryJson` | Binary-encoded decomposed JSON |

---

## 3. SQL Grammar Subsets

### 3.1 Data Definition Language (DDL)
```sql
CREATE DATABASE database_name;
DROP DATABASE database_name;

CREATE TABLE [IF NOT EXISTS] table_name (
    column_name data_type [PRIMARY KEY] [NOT NULL] [UNIQUE] [DEFAULT expr],
    ...
    [CONSTRAINT constraint_name FOREIGN KEY (cols) REFERENCES target_table(cols)]
);

DROP TABLE [IF EXISTS] table_name;
ALTER TABLE table_name ADD COLUMN column_name data_type;
ALTER TABLE table_name DROP COLUMN column_name;

CREATE [UNIQUE] INDEX index_name ON table_name (column_name, ...);
DROP INDEX index_name;
```

### 3.2 Data Manipulation Language (DML)
```sql
INSERT INTO table_name (col1, col2, ...) VALUES (val1, val2, ...), (...);

SELECT [DISTINCT] expr1, expr2, ...
FROM table_name
[WHERE condition]
[ORDER BY expr [ASC | DESC]]
[LIMIT n] [OFFSET m];

UPDATE table_name
SET col1 = expr1, col2 = expr2
[WHERE condition];

DELETE FROM table_name
[WHERE condition];
```

### 3.3 Transaction Control Language (TCL)
```sql
BEGIN [TRANSACTION] [ISOLATION LEVEL {READ COMMITTED | REPEATABLE READ | SERIALIZABLE}];
COMMIT;
ROLLBACK;
SAVEPOINT savepoint_name;
ROLLBACK TO SAVEPOINT savepoint_name;
RELEASE SAVEPOINT savepoint_name;
```
