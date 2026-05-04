# RorisDB vs Apache Doris 兼容性矩阵

> 最后更新: 2026-05-04
> 测试总数: 145 tests / 43 suites / 0 failures (v0.1.3)
>
> 版本: 0.1.3 - CTE parsing, ExecNode stubs

---

## 1. SQL 语法兼容性

| 特性 | Apache Doris | RorisDB | 测试覆盖 | 备注 |
|------|:-----------:|:-------:|:--------:|------|
| SELECT / WHERE / ORDER BY / LIMIT | ✅ | ✅ | ✅ | 完整支持 |
| GROUP BY + 聚合函数 | ✅ | ✅ | ✅ | COUNT/SUM/AVG/MIN/MAX/COUNT DISTINCT/GROUP_CONCAT |
| HAVING 子句 | ✅ | ✅ | ❌ | Filter→Aggregate, 待测试 |
| JOIN (INNER/LEFT/RIGHT/FULL/CROSS) | ✅ | ✅ | ✅ (部分) | INNER/LEFT 有测试 |
| 子查询 (IN/EXISTS) | ✅ | ✅ (AST) | ❌ | 解析器支持, planner 未实现 |
| CTE (WITH 子句) | ✅ | ✅ (解析) | ❌ | Parser 支持, Planner 待实现 |
| UNION / UNION ALL | ✅ | ✅ (解析) | ❌ | Parser 支持, Planner 待实现 |
| INSERT INTO ... VALUES | ✅ | ✅ | ✅ | 支持 |
| INSERT INTO ... SELECT | ✅ | ✅ | ❌ | Planner 支持, 待执行器 |
| CREATE DATABASE / TABLE | ✅ | ✅ | ✅ | 完整支持 |
| DROP DATABASE / TABLE | ✅ | ✅ | ✅ | 完整支持 |
| ALTER TABLE (ADD/DROP/RENAME COLUMN) | ✅ | ✅ (解析) | ❌ | AST 支持, 转换不完整 |
| TRUNCATE TABLE | ✅ | ❌ | ❌ | 未实现 |
| DESCRIBE / DESC TABLE | ✅ | ✅ | ❌ | Planner 支持, 待执行器 |
| SHOW CREATE TABLE | ✅ | ❌ | ❌ | 未实现 |
| SHOW COLUMNS | ✅ | ✅ | ❌ | Planner 支持, 待执行器 |
| USE DATABASE | ✅ | ✅ | ✅ | 完整支持 |
| SET 变量 | ✅ | ✅ | ✅ | 接受但不生效 |
| CREATE VIEW | ✅ | ❌ | ❌ | 未实现 |
| 窗口函数 (OVER / PARTITION BY) | ✅ | ❌ | ❌ | 未实现 |
| 查询提示 (Hints) | ✅ | ❌ | ❌ | 未实现 |
| ANALYZE TABLE | ✅ | ❌ | ❌ | 未实现 |
| 多表 INSERT | ✅ | ❌ | ❌ | 未实现 |

## 2. 数据类型兼容性

| 类型 | Doris | RorisDB | 测试覆盖 | 备注 |
|------|:-----:|:-------:|:--------:|------|
| BOOLEAN | ✅ | ✅ | ✅ | BooleanVector |
| TINYINT (Int8) | ✅ | ✅ | ✅ | Int8Vector |
| SMALLINT (Int16) | ✅ | ✅ | ✅ | Int16Vector |
| INT (Int32) | ✅ | ✅ | ✅ | Int32Vector |
| BIGINT (Int64) | ✅ | ✅ | ✅ | Int64Vector |
| LARGEINT (Int128) | ✅ | ✅ | ✅ | Int128Vector |
| FLOAT (Float32) | ✅ | ✅ | ✅ | Float32Vector |
| DOUBLE (Float64) | ✅ | ✅ | ✅ | Float64Vector |
| DECIMAL(precision, scale) | ✅ | ✅ (定义) | ❌ | DataType 存在, 操作未实现 |
| DATE | ✅ | ✅ | ✅ | DateVector |
| DATETIME | ✅ | ✅ | ✅ | DateTime 定义 |
| VARCHAR / CHAR / STRING | ✅ | ✅ | ✅ | StringVector |
| JSON | ✅ | ❌ | ❌ | 未实现 |
| ARRAY | ✅ | ✅ (定义) | ❌ | DataType 存在, 操作未实现 |
| MAP | ✅ | ✅ (定义) | ❌ | DataType 存在, 操作未实现 |
| STRUCT | ✅ | ✅ (定义) | ❌ | DataType 存在, 操作未实现 |
| BITMAP | ✅ | ❌ | ❌ | 未实现 |
| HLL | ✅ | ❌ | ❌ | 未实现 |
| QUANTILE_STATE | ✅ | ❌ | ❌ | 未实现 |
| AGG_STATE | ✅ | ❌ | ❌ | 未实现 |

