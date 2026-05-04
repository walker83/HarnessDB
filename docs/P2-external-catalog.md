# P2: 外部 Catalog (Hive/Iceberg/Hudi)

**优先级**: P2
**模块**: fe-catalog, fe-sql-planner, be-execution
**状态**: 🔄 进行中 (Iceberg 核心框架完成)

## 背景

RorisDB 当前不支持外部数据源。需要实现 Multi-Catalog 架构，支持查询 Hive、Iceberg、Hudi 等外部数据湖中的数据。

## 已完成 ✅

### 1. Catalog 框架
- [x] 定义 `Catalog` trait，统一内部/外部 Catalog 接口 (`fe-catalog/src/external/catalog.rs`)
- [x] `CatalogType` 枚举 (Internal, Iceberg, Hive, Hudi)
- [x] `DatabaseInfo`, `TableInfo`, `ColumnInfo`, `FileFormat` 数据结构
- [x] `CatalogCache` 缓存结构

### 2. SQL 语法
- [x] `CREATE CATALOG catalog_name WITH (key=value,...)` 语法 (`fe-sql-parser/src/parser.rs`)
- [x] `DROP CATALOG catalog_name` 语法
- [x] `SHOW CATALOGS` 语法
- [x] `REFRESH CATALOG catalog_name` 语法
- [x] AST 新增 `CreateCatalogStmt`, `DropCatalogStmt`, `RefreshCatalogStmt`

### 3. Planner 扩展
- [x] `ScanNode` 新增 `catalog: Option<String>` 字段支持三段式表名
- [x] `resolve_table_name` 支持解析 `catalog.db.table` 三段式名称
- [x] `Planner` 新增 `external_catalogs` HashMap 支持外部 Catalog 注册

### 4. Iceberg Catalog 实现
- [x] `IcebergCatalog` 结构体 (`fe-catalog/src/external/iceberg/catalog.rs`)
- [x] `IcebergCatalogConfig` 配置 (uri, warehouse, auth_token)
- [x] REST API 客户端 (reqwest)
- [x] 解析 Iceberg REST API 响应 (Namespace, TableLoadResponse, TableMetadata, IcebergSchema, IcebergColumn, Snapshot)
- [x] Mock Catalog 支持 (`IcebergCatalog::mock()`)

### 5. FileSystem 抽象
- [x] `FileSystem` trait 定义 (`data-io/src/fs.rs`)
- [x] `LocalFileSystem` 实现
- [x] `S3FileSystem` 实现 (mock)
- [x] `parse_s3_path` 工具函数

### 6. ExternalFileScanExecNode
- [x] `ExternalFileScanExecNode` 实现 (`be-execution/src/external_file_scan.rs`)
- [x] `ExternalFileSystem` trait
- [x] `MockExternalFileSystem` 测试用 mock

## 进行中 🔄

### Planner Catalog DDL 支持
- [ ] `plan_create_catalog` 实现 - 需要连接 FE CatalogManager 注册外部 Catalog
- [ ] `plan_drop_catalog` 实现
- [ ] `plan_show_catalogs` 实现
- [ ] `plan_refresh_catalog` 实现

### Iceberg 增强
- [ ] Time Travel 支持 (AS OF TIMESTAMP / AS OF VERSION)
- [ ] Manifest 解析
- [ ] Partition spec 转换

## 待开始 📋

### 2. Hive Catalog
- [ ] 对接 Hive Metastore 获取 Database/Table 元数据
- [ ] 解析 Hive 表的 Partition 信息
- [ ] 读取 HDFS/S3 上的 ORC/Parquet 文件
- [ ] Hive 类型 → RorisDB 类型映射

### 3. Hudi Catalog
- [ ] 对接 Hudi Timeline 获取表元数据
- [ ] 解析 Hudi 的 COW/MOR 表结构
- [ ] 读取 Parquet 基础文件 + Log 文件

### 4. 通用文件读取
- [x] Parquet Reader（已有部分支持）
- [ ] ORC Reader
- [ ] S3 文件系统适配（当前是 mock）
- [ ] HDFS 文件系统适配

### 5. 测试
- [ ] 使用本地文件模拟 Hive Metastore 测试
- [ ] Parquet/ORC 文件读取正确性
- [ ] 跨 Catalog JOIN 查询

## 涉及文件

- `crates/fe-catalog/src/external/` - 外部 Catalog 实现
  - `catalog.rs` - Catalog trait 和基础类型
  - `internal_catalog.rs` - 内部 Catalog 适配器
  - `iceberg/` - Iceberg REST Catalog 实现
- `crates/fe-sql-parser/src/parser.rs` - Catalog DDL 语法
- `crates/fe-sql-parser/src/ast.rs` - Catalog 语句 AST
- `crates/fe-sql-planner/src/planner.rs` - 三段式表名解析
- `crates/fe-sql-planner/src/plan_node.rs` - ScanNode 新增 catalog 字段
- `crates/data-io/src/fs.rs` - FileSystem 抽象
- `crates/be-execution/src/external_file_scan.rs` - 外部文件扫描执行节点