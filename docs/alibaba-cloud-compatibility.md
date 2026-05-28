# RorisDB Database Chameleon - 兼容性矩阵

RorisDB 作为通用数据库仿真底座，支持模拟多种数据库协议和 SQL 方言。

## 支持的数据库协议

| 数据库 | 协议类型 | 端口 | 状态 | 说明 |
|--------|---------|------|------|------|
| MySQL | TCP 二进制 (MySQL Wire Protocol) | 9030 | ✅ 已支持 | 基础协议 |
| MaxCompute (ODPS) | HTTP/REST + XML | 9031 | 🚧 开发中 | 阿里云离线大数据 |
| Hologres | TCP 二进制 (PostgreSQL v3) | 5432 | 🚧 开发中 | 阿里云实时数仓 |

---

## MaxCompute 兼容性

### 协议兼容

| 功能 | 状态 | 说明 |
|------|------|------|
| REST API (HTTP/HTTPS) | ✅ | 基础端点实现 |
| HMAC-SHA1 签名 (V2) | ✅ | 标准 AccessKey 认证 |
| HMAC-SHA256 签名 (V4) | 🚧 | 带 region 的增强签名 |
| XML 请求/响应 | ✅ | 完整 XML 序列化 |
| SQL 作业提交 | ✅ | 异步提交 + 轮询结果 |
| Tunnel 协议 (批量传输) | ❌ | Phase 2 |
| Instance 管理 | ✅ | 状态查询/结果获取/停止 |

### 数据类型兼容

| MaxCompute 类型 | RorisDB 映射 | 状态 |
|----------------|-------------|------|
| BIGINT | BIGINT | ✅ |
| INT | INT | ✅ |
| SMALLINT | SMALLINT | ✅ |
| TINYINT | TINYINT | ✅ |
| STRING | STRING | ✅ |
| STRING(n) | VARCHAR(n) | ✅ |
| DOUBLE | DOUBLE | ✅ |
| FLOAT | FLOAT | ✅ |
| DECIMAL(p,s) | DECIMAL(p,s) | ✅ |
| BOOLEAN | BOOLEAN | ✅ |
| DATETIME | DATETIME | ✅ |
| DATE | DATE | ✅ |
| TIMESTAMP | TIMESTAMP | ✅ |
| BINARY | BLOB | ✅ |
| ARRAY<T> | — | ❌ 不支持 |
| MAP<K,V> | — | ❌ 不支持 |
| STRUCT<...> | — | ❌ 不支持 |

### SQL 语法兼容

#### DDL

| MaxCompute 语法 | 处理方式 | 状态 |
|----------------|---------|------|
| `CREATE TABLE (col TYPE, ...)` | 直接执行 | ✅ |
| `PARTITIONED BY (col TYPE)` | 分区列转为普通列 | ✅ 自动转换 |
| `LIFECYCLE N` | 剥离 | ✅ 静默忽略 |
| `STORED AS ORC/PARQUET/...` | 剥离 | ✅ 内部统一 Parquet |
| `CLUSTERED BY ... INTO N BUCKETS` | 剥离 | ✅ 静默忽略 |
| `TBLPROPERTIES (...)` | 剥离 | ✅ 静默忽略 |
| `COMMENT 'text'` | 保留 | ✅ |

#### DML

| MaxCompute 语法 | 处理方式 | 状态 |
|----------------|---------|------|
| `INSERT INTO t VALUES (...)` | 直接执行 | ✅ |
| `INSERT INTO t SELECT ...` | 直接执行 | ✅ |
| `INSERT OVERWRITE TABLE t SELECT ...` | 转为 INSERT INTO | ✅ 自动转换 |
| `INSERT INTO t PARTITION(ds='x') VALUES ...` | 剥离 PARTITION 子句 | ✅ 自动转换 |
| `UPDATE t SET ... WHERE ...` | 不支持 | ❌ MC 本身不支持 |
| `DELETE FROM t WHERE ...` | 不支持 | ❌ MC 本身不支持 |
| `MERGE INTO ...` | 不支持 | ❌ Phase 1 |

#### 查询

