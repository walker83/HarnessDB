# RorisDB 功能缺失分析

本文档对比 Apache Doris 与 RorisDB 的功能差异，列出 RorisDB 尚未实现的功能。

> 参考文档：https://doris.apache.org/docs/sql-manual/sql-statements/
> RorisDB 版本：v0.1.3
> 更新时间：2026/05/05

---

## 1. SQL 语句支持现状

### 1.0 当前已实现的语句

| 语句 | 解析 | 执行 | 备注 |
|------|------|------|------|
| SELECT (含 JOIN/CTE/UNION/SUBQUERY) | ✅ | ✅ | 完整支持 |
| INSERT INTO / INSERT OVERWRITE | ✅ | ✅ | VALUES + SELECT 子查询 |
| UPDATE | ✅ | ✅ | 解析完成，执行层已完成 |
| DELETE | ✅ | ✅ | 解析完成，执行层已完成 |
| CREATE DATABASE | ✅ | ✅ | 含 IF NOT EXISTS, PROPERTIES |
| DROP DATABASE | ✅ | ✅ | 含 IF EXISTS |
| CREATE TABLE | ✅ | ✅ | 含 DISTRIBUTED BY, PARTITION BY, PROPERTIES, KeysType |
| DROP TABLE | ✅ | ✅ | 含 IF EXISTS |
| ALTER TABLE (ADD/DROP/MODIFY COLUMN, RENAME) | ✅ | 🚧 | 解析完成，部分执行 |
| TRUNCATE TABLE | ✅ | 🚧 | 解析完成，执行层未实现 |
| CREATE VIEW | ✅ | ✅ | |
| CREATE/DROP/ALTER/REFRESH MATERIALIZED VIEW | ✅ | ✅ | |
| CREATE/DROP REPOSITORY | ✅ | ✅ | Local/S3/HDFS |
| BACKUP/RESTORE DATABASE | ✅ | ✅ | |
| USE DATABASE | ✅ | ✅ | |
| SET VARIABLE | ✅ | ❌ | 解析完成，未实现 session variable |
| SHOW DATABASES | ✅ | ✅ | |
| SHOW TABLES | ✅ | ✅ | |
| SHOW COLUMNS | ✅ | ✅ | |
| SHOW CREATE TABLE | ✅ | ✅ | |
| SHOW REPOSITORIES | ✅ | ✅ | |
| DESCRIBE | ✅ | ✅ | |
| EXPLAIN | ✅ | ✅ | |
| CREATE USER / DROP USER / SHOW USERS | ✅ | ❌ | 解析完成，未实现 |
| CREATE/DROP/SHOW/REFRESH CATALOG | ✅ | ❌ | 解析完成，未实现 |

### 1.1 DDL 语句缺失

| 功能 | Apache Doris | RorisDB | 计划批次 |
|------|-------------|---------|---------|
| ALTER DATABASE | ✅ | ❌ | 第1批 |
| SHOW CREATE DATABASE | ✅ | ❌ | 第1批 |
| DROP VIEW | ✅ | ❌ | 第1批 |
| ALTER VIEW | ✅ | ❌ | 第1批 |
| SHOW CREATE VIEW | ✅ | ❌ | 第1批 |
| ALTER TABLE RENAME COLUMN | ✅ | ❌ | 第1批 |
| ALTER TABLE COMMENT | ✅ | ❌ | 第1批 |
| ALTER TABLE SET PROPERTY | ✅ | ❌ | 第1批 |
| ALTER TABLE ADD/DROP PARTITION | ✅ | ❌ | 第2批 |
| ALTER TABLE ADD/DROP ROLLUP | ✅ | ❌ | 第2批 |
| ALTER TABLE REPLACE | ✅ | ❌ | 第2批 |
| ALTER TABLE ADD GENERATED COLUMN | ✅ | ❌ | 第3批 |
| CANCEL ALTER TABLE | ✅ | ❌ | 第3批 |
| CREATE INDEX | ✅ | ❌ | 第2批 |
| DROP INDEX | ✅ | ❌ | 第2批 |
| ALTER COLOCATE GROUP | ✅ | ❌ | 第4批 |

### 1.2 SHOW 语句缺失