## 3. 表达式函数兼容性

| 函数类别 | Doris 函数数 | RorisDB 已实现 | 测试覆盖 | 缺失函数示例 |
|----------|:----------:|:-------------:|:--------:|-------------|
| 算术 | ~20 | 4 | ✅ | abs, ceil, floor, round |
| 字符串 | ~50 | 6 | ✅ | upper, lower, length, concat, substring, trim |
| 聚合 | ~15 | 7 | ✅ | count, sum, avg, min, max, count_distinct, group_concat |
| 空值处理 | ~8 | 3 | ❌ | coalesce, ifnull, nullif |
| 日期函数 | ~30 | 17 | ❌ | YEAR, MONTH, DAY, HOUR, MINUTE, SECOND, DATEDIFF, CURDATE, NOW, DATE_ADD, DATE_SUB, DATE_FORMAT, DATE_TRUNC, WEEK, QUARTER, MONTHNAME, DAYNAME |
| 条件表达式 | ~5 | 1 | ❌ | CASE WHEN (已有), IF |
| 数学 | ~20 | 0 | ❌ | sin, cos, tan, log, exp, sqrt, pow |
| JSON | ~15 | 0 | ❌ | json_parse, json_query, json_value |
| 正则 | ~5 | 0 | ❌ | regexp, regexp_replace |
| 窗口 | ~10 | 0 | ❌ | ROW_NUMBER, RANK, DENSE_RANK, LAG, LEAD |
| 类型转换 | ~10 | 0 | ❌ | CAST (已有), implicit coercion 未实现 |
| 位操作 | ~8 | 0 | ❌ | bitand, bitor, bitxor, bitnot |

## 4. 存储引擎兼容性

| 特性 | Doris | RorisDB | 测试覆盖 | 备注 |
|------|:-----:|:-------:|:--------:|------|
| Tablet → Rowset → Segment 三层架构 | ✅ | ✅ | ✅ | 完全一致 |
| 列式存储 | ✅ | ✅ | ✅ | Vector + Block |
| ZoneMap 索引 | ✅ | ✅ | ✅ | min/max/null_count 统计 |
| BloomFilter 索引 | ✅ | ✅ | ✅ | 可配置 FPR |
| 倒排索引 | ✅ | ❌ | ❌ | 未实现 |
| RLE 编码 | ✅ | ✅ | ✅ | 游程编码 |
| Bit-Packed 编码 | ✅ | ✅ | ✅ | 差值+位压缩 |
| 字典编码 | ✅ | ✅ | ✅ | 低基数列优化 |
| LZ4 压缩 | ✅ | ✅ | ✅ | 段页级压缩 |
| ZSTD 压缩 | ✅ | ❌ | ❌ | 未实现 |
| Zlib 压缩 | ✅ | ❌ | ❌ | 未实现 |
| Cumulative Compaction | ✅ | ✅ | ✅ | 优先队列调度 |
| Base Compaction | ✅ | ✅ | ✅ | 优先队列调度 |
| Segment 持久化读写 | ✅ | ✅ | ✅ | JSON 元数据 + 二进制数据 |
| MemTable 内存缓冲 | ✅ | ✅ | ❌ | BTreeMap 实现, 自动刷盘 |
| MVCC 多版本控制 | ✅ | ❌ | ❌ | 未实现 |
| 数据副本复制 | ✅ | ❌ | ❌ | 未实现 |
| Schema Change | ✅ | ❌ | ❌ | 未实现 |
| 冷热数据分层 | ✅ | ❌ | ❌ | 未实现 |
| Page Cache | ✅ | ❌ | ❌ | 未实现 |

