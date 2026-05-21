# RorisDB

> A single-node OLAP database with Doris-compatible SQL, built in Rust.
>
> Learn Doris SQL syntax, experiment with OLAP patterns, and explore columnar storage — all in one binary.

[![Apache-2.0 License](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-2024--edition-orange.svg)](https://www.rust-lang.org)
[![Version](https://img.shields.io/badge/Version-0.2.0-green.svg)]()
[![Documentation](https://img.shields.io/badge/Docs-English-blue)](docs/en/)
[![中文文档](https://img.shields.io/badge/Docs-中文-green)](docs/zh/)

## What is RorisDB?

RorisDB is a **Doris-compatible single-node OLAP database** for learning and experimentation. It speaks Doris SQL dialect — including `DUPLICATE KEY`, `DISTRIBUTED BY HASH`, `date_trunc`, `months_add` — but runs as a single binary with no cluster setup required.

**Use cases:**
- Learn Doris/OLAP SQL syntax without deploying a cluster
- Experiment with columnar storage and Parquet file formats
- Prototype analytical queries locally before moving to production Doris
- Study OLAP database internals in readable Rust code

### How it works

- **Apache DataFusion** as the query engine (SQL → Arrow → execution)
- **Apache Parquet** as the storage format (columnar, compressed, portable)
- **MySQL wire protocol** for connectivity (works with any MySQL client)
- **Rust** for memory safety and single-binary deployment

### Naming

**RorisDB** = **R**ust + (D)**oris** + **DB**

## Quick Start

```bash
# Build
cargo build --release

# Run
./target/release/roris-fe --http-port 8030

# Connect with any MySQL client
mysql -h 127.0.0.1 -P 9030 -uroot
```

```sql
CREATE DATABASE test;
USE test;

-- Doris-compatible table syntax
CREATE TABLE users (
    id INT,
    name VARCHAR(100),
    age INT,
    created_at DATE
) DUPLICATE KEY(id)
DISTRIBUTED BY HASH(id) BUCKETS 1;

INSERT INTO users VALUES (1, 'Alice', 30, '2024-01-15'), (2, 'Bob', 25, '2024-02-20');

SELECT name, age FROM users WHERE age > 20 ORDER BY age;
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      MySQL Client                            │
│              (any mysql CLI, JDBC, ORM, etc.)                │
└──────────────────────────┬──────────────────────────────────┘
                           │ MySQL wire protocol
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                    mysql-protocol                             │
│              (handshake, auth, COM_QUERY)                     │
└──────────────────────────┬──────────────────────────────────┘
                           │ QueryHandler trait
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                      roris-server                             │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │
│  │  DDL     │  │  DML     │  │  SELECT  │  │  SHOW    │   │
│  │ handler  │  │ handler  │  │(DataFusion│  │ commands │   │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘   │
└───────┼──────────────┼──────────────┼──────────────┼────────┘
        │              │              │              │
        ▼              ▼              ▼              ▼
┌──────────────┐ ┌──────────┐ ┌──────────────┐ ┌──────────┐
│  fe-catalog  │ │fe-storage│ │ fe-datafusion│ │fe-monitor│
│  (metadata)  │ │ (Parquet)│ │  (UDFs +    │ │ (HTTP +  │
│              │ │          │ │   types)    │ │ metrics) │
└──────────────┘ └────┬─────┘ └──────────────┘ └──────────┘
                      │
                      ▼
              ┌──────────────┐
              │    Parquet   │
              │    files     │
              └──────────────┘
```

### Query Flow

1. **MySQL client** sends SQL over wire protocol
2. **mysql-protocol** parses packet, authenticates, dispatches to query handler
3. **roris-server** routes by statement type:
   - `SELECT` / `UNION` / `EXPLAIN` → DataFusion `SessionContext`
   - `INSERT` / `UPDATE` / `DELETE` → DML handler → Parquet storage
   - `CREATE` / `DROP` / `ALTER` → DDL handler → Catalog
   - `SHOW` / `DESCRIBE` → Query executor → Catalog
4. **DataFusion** optimizes and executes queries via `ParquetTableProvider`
5. **ParquetTableProvider** reads Parquet files with filter/projection pushdown

## Features

### SQL Support

| Category | Support |
|----------|---------|
| **DDL** | `CREATE/DROP DATABASE`, `CREATE/DROP TABLE`, `ALTER TABLE`, `TRUNCATE TABLE` |
| **DML** | `INSERT INTO ... VALUES`, `UPDATE`, `DELETE` with `WHERE` |
| **Queries** | `SELECT`, `JOIN` (INNER/LEFT/RIGHT/FULL/CROSS), `UNION`, subqueries, CTEs (`WITH`) |
| **Aggregates** | `COUNT`, `SUM`, `AVG`, `MIN`, `MAX`, `COUNT(DISTINCT)`, `GROUP_CONCAT` |
| **Window** | `ROW_NUMBER`, `RANK`, `DENSE_RANK`, `LAG`, `LEAD` |
| **Functions** | Math (30+), String, Date/Time (`date_trunc`, `months_add`, `days_add`) |
| **Data Types** | Boolean, Int8-64, UInt8-64, Float32/64, Decimal, Date, DateTime, String, Binary, Array, Map, Struct |

### Storage

| Feature | Description |
|---------|-------------|
| **Format** | Apache Parquet (columnar, ZSTD compressed, page-level statistics) |
| **Layout** | `data/{database}/{table}/data.parquet` — one file per table |
| **Atomicity** | Write temp file → fsync → rename (crash-safe) |
| **Pushdown** | Filter + projection + limit pushed to Parquet reader |

### Infrastructure

| Feature | Description |
|---------|-------------|
| **Protocol** | MySQL wire protocol (handshake, `mysql_native_password`, `COM_QUERY`) |
| **Monitoring** | HTTP server (`:8030`), Prometheus metrics, audit log, query profiles |
| **Metadata** | JSON catalog (`catalog.json`) + optional RocksDB backend |
| **UDFs** | Doris-compatible: `date_trunc`, `months_add`, `concat_ws`, `substring_index`, `bitmap_count` |

## Project Structure

```
RorisDB/
├── roris-server/          # Main binary: query routing, DDL/DML handlers
├── crates/
│   ├── fe-sql-parser/     # SQL parsing (sqlparser + Doris extensions)
│   ├── fe-catalog/        # Metadata: databases, tables, partitions, views
│   ├── fe-datafusion/     # DataFusion integration, UDFs, type conversion
│   ├── fe-storage/        # Parquet storage + DataFusion TableProvider
│   ├── fe-common/         # EditLog, shared FE utilities
│   ├── fe-monitor/        # HTTP server, metrics, audit log
│   ├── mysql-protocol/    # MySQL wire protocol implementation
│   ├── be-rocks/          # RocksDB metadata backend
│   ├── types/             # DataType, Block, Vector, Bitmap, Schema
│   └── common/            # Error types (DrorisError)
├── benches/tpch/          # TPC-H benchmark suite
├── tests/integration/     # Integration tests (47 test cases)
└── docs/                  # Documentation (English + 中文)
```

**Total**: ~27,000 lines of Rust across 11 crates.

## Tech Stack

| Component | Technology | Version |
|-----------|-----------|---------|
| Query Engine | Apache DataFusion | 47 |
| Columnar Format | Apache Arrow | 55 |
| Storage Format | Apache Parquet | 55 |
| SQL Parser | sqlparser-rs | 0.53 |
| Async Runtime | Tokio | 1.x |
| Metadata | RocksDB / JSON | 0.23 |
| HTTP Server | Axum | 0.7 |
| Metrics | Prometheus | 0.13 |

## Known Limitations

| Limitation | Details |
|------------|---------|
| **Single file per table** | INSERT is O(N) — reads entire Parquet + concat + rewrite. Multi-segment append writes planned. |
| **No transactions** | `BEGIN/COMMIT/ROLLBACK` parsed but not enforced (no MVCC) |
| **Weak auth** | Any password accepted (`mysql_native_password` handshake only) |
| **Single node** | No clustering, replication, or distributed execution |
| **No streaming import** | No CSV/JSON bulk load (Stream Load not implemented) |

## Roadmap

| Priority | Item | Status |
|----------|------|--------|
| P0 | Multi-segment storage (append writes + compaction) | Planned |
| P0 | Real transactions (MVCC or simplified WAL) | Planned |
| P1 | Parquet predicate pushdown (row group pruning) | Planned |
| P1 | Partition table execution (parser ready) | Planned |
| P2 | Replace `types` crate with native Arrow types | Planned |
| P2 | Arrow-native QueryResult (eliminate string conversion) | Planned |

## Doris Compatibility

RorisDB implements a subset of Apache Doris SQL dialect for learning purposes:

| Feature | Status |
|---------|--------|
| `DUPLICATE KEY` / `DISTRIBUTED BY HASH` | ✅ Parsed and accepted (single-node, no actual distribution) |
| `PARTITION BY RANGE/LIST` | ✅ Parsed, execution planned |
| Doris UDFs (`date_trunc`, `months_add`, `days_add`, `concat_ws`) | ✅ Compatible |
| `INSERT INTO ... VALUES` / `INSERT INTO ... SELECT` | ✅ Supported |
| `UPDATE` / `DELETE` with `WHERE` | ✅ Supported |
| `SHOW DATABASES/TABLES/COLUMNS` | ✅ Supported |

**Not a Doris replacement**: RorisDB is designed for learning Doris SQL syntax and OLAP concepts locally. For production workloads, use [Apache Doris](https://doris.apache.org).

## Relationship to Apache Doris

RorisDB is an **independent open-source project**. It is not a fork, wrapper, or derivative of Apache Doris. It reimplements similar OLAP concepts (columnar storage, MySQL compatibility, Doris SQL dialect) in Rust, with its own query engine (DataFusion) and storage layer (Parquet).

We deeply respect the Apache Doris community and their pioneering work in real-time OLAP.

## Building from Source

```bash
# Prerequisites: Rust 2024 edition (rustup update)
git clone https://github.com/walker83/RorisDB.git
cd RorisDB

cargo build --release
# Binary: target/release/roris-fe

# Run tests
cargo test --workspace --exclude fe-catalog
```

## License

Apache License 2.0. See [LICENSE](LICENSE).

## Contributing

Issues and pull requests are welcome. Please open an issue first to discuss major changes.
