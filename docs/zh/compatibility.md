# RorisDB — Doris 兼容性说明

> 版本 0.3.0

RorisDB 目标是**SQL 级别的兼容性**，同时使用完全不同的内部架构（DataFusion + Parquet 而非自研存储引擎）。

## SQL 兼容性概要

| 类别 | Doris | RorisDB | 兼容性 |
|------|-------|---------|--------|
| DDL | 完整 MySQL DDL | 核心 DDL | ~80% |
| DML | 完整 MySQL DML | INSERT/UPDATE/DELETE | ~70% |
| 查询 | 完整 MySQL SELECT | DataFusion 驱动 | ~90% |
| 函数 | 200+ 函数 | 50+ 函数 | ~25% |
| 数据类型 | 20+ 类型 | 17 类型 | ~85% |
| 协议 | MySQL 线协议 | MySQL 线协议 | ~90% |

## 与 Apache Doris 的关键差异

| 方面 | Apache Doris | RorisDB |
|------|-------------|---------|
| 架构 | 分布式 MPP（FE + BE 集群） | 单机（DataFusion） |
| 存储 | 自研 Tablet/Rowset/Segment | Apache Parquet 文件 |
| 查询引擎 | 自研向量化执行器 | Apache DataFusion |
| 列式格式 | 自研 Block/Vector | Apache Arrow |
| 语言 | C++ | Rust |
| 部署 | 多节点集群 | 单二进制 |
| 共识 | BDBJE（主/从） | N/A（单机） |
| Compaction | Cumulative + Base 后台 | N/A（每表单文件） |
| 索引 | ZoneMap、BloomFilter、Inverted | Parquet 页统计 |

## DDL 兼容性

| 语句 | Doris | RorisDB | 说明 |
|------|-------|---------|------|
| `CREATE DATABASE` | ✅ | ✅ | 兼容 |
| `CREATE TABLE` | ✅ | ✅ | 支持 Doris `KEYS` 类型语法 |
| `PARTITION BY RANGE` | ✅ | ⚠️ | 已解析，未强制执行 |
| `PARTITION BY LIST` | ✅ | ⚠️ | 已解析，未强制执行 |
| `DISTRIBUTED BY HASH` | ✅ | ⚠️ | 已解析，未强制执行 |
| `PROPERTIES (...)` | ✅ | ⚠️ | 已解析，存储但未应用 |
| `ROLLUP` | ✅ | ⚠️ | 已解析，未执行 |
| `COLOCATE WITH` | ✅ | ⚠️ | 已解析，未强制执行 |
| `CREATE MATERIALIZED VIEW` | ✅ | ⚠️ | 仅框架 |
| `CREATE EXTERNAL TABLE` | ✅ | ⚠️ | 仅框架 |

## DML 兼容性

| 语句 | Doris | RorisDB | 说明 |
|------|-------|---------|------|
| `INSERT INTO ... VALUES` | ✅ | ✅ | 多行、部分列 |
| `INSERT INTO ... SELECT` | ✅ | ⚠️ | 尚未执行 |
| `INSERT ... ON DUPLICATE KEY UPDATE` | ✅ | ⚠️ | 已解析，未执行 |
| `INSERT ... SET col=val` | ✅ | ⚠️ | 已解析，未执行 |
| `UPDATE` | ✅ | ✅ | 所有 Arrow 类型 |
| `DELETE` | ✅ | ✅ | 带 WHERE（AND/OR） |
| `DELETE ... ORDER BY ... LIMIT` | ✅ | ❌ | 不支持 |
| Stream Load（CSV/JSON） | ✅ | ❌ | 未实现 |
| Export | ✅ | ❌ | 未实现 |

## 查询兼容性

RorisDB 将所有查询执行委托给 **Apache DataFusion**，提供优秀的 SQL 覆盖：

| 功能 | Doris | RorisDB | 说明 |
|------|-------|---------|------|
| `SELECT` / `WHERE` / `ORDER BY` / `LIMIT` | ✅ | ✅ | 通过 DataFusion 完整支持 |
| `GROUP BY` + 聚合 | ✅ | ✅ | |
| `HAVING` | ✅ | ✅ | |
| `JOIN`（所有类型） | ✅ | ✅ | INNER/LEFT/RIGHT/FULL/CROSS |
| 子查询（`IN`、`EXISTS`） | ✅ | ✅ | |
| CTE（`WITH`） | ✅ | ✅ | 包括递归 |
| `UNION` / `UNION ALL` | ✅ | ✅ | |
| `INTERSECT` / `EXCEPT` | ✅ | ✅ | |
| 窗口函数 | ✅ | ✅ | ROW_NUMBER、RANK、LAG、LEAD |
| `EXPLAIN` | ✅ | ✅ | 显示 DataFusion 计划 |
| 查询提示 | ✅ | ❌ | DataFusion 不使用提示 |
| Lateral view | ✅ | ❌ | |

## 数据类型兼容性

