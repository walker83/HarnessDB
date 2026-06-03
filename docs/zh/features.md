# HarnessDB 功能列表

> 版本 0.3.0

## SQL 语言支持

### DDL（数据定义）

| 语句 | 状态 | 说明 |
|------|------|------|
| `CREATE DATABASE` | ✅ | 支持 `IF NOT EXISTS` |
| `DROP DATABASE` | ✅ | 支持 `IF EXISTS` |
| `CREATE TABLE` | ✅ | 列定义、`KEYS` 类型、`PARTITION BY`、`DISTRIBUTED BY` |
| `DROP TABLE` | ✅ | 支持 `IF EXISTS` |
| `ALTER TABLE` | ✅ | `ADD COLUMN`、`DROP COLUMN`、`RENAME COLUMN`、`MODIFY COLUMN` |
| `TRUNCATE TABLE` | ✅ | 删除数据，保留 schema |
| `CREATE VIEW` | ✅ | 存储查询定义 |
| `DROP VIEW` | ✅ | |
| `CREATE INDEX` | ⚠️ | 已解析，未强制执行 |
| `CREATE MATERIALIZED VIEW` | ⚠️ | 已解析，仅框架 |

### DML（数据操作）

| 语句 | 状态 | 说明 |
|------|------|------|
| `INSERT INTO ... VALUES` | ✅ | 多行、部分列、类型感知 |
| `INSERT INTO ... SELECT` | ⚠️ | 已解析，未执行 |
| `UPDATE ... SET ... WHERE` | ✅ | 支持所有 Arrow 类型 |
| `DELETE FROM ... WHERE` | ✅ | 支持 `AND`/`OR` 条件 |

### 查询

| 功能 | 状态 | 说明 |
|------|------|------|
| `SELECT` | ✅ | 通过 DataFusion |
| `WHERE` | ✅ | 完整的 DataFusion 谓词支持 |
| `ORDER BY` | ✅ | |
| `LIMIT` / `OFFSET` | ✅ | |
| `GROUP BY` | ✅ | |
| `HAVING` | ✅ | |
| `JOIN`（INNER） | ✅ | |
| `JOIN`（LEFT/RIGHT/FULL/CROSS） | ✅ | |
| 子查询 | ✅ | `IN`、`EXISTS`、相关子查询 |
| CTE（`WITH`） | ✅ | 支持递归 |
| `UNION` / `UNION ALL` | ✅ | |
| `INTERSECT` / `EXCEPT` | ✅ | |
| `EXPLAIN` | ✅ | 显示 DataFusion 执行计划 |

### 聚合函数

| 函数 | 状态 |
|------|------|
| `COUNT(*)` | ✅ |
| `COUNT(expr)` | ✅ |
| `COUNT(DISTINCT expr)` | ✅ |
| `SUM` | ✅ |
| `AVG` | ✅ |
| `MIN` / `MAX` | ✅ |
| `GROUP_CONCAT` | ✅ |

### 窗口函数

| 函数 | 状态 |
|------|------|
| `ROW_NUMBER()` | ✅ |
| `RANK()` | ✅ |
| `DENSE_RANK()` | ✅ |
| `LAG(expr, n)` | ✅ |
| `LEAD(expr, n)` | ✅ |

### 内置函数

| 类别 | 示例 | 数量 |
|------|------|------|
| 数学 | `abs`、`ceil`、`floor`、`round`、`sqrt`、`pow`、`log`、`sin`、`cos`、`tan`、`rand` | 30+ |
| 字符串 | `concat`、`concat_ws`、`length`、`upper`、`lower`、`trim`、`substring`、`replace`、`substring_index` | 15+ |
| 日期/时间 | `date_trunc`、`months_add`、`days_add`、`hours_add`、`now`、`curdate` | 8+ |
| 聚合 | `bitmap_count` | 1 |
| 类型转换 | `CAST(expr AS type)` | 所有 Arrow 类型 |

