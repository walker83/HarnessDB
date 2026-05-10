# RorisDB 后端存储架构迁移文档

## 背景

RorisDB 后端存储原本使用自定义实现：
- **持久化格式**: 自定义 `.dat` segment 文件
- **内存格式**: 自定义 Vector 类型
- **元数据存储**: JSON 文件

迁移目标是切换到业界标准组件：
- **持久化格式** → Parquet
- **内存格式** → Arrow RecordBatch
- **元数据存储** → RocksDB

## 迁移架构

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           Query Path                                     │
│                                                                          │
│  MySQL → Parser → DataFusion → RorisTableProvider                       │
│                                    ↓                                     │
│                           StorageEngine                                  │
│                                    ↓                                     │
│               ┌─────────────────────────────────────┐                   │
│               │         Segment Format              │                   │
│               │  ┌─────────────┐  ┌─────────────┐  │                   │
│               │  │  Parquet    │  │  Legacy .dat │  │                   │
│               │  │  (新格式)    │  │  (旧格式)    │  │                   │
│               │  └─────────────┘  └─────────────┘  │                   │
│               └─────────────────────────────────────┘                   │
│                                    ↓                                     │
│                           RecordBatch                                    │
│                                    ↓                                     │
│                        DataFusion Execution                              │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│                           Write Path                                     │
│                                                                          │
│  INSERT Data → StorageEngine.write_batch_arrow(RecordBatch)             │
│                        ↓                                                 │
│                   Tablet.write_arrow()                                   │
│                        ↓                                                 │
│                   MemTable (Arrow RecordBatch)                           │
│                        ↓                                                 │
│                   flush_parquet()                                        │
│                        ↓                                                 │
│               ┌─────────────────────────────────────┐                   │
│               │         .parquet file               │                   │
│               │  - ZSTD compression                 │                   │
│               │  - Column statistics                │                   │
│               │  - Bloom filters                    │                   │
│               └─────────────────────────────────────┘                   │
│                        ↓                                                 │
│               ┌─────────────────────────────────────┐                   │
│               │         RocksDB Metadata            │                   │
│               │  - TabletSchema                     │                   │
│               │  - RowsetMeta                       │                   │
│               │  - SegmentRef                       │                   │
│               └─────────────────────────────────────┘                   │
└─────────────────────────────────────────────────────────────────────────┘
```

## 新增组件

### 1. be-rocks Crate (新增)

位置: `crates/be-rocks/`

提供 RocksDB 元数据存储后端。

| 文件 | 行数 | 功能 |
|------|------|------|
| `meta_store.rs` | 245 | RocksDB wrapper，三个 column families |
| `catalog_store.rs` | 538 | Database/Table CRUD 操作 |
| `tablet_store.rs` | 428 | Tablet schema/rowset 元数据 |
| `edit_log_store.rs` | 332 | WAL 操作 |
| `lib.rs` | 27 | 导出公共接口 |

**RocksDB Column Families 设计**:

| Column Family | Key Pattern | Value |
|---------------|-------------|-------|
| `catalog` | `db:{name}` | Database JSON |
| `catalog` | `db:{name}:table:{tbl}` | Table JSON |
| `catalog` | `next_id` | Atomic u64 counter |
| `tablet` | `tablet:{id}:schema` | TabletSchema JSON |
| `tablet` | `tablet:{id}:rowset:{rs_id}` | RowsetMeta + Segments |
| `tablet` | `tablet:{id}:next_rowset_id` | Atomic u64 |
| `tablet` | `tablet:{id}:next_segment_id` | Atomic u64 |
| `edit_log` | `log:{index}` | EditLogEntry JSON |
| `edit_log` | `last_applied` | u64 |
| `edit_log` | `current_term` | u64 |

**特性**:
- 原子计数器 (CAS 操作)
- 批量写入 (WriteBatch)
- 双写模式支持 (JSON + RocksDB 并行写入)
- 迁移工具 `migrate-meta`

### 2. Parquet Storage (新增)

位置: `crates/be-storage/src/segment/`

| 文件 | 行数 | 功能 |
|------|------|------|
| `parquet_writer.rs` | 289 | RecordBatch → Parquet |
| `parquet_reader.rs` | 468 | Parquet → RecordBatch |
| `mod.rs` | 524 | 导出接口 |

**Parquet Writer 配置**:

```rust
ParquetWriterConfig {
    compression: Compression::ZSTD,    // ZSTD 压缩
    row_group_size: 64 * 1024,         // 64KB row group
    enable_bloom_filter: true,         // 启用 bloom filter
    bloom_filter_ndv_threshold: 10000, // NDV threshold
}
```

**Parquet Reader 功能**:
- 列投影 (projection)
- 谓词下推 (predicate pushdown using statistics)
- LIMIT 下推
- 格式自动检测 (magic header: "PAR1")

**Segment Metadata**:

```rust
ParquetSegmentMeta {
    path: String,           // 文件路径
    num_rows: u64,          // 行数
    size: u64,              // 文件大小
    column_stats: Vec<ColumnStats>,  // 列统计信息
}