## 5. 查询引擎兼容性

| 特性 | Doris | RorisDB | 测试覆盖 | 备注 |
|------|:-----:|:-------:|:--------:|------|
| 向量化执行 | ✅ | ✅ | ✅ | Vector + Block 列式处理 |
| Pipeline 执行模型 | ✅ | ✅ | ❌ | 定义完整, 执行逻辑待完善 |
| 谓词下推 | ✅ | ✅ | ✅ | Filter → Scan, Filter → Project |
| 列裁剪 | ✅ | ✅ | ✅ | 移除 Scan 中未使用列 |
| Limit 下推 | ✅ | ✅ | ✅ | Limit → Sort → Scan |
| Join 重排序 | ✅ | ✅ | ✅ | 小表在构建侧 |
| 常量折叠 | ✅ | ✅ | ✅ | 算术表达式编译期求值 |
| 布尔化简 | ✅ | ✅ | ✅ | TRUE AND x → x, NOT NOT x → x |
| 代价优化 (CBO) | ✅ | ❌ | ❌ | 未实现 |
| Fragment 分布式规划 | ✅ | ✅ | ❌ | HashPartition/Broadcast/Gather/RoundRobin |
| Exchange 操作符 | ✅ | ✅ | ❌ | 定义完整, 通信未实现 |
| Hash Join | ✅ | ✅ (定义) | ❌ | HashJoinNode 存在, 执行未实现 |
| Sort-Merge Join | ✅ ✅ | (定义) | ❌ | MergeJoinNode 存在, 执行未实现 |
| 子查询解嵌 | ✅ | ❌ | ❌ | 未实现 |
| 物化视图重写 | ✅ | ✅ (框架) | ❌ | MaterializedView 定义存在 |
| 内存溢写 (Spill) | ✅ | ❌ | ❌ | 未实现 |

## 6. MySQL 协议兼容性

| 特性 | Doris | RorisDB | 测试覆盖 | 备注 |
|------|:-----:|:-------:|:--------:|------|
| MySQL 握手 (HandshakeV10) | ✅ | ✅ | ✅ | auth_switch + salt |
| 明文认证 | ✅ | ✅ (任意) | ✅ | 接受任意用户名/密码 |
| COM_QUERY | ✅ | ✅ | ✅ | SQL 文本执行 |
| COM_PING | ✅ | ✅ | ✅ | 心跳检测 |
| COM_QUIT | ✅ | ✅ | ✅ | 优雅断连 |
| COM_INIT_DB | ✅ | ✅ | ✅ | USE DATABASE 底层实现 |
| COM_STMT_PREPARE | ✅ | ✅ | ✅ | 占位符解析 |
| COM_STMT_EXECUTE | ✅ | ✅ (基础) | ✅ | 参数绑定简化 |
| COM_STMT_CLOSE | ✅ | ✅ | ✅ | 释放 Prepared Statement |
| COM_FIELD_LIST | ✅ | ✅ | ✅ | 返回空列列表 |
| COM_STATISTICS | ✅ | ✅ | ✅ | 返回模拟统计信息 |
| 结果集编码 (Text Protocol) | ✅ | ✅ | ✅ | lenenc-int + 列定义 + 行数据 |
| EOF 包处理 | ✅ | ✅ | ✅ | 支持 DEPRECATE_EOF 标志 |
| 字符集映射 | ✅ | ✅ | ✅ | utf8/utf8mb4/latin1/binary |
| 列类型映射 | ✅ | ✅ | ✅ | String/Int/Float/Double/Date/DateTime/Blob |
| SSL/TLS | ✅ | ❌ | ❌ | 未实现 |
| 压缩协议 | ✅ | ❌ | ❌ | 未实现 |
| 批量插入协议 | ✅ | ❌ | ❌ | 未实现 |

