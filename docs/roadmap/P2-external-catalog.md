# P2: 外部 Catalog (Hive/Iceberg/Hudi)

**优先级**: P2
**模块**: fe-catalog, fe-sql-planner, be-execution
**状态**: ❌ 未开始

## 背景

RorisDB 当前不支持外部数据源。需要实现 Multi-Catalog 架构，支持查询 Hive、Iceberg、Hudi 等外部数据湖中的数据。

## 任务清单

### 1. Catalog 框架
- [ ] 定义 `Catalog` trait，统一内部/外部 Catalog 接口
- [ ] `CREATE CATALOG` / `DROP CATALOG` / `SHOW CATALOGS` 语法
- [ ] `REFRESH CATALOG` 元数据刷新
- [ ] Catalog 属性配置（连接信息、认证等）
- [ ] SQL 中通过 `catalog.database.table` 三段式引用

### 2. Hive Catalog
- [ ] 对接 Hive Metastore 获取 Database/Table 元数据
- [ ] 解析 Hive 表的 Partition 信息
- [ ] 读取 HDFS/S3 上的 ORC/Parquet 文件
- [ ] Hive 类型 → RorisDB 类型映射

### 3. Iceberg Catalog
- [ ] 对接 Iceberg REST Catalog / Hive Metastore
- [ ] 解析 Iceberg 表的 Snapshot/Manifest 元数据
- [ ] 读取 Parquet 数据文件
- [ ] 支持 Iceberg 的 Time Travel 查询

### 4. Hudi Catalog
- [ ] 对接 Hudi Timeline 获取表元数据
- [ ] 解析 Hudi 的 COW/MOR 表结构
- [ ] 读取 Parquet 基础文件 + Log 文件

### 5. 通用文件读取
- [ ] Parquet Reader（已有部分支持）
- [ ] ORC Reader
- [ ] S3 文件系统适配
- [ ] HDFS 文件系统适配

### 6. 测试
- [ ] 使用本地文件模拟 Hive Metastore 测试
- [ ] Parquet/ORC 文件读取正确性
- [ ] 跨 Catalog JOIN 查询

## 涉及文件

- `crates/fe-catalog/src/external/` - 新建，外部 Catalog 实现
- `crates/fe-sql-parser/src/parser.rs` - Catalog DDL 语法
- `crates/be-execution/src/` - 外部文件 Scan 执行
- `crates/data-io/src/parquet.rs` - Parquet 读取
