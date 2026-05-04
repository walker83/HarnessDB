# RorisDB

> **RorisDB** is a real-time OLAP database reimagined in Rust. It is architecturally inspired by Apache Doris — adopting its proven MPP architecture, columnar storage, and materialized view design — while rebuilt from the ground up in Rust for memory safety, zero-cost abstractions, and fine-grained resource control.

[![MIT/Apache-2.0 License](https://img.shields.io/badge/License-MIT%2FApache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.75+-orange.svg)](https://www.rust-lang.org)
[![Status](https://img.shields.io/badge/Status-Proof--of--Concept-yellow.svg)]()

## Naming

**RorisDB** = **R**ust + (D)**oris** + **DB**

GitHub topics: `rust`, `olap`, `analytical-database`, `columnar-storage`, `mpp`, `data-warehouse`, `real-time-analytics`, `doris-inspired`

## Why RorisDB?

Doris is battle-tested at scale (C++). RorisDB explores what the same architectural philosophy looks like when expressed in Rust — targeting:

- **Lower tail latency** through ownership-based memory management and zero-copy data paths
- **Safer concurrency** with Rust's borrow checker preventing data races at compile time
- **Cloud-native deployment** with no native memory unsafe code in hot paths
- **Fine-grained resource control** with async I/O via Tokio and configurable memory limits

## Relationship to Apache Doris

RorisDB is an independent open-source project. It is **not** a fork, wrapper, or derivative of Apache Doris. It reimplements similar OLAP concepts in Rust with its own query engine and storage layer. We deeply respect the Doris community and their pioneering work.

## Architecture

RorisDB follows the same proven MPP (Massively Parallel Processing) architecture as Doris:

```
┌─────────────────────────────────────────────────────────────────┐
│                        RorisDB Cluster                            │
│                                                                 │
│   ┌──────────────┐                        ┌──────────────────┐    │
│   │  FE (Rust)  │◄────── RPC ─────────►│   BE 1 (Rust)   │    │
│   │              │     (gRPC)            │                  │    │
│   │  ┌────────┐ │                        │  ┌────────────┐  │    │
│   │  │ Parser │ │                        │  │ Storage   │  │    │
│   │  │Planner │ │                        │  │ Engine    │  │    │
│   │  │Scheduler│ │                        │  │ (Segment) │  │    │
│   │  │Catalog │ │                        │  └────────────┘  │    │
│   │  └────────┘ │                        │  ┌────────────┐  │    │
│   │              │                        │  │ Execution  │  │    │
│   │  MySQL      │                        │  │ Pipeline   │  │    │
│   │  Protocol   │                        │  └────────────┘  │    │
│   └──────────────┘                        └──────────────────┘    │
│          │                                        │               │
│          │             ┌──────────────────┐    │               │
│          └───────────►│   BE 2 (Rust)   │◄───┘               │
│                         └──────────────────┘                   │
└─────────────────────────────────────────────────────────────────┘
```

## Features

### Completed

| Feature | Status | Description |
|---------|--------|-------------|
| **SQL Parser** | ✅ | MySQL-compatible SQL parsing via `sqlparser` crate |
| **Query Planner** | ✅ | AST → Logical Plan → Physical Plan with rule-based optimization |
| **Optimizer** | ✅ | Predicate pushdown, column pruning, limit pushdown, join reordering |
| **Expression Engine** | ✅ | Vectorized batch evaluation, 30+ built-in scalar functions |
| **Aggregate Functions** | ✅ | COUNT, SUM, AVG, MIN, MAX, COUNT DISTINCT, GROUP_CONCAT |
| **Window Functions** | ✅ | ROW_NUMBER, RANK, DENSE_RANK, LAG, LEAD |
| **Math Functions** | ✅ | sin, cos, tan, asin, acos, log, log10, exp, sqrt, pow, pi, rand |
| **Vectorized Storage** | ✅ | Columnar memory layout with typed vectors (Int64, Float64, String, Date...) |
| **Null Bitmap** | ✅ | Bit-set null tracking with fast AND/OR/NOT operations |
| **Block** | ✅ | Batch columnar data (schema + vectors) with projection/filter/slice |
| **Storage Engine** | ✅ | Tablet → Rowset → Segment file format with memtable buffering |
| **Segment Format** | ✅ | Column-oriented pages, ZoneMap index, LZ4 compression, RLE encoding |
| **BloomFilter Index** | ✅ | Probabilistic filter for high-cardinality column pruning |
| **Compaction** | ✅ | Cumulative + Base compaction with priority queue scheduler |
| **MySQL Protocol** | ✅ | MySQL wire protocol server (handshake, auth, COM_QUERY, result sets) |
| **CTE (WITH)** | ✅ | Common Table Expressions with recursive support |
| **UNION/INTERSECT/EXCEPT** | ✅ | Set operations on queries |
| **Subqueries** | ✅ | IN, EXISTS, NOT IN, NOT EXISTS with SemiJoin/AntiSemiJoin |
| **TRUNCATE TABLE** | ✅ | Fast table truncation |
| **CREATE VIEW** | ✅ | View creation and metadata |
| **SHOW CREATE TABLE** | ✅ | DDLShowCreate result |
| **gRPC FE-BE** | ✅ | Backend service communication via tonic/prost |
| **Distributed Query** | ✅ | Fragment planning, exchange operators (HashPartition/Broadcast/Gather) |
| **Query Scheduler** | ✅ | Load-aware BE node selection, round-robin assignment, failure re-schedule |
| **Query Coordinator** | ✅ | Full query lifecycle (plan → fragment → schedule → execute → collect) |
| **Cluster Manager** | ✅ | BE node registration, heartbeat, load score tracking |
| **CLI Client** | ✅ | REPL with SQL parsing and plan visualization |
| **Data Import** | ✅ | CSV reader/writer, JSON Lines parser, Stream Load framework |
| **Data Export** | ✅ | CSV writer from query results |

### In Progress

| Feature | Status |
|---------|--------|
| **Materialized Views** | 🚧 Transparent query rewrite |
| **HA Consensus** | 🚧 Raft-based FE metadata replication |
| **Catalog Persistence** | 🚧 EditLog + BDBJE-style durability |
| **Federation Queries** | 🚧 Hive/Iceberg/Hudi external catalog |
| **Cloud Mode** | 🚧 S3 shared storage, meta service |

### Not Yet Implemented

| Feature |
|---------|
| UDF / UDAF |
| Multi-database transactions |
| Row-level security |
| Workload management |
| TPC-H end-to-end benchmarks |
| Kubernetes operator |

## Quick Start

### Build

```bash
cargo build --release
```

### Run FE

```bash
./target/release/roris-fe --http-port 8030 --rpc-port 9020
```

### Run BE

```bash
./target/release/roris-be --http-port 8060 --rpc-port 9060
```

### Connect via MySQL Client

```bash
mysql -h 127.0.0.1 -P 9030 -uroot
```

```sql
CREATE DATABASE IF NOT EXISTS test;
USE test;
CREATE TABLE user (
    id BIGINT PRIMARY KEY,
    name VARCHAR(64),
    age INT
) DUPLICATE KEY;

INSERT INTO user VALUES (1, 'Alice', 30), (2, 'Bob', 25);
SELECT * FROM user WHERE age > 20;
```

## Project Structure

```
rovisdb/
├── roris-server/          # FE and BE binary entry points
├── crates/
│   ├── fe-sql-parser/    # SQL parsing → AST
│   ├── fe-sql-planner/   # AST → Logical/Physical Plan → Optimization
│   ├── fe-catalog/      # Database/Table/Partition metadata
│   ├── fe-scheduler/     # Fragment planning, distributed scheduling
│   ├── fe-expression/    # Vectorized expression evaluation
│   ├── fe-common/        # FE shared (EditLog, MetaService)
│   ├── mysql-protocol/   # MySQL wire protocol server
│   ├── be-storage/       # Tablet, Rowset, Segment, Compaction
│   ├── be-execution/     # Pipeline execution engine
│   ├── be-segment/       # Columnar segment format
│   ├── be-common/        # BE shared (config, metrics)
│   ├── data-io/          # CSV/JSON import, Stream Load
│   ├── types/            # Vector, Bitmap, Block, DataType, Schema
│   ├── common/           # Error handling, config, utilities
│   ├── proto/            # RPC protocol definitions
│   └── rpc/              # gRPC service implementations
├── tools/
│   └── roris-cli/        # Command-line client
├── benches/
│   └── tpch/             # TPC-H benchmark suite
└── tests/
    └── integration/      # SQL and protocol integration tests
```

## Performance

RorisDB uses vectorized execution with columnar memory layout. Expression evaluation processes entire columns at once using typed vector operations:

```rust
// Vectorized comparison: processes 1M rows in one call
fn eval_binary(&self, left: &Vector, right: &Vector) -> Vector {
    match (left, right) {
        (Vector::Int64(l), Vector::Int64(r)) => {
            let result: Vec<bool> = l.data()
                .iter()
                .zip(r.data())
                .map(|(&a, &b)| a + b)  // SIMD-friendly tight loop
                .collect();
            Vector::Boolean(BooleanVector::from_vec(result))
        }
        // ...
    }
}
```

## Comparison with Apache Doris

| Feature | Apache Doris | RorisDB | Notes |
|---------|-------------|---------|-------|
| Language | C++ | Rust | Memory safety |
| SQL Compatibility | MySQL | MySQL | Via mysql-protocol |
| Storage | Tablet/Rowset/Segment | Tablet/Rowset/Segment | Analogous design |
| Indexes | ZoneMap, BloomFilter, Inverted | ZoneMap, BloomFilter | RorisDB adds Inverted |
| Compression | zstd, LZ4, Zlib | LZ4 | More codecs planned |
| Execution Model | Vectorized + Pipeline | Vectorized + Pipeline | Same philosophy |
| Compaction | Cumulative + Base | Cumulative + Base | Same strategy |
| HA | BDBJE Master/Follower | Raft (planned) | Different consensus |
| Cloud Mode | Shared-nothing + S3 | Shared-nothing + S3 (planned) | |
| Materialized Views | ✅ | 🚧 | Planned |
| Federation | ✅ Hive/Iceberg/Hudi | 🚧 | Planned |

## License

RorisDB is licensed under either of:
- [MIT License](LICENSE-MIT)
- [Apache License 2.0](LICENSE-APACHE)

at your option.

## Contributing

Contributions are welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.
