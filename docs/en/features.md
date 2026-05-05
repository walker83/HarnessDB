# RorisDB Features

This document provides a detailed overview of the features currently supported by RorisDB.

## Version Information

- **Current Version**: v0.1.3
- **Project Status**: Proof-of-Concept
- **License**: MIT / Apache-2.0

## Completed Features (v0.1.3)

### SQL Parsing and Planning

| Feature | Status | Description |
|---------|--------|-------------|
| **SQL Parser** | ✅ | MySQL-compatible SQL parsing (via sqlparser crate) |
| **Query Planner** | ✅ | AST → Logical Plan → Physical Plan, rule-based optimization |
| **Optimizer** | ✅ | Predicate pushdown, column pruning, limit pushdown, join reordering |

### Expression Engine

| Feature | Status | Description |
|---------|--------|-------------|
| **Vectorized Expressions** | ✅ | Batch evaluation for improved CPU cache utilization |
| **Scalar Functions** | ✅ | 30+ built-in functions (math, string, date, etc.) |
| **Type Conversion** | ✅ | Implicit and explicit type conversion |

### Aggregate Functions

| Function | Status | Description |
|----------|--------|-------------|
| `COUNT` | ✅ | Counting (supports `COUNT(*)` and `COUNT(DISTINCT)`) |
| `SUM` | ✅ | Summation |
| `AVG` | ✅ | Average |
| `MIN` | ✅ | Minimum value |
| `MAX` | ✅ | Maximum value |
| `GROUP_CONCAT` | ✅ | String concatenation |

### Window Functions

| Function | Status | Description |
|----------|--------|-------------|
| `ROW_NUMBER` | ✅ | Row number (no ties) |
| `RANK` | ✅ | Ranking (with ties, gaps in numbering) |
| `DENSE_RANK` | ✅ | Dense ranking (with ties, no gaps) |
| `LAG` | ✅ | Access previous n rows |
| `LEAD` | ✅ | Access next n rows |

### Math Functions

| Function | Status | Description |
|----------|--------|-------------|
| Basic Math | ✅ | sin, cos, tan, asin, acos, atan |
| Exponential & Logarithmic | ✅ | exp, log, log10, sqrt, pow |
| Others | ✅ | pi, rand, abs, ceil, floor, round, sign |

### Data Types

| Type | Status | Description |
|------|--------|-------------|
| **Integer Types** | ✅ | Int8, Int16, Int32, Int64 |
| **Floating-Point Types** | ✅ | Float32, Float64 |
| **String Type** | ✅ | String (VARCHAR) |
| **Date/Time** | ✅ | Date, DateTime |
| **Boolean Type** | ✅ | Boolean |
| **Null Values** | ✅ | Null (tracked via Null Bitmap) |

### Storage Engine

| Feature | Status | Description |
|---------|--------|-------------|
| **Vectorized Storage** | ✅ | Columnar memory layout supporting multiple data types |
| **Null Bitmap** | ✅ | Bitset-based null tracking with fast AND/OR/NOT operations |
| **Block** | ✅ | Batch columnar data (schema + vectors), supports projection/filter/slice |
| **Tablet** | ✅ | Basic unit of data sharding |
| **Rowset** | ✅ | Data collection from a single import or compaction |
| **Segment** | ✅ | Columnar storage file containing multiple Column Pages |
| **ZoneMap Index** | ✅ | Records min/max values per column for range filtering |
| **BloomFilter Index** | ✅ | Probabilistic filter for high-cardinality column filtering |
| **LZ4 Compression** | ✅ | Lightweight compression algorithm |
| **RLE Encoding** | ✅ | Run-length encoding for repeated values |

### Compaction

| Feature | Status | Description |
|---------|--------|-------------|
| **Cumulative Compaction** | ✅ | Small file merging for rapid latest data consolidation |
| **Base Compaction** | ✅ | Large file merging for optimized query performance |
| **Priority Scheduling** | ✅ | Priority queue-based compaction scheduling |

### Query Execution

| Feature | Status | Description |
|---------|--------|-------------|
| **Pipeline Execution** | ✅ | Pipeline execution engine |
| **Vectorized Execution** | ✅ | Batch data processing for improved performance |
| **Scan Operator** | ✅ | Table data scanning |
| **Filter Operator** | ✅ | Data filtering |
| **Project Operator** | ✅ | Column projection |
| **Aggregate Operator** | ✅ | Aggregation computation (HashAggregate) |
| **Join Operator** | ✅ | Join operations (Hash Join, Nested Loop Join) |
| **Exchange Operator** | ✅ | Data exchange (HashPartition, Broadcast, Gather) |

### Subqueries and Set Operations

