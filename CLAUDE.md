
一旦你完成了docs/roadmap以后，你就可以吧对应的文件挪到docs/roadmap/done里，这样就清楚哪些任务未完成
每次用户发起新的大的任务请求钱，尽量先git commit

## Version

- **Current**: 0.3.0
- **Repository**: https://github.com/walker83/RorisDB

## Build

```bash
cargo build --release
```

Builds binary: `target/release/roris-fe`.

## Run

```bash
# Default: MySQL port 9030, data dir data/fe/storage
./target/release/roris-fe

# Custom port and data directory
./target/release/roris-fe --mysql-port 3306 --data-dir /path/to/data
```

Connect via MySQL client: `mysql -h 127.0.0.1 -P 9030 -uroot`

## Test

```bash
# Run all tests
cargo test --workspace

# Run integration tests only
cargo test -p integration-tests

# Run a specific test
cargo test -p integration-tests -- <test_name>
```

## Architecture

RorisDB is a single-node OLAP database using DataFusion as the query engine with Parquet storage.

### Frontend (FE) - Query Processing
- **fe-sql-parser** - MySQL-compatible SQL parsing via `sqlparser` crate → AST
- **fe-catalog** - Database/Table metadata management (JSON + RocksDB backends)
- **fe-storage** - Parquet storage layer (DataFusion TableProvider, filter/projection pushdown)
- **fe-datafusion** - Type conversion (Roris ↔ Arrow), UDFs, Block↔Arrow conversion
- **fe-common** - Shared FE utilities (EditLog)
- **fe-monitor** - Audit log
- **mysql-protocol** - MySQL wire protocol server (handshake, auth, COM_QUERY)

### Metadata
- **be-rocks** - RocksDB-based metadata store (optional, used by fe-catalog)

### Shared
- **types** - DataType, Field, Schema, Vector, Bitmap, Block (columnar memory layout)
- **common** - Error handling (DrorisError), configuration

### Query Flow
1. MySQL protocol receives SQL
2. Parser generates AST
3. DDL handled by catalog directly; DML dispatched to storage
4. SELECT queries go through DataFusion SessionContext → ParquetTableProvider → Parquet files
5. INSERT: Expr → Arrow Array (direct, no string intermediate) → read existing Parquet + concat + atomic write
6. UPDATE/DELETE: read → evaluate_where_filter (recursive AND/OR) → modify batch → atomic write

### Storage Layout
- `data/{database}/{table}/data.parquet` — one Parquet file per table
- Atomic writes via temp file + fsync + rename
- ZSTD compression with page-level statistics

### Tech Stack
- DataFusion 48, Arrow 55, Parquet 55
- sqlparser 0.53, Tokio 1.x
- RocksDB 0.23

### Crate Count: 11