| MaxCompute 语法 | 处理方式 | 状态 |
|----------------|---------|------|
| `SELECT ... FROM ... WHERE ...` | 直接执行 | ✅ |
| `SELECT ... JOIN ...` | 直接执行 | ✅ |
| `SELECT ... GROUP BY ...` | 直接执行 | ✅ |
| `SELECT ... ORDER BY ...` | 直接执行 | ✅ |
| `WITH ... AS ... SELECT ...` (CTE) | 直接执行 | ✅ |
| 窗口函数 | 直接执行 | ✅ |
| `GROUPING SETS / ROLLUP / CUBE` | 直接执行 | ✅ |
| `/*+ MAPJOIN(alias) */` | 剥离 hint | ✅ 静默忽略 |
| `/*+ SKEWJOIN(alias) */` | 剥离 hint | ✅ 静默忽略 |
| `DISTRIBUTE BY ... SORT BY ...` | 转为 ORDER BY | ✅ 自动转换 |
| `SET odps.sql.xxx=yyy` | 忽略 | ✅ 静默忽略 |
| `SETPROJECT xxx=yyy` | 忽略 | ✅ 静默忽略 |
| `SELECT * EXCEPT(col1, col2)` | 不支持 | ❌ Phase 1 |
| `SELECT * REPLACE(expr AS col)` | 不支持 | ❌ Phase 1 |
| `LATERAL VIEW explode(col)` | 不支持 | ❌ Phase 1 |
| `SELECT TRANSFORM(...) USING 'script'` | 不支持 | ❌ |
| `SELECT /*+ MAPJOIN */ ... EXCEPT(...)` | hint 剥离, EXCEPT 不支持 | ⚠️ 部分 |

---

## Hologres 兼容性

### 协议兼容

| 功能 | 状态 | 说明 |
|------|------|------|
| PostgreSQL v3 Wire Protocol | ✅ | 标准 PG 协议 |
| MD5 认证 | ✅ | AccessKey ID/Secret |
| Simple Query | ✅ | Q 消息 |
| Extended Query (Parse/Bind/Execute) | 🚧 | Phase 2 |
| SSL | ❌ | 返回 'N' 拒绝 |
| pg_catalog 系统表 | 🚧 | 部分模拟 |

### 数据类型兼容

| Hologres/PG 类型 | RorisDB 映射 | 状态 |
|-----------------|-------------|------|
| BIGINT | BIGINT | ✅ |
| INTEGER / INT | INT | ✅ |
| SMALLINT | SMALLINT | ✅ |
| TEXT | STRING | ✅ |
| VARCHAR(n) | VARCHAR(n) | ✅ |
| CHAR(n) | CHAR(n) | ✅ |
| REAL / FLOAT4 | FLOAT | ✅ |
| DOUBLE PRECISION | DOUBLE | ✅ |
| BOOLEAN | BOOLEAN | ✅ |
| NUMERIC(p,s) / DECIMAL(p,s) | DECIMAL(p,s) | ✅ |
| TIMESTAMP | TIMESTAMP | ✅ |
| TIMESTAMPTZ | TIMESTAMP | ✅ 时区忽略 |
| DATE | DATE | ✅ |
| BYTEA | BLOB | ✅ |
| JSON / JSONB | — | ❌ Phase 1 |
| INT[] / TEXT[] | — | ❌ Phase 1 |
| UUID | — | ❌ Phase 1 |
| TIME | — | ❌ Phase 1 |
| INTERVAL | — | ❌ Phase 1 |
| SERIAL | INT | ⚠️ 无自增 |
| BIGSERIAL | BIGINT | ⚠️ 无自增 |

### Hologres 特有 DDL

| Hologres 语法 | 处理方式 | 状态 |
|--------------|---------|------|
| `CREATE TABLE (...) WITH (orientation='column', ...)` | 剥离 WITH 子句 | ✅ 静默忽略 |
| `CALL set_table_property(...)` | 忽略 | ✅ 静默忽略 |
| `PARTITION BY LIST(col)` | 剥离 | ✅ |
| `CREATE TABLE child PARTITION OF parent` | 忽略 | ✅ |

### 不支持的 PG 特性

