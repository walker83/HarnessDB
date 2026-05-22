# Changelog

All notable changes to RorisDB are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.3.0] — 2026-05-23

### Added
- **481 E2E integration tests** across 14 test suites (aggregate, datetime, DDL, DML, Doris compat, Doris syntax, edge cases, joins, math, null types, select basics, strings, subqueries, window functions)
- `INSERT INTO ... SELECT` execution support
- WHERE evaluation delegated to DataFusion (replaced custom evaluator)
- Custom MySQL-compatible `SUBSTRING` UDF (1-based indexing, negative position, optional length)

### Changed
- **DataFusion upgraded from 47 to 48** (ScalarUDFImpl, AggregateUDFImpl trait changes, Expr::Literal metadata field)
- Startup simplified from 4 CLI port args to 1: `./target/release/roris-fe` (only MySQL port 9030)
- Removed dead CLI args: `--http-port`, `--rpc-port`, `--metrics-port`, `--config`
- Removed fe-monitor HTTP server (axum), Prometheus metrics, query profiles, information_schema
- Removed unused config files: `conf/fe.conf`, `conf/be.conf`
- Removed axum and prometheus from dependencies (−1,833 lines)
- `SHOW FRONTENDS` port updated from hardcoded `8030` to `9030`

### Fixed
- **DATE/DATETIME NULL**: parser produces `LiteralValue::String` for date strings, but Arrow builders only handled `LiteralValue::Date` — added string-to-date conversion
- **SUBSTRING NULL**: DataFusion 47+ returns `Utf8View` (StringViewArray), not `Utf8` — result converter now handles Utf8View/LargeUtf8
- **Window function NULL**: `ROW_NUMBER` returns UInt64, `LAG` returns Utf8View — added UInt and Utf8View support to `arrow_value_to_string`
- **UPDATE expressions**: `SET col = col + 1` style expressions now evaluated per-row via DataFusion MemTable
- `ArrayFormatter` fallback for unknown Arrow types in result conversion
- Integration test expectations corrected for aggregates, null types, and subqueries

## [0.2.0] — 2026-05-21

### Added
- **DataFusion query engine** integration — SELECT queries executed via DataFusion SessionContext
- **Parquet storage** — one Parquet file per table (`data/{db}/{table}/data.parquet`), ZSTD compressed
- DataFusion `TableProvider` with filter + projection + limit pushdown
- Arrow type conversion: full Roris ↔ Arrow mapping (Boolean, Int8-64, UInt8-64, Float, Decimal, Date, DateTime, String, Array, Map, Struct)
- Doris UDFs: `date_trunc`, `months_add`, `days_add`, `hours_add`, `concat_ws`, `substring_index`, `bitmap_count`
- Aggregate UDFs: `count_distinct`, `group_concat`

### Changed
- INSERT rewritten: Expr → Arrow Array (direct, no string intermediate) → read existing Parquet + concat + atomic write
- UPDATE/DELETE: read → evaluate WHERE filter (recursive AND/OR) → modify batch → atomic write
- Type system migrated from custom `types::Block` to native Arrow arrays
- Storage backend replaced custom BE with DataFusion + Parquet

### Fixed
- INSERT NULL values: NULL literals now produce correct null Arrow values
- DELETE inverted filter: WHERE clause logic corrected
- Double projection: column projection applied only once
- Parser `unwrap()` panics: replaced with proper error handling
- RwLock poisoning on panic: switched to `parking_lot::RwLock`

### Removed
- BE storage crates (be-storage, be-common) — replaced by fe-storage (DataFusion + Parquet)
- 5,500+ lines of dead code across the codebase

## [0.1.5] — 2026-05-21

### Changed
- Massive codebase cleanup: removed 5,500+ lines of dead code
- Fixed critical bugs in DML execution path

### Fixed
- Parser `unwrap()` calls replaced with proper error propagation
- RwLock poisoning issues resolved
- DualWriteBackend removed (no longer needed)
- fe_main.rs modularized into handler files

## [0.1.0 — 0.1.4] — 2026-05

### Added
- Initial RorisDB development
- **fe-sql-parser**: MySQL-compatible SQL parsing via sqlparser with Doris extensions (DUPLICATE KEY, DISTRIBUTED BY HASH, PROPERTIES)
- **fe-catalog**: Database/table metadata management (JSON + RocksDB backends)
- **mysql-protocol**: MySQL wire protocol server (handshake, `mysql_native_password` auth, COM_QUERY, COM_INIT_DB)
- **types**: DataType, Field, Schema, Vector, Bitmap, Block (columnar memory layout)
- **common**: Error handling (DrorisError), configuration
- **fe-common**: EditLog for metadata persistence
- DDL support: CREATE/DROP DATABASE, CREATE/DROP TABLE, ALTER TABLE, TRUNCATE TABLE
- SHOW commands: SHOW DATABASES, SHOW TABLES, SHOW COLUMNS, SHOW CREATE TABLE, SHOW FRONTENDS

[0.3.0]: https://github.com/walker83/RorisDB/releases/tag/v0.3.0
[0.2.0]: https://github.com/walker83/RorisDB/releases/tag/v0.2.0
[0.1.5]: https://github.com/walker83/RorisDB/releases/tag/v0.1.5
