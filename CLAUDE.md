
一旦你完成了docs/roadmap以后，你就可以吧对应的文件挪到docs/roadmap/done里，这样就清楚哪些任务未完成
每次用户发起新的大的任务请求钱，尽量先git commit

```bash
cargo build --release
```

Builds binary: `target/release/roris-fe`.

## Run

```bash
./target/release/roris-fe --http-port 8030 --rpc-port 9020
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
- **fe-storage** - Parquet storage layer (DataFusion TableProvider, atomic read-modify-write)
- **fe-datafusion** - Type conversion, UDFs, Block↔Arrow conversion
- **fe-common** - Shared FE utilities (EditLog, MetaService)
- **fe-monitor** - HTTP monitoring server, metrics, audit log
- **mysql-protocol** - MySQL wire protocol server (handshake, auth, COM_QUERY, prepared statements)

### Metadata
- **be-rocks** - RocksDB-based metadata store (used by fe-catalog)

### Shared
- **types** - Vector, Bitmap, Block, DataType, Schema (columnar memory layout with null bitmaps)
- **common** - Error handling, configuration
- **rpc** - gRPC service implementations (tonic/prost)
- **proto** - gRPC protocol definitions
- **data-io** - CSV/JSON import, Stream Load framework

### Query Flow
1. MySQL protocol receives SQL
2. Parser generates AST
3. DDL handled by catalog directly; DML dispatched to storage
4. SELECT queries go through DataFusion SessionContext → ParquetTableProvider → Parquet files
5. INSERT: read existing Parquet + concat new rows + atomic write
6. UPDATE/DELETE: read-modify-write pattern on Parquet files

### Storage Layout
- `data/{database}/{table}/data.parquet` — one Parquet file per table
- Atomic writes via temp file + fsync + rename
- ZSTD compression with page-level statistics