## 数据类型

| HarnessDB 类型 | SQL 语法 | Arrow 类型 | 说明 |
|-------------|---------|-----------|------|
| `Boolean` | `BOOLEAN`、`BOOL` | Boolean | |
| `Int8` | `TINYINT` | Int8 | |
| `Int16` | `SMALLINT` | Int16 | |
| `Int32` | `INT`、`INTEGER` | Int32 | |
| `Int64` | `BIGINT` | Int64 | |
| `Float32` | `FLOAT` | Float32 | |
| `Float64` | `DOUBLE` | Float64 | |
| `Decimal(p,s)` | `DECIMAL(10,2)` | Decimal128 | 完整精度 |
| `Date` | `DATE` | Date32 | epoch 以来的天数 |
| `DateTime` | `DATETIME`、`TIMESTAMP` | Timestamp(Second) | |
| `String` | `VARCHAR`、`CHAR`、`TEXT`、`STRING` | Utf8 | |
| `Binary` | `BINARY`、`BLOB` | Binary | |
| `Array(T)` | `ARRAY<INT>` | List | 嵌套类型 |
| `Map(K,V)` | `MAP<STRING, INT>` | Map | 嵌套类型 |
| `Struct` | `STRUCT<...>` | Struct | 嵌套类型 |
| `Json` | `JSON` | Utf8 | 存储为字符串 |

## SHOW / DESCRIBE 命令

| 命令 | 状态 |
|------|------|
| `SHOW DATABASES` | ✅ |
| `SHOW TABLES` | ✅ | 支持 `LIKE` 模式 |
| `SHOW CREATE TABLE` | ✅ | |
| `SHOW CREATE DATABASE` | ✅ | |
| `DESCRIBE table` / `DESC table` | ✅ | |
| `SHOW TABLE STATUS` | ✅ | |
| `SHOW VARIABLES` | ✅ | 返回空 |
| `SHOW PROCESSLIST` | ✅ | 返回当前连接 |

## 事务支持

| 功能 | 状态 | 说明 |
|------|------|------|
| `BEGIN` / `START TRANSACTION` | ⚠️ | 已解析，跟踪状态 |
| `COMMIT` | ⚠️ | 跟踪状态，无 MVCC |
| `ROLLBACK` | ⚠️ | 跟踪状态，无实际回滚 |
| `SAVEPOINT` | ⚠️ | 已解析，跟踪状态 |
| 隔离级别 | ⚠️ | `SET TRANSACTION ISOLATION LEVEL` 已解析，未强制执行 |

> 事务在**语法层面支持**但未强制执行 — 没有 MVCC 或 WAL。操作是单独原子的（Parquet 原子写入）。

## MySQL 协议兼容性

| 功能 | 状态 |
|------|------|
| 线协议 | ✅ MySQL 文本协议 |
| 认证 | ✅ `mysql_native_password`（接受任意密码） |
| `COM_QUERY` | ✅ |
| `COM_INIT_DB` | ✅（USE 数据库） |
| `COM_FIELD_LIST` | ✅ |
| `COM_QUIT` | ✅ |
| 预编译语句 | ⚠️ 仅框架 |
| SSL/TLS | ❌ 未实现 |

## 监控与可观测性

| 功能 | 状态 | 说明 |
|------|------|------|
| 审计日志 | ✅ | 查询日志记录 |

## 用户管理

| 功能 | 状态 | 说明 |
|------|------|------|
| `CREATE USER` | ⚠️ | 已解析，存根执行 |
| `DROP USER` | ⚠️ | 已解析，存根执行 |
| `GRANT` / `REVOKE` | ⚠️ | 已解析，存根执行 |
| `SET PASSWORD` | ⚠️ | 已解析，存根执行 |

> 用户管理 SQL **已解析并接受**以保持兼容性，但未强制执行 — 所有连接都有完全访问权限。