| Feature | Status | Description |
|---------|--------|-------------|
| **IN Subquery** | ✅ | `WHERE col IN (SELECT ...)` |
| **EXISTS Subquery** | ✅ | `WHERE EXISTS (SELECT ...)` |
| **NOT IN** | ✅ | `WHERE col NOT IN (SELECT ...)` |
| **NOT EXISTS** | ✅ | `WHERE NOT EXISTS (SELECT ...)` |
| **SemiJoin/AntiSemiJoin** | ✅ | Optimized subquery execution |
| **UNION** | ✅ | Set union (deduplicated) |
| **UNION ALL** | ✅ | Set union (preserving duplicates) |
| **INTERSECT** | ✅ | Set intersection |
| **EXCEPT** | ✅ | Set difference |

### CTE and Views

| Feature | Status | Description |
|---------|--------|-------------|
| **CTE (WITH)** | ✅ | Common Table Expressions, including recursive CTEs |
| **CREATE VIEW** | ✅ | View creation and metadata management |
| **SHOW CREATE TABLE** | ✅ | View table creation statements |

### Data Import and Export

| Feature | Status | Description |
|---------|--------|-------------|
| **CSV Read/Write** | ✅ | CSV format import and export |
| **JSON Lines Parsing** | ✅ | JSON Lines format parsing |
| **Stream Load** | ✅ | HTTP streaming import framework |

### Network Protocol

| Feature | Status | Description |
|---------|--------|-------------|
| **MySQL Protocol** | ✅ | MySQL wire protocol server (handshake, authentication, query, result set) |
| **gRPC FE-BE** | ✅ | gRPC communication between FE and BE (tonic/prost) |

### Distributed Query

| Feature | Status | Description |
|---------|--------|-------------|
| **Fragment Planning** | ✅ | Splits physical plans into distributable Fragments |
| **Distributed Scheduling** | ✅ | Load-aware BE node selection, round-robin allocation |
| **Query Coordinator** | ✅ | Complete query lifecycle management (plan → fragment → schedule → execute → collect) |
| **Failure Rescheduling** | ✅ | Query rescheduling on failure |

### Cluster Management

| Feature | Status | Description |
|---------|--------|-------------|
| **BE Node Registration** | ✅ | BE registers with FE on startup |
| **Heartbeat Mechanism** | ✅ | BE periodically sends heartbeats to FE (including load info) |
| **Load Tracking** | ✅ | FE tracks load score for each BE node |

### Client Tools

| Feature | Status | Description |
|---------|--------|-------------|
| **roris-cli** | ✅ | Command-line client (REPL) with SQL parsing and plan visualization |
| **MySQL Client Compatible** | ✅ | Can connect directly using mysql command-line tool |

### DDL and DML

| Feature | Status | Description |
|---------|--------|-------------|
| **CREATE DATABASE** | ✅ | Create database |
| **DROP DATABASE** | ✅ | Drop database |
| **ALTER DATABASE** | ✅ | Alter database properties |
| **SHOW CREATE DATABASE** | ✅ | Show database creation statement |
| **CREATE TABLE** | ✅ | Create table (supports DUPLICATE KEY, partition tables) |
| **ALTER TABLE** | ✅ | Alter table (RENAME COLUMN, COMMENT, SET PROPERTY) |
| **DROP TABLE** | ✅ | Drop table |
| **TRUNCATE TABLE** | ✅ | Quick table truncation |
| **INSERT** | ✅ | Insert data (single and multiple rows) |
| **SELECT** | ✅ | Query data (supports complex queries) |
| **CREATE VIEW** | ✅ | Create view |
| **DROP VIEW** | ✅ | Drop view |
| **ALTER VIEW** | ✅ | Alter view definition |
| **SHOW CREATE VIEW** | ✅ | Show view creation statement |

### Partition Support

| Feature | Status | Description |
|---------|--------|-------------|
| **Range Partition** | ✅ | Partition by range of values |
| **List Partition** | ✅ | Partition by list of values |
| **Hash Partition** | ✅ | Partition by hash of values |
| **Partition Management** | ✅ | Add/drop partitions dynamically |

### Materialized Views

| Feature | Status | Description |
|---------|--------|-------------|
| **MV Framework** | ✅ | Materialized view creation and metadata management |
| **Query Rewrite** | ✅ | Transparent query rewrite using materialized views |
| **MV Maintenance** | 🚧 | Automatic refresh and consistency maintenance |

### CBO Optimizer

| Feature | Status | Description |
|---------|--------|-------------|
| **Cost Model** | ✅ | Cost-based optimization with CPU/I/O estimation |
| **Statistics Collection** | ✅ | Table statistics via ANALYZE TABLE |
| **Join Reordering** | ✅ | Cost-based join order optimization |
| **Plan Selection** | ✅ | Optimal plan selection based on statistics |

### Runtime Filter

| Feature | Status | Description |
|---------|--------|-------------|
| **Runtime Filter Pushdown** | ✅ | Dynamic filter pushdown for join optimization |
| **Bloom Filter** | ✅ | Runtime Bloom filters for selective joins |
| **Filter Propagation** | ✅ | Cross-fragment filter propagation |

### External Catalog

| Feature | Status | Description |
|---------|--------|-------------|
| **Catalog Framework** | ✅ | External catalog framework (Hive/Iceberg/Hudi) |
| **Federation Queries** | 🚧 | Query external data sources directly |
| **Metadata Sync** | 🚧 | Catalog metadata synchronization |