ColumnStats {
    column_name: String,
    min_value: Option<String>,
    max_value: Option<String>,
    null_count: u64,
    distinct_count: Option<u64>,
}
```

### 3. StorageEngine Arrow 接口 (新增)

位置: `crates/be-storage/src/engine.rs`

**新增方法** (feature: `parquet-storage`):

```rust
// 读取为 RecordBatch
fn read_tablet_arrow(
    tablet_id: u64,
    projection: Option<&[String]>,
    predicates: &[ReadPredicate],
    limit: Option<usize>,
) -> Result<RecordBatch>

// 写入 RecordBatch
fn write_batch_arrow(tablet_id: u64, batch: &RecordBatch) -> Result<()>

// Flush 到 Parquet
fn flush_to_parquet(tablet_id: u64) -> Result<()>
```

### 4. Tablet Arrow 接口 (新增)

位置: `crates/be-storage/src/tablet.rs`

**新增方法** (feature: `parquet-storage`):

```rust
// Arrow 读取
fn read_arrow(
    projection: Option<&[String]>,
    predicates: &[ReadPredicate],
    limit: Option<usize>,
) -> Result<RecordBatch>

// Arrow 写入
fn write_arrow(batch: &RecordBatch) -> Result<()>

// Flush 到 Parquet
fn flush_parquet() -> Result<()>
```

**类型转换函数**:

```rust
// DataType → Arrow DataType
fn to_arrow_data_type(dt: &types::DataType) -> arrow_schema::DataType

// Block → RecordBatch
fn block_to_record_batch(block: &Block) -> Result<RecordBatch, String>

// RecordBatch → Block
fn record_batch_to_block(batch: &RecordBatch) -> Result<Block, String>
```

## 配置选项

位置: `crates/common/src/config.rs`

```rust
FeConfig {
    use_rocks_meta: bool,          // 使用 RocksDB 元数据
    rocks_meta_path: Option<String>, // RocksDB 路径
}

BeConfig {
    use_rocks_meta: bool,          // 使用 RocksDB 元数据
    rocks_meta_path: Option<String>, // RocksDB 路径
}
```

## Feature Flags

位置: `crates/be-storage/Cargo.toml`

```toml
[features]
default = []
rocksdb = ["be-rocks"]            # RocksDB 元数据后端
parquet-storage = ["parquet", "arrow-array", "arrow-schema"]  # Parquet 存储
```

## 数据类型映射

| RorisDB DataType | Arrow DataType |
|------------------|----------------|
| Boolean | Boolean |
| Int8 | Int8 |
| Int16 | Int16 |
| Int32 | Int32 |
| Int64 | Int64 |
| Int128 | Int128 |
| Float32 | Float32 |
| Float64 | Float64 |
| String | Utf8 |
| Date | Date32 |
| DateTime | Timestamp(Second, None) |

## 迁移工具

位置: `roris-server/src/bin/migrate-meta.rs`

**功能**:
- 读取 JSON 元数据 (catalog.json, tablet_*/schema.json, rowset_*.json)
- 写入 RocksDB
- 验证一致性
- 支持 dry-run 模式

**使用**:

```bash
migrate-meta \
  --fe-meta-dir data/fe/doris-meta \
  --be-storage-dir data/be/storage \
  --rocks-dir data/rocks-meta \
  --verify true