| 功能 | 状态 | 错误信息 |
|------|------|---------|
| `CREATE TRIGGER` | ❌ | Hologres 不支持触发器 |
| `PL/pgSQL` 存储过程 | ❌ | Hologres 不支持存储过程 |
| `WITH RECURSIVE` | ❌ | Hologres 不支持递归 CTE |
| `SELECT ... FOR UPDATE` | ❌ | Hologres 无行级锁 |
| `CREATE EXTENSION` | ⚠️ | 静默忽略 |
| `GIN/GiST/BRIN 索引` | ❌ | Hologres 仅支持 B-tree |
| `CREATE DOMAIN` | ❌ | 不支持 |
| `LISTEN/NOTIFY` | ❌ | 不支持 |
| 表继承 | ❌ | 不支持 |
| 外键约束 | ⚠️ | 剥离（Hologres 不强制） |
| `EXPLAIN ANALYZE` | ⚠️ | 转为 EXPLAIN |
| `DISTINCT ON` | ❌ | 不支持 |
| `INSERT ON CONFLICT` | ⚠️ | 转为 INSERT INTO |
| 标准视图 | ❌ | 仅支持物化视图 |

### pg_catalog 兼容

| 系统表/视图 | 状态 | 说明 |
|------------|------|------|
| `pg_tables` | 🚧 | 部分模拟 |
| `pg_class` | 🚧 | 部分列 |
| `pg_namespace` | 🚧 | 映射到 database |
| `pg_attribute` | 🚧 | 映射到 column |
| `pg_user` / `pg_roles` | 🚧 | 模拟返回 |
| `hg_stat_activity` | 🚧 | Hologres 特有视图 |
| `version()` | ✅ | 返回 "PostgreSQL 15.x (RorisDB)" |

---

## 快速开始

### MaxCompute 兼容

```bash
# 启动 RorisDB
./target/release/roris-fe --mysql-port 9030 --maxcompute-port 9031

# 使用 pyodps 连接
python3 <<EOF
from odps import ODPS
o = ODPS('roris', 'roris-secret', 'default',
         endpoint='http://127.0.0.1:9031/api')

# 创建表
o.execute_sql("""
CREATE TABLE users (
    id BIGINT COMMENT 'user id',
    name STRING COMMENT 'user name',
    age INT
) PARTITIONED BY (ds STRING) LIFECYCLE 365
""").wait_for_success()

# 列出表
for t in o.list_tables():
    print(t.name)

# 查询
with o.execute_sql('SELECT * FROM users').open_reader() as reader:
    for record in reader:
        print(record)
EOF
```

### Hologres 兼容

```bash
# 启动 RorisDB
./target/release/roris-fe --mysql-port 9030 --hologres-port 5432

# 使用 psql 连接
psql -h 127.0.0.1 -p 5432 -U roris -d default

# 建表 (Hologres 语法)
CREATE TABLE orders (
    id BIGINT NOT NULL,
    user_id BIGINT,
    amount DOUBLE PRECISION,
    created_at TIMESTAMP,
    PRIMARY KEY (id)
) WITH (
    orientation = 'column',
    distribution_key = 'id',
    time_to_live_in_seconds = '2592000'
);

# 插入数据
INSERT INTO orders VALUES (1, 100, 99.99, now());

# 查询
SELECT * FROM orders WHERE user_id = 100;
```

---

## 架构说明

```
+-------------------------------------------------------------------+
|                      RorisDB Core Engine                          |
|  DataFusion 48 | Parquet Storage | Catalog Manager                |
+-------------------------------------------------------------------+
         ^                    ^                    ^
         |                    |                    |
   QueryHandler         QueryHandler         QueryHandler
         |                    |                    |
+----------------+  +------------------+  +---------------------+
| MySQL Protocol |  | MaxCompute       |  | PG Protocol         |
| (TCP :9030)    |  | Protocol (HTTP)  |  | (TCP :5432)         |
| [已支持]       |  | :9031            |  | Hologres 兼容       |
+----------------+  +------------------+  +---------------------+
                           |                       |
                    MC SQL Translator       Hologres SQL Translator
                    (strip MC-specific)     (strip PG-unsupported,
                                             add set_table_property)
```