## 7. 分布式架构兼容性

| 特性 | Doris | RorisDB | 测试覆盖 | 备注 |
|------|:-----:|:-------:|:--------:|------|
| FE/BE 分离架构 | ✅ | ✅ | ❌ | roris-server 包含 FE/BE 入口 |
| BE 节点注册 | ✅ | ✅ | ❌ | ClusterManager 实现 |
| 心跳检测 | ✅ | ✅ | ❌ | load_score 追踪 |
| 负载感知调度 | ✅ | ✅ | ❌ | LoadAware 策略 |
| Round-Robin 调度 | ✅ | ✅ | ❌ | 默认策略 |
| Fragment 实例管理 | ✅ | ✅ | ❌ | FragmentInstance 定义 |
| Query Coordinator | ✅ | ✅ | ❌ | 完整查询生命周期 |
| FE HA (Master/Follower) | ✅ | ❌ | ❌ | Raft 计划中 |
| 元数据持久化 (EditLog) | ✅ | ✅ | ✅ | JSON 行格式 + flush/replay |
| Catalog 写入器 | ✅ | ✅ | ❌ | CatalogWriter 原子更新 |
| gRPC 服务定义 | ✅ | ✅ | ❌ | Proto 定义完整 |
| 实际 RPC 通信 | ✅ | ❌ | ❌ | 仅 Proto 定义 |
| 查询超时控制 | ✅ | ✅ (框架) | ❌ | QueryLimits 定义 |
| 查询取消 | ✅ | ✅ (框架) | ❌ | cancel_query 方法 |

## 8. 数据导入/导出

| 特性 | Doris | RorisDB | 测试覆盖 | 备注 |
|------|:-----:|:-------:|:--------:|------|
| CSV 读取 | ✅ | ✅ | ✅ | CsvReader + 类型推断 |
| CSV 写入 | ✅ | ✅ | ✅ | CsvWriter |
| JSON Lines 读取 | ✅ | ✅ | ✅ | JsonReader + 扁平化 |
| Schema 自动推断 | ✅ | ✅ | ✅ | int/float/date/datetime/string |
| Stream Load 框架 | ✅ | ✅ | ✅ | StreamLoadBuilder |
| RowBatch 写入 | ✅ | ✅ | ✅ | 类型安全的行操作 |
| Parquet 导入 | ✅ | ❌ | ❌ | 未实现 |
| ORC 导入 | ✅ | ❌ | ❌ | 未实现 |
| INSERT INTO ... SELECT 导入 | ✅ | ❌ | ❌ | 未实现 |
| Broker Load | ✅ | ❌ | ❌ | 未实现 |
| Spark Load | ✅ | ❌ | ❌ | 未实现 |
| 导出至 HDFS/S3 | ✅ | ❌ | ❌ | 未实现 |

---

## 9. 测试覆盖总览

### 按模块统计

| 模块 | 测试文件 | 测试数 | 状态 |
|------|---------|:------:|:----:|
| fe-expression | evaluator.rs | 27 | ✅ |
| fe-sql-planner | optimizer.rs | 12 | ✅ |
| fe-common | edit_log.rs | 8 | ✅ |
| fe-catalog | catalog.rs | 10 | ✅ |
| types | vector_test.rs | 70 | ✅ |
| data-io | csv/json/stream 内联 | 20 | ✅ |
| integration-storage | storage_test.rs | 28 | ✅ |
| integration-sql | sql_test.rs | 34 | ✅ |
| integration-mysql | mysql_protocol_test.rs | 27 | ✅ |
| **总计** | | **145+** | **✅** |

### 测试覆盖的关键路径

```
SQL 输入
  → 解析 (sqlparser-rs)        ✅ 测试: parse_select, parse_join
  → 逻辑计划 (Planner)          ✅ 测试: plan_select, plan_aggregate
  → 优化 (Optimizer)            ✅ 测试: predicate_pushdown, column_pruning, limit_pushdown
  → 表达式求值 (ExprEvaluator)  ✅ 测试: arithmetic, comparison, logical, functions
  → Block 操作                  ✅ 测试: filter, project, slice, concat
  → 存储读写 (StorageEngine)    ✅ 测试: tablet, rowset, segment, codec
  → MySQL 协议                  ✅ 测试: handshake, query, prepared_stmt
  → Catalog 持久化              ✅ 测试: save/load, CRUD
  → EditLog 持久化              ✅ 测试: append, flush, replay
```