```

## 兼容性设计

### 格式自动检测

```rust
pub fn is_parquet_file(path: &Path) -> bool {
    // 检查 magic header "PAR1"
}
```

读取时自动检测格式：
- `"PAR1"` → Parquet → `read_parquet_segment()`
- `"RORISSEG"` → Legacy .dat → `SegmentReader::scan_segment()`

### 双写模式

`DualWriteBackend` 同时写入 JSON 和 RocksDB：
- Primary: RocksDB (原子计数器)
- Secondary: JSON (fallback)

```rust
pub struct DualWriteBackend {
    primary: Arc<dyn TabletMetaBackend>,  // RocksDB
    secondary: Arc<dyn TabletMetaBackend>, // JSON
}
```

### 渐进迁移

Compaction 时自动转换：
- 读取 legacy .dat segments
- 合并后写入 Parquet 格式
- 删除旧的 .dat 文件

## 性能优化

### 1. 谓词下推

利用 Parquet column statistics：

```rust
// 检查谓词是否可以跳过 row group
fn can_prune_with_predicate(row_groups, predicate) -> bool {
    // 如果 value < min 或 value > max，跳过
}
```

### 2. Bloom Filter

Parquet 内置 bloom filter 用于高基数列查询：

```rust
// Writer 配置
props_builder.set_bloom_filter_enabled(true);
```

### 3. 列投影

只读取需要的列，减少 IO：

```rust
ParquetReadOptions {
    projection: Some(vec!["id", "name"]),
}
```

### 4. LIMIT 下推

在读取时限制行数，避免读取全量数据：

```rust
ParquetReadOptions {
    limit: Some(100),
}
```

## 代码变更统计

| 类别 | 新增文件 | 新增行数 |
|------|---------|---------|
| be-rocks crate | 5 | 1,570 |
| Parquet storage | 2 | 757 |
| StorageEngine Arrow | engine.rs | ~50 |
| Tablet Arrow | tablet.rs | ~200 |
| 配置 | config.rs | ~20 |
| 迁移工具 | migrate-meta.rs | ~200 |
| **总计** | **约 10 个文件** | **约 2,800 行** |

## 后续工作

1. **集成测试**
   - Parquet 写入/读取正确性
   - 谓词下推验证
   - RocksDB 元数据一致性

2. **fe-datafusion 更新**
   - 使用 `read_tablet_arrow()` 替代 Block 转换
   - 移除 `block_convert.rs`

3. **性能基准**
   - TPC-H 查询性能对比
   - Parquet vs legacy .dat 压缩率对比

4. **清理旧代码**
   - 删除 legacy segment writer/reader (可选)
   - 删除 Block/Vector 类型 (可选，取决于是否保留兼容)

## 文件位置总结

```
crates/
├── be-rocks/                   # 新增 crate
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs              # 公共接口导出
│       ├── meta_store.rs       # RocksDB wrapper
│       ├── catalog_store.rs    # Catalog 元数据
│       ├── tablet_store.rs     # Tablet 元数据
│       └── edit_log_store.rs   # WAL
│
├── be-storage/
│   ├── Cargo.toml              # 新增 parquet-storage feature
│   └── src/
│       ├── engine.rs           # 新增 Arrow 接口
│       ├── tablet.rs           # 新增 Arrow 接口
│       └── segment/
│           ├── mod.rs          # 新增 Parquet 导出
│           ├── parquet_writer.rs  # 新增
│           ├── parquet_reader.rs  # 新增
│           ├── writer.rs       # 保留 (legacy)
│           └── reader.rs       # 保留 (legacy)
│
├── common/
│   └ src/config.rs             # 新增 use_rocks_meta 配置
│
└── fe-datafusion/
    └ src/table_provider.rs     # 待更新：使用 Arrow 接口

roris-server/
└── src/bin/
    └ migrate-meta.rs           # 新增：迁移工具
```

## 参考

- Apache Parquet: https://parquet.apache.org/
- Apache Arrow: https://arrow.apache.org/
- RocksDB: https://rocksdb.org/
- DataFusion Parquet integration: https://arrow.apache.org/datafusion/