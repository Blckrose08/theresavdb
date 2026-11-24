# theresavdb

Official Python client driver for [TheresavDB](https://github.com/theresav/theresavdb) relational database engine.

<p align="center">
  <img src="https://raw.githubusercontent.com/theresav/theresavdb/master/assets/theresavdb-banner.png" alt="TheresavDB Banner" width="100%">
</p>

## Overview

`theresavdb` is a lightweight, zero-dependency Python client communicating directly over TheresavDB's binary wire protocol (`THRP`). It does not require a C compiler, libpq, or a local Rust toolchain.

## Installation

```bash
pip install theresavdb
```

## Quick Start

```python
from theresavdb import connect

# Connect to a TheresavDB server instance
conn = connect(host="db.theresav.eu", port=7432)
cursor = conn.cursor()

# Execute DDL & DML
cursor.execute("CREATE TABLE users (id BIGINT, name TEXT, balance DECIMAL);")
cursor.execute("INSERT INTO users VALUES (1, 'Alice', 1500.50);")
cursor.execute("INSERT INTO users VALUES (2, 'Bob', 2750.00);")

# Query data
cursor.execute("SELECT id, name, balance FROM users WHERE id = 1;")
row = cursor.fetchone()
print(f"User: {row}")

# Fetch all rows
cursor.execute("SELECT * FROM users;")
for r in cursor.fetchall():
    print(r)

# Close connection
cursor.close()
conn.close()
```

## Features

- **Zero External Dependencies**: Uses only standard Python library (`socket`, `struct`, `json`).
- **Binary Wire Protocol**: Low latency, compact 20-byte frame header communication.
- **Cross-Platform**: Compatible with Python 3.8+ on Linux, macOS, and Windows.
- **PEP 249 Inspired API**: Familiar `connect()`, `cursor()`, `execute()`, `fetchone()`, and `fetchall()`.