| 功能 | Apache Doris | RorisDB | 计划批次 |
|------|-------------|---------|---------|
| SHOW PARTITIONS | ✅ | ❌ | 第2批 |
| SHOW TABLE STATUS | ✅ | ❌ | 第2批 |
| SHOW VARIABLES | ✅ | ❌ | 第2批 |
| SHOW PROCESSLIST | ✅ | ❌ | 第2批 |
| SHOW INDEX | ✅ | ❌ | 第2批 |
| SHOW ALTER TABLE | ✅ | ❌ | 第3批 |
| SHOW CREATE VIEW | ✅ | ❌ | 第1批 |
| SHOW BACKENDS | ✅ | ❌ | 第3批 |
| SHOW FRONTENDS | ✅ | ❌ | 第3批 |
| SHOW CREATE DATABASE | ✅ | ❌ | 第1批 |
| SHOW ALTER TABLE (MV) | ✅ | ❌ | 第3批 |
| SHOW TABLE ID / PARTITION ID | ✅ | ❌ | 第4批 |
| SHOW DYNAMIC PARTITION TABLES | ✅ | ❌ | 第4批 |
| SHOW VIEW | ✅ | ❌ | 第3批 |
| SHOW CREATE MATERIALIZED VIEW | ✅ | ❌ | 第3批 |

### 1.3 DML 语句缺失

| 功能 | Apache Doris | RorisDB | 计划批次 |
|------|-------------|---------|---------|
| UPDATE 执行层 | ✅ | ✅ | 第4批 → 已完成 |
| DELETE 执行层 | ✅ | ✅ | 第4批 → 已完成 |
| EXPORT TABLE | ✅ | ❌ | 第3批 |
| SHOW DELETE | ✅ | ❌ | 第3批 |
| SHOW LAST INSERT | ✅ | ❌ | 第4批 |
| BROKER LOAD | ✅ | ❌ | 第4批 |
| ROUTINE LOAD | ✅ | ❌ | 第4批 |
| MYSQL LOAD | ✅ | ❌ | 第4批 |

### 1.4 Account/Security 语句缺失

| 功能 | Apache Doris | RorisDB | 计划批次 |
|------|-------------|---------|---------|
| SET VARIABLE 执行 | ✅ | ❌ | 第2批 |
| GRANT | ✅ | ❌ | 第3批 |
| REVOKE | ✅ | ❌ | 第3批 |
| CREATE ROLE / DROP ROLE / ALTER ROLE | ✅ | ❌ | 第3批 |
| ALTER USER | ✅ | ❌ | 第3批 |
| SET PASSWORD | ✅ | ❌ | 第3批 |
| SET PROPERTY | ✅ | ❌ | 第3批 |
| SHOW GRANTS / SHOW ROLES / SHOW PRIVILEGES | ✅ | ❌ | 第3批 |

### 1.5 Session/Transaction 语句缺失

| 功能 | Apache Doris | RorisDB | 计划批次 |
|------|-------------|---------|---------|
| BEGIN / COMMIT / ROLLBACK | ✅ | ❌ | 第3批 |
| SHOW TRANSACTION | ✅ | ❌ | 第4批 |
| KILL QUERY / KILL CONNECTION | ✅ | ❌ | 第3批 |
| SWITCH CATALOG | ✅ | ❌ | 第4批 |
| UNSET VARIABLE | ✅ | ❌ | 第4批 |

### 1.6 其他语句缺失

| 功能 | Apache Doris | RorisDB | 计划批次 |
|------|-------------|---------|---------|
| CREATE/DROP FUNCTION | ✅ | ❌ | 第4批 |
| ANALYZE TABLE | ✅ | ❌ | 第4批 |
| EXPORT | ✅ | ❌ | 第3批 |
| INSTALL/UNINSTALL PLUGIN | ✅ | ❌ | 第4批 |
| CREATE/DROP JOB | ✅ | ❌ | 第4批 |
| RECOVER (回收站恢复) | ✅ | ❌ | 第4批 |

---

## 2. 存储引擎

### 2.1 存储格式

| 功能 | Apache Doris | RorisDB | 状态 |
|------|-------------|---------|------|
| Tablet/Rowset/Segment | ✅ | ✅ | 已完成 |
| 列式存储 (Vectorized) | ✅ | ✅ | 已完成 |
| Primary Key Index | ✅ | ✅ | 已完成 |

### 2.2 索引类型

