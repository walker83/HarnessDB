# HarnessDB 架构设计

> 版本 0.3.0 | 单机 OLAP，基于 DataFusion + Parquet

## 概述

HarnessDB 是一个**单机 OLAP 数据库**，分层架构：

```
MySQL 客户端
    │
    ▼
┌─────────────────────┐
│   mysql-protocol     │  线协议、认证、包 I/O
├─────────────────────┤
│   harness-server       │  查询路由、DDL/DML 处理
├─────────────────────┤
│   fe-sql-parser      │  SQL 文本 → AST
│   fe-catalog         │  元数据管理
│   fe-datafusion      │  UDF、类型转换
│   fe-storage         │  Parquet 读写、TableProvider
│   fe-monitor         │  审计日志
├─────────────────────┤
│   DataFusion 48      │  查询引擎（优化器 + 执行器）
│   Arrow 55           │  列式内存格式
│   Parquet 55         │  列式磁盘格式
└─────────────────────┘
```

## Crate 依赖关系图

```
harness-server（二进制入口）
├── fe-sql-parser
├── fe-catalog
│   ├── fe-common
│   │   └── common
│   ├── be-rocks（可选）
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

## 查询执行路径

### SELECT（DataFusion 路径）

```
SQL → 解析器 → AST → DataFusion SessionContext
    → 逻辑计划 → 优化后计划 → 物理计划
    → ParquetTableProvider.scan()
        → read_with_options（投影、限制）
        → apply_filters（下推）
    → RecordBatch → MySQL 结果集
```

关键特性：
- DataFusion 负责所有优化（谓词下推、列裁剪、JOIN 重排序）
- `ParquetTableProvider` 实现 DataFusion 的 `TableProvider` trait
- 过滤下推：简单的 `列 op 字面量` 在 Parquet 读取层应用
- 投影下推：只从磁盘读取请求的列
- 返回 `MemorySourceConfig` 包裹过滤后的 `RecordBatch`

### INSERT（直接写入路径）

```
SQL → 解析器 → InsertStmt
    → 从 Expr 直接构建 Arrow 数组（无字符串中间层）
    → ParquetStorage.insert()
        → 读取现有 data.parquet
        → concat_batches（已有数据 + 新数据）
        → write_parquet_atomic（临时文件 + fsync + 重命名）
```

### UPDATE/DELETE（读-改-写）

```
SQL → 解析器 → UpdateStmt/DeleteStmt
    → ParquetStorage.update/delete()
        → 读取现有 data.parquet
        → evaluate_where_filter() — 递归 AND/OR
        → 应用变更到 RecordBatch（类型化 Arrow compute）
        → write_parquet_atomic
```

## 存储布局

```
data/
└── {数据库}/
    └── {表}/
        └── data.parquet    ← 单文件，ZSTD 压缩
```

- **原子写入**：写入 `.tmp_data.parquet` → `fsync` → `rename`
- **压缩**：ZSTD，带页级统计信息
- **Schema**：嵌入在 Parquet 文件元数据中（Arrow schema）

## 元数据

### Catalog（`fe-catalog`）

- **默认后端**：JSON 文件（`catalog.json`）
- **可选后端**：RocksDB（`be-rocks`）
- 存储：数据库、表、列、分区、视图、物化视图

### 表元数据

```rust
struct Table {
    id: u64,
    tablet_id: u64,
    name: String,
    columns: Vec<TableColumn>,  // 名称、数据类型、可空、默认值
    keys_type: KeysType,        // Duplicate, Aggregate, Unique, Primary
    partition_info: Option<PartitionInfo>,
    distribution_info: Option<DistributionInfo>,
    // ...
}
```

## MySQL 协议（`mysql-protocol`）

完整的 MySQL 线协议实现：

- **握手**：服务器问候，带能力协商
- **认证**：`mysql_native_password`（基于 SHA1 的挑战-响应）
- **命令**：`COM_QUERY`、`COM_INIT_DB`、`COM_FIELD_LIST`、`COM_QUIT`
- **结果集**：MySQL 文本协议中的列定义 + 行数据

## 监控（`fe-monitor`）

- **审计日志**：查询审计日志，含慢查询跟踪

## 数据类型映射

| HarnessDB 类型 | Arrow 类型 | Parquet 类型 |
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

## 性能特征

| 操作 | 复杂度 | 说明 |
|------|--------|------|
| SELECT（全表扫描） | O(N) | 读取所有行，但投影减少 I/O |
| SELECT（带过滤） | O(N) | 过滤下推减少物化的行 |
| INSERT | O(N) | 读取已有数据 + 拼接 + 重写 |
| UPDATE | O(N) | 读-改-写 |
| DELETE | O(N) | 读-改-写 |
| CREATE TABLE | O(1) | 创建目录 + 空 Parquet |
| DROP TABLE | O(1) | 删除目录 |

INSERT/UPDATE/DELETE 的 O(N) 代价是主要的架构限制。多 Segment 追加写入可以将其变为 O(1)。
