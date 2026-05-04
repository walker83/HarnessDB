# P2: 高级 SQL 功能

**优先级**: P2
**模块**: fe-sql-parser, fe-sql-planner
**状态**: ❌ 未开始

## 背景

RorisDB 缺少一些常用的 SQL 功能，包括 LATERAL VIEW、INDEX 管理、外部加载等。

## 任务清单

### 1. LATERAL VIEW
- [ ] Parser: LATERAL VIEW table_gen_func AS alias 语法
- [ ] Planner: LATERAL VIEW 生成 Logical Plan（与表生成函数配合）
- [ ] 常用表生成函数: EXPLODE(ARRAY)、EXPLODE(MAP)
- [ ] 多个 LATERAL VIEW 组合

### 2. INDEX DDL
- [ ] `CREATE INDEX idx ON table(col)` 语法
- [ ] `DROP INDEX idx ON table` 语法
- [ ] 索引元数据管理
- [ ] 与已有的 Bitmap Index / Inverted Index 集成

### 3. 高级加载方式
- [ ] Broker Load: `LOAD LABEL ... FROM broker` 语法和执行
- [ ] Routine Load: 持续从 Kafka 等源加载数据
- [ ] S3 Load: 直接从 S3 导入 Parquet/CSV

### 4. 其他缺失类型
- [ ] TIME 类型（仅时间，无日期）
- [ ] VARIANT 类型（半结构化数据，类似 Snowflake VARIANT）

### 5. 测试
- [ ] LATERAL VIEW + EXPLODE 正确性
- [ ] INDEX 创建/使用/删除完整流程
- [ ] 各加载方式的端到端测试

## 涉及文件

- `crates/fe-sql-parser/src/parser.rs` - 新语法解析
- `crates/fe-sql-planner/src/planner.rs` - Plan 生成
- `crates/fe-expression/src/functions.rs` - 表生成函数
- `crates/types/src/data_type.rs` - 新类型定义