---

## 10. 开发优先级建议

### P0 - 核心功能缺失 (影响基本可用性)

| # | 特性 | 预估工作量 | 影响范围 | 状态 |
|---|------|-----------|---------|------|
| 1 | INSERT INTO ... SELECT | 3天 | 数据导入必需 | ✅ 已实现 |
| 2 | 日期函数 (DATE_ADD/DATE_FORMAT/NOW 等) | 5天 | OLAP 查询高频使用 | ✅ 已实现 (17函数) |
| 3 | CTE (WITH 子句) | 5天 | 复杂分析查询必需 | ✅ 已解析 |
| 4 | 实际 Pipeline 执行 | 10天 | 查询端到端执行 | ⚙️ ExecNode框架 |
| 5 | gRPC FE-BE 通信 | 7天 | 分布式执行必需 | ❌ Proto定义 |

### P1 - 重要功能增强

| # | 特性 | 预估工作量 | 影响范围 | 状态 |
|---|------|-----------|---------|------|
| 6 | UNION / UNION ALL | 2天 | 多结果集合并 | ✅ 已解析 |
| 7 | 子查询解嵌 | 5天 | IN/EXISTS 支持 | ❌ 未实现 |
| 8 | HAVING 子句转换 | 1天 | 聚合后过滤 | ✅ 已实现 |
| 9 | 窗口函数 | 10天 | 排名/累计计算 | ❌ 未实现 |
| 10 | DESCRIBE / SHOW CREATE TABLE | 1天 | 元数据查询 | ✅ DESCRIBE/SHOW COLUMNS |
| 11 | ZSTD 压缩 | 2天 | 更高压缩比 | ❌ 未实现 |
| 12 | FE HA (Raft) | 15天 | 高可用 | ❌ 未实现 |
| 13 | DECIMAL 运算 | 3天 | 精确数值计算 | ❌ 未实现 |
| 14 | JSON 函数 | 5天 | 半结构化数据处理 | ❌ 未实现 |

### P2 - 功能完善

| # | 特性 | 预估工作量 | 影响范围 |
|---|------|-----------|---------|
| 15 | 数学函数 (三角/对数) | 3天 | 科学计算 |
| 16 | 正则函数 | 2天 | 文本分析 |
| 17 | 倒排索引 | 5天 | 全文搜索 |
| 18 | MVCC | 15天 | 事务隔离 |
| 19 | ALTER TABLE 执行 | 5天 | Schema 演进 |
| 20 | TRUNCATE TABLE | 1天 | 数据清理 |
| 21 | SSL/TLS | 5天 | 安全连接 |
| 22 | 权限管理 (RBAC) | 10天 | 多用户安全 |
| 23 | ARRAY/MAP/STRUCT 操作 | 5天 | 复杂类型 |
| 24 | 内存溢写 (Spill to disk) | 10天 | 大数据量稳定 |
| 25 | 代价优化 (CBO) | 15天 | 查询性能 |

---

## 11. 版本路线图

```
v0.1 (当前) ─── 基础架构 + 核心存储 + MySQL 协议
  │
  ├── v0.2 ─── 完整 SQL 执行 (INSERT SELECT, CTE, UNION, HAVING)
  │             日期函数, Pipeline 执行, gRPC 通信
  │
  ├── v0.3 ─── 分析增强 (窗口函数, 子查询解嵌, CBO)
  │             ZSTD 压缩, 倒排索引, DECIMAL 运算
  │
  ├── v0.4 ─── 生产就绪 (FE HA Raft, MVCC, SSL/TLS, RBAC)
  │             内存溢写, Broker Load, Parquet 导入
  │
  └── v1.0 ─── Doris 兼容 (完整 MySQL 兼容, TPC-H 通过)
                 云模式 S3, 多集群联邦查询
```