| Doris 类型 | RorisDB 类型 | 状态 |
|-----------|-------------|------|
| `BOOLEAN` | `Boolean` | ✅ |
| `TINYINT` | `Int8` | ✅ |
| `SMALLINT` | `Int16` | ✅ |
| `INT` | `Int32` | ✅ |
| `BIGINT` | `Int64` | ✅ |
| `LARGEINT` | `Int128`（Decimal128） | ✅ |
| `FLOAT` | `Float32` | ✅ |
| `DOUBLE` | `Float64` | ✅ |
| `DECIMAL(p,s)` | `Decimal(p,s)` | ✅ |
| `DATE` | `Date`（Date32） | ✅ |
| `DATETIME` | `DateTime`（Timestamp） | ✅ |
| `VARCHAR(n)` | `String`（Utf8） | ✅ |
| `CHAR(n)` | `String`（Utf8） | ✅ |
| `STRING` | `String`（Utf8） | ✅ |
| `JSON` | `Json`（Utf8） | ✅ 存储为字符串 |
| `ARRAY<T>` | `Array(T)` | ✅ 类型映射 |
| `MAP<K,V>` | `Map(K,V)` | ✅ 类型映射 |
| `STRUCT<...>` | `Struct(...)` | ✅ 类型映射 |
| `BITMAP` | ❌ | 未实现 |
| `HLL` | ❌ | 未实现 |
| `QUANTILE_STATE` | ❌ | 未实现 |

## 函数兼容性

### RorisDB 中实现的 Doris UDF

| 函数 | 类别 | 状态 |
|------|------|------|
| `date_trunc(precision, date)` | 日期/时间 | ✅ |
| `months_add(date, n)` | 日期/时间 | ✅ |
| `days_add(date, n)` | 日期/时间 | ✅ |
| `hours_add(datetime, n)` | 日期/时间 | ✅ |
| `concat_ws(sep, s1, s2, ...)` | 字符串 | ✅ |
| `substring_index(str, delim, count)` | 字符串 | ✅ |
| `bitmap_count(expr)` | 聚合 | ✅ |

### 通过 DataFusion 提供的函数（内置）

DataFusion 提供 100+ 内置函数，包括：

- **数学**：`abs`、`ceil`、`floor`、`round`、`sqrt`、`power`、`log`、`ln`、`exp`、`sin`、`cos`、`tan`、`asin`、`acos`、`atan`、`pi`、`random`
- **字符串**：`concat`、`length`、`lower`、`upper`、`trim`、`ltrim`、`rtrim`、`substring`、`replace`、`reverse`、`repeat`、`lpad`、`rpad`、`split_part`、`starts_with`、`ends_with`、`contains`
- **日期/时间**：`now`、`current_date`、`current_timestamp`、`date_part`、`date_trunc`、`to_char`、`to_date`、`to_timestamp`
- **聚合**：`count`、`sum`、`avg`、`min`、`max`、`count(distinct)`、`array_agg`、`string_agg`
- **窗口**：`row_number`、`rank`、`dense_rank`、`lag`、`lead`、`first_value`、`last_value`、`nth_value`

### 尚未实现的 Doris 函数

| 函数 | 类别 | 优先级 |
|------|------|--------|
| `bitmap_union`、`bitmap_intersect` | Bitmap | 低 |
| `hll_union`、`hll_cardinality` | HLL | 低 |
| `json_query`、`json_value` | JSON | 中 |
| `array_contains`、`array_length` | Array | 中 |
| `grouping_id`、`grouping` | 聚合 | 低 |
| `collect_set`、`collect_list` | 聚合 | 中 |

## 协议兼容性

| 功能 | Doris | RorisDB | 说明 |
|------|-------|---------|------|
| MySQL 线协议 | ✅ | ✅ | |
| `mysql_native_password` | ✅ | ✅ | 接受任意密码 |
| `COM_QUERY` | ✅ | ✅ | |
| `COM_INIT_DB` | ✅ | ✅ | |
| `COM_FIELD_LIST` | ✅ | ✅ | |
| `COM_STMT_PREPARE` | ✅ | ⚠️ | 仅框架 |
| SSL/TLS | ✅ | ❌ | |
| 压缩 | ✅ | ❌ | |
| 连接池 | ✅ | ❌ | |

## 迁移路径

对于从 Apache Doris 迁移的用户：

1. **SQL**：大多数 SELECT 查询可以直接工作。DDL 需要简化（无分区强制）。
2. **数据**：从 Doris 导出为 Parquet，复制到 `data/{db}/{table}/data.parquet`。
3. **客户端**：任何 MySQL 客户端都可以工作 — 无需驱动更改。
4. **函数**：大多数 DataFusion 函数与 Doris 行为匹配。检查上面的 UDF 列表。

## 版本历史

| 版本 | 日期 | 亮点 |
|------|------|------|
| 0.3.0 | 2026-05-23 | DataFusion 48 升级、481 个 E2E 测试、SQL bug 修复、启动简化（-1833 行） |
| 0.2.0 | 2026-05-21 | DataFusion/Arrow 迁移、类型系统完善、下推 |
| 0.1.5 | 2026-05-21 | 大规模代码清理（-5500 行）、bug 修复 |
| 0.1.0–0.1.4 | 2026-05 | 初始开发、解析器、协议、catalog |