### Authentication Framework

| Feature | Status | Description |
|---------|--------|-------------|
| **MySQL Native Password** | ✅ | MySQL native password authentication |
| **LDAP Authentication** | ✅ | External LDAP authentication support |
| **Token Authentication** | ✅ | Token-based authentication |
| **Pluggable Auth** | ✅ | Pluggable authentication framework |

### Backup & Restore

| Feature | Status | Description |
|---------|--------|-------------|
| **Backup Framework** | ✅ | Backup and restore framework |
| **Incremental Backup** | 🚧 | Incremental backup support |
| **Remote Storage** | 🚧 | Backup to S3/GCS remote storage |

### Codec & Compression

| Feature | Status | Description |
|---------|--------|-------------|
| **LZ4 Compression** | ✅ | Improved LZ4 compression with optimizations |
| **Codec Framework** | ✅ | Extensible codec framework |
| **External File Scan** | ✅ | Direct scanning of external files (CSV, JSON) |

## Features In Progress

| Feature | Status | Description |
|---------|--------|-------------|
| **Materialized Views** | 🚧 | Transparent query rewriting (Materialized Views) |
| **HA High Availability** | 🚧 | Raft-based FE metadata replication |
| **Catalog Persistence** | 🚧 | EditLog + BDBJE-style metadata persistence |
| **Federated Query** | 🚧 | Hive/Iceberg/Hudi external catalogs |
| **Cloud-Native Mode** | 🚧 | S3 shared storage, metadata service |

## Not Yet Implemented

| Feature | Description |
|---------|-------------|
| **UDF / UDAF** | User-defined functions and aggregate functions |
| **Multi-Database Transactions** | Cross-database transaction support |
| **Row-Level Security** | Row-level permission control |
| **Workload Management** | Query resource isolation and prioritization |
| **TPC-H End-to-End** | Complete TPC-H benchmark testing |
| **Kubernetes Operator** | K8s deployment and management tool |
| **UPDATE / DELETE** | Data update and delete operations |
| **Foreign Key Constraints** | Inter-table foreign key constraints |
| **Stored Procedures** | Stored procedure support |
| **Triggers** | Trigger support |

## Feature Comparison with Apache Doris

| Feature Category | Apache Doris | RorisDB | Notes |
|------------------|--------------|---------|-------|
| **Language** | C++ | Rust | Memory safety |
| **SQL Compatibility** | MySQL | MySQL | Via mysql-protocol |
| **Storage Format** | Tablet/Rowset/Segment | Tablet/Rowset/Segment | Similar design |
| **Indexes** | ZoneMap, BloomFilter, Inverted | ZoneMap, BloomFilter | RorisDB adds Inverted |
| **Compression Algorithms** | zstd, LZ4, Zlib | LZ4 | More codecs planned |
| **Execution Model** | Vectorized + Pipeline | Vectorized + Pipeline | Same philosophy |
| **Compaction** | Cumulative + Base | Cumulative + Base | Same strategy |
| **High Availability** | BDBJE Master/Follower | Raft (planned) | Different consensus mechanisms |
| **Cloud Mode** | Shared-nothing + S3 | Shared-nothing + S3 (planned) | |
| **Materialized Views** | ✅ | 🚧 | Planned |
| **Federated Query** | ✅ Hive/Iceberg/Hudi | 🚧 | Planned |
| **Transactions** | ✅ | ❌ | Planned |

## Performance Features

### Vectorized Execution

RorisDB uses a vectorized execution model for batch data processing:

- **Batch Size**: Default 1024 rows per batch
- **CPU Cache Friendly**: Contiguous memory layout reduces cache misses
- **SIMD Friendly**: Tight loops enable compiler optimizations

### Zero-Copy

- Uses Rust's borrowing mechanism to avoid unnecessary data copies
- Uses references instead of ownership transfers where possible

### Late Materialization

- Materializes data only when necessary
- Filters data early to reduce the amount of data for subsequent processing

### Index Optimization

- **ZoneMap**: Quickly skips Segments that don't satisfy range conditions
- **BloomFilter**: Quickly determines if a value exists in a Segment
- **Column Pruning**: Only reads columns needed by the query

## Scalability

### Horizontal Scaling

- Storage and compute capacity can be horizontally scaled by adding BE nodes
- FE handles query planning and scheduling; BE handles data storage and execution

### Distributed Query

- Fragment-level parallel execution
- Supports multiple data exchange modes:
  - **HashPartition**: Partition by hash
  - **Broadcast**: Broadcast small tables
  - **Gather**: Collect results to a single node

## Next Steps

- Check the [Product Overview](product-overview.md) to understand RorisDB's positioning
- Read the [Architecture Design Document](architecture.md) to learn about the system architecture
- Refer to the [SQL Reference Manual](sql-reference.md) to learn SQL syntax
- Check the [Developer Guide](developer-guide.md) to contribute to the project
