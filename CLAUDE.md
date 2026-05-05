
一旦你完成了docs/roadmap以后，你就可以吧对应的文件挪到docs/roadmap/done里，这样就清楚哪些任务未完成
每次用户发起新的大的任务请求钱，尽量先git commit

```bash
cargo build --release
```

Builds two binaries: `target/release/roris-fe` (Frontend) and `target/release/roris-be` (Backend).

## Run

```bash
./target/release/roris-fe --http-port 8030 --rpc-port 9020
./target/release/roris-be --http-port 8060 --rpc-port 9060
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

RorisDB is an OLAP database inspired by Apache Doris, implemented in Rust with an MPP (Massively Parallel Processing) architecture.

### Frontend (FE) - Query Processing
- **fe-sql-parser** - MySQL-compatible SQL parsing via `sqlparser` crate → AST
- **fe-sql-planner** - AST → Logical Plan → Physical Plan with rule-based optimization
- **fe-catalog** - Database/Table/Partition metadata management
- **fe-scheduler** - Fragment planning and distributed query scheduling across BE nodes
- **fe-expression** - Vectorized expression evaluation (30+ scalar functions, aggregates, window functions)
- **fe-common** - Shared FE utilities (EditLog, MetaService)
- **fe-scheduler** - Load-aware BE node selection, round-robin assignment, failure re-schedule
- **mysql-protocol** - MySQL wire protocol server (handshake, auth, COM_QUERY, result sets)

### Backend (BE) - Storage & Execution
- **be-storage** - Tablet → Rowset → Segment storage hierarchy with compaction
- **be-segment** - Columnar segment format (LZ4 compression, RLE encoding, ZoneMap/BloomFilter indexes)
- **be-execution** - Async pipeline execution engine with non-blocking operators
- **be-common** - BE shared utilities (config, metrics, memory tracking)

### Shared
- **types** - Vector, Bitmap, Block, DataType, Schema (columnar memory layout with null bitmaps)
- **common** - Error handling, configuration
- **rpc** - gRPC service implementations (tonic/prost)
- **proto** - gRPC protocol definitions
- **data-io** - CSV/JSON import, Stream Load framework

### Query Flow
1. MySQL protocol receives SQL
2. Parser generates AST
3. Planner creates Logical Plan → Physical Plan (with optimizations: predicate pushdown, column pruning, limit pushdown, join reordering)
4. Scheduler fragments the plan and distributes across BE nodes
5. BE executes via async pipeline with vectorized operations
6. Results collected and returned through gRPC

### Storage Hierarchy
- **Tablet** - Logical unit of data (like a partition)
- **Rowset** - Set of rows stored together
- **Segment** - Columnar file format with pages, ZoneMap index, BloomFilter, LZ4 compression