| 功能 | Apache Doris | RorisDB | 状态 |
|------|-------------|---------|------|
| ZoneMap Index | ✅ | ✅ | 已完成 |
| BloomFilter Index | ✅ | ✅ | 已完成 |
| Bitmap Index | ✅ | ✅ | 已完成 |
| Inverted Index | ✅ | ✅ | 已完成 |
| NGram Bloom Filter | ✅ | ❌ | 缺失 |
| ANN Index (向量检索) | ✅ | ✅ | 已完成 |

### 2.3 压缩算法

| 功能 | Apache Doris | RorisDB | 状态 |
|------|-------------|---------|------|
| LZ4 | ✅ | ✅ | 已完成 |
| zstd | ✅ | ✅ | 已完成 |
| Snappy | ✅ | ✅ | 已完成 |
| Zlib | ✅ | ❌ | 缺失 |
| RLE | ✅ | ✅ | 已完成 |

### 2.4 Compaction

| 功能 | Apache Doris | RorisDB | 状态 |
|------|-------------|---------|------|
| Base Compaction | ✅ | ✅ | 已完成 |
| Cumulative Compaction | ✅ | ✅ | 已完成 |
| Full Compaction | ✅ | ✅ | 已完成 |
| Single Replica Compaction | ✅ | ❌ | 缺失 |
| Segment Compaction | ✅ | ❌ | 缺失 |
| 优先级调度 | ✅ | ✅ | 已完成 |

---

## 3. 查询优化

### 3.1 优化器

| 功能 | Apache Doris | RorisDB | 状态 |
|------|-------------|---------|------|
| RBO (基于规则) | ✅ | ✅ | 已完成 |
| CBO (基于代价) | ✅ (Nereids) | 🚧 | 框架存在，无实际代价模型 |
| 统计信息管理 | ✅ | 🚧 | 结构已定义，无收集机制 |
| 直方图 | ✅ | ❌ | 缺失 |

### 3.2 优化规则

| 功能 | Apache Doris | RorisDB | 状态 |
|------|-------------|---------|------|
| 谓词下推 | ✅ | ✅ | 已完成 |
| 列裁剪 | ✅ | ✅ | 已完成 |
| Limit 下推 | ✅ | ✅ | 已完成 |
| Join 重排序 | ✅ | ✅ | 已完成 |
| 子查询解嵌套 | ✅ | ✅ | 已完成 |
| 常量折叠 | ✅ | ✅ | 已完成 |
| Runtime Filter | ✅ | ❌ | 缺失 |
| Partition Pruning | ✅ | ❌ | 缺失 |
| Short Circuit Query | ✅ | ❌ | 缺失 |
| CTE 复用 | ✅ | ❌ | 缺失 |
| Outer Join 转 Inner Join | ✅ | ❌ | 缺失 |

### 3.3 执行计划优化

| 功能 | Apache Doris | RorisDB | 状态 |
|------|-------------|---------|------|
| Broadcast Join | ✅ | ✅ | 已完成 |
| Shuffle Join | ✅ | ✅ | 已完成 |
| Colocate Join | ✅ | ❌ | 缺失 |
| Bucket Shuffle Join | ✅ | ❌ | 缺失 |
| 物化视图透明改写 | ✅ | 🚧 | 数据结构存在，逻辑未实现 |

---

## 4. 分布与分区

### 4.1 分区策略

| 功能 | Apache Doris | RorisDB | 状态 |
|------|-------------|---------|------|
| Range Partition | ✅ | ❌ | 缺失 |
| List Partition | ✅ | ❌ | 缺失 |
| Hash Partition | ✅ | ❌ | 缺失 |
| 二级分区 (Composite) | ✅ | ❌ | 缺失 |
| 动态分区 | ✅ | ❌ | 缺失 |
| Auto Partition | ✅ | ❌ | 缺失 |

### 4.2 副本管理

| 功能 | Apache Doris | RorisDB | 状态 |
|------|-------------|---------|------|
| Tablet 副本数配置 | ✅ | ❌ | 缺失 |
| 副本自动分配 | ✅ | ❌ | 缺失 |
| 副本迁移 | ✅ | ❌ | 缺失 |
| 副本均衡 | ✅ | ❌ | 缺失 |
| Colocate Table | ✅ | ❌ | 缺失 |

---

## 5. 安全

| 功能 | Apache Doris | RorisDB | 状态 |
|------|-------------|---------|------|
| MySQL 协议认证 | ✅ | ✅ | 已完成 |
| RBAC (角色权限) | ✅ | ❌ | 缺失 |
| 列级权限 | ✅ | ❌ | 缺失 |
| 行级权限 | ✅ | ❌ | 缺失 |
| LDAP 认证 | ✅ | ❌ | 缺失 |

---

## 6. 数据类型

### 6.1 基础类型

| 类型 | Apache Doris | RorisDB | 状态 |
|------|-------------|---------|------|
| TINYINT/SMALLINT/INT/BIGINT | ✅ | ✅ | 已完成 |
| LARGEINT (Int128) | ✅ | ✅ | 已完成 |
| FLOAT/DOUBLE | ✅ | ✅ | 已完成 |
| DECIMAL | ✅ | ✅ | 已完成 |
| CHAR/VARCHAR/STRING | ✅ | ✅ | 已完成 |
| DATE/DATETIME | ✅ | ✅ | 已完成 |
| BOOLEAN | ✅ | ✅ | 已完成 |
| TIME | ✅ | ❌ | 缺失 |

### 6.2 复杂类型

| 类型 | Apache Doris | RorisDB | 状态 |
|------|-------------|---------|------|
| ARRAY | ✅ | ✅ | 已完成 |
| MAP | ✅ | ✅ | 已完成 |
| STRUCT | ✅ | ✅ | 已完成 |
| JSON | ✅ | ✅ | 已完成 |
| BITMAP | ✅ | ✅ | 已完成 |
| VARIANT | ✅ | ❌ | 缺失 |
| HLL (HyperLogLog) | ✅ | ❌ | 缺失 |
| IPV4/IPV6 | ✅ | ❌ | 缺失 |
| BINARY/VARBINARY | ✅ | ❌ | 缺失 |

---

## 7. 外部表和集成

| 功能 | Apache Doris | RorisDB | 状态 |
|------|-------------|---------|------|
| Hive Catalog | ✅ | ❌ | 缺失 |
| Iceberg Catalog | ✅ | ❌ | 缺失 |
| Hudi Catalog | ✅ | ❌ | 缺失 |
| Paimon Catalog | ✅ | ❌ | 缺失 |
| JDBC Catalog | ✅ | ❌ | 缺失 |
| MySQL 外部表 | ✅ | ❌ | 缺失 |

---

## 8. 高可用

| 功能 | Apache Doris | RorisDB | 状态 |
|------|-------------|---------|------|
| 心跳机制 | ✅ | ✅ | 已完成 |
| FE HA (BDBJE/Raft) | ✅ | ❌ | 缺失 |
| Master 选举 | ✅ | ❌ | 缺失 |
| Tablet 自动修复 | ✅ | ❌ | 缺失 |
| Binlog CDC | ✅ | ❌ | 缺失 |

---

## 9. 管理与监控

| 功能 | Apache Doris | RorisDB | 状态 |
|------|-------------|---------|------|
| Information Schema | ✅ | ✅ | 已完成 |
| Metrics API (Prometheus) | ✅ | ✅ | 已完成 |
| Query Profile | ✅ | ✅ | 已完成 |
| Audit Log | ✅ | ✅ | 已完成 |

---

## 10. SQL 语句补全实施计划

### 第1批：DDL 补全（优先级最高）

> 目标：补全最基础的 DDL 语句，使数据库元数据操作完整

| # | 语句 | 语法 |
|---|------|------|
| 1 | ALTER DATABASE | `ALTER DATABASE db SET PROPERTIES ("key"="value")` |
| 2 | SHOW CREATE DATABASE | `SHOW CREATE DATABASE db_name` |
| 3 | DROP VIEW | `DROP VIEW [IF EXISTS] view_name` |
| 4 | ALTER VIEW | `ALTER VIEW view_name AS select_query` |
| 5 | SHOW CREATE VIEW | `SHOW CREATE VIEW view_name` |
| 6 | ALTER TABLE RENAME COLUMN | `ALTER TABLE t RENAME COLUMN old TO new` |
| 7 | ALTER TABLE COMMENT | `ALTER TABLE t COMMENT 'comment'` |
| 8 | ALTER TABLE SET PROPERTY | `ALTER TABLE t SET PROPERTIES ("key"="value")` |

**涉及文件**：
- `crates/fe-sql-parser/src/ast.rs` — 新增 Statement variant 和 struct
- `crates/fe-sql-parser/src/parser.rs` — 新增关键字匹配和解析函数
- `roris-server/src/fe_main.rs` — 新增 execute_statement 分支

### 第2批：SHOW 语句 + 索引 + Session

> 目标：补全运维常用的 SHOW 语句，添加索引管理，实现 session variable

| # | 语句 | 语法 |
|---|------|------|
| 9 | CREATE INDEX | `CREATE INDEX idx ON t (col1, col2) [USING BITMAP]` |
| 10 | DROP INDEX | `DROP INDEX idx ON t` |
| 11 | SHOW INDEX | `SHOW INDEX FROM t` |
| 12 | SHOW PARTITIONS | `SHOW PARTITIONS FROM t` |
| 13 | SHOW TABLE STATUS | `SHOW TABLE STATUS [FROM db]` |
| 14 | SHOW VARIABLES | `SHOW [GLOBAL\|SESSION] VARIABLES [LIKE 'pattern']` |
| 15 | SHOW PROCESSLIST | `SHOW [FULL] PROCESSLIST` |
| 16 | SET VARIABLE 执行 | `SET var = value` / `SET GLOBAL var = value` |
| 17 | ALTER TABLE ADD/DROP PARTITION | `ALTER TABLE t ADD/DROP PARTITION ...` |
| 18 | ALTER TABLE ADD/DROP ROLLUP | `ALTER TABLE t ADD/DROP ROLLUP ...` |
| 19 | ALTER TABLE REPLACE | `ALTER TABLE t REPLACE WITH TABLE ...` |

### 第3批：Account/Security + 事务 + 高级功能

> 目标：实现基本的权限控制、事务支持、数据导出

| # | 语句 | 语法 |
|---|------|------|
| 20 | GRANT | `GRANT priv ON db.table TO user` |
| 21 | REVOKE | `REVOKE priv ON db.table FROM user` |
| 22 | CREATE/DROP/ALTER ROLE | `CREATE ROLE role_name` |
| 23 | ALTER USER / SET PASSWORD | `ALTER USER user IDENTIFIED BY 'pwd'` |
| 24 | SET PROPERTY | `SET PROPERTY FOR user 'key'='value'` |
| 25 | SHOW GRANTS / ROLES / PRIVILEGES | 各类 SHOW 权限语句 |
| 26 | BEGIN / COMMIT / ROLLBACK | 事务控制 |
| 27 | KILL QUERY / KILL CONNECTION | `KILL QUERY id` / `KILL CONNECTION id` |
| 28 | EXPORT TABLE | `EXPORT TABLE t TO 'path' PROPERTIES (...)` |
| 29 | SHOW ALTER TABLE | `SHOW ALTER TABLE [FROM db]` |
| 30 | SHOW BACKENDS / SHOW FRONTENDS | 集群信息查看 |
| 31 | SHOW CREATE VIEW | `SHOW CREATE VIEW view_name` |
| 32 | SHOW DELETE | `SHOW DELETE` |
| 33 | SHOW CREATE MATERIALIZED VIEW | `SHOW CREATE MATERIALIZED VIEW mv` |

### 第4批：高级功能（长期）

| # | 语句 | 语法 |
|---|------|------|
| 34 | CREATE/DROP FUNCTION | UDF 管理 |
| 35 | ANALYZE TABLE | 统计信息收集 |
| 36 | INSTALL/UNINSTALL PLUGIN | 插件管理 |
| 37 | CREATE/DROP JOB | 定时任务 |
| 38 | RECOVER | 回收站恢复 |
| 39 | BROKER LOAD / ROUTINE LOAD | 高级导入 |
| 40 | UPDATE/DELETE 执行层 | 实际数据操作 |
| 41 | SHOW TABLE ID / PARTITION ID | 高级运维 |

---

## 实现模式

每个语句遵循相同的 3 步模式：

```
1. ast.rs      → 定义 Statement variant + struct
2. parser.rs   → 在 parse_sql() 开头的 if-chain 中添加关键字匹配 + 解析函数
3. fe_main.rs  → 在 execute_statement() match 中添加处理分支
```

对于非查询语句（DDL/DCL/SET），直接在 `fe_main.rs` 的 handler 中对 catalog 操作，不走 planner。只有 SELECT 查询走 planner。

## 验证方式

每批实现后：
```bash
cargo build --release  # 确保编译通过
cargo test --workspace  # 确保现有测试不回归
mysql -h 127.0.0.1 -P 9030 -uroot  # 手动测试新语句
```
