# HarnessDB Architecture

> Version 0.3.0 | Single-node OLAP with DataFusion + Parquet

## Overview

HarnessDB is a **single-node OLAP database** with a layered architecture:

```
MySQL Client
    │
    ▼
┌─────────────────────┐
│   mysql-protocol     │  Wire protocol, auth, packet I/O
├─────────────────────┤
│   harness-server       │  Query routing, DDL/DML handlers
├─────────────────────┤
│   fe-sql-parser      │  SQL text → AST
│   fe-catalog         │  Metadata management
│   fe-datafusion      │  UDFs, type conversion
│   fe-storage         │  Parquet I/O, TableProvider
│   fe-monitor         │  Audit log
├─────────────────────┤
│   DataFusion 48      │  Query engine (optimizer + executor)
│   Arrow 55           │  Columnar in-memory format
│   Parquet 55         │  Columnar on-disk format
└─────────────────────┘
```

## Crate Dependency Graph

```
harness-server (binary)
├── fe-sql-parser
├── fe-catalog
│   ├── fe-common
│   │   └── common
│   ├── be-rocks (optional)
│   └── types
├── fe-datafusion
│   ├── types
│   └── fe-catalog
├── fe-storage
│   ├── fe-catalog
│   └── fe-datafusion
├── fe-monitor
│   └── types
├── mysql-protocol
│   └── types
├── common
└── types
```

## Query Execution Paths

### SELECT (DataFusion Path)

```
SQL → Parser → AST → DataFusion SessionContext
    → LogicalPlan → OptimizedPlan → ExecutionPlan
    → ParquetTableProvider.scan()
        → read_with_options(projection, limit)
        → apply_filters (pushdown)
    → RecordBatch → MySQL result set
```

Key characteristics:
- DataFusion handles all optimization (predicate pushdown, projection, join reordering)
- `ParquetTableProvider` implements DataFusion's `TableProvider` trait
- Filter pushdown: simple `column op literal` filters applied at Parquet read level
- Projection pushdown: only requested columns read from disk
- Returns `MemorySourceConfig` wrapping filtered `RecordBatch`

### INSERT (Direct Write Path)

```
SQL → Parser → InsertStmt
    → Build Arrow arrays directly from Expr (no string intermediate)
    → ParquetStorage.insert()
        → read existing data.parquet
        → concat_batches(existing, new)
        → write_parquet_atomic (temp + fsync + rename)
```

### UPDATE/DELETE (Read-Modify-Write)

```
SQL → Parser → UpdateStmt/DeleteStmt
    → ParquetStorage.update/delete()
        → read existing data.parquet
        → evaluate_where_filter() — recursive AND/OR
        → apply changes to RecordBatch (typed Arrow compute)
        → write_parquet_atomic
```

## Storage Layout

```
data/
└── {database}/
    └── {table}/
        └── data.parquet    ← single file, ZSTD compressed
```

- **Atomic writes**: Write to `.tmp_data.parquet` → `fsync` → `rename`
- **Compression**: ZSTD with page-level statistics
- **Schema**: Embedded in Parquet file metadata (Arrow schema)

## Metadata

### Catalog (`fe-catalog`)

- **Default backend**: JSON file (`catalog.json`)
- **Optional backend**: RocksDB (`be-rocks`)
- Stores: databases, tables, columns, partitions, views, materialized views

### Table Metadata

```rust
struct Table {
    id: u64,
    tablet_id: u64,
    name: String,
    columns: Vec<TableColumn>,  // name, data_type, nullable, default
    keys_type: KeysType,        // Duplicate, Aggregate, Unique, Primary
    partition_info: Option<PartitionInfo>,
    distribution_info: Option<DistributionInfo>,
    // ...
}
```

## MySQL Protocol (`mysql-protocol`)

Full MySQL wire protocol implementation:

- **Handshake**: Server greeting with capability negotiation
- **Auth**: `mysql_native_password` (SHA1-based challenge-response)
- **Commands**: `COM_QUERY`, `COM_INIT_DB`, `COM_FIELD_LIST`, `COM_QUIT`
- **Result sets**: Column definitions + row data in MySQL text protocol

## Monitoring (`fe-monitor`)

- **Audit log**: Query audit log with slow query tracking

## Data Type Mapping

| HarnessDB Type | Arrow Type | Parquet Type |
|-------------|-----------|-------------|
| Boolean | Boolean | BOOLEAN |
| Int8/16/32/64 | Int8/16/32/64 | INT32/INT64 |
| UInt8/16/32/64 | UInt8/16/32/64 | INT32/INT64 |
| Float32/64 | Float32/64 | FLOAT/DOUBLE |
| Decimal(p,s) | Decimal128(p,s) | FIXED_LEN_BYTE_ARRAY |
| String/Varchar/Char | Utf8 | BYTE_ARRAY (UTF8) |
| Date | Date32 | INT32 (DATE) |
| DateTime | Timestamp(Second) | INT64 (TIMESTAMP) |
| Binary | Binary | BYTE_ARRAY |
| Array(T) | List(T) | LIST |
| Map(K,V) | Map(Struct(K,V)) | MAP |
| Struct | Struct | STRUCT |
| Json | Utf8 | BYTE_ARRAY (UTF8) |

## Performance Characteristics

| Operation | Complexity | Notes |
|-----------|-----------|-------|
| SELECT (full scan) | O(N) | Reads all rows, but projection reduces I/O |
| SELECT (with filter) | O(N) | Filter pushdown reduces materialized rows |
| INSERT | O(N) | Reads existing + concat + rewrite |
| UPDATE | O(N) | Read-modify-write |
| DELETE | O(N) | Read-modify-write |
| CREATE TABLE | O(1) | Creates directory + empty Parquet |
| DROP TABLE | O(1) | Removes directory |

The INSERT/UPDATE/DELETE O(N) cost is the primary architectural limitation. Multi-segment storage with append writes would make these O(1).
