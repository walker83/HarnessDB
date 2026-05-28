# RorisDB Database Chameleon - 兼容性矩阵

RorisDB 作为通用数据库仿真底座，支持模拟多种数据库协议和 SQL 方言。

## Phase 1 完成状态

Phase 1 基础协议兼容性已完成。所有核心端点、认证和基础 SQL 功能均已实现。

## 支持的数据库协议

| 数据库 | 协议类型 | 端口 | 状态 | 说明 |
|--------|---------|------|------|------|
| MySQL | TCP 二进制 (MySQL Wire Protocol) | 9030 | ✅ 已支持 | 基础协议 |
| MaxCompute (ODPS) | HTTP/REST + XML | 9031 | ✅ Phase 1 完成 | 阿里云离线大数据 |
| Hologres | TCP 二进制 (PostgreSQL v3) | 15432 | ✅ Phase 1 完成 | 阿里云实时数仓 |

## 端口配置

| 服务 | 默认端口 | CLI 参数 | 说明 |
|------|---------|---------|------|
| MySQL Wire Protocol | 9030 | `--mysql-port` | 基础 MySQL 协议 |
| MaxCompute REST API | 9031 | `--maxcompute-port` | HTTP REST 端点 |
| Hologres (PostgreSQL) | 15432 | `--hologres-port` | PostgreSQL v3 协议 |
| Web SQL Editor | 8080 (configurable) | (config: `server.http_port`) | 内嵌 Web UI |

---
---

## MaxCompute 兼容性

### 协议兼容

| 功能 | 状态 | 说明 |
|------|------|------|
| REST API (HTTP/HTTPS) | ✅ | 基础端点实现 |
| HMAC-SHA1 签名 (V2) | ✅ | 标准 AccessKey 认证 |
| HMAC-SHA256 签名 (V4) | ✅ | 带 region 的增强签名 |
| XML 请求/响应 | ✅ | 完整 XML 序列化 |
| SQL 作业提交 | ✅ | 异步提交 + 轮询结果 |
| Tunnel 协议 (批量传输) | ❌ | Phase 2 |
| Instance 管理 | ✅ | 状态查询/结果获取/停止 |
| 表删除 (REST API) | ✅ | DELETE /api/projects/{project}/tables/{table} |

### 认证方式

MaxCompute 协议支持两种签名方式：

| 方式 | 算法 | 实现状态 | a 说明 |
|------|------|---------|--------|
| V2 签名 | HMAC-SHA1 | ✅ | `Authorization: ODPS {ak}:{signature}` |
| V4 签名 | HMAC-SHA256 | ✅ | `Authorization: ODPS2-HMAC-SHA256 ...` |

默认凭据: `AccessKey ID = "roris"`, `AccessKey Secret = "roris-secret"`

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
| ARRAY<T> | — | ❌ Phase 2 |
| MAP<K,V> | — | ❌ Phase 2 |
| STRUCT<...> | — | ❌ Phase 2 |

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
| `DROP TABLE [IF EXISTS]` | 直接执行 | ✅ |

#### DML

| MaxCompute 语法 | 处理方式 | 状态 |
|----------------|---------|------|
| `INSERT INTO t VALUES (...)` | 直接执行 | ✅ |
| `INSERT INTO t SELECT ...` | 直接执行 | ✅ |
| `INSERT OVERWRITE TABLE t SELECT ...` | 转为 INSERT INTO | ✅ 自动转换 |
| `INSERT INTO t PARTITION(ds='x') VALUES ...` | 剥离 PARTITION 子句 | ✅ 自动转换 |
| `UPDATE t SET ... WHERE ...` | 直接执行 | ✅ RorisDB 支持 |
| `DELETE FROM t WHERE ...` | 直接执行 | ✅ RorisDB 支持 |
| `MERGE INTO ...` | 不支持 | ❌ Phase 2 |
| `FROM src INSERT INTO t1 ... INSERT INTO t2 ...` (MULTI INSERT) | 直接执行 | ✅ 透传 |

#### 查询

| MaxCompute 语法 | 处理方式 | 状态 |
|----------------|---------|------|
| `SELECT ... FROM ... WHERE ...` | 直接执行 | ✅ |
| `SELECT ... JOIN ...` | 直接执行 | ✅ |
| `SELECT ... GROUP BY ...` | 直接执行 | ✅ |
| `SELECT ... ORDER BY ...` | 直接执行 | ✅ |
| `WITH ... AS ... SELECT ...` (CTE) | 直接执行 | ✅ |
| 窗口函数 | 直接执行 | ✅ |
| `GROUPING SETS / ROLLUP / CUBE` | 直接执行 | ✅ DataFusion 支持 |
| `/*+ MAPJOIN(alias) */` | 剥离 hint | ✅ 静默忽略 |
| `/*+ SKEWJOIN(alias) */` | 剥离 hint | ✅ 静默忽略 |
| `DISTRIBUTE BY ... SORT BY ...` | 转为 ORDER BY | ✅ 自动转换 |
| `CLUSTER BY col` | 转为 ORDER BY | ✅ 自动转换 |
| `ZORDER BY col` | 剥离 | ✅ 静默忽略 |
| `SET odps.sql.xxx=yyy` | 忽略 | ✅ 静默忽略 |
| `SET project.xxx=yyy` | 忽略 | ✅ 静默忽略 |
| `SET hive.xxx=yyy` | 忽略 | ✅ 静默忽略 |
| `SETPROJECT xxx=yyy` | 忽略 | ✅ 静默忽略 |
| `SELECT * EXCEPT(col1, col2)` | 直接执行 | ✅ 透传 DataFusion |
| `SELECT * REPLACE(expr AS col)` | 直接执行 | ✅ 透传 DataFusion |
| `TABLESAMPLE(N PERCENT)` | 直接执行 | ✅ 透传 |
| `QUALIFY ...` | 直接执行 | ✅ 透传 |
| `LATERAL VIEW explode(col)` | 不支持 | ❌ Phase 2 |
| `SELECT TRANSFORM(...) USING 'script'` | 不支持 | ❌ 高级特性 |

#### DDL 扩展

| MaxCompute 语法 | 处理方式 | 状态 |
|----------------|---------|------|
| `CREATE TABLE new LIKE existing` | 剥离 MC 后缀，透传 | ✅ |
| `CREATE TABLE t AS SELECT ...` (CTAS) | 剥离 MC 子句 (LIFECYCLE 等) | ✅ |

---
---

## Hologres 兼容性

### 协议兼容

| 功能 | 状态 | 说明 |
|------|------|------|
| PostgreSQL v3 Wire Protocol | ✅ | 标准 PG 协议 |
| MD5 认证 | ✅ | AccessKey ID/Secret |
| Simple Query | ✅ | 'Q' 消息 |
| Extended Query (Parse/Bind/Execute) | ✅ | Phase 1 完成 |
| Close (portal/statement) | ✅ | 支持 portal/statement 关闭 |
| Describe (portal/statement) | ✅ | RowDescription / NoData |
| Flush / Sync | ✅ | 协议同步 |
| FunctionCall | ❌ | 罕见, 未实现 |
| SSL | ❌ | 返回 'N' 拒绝 |
| CancelRequest | ❌ | Phase 2 |
| pg_catalog 系统表 | ✅ | Phase 1 基础模拟完成 |

### 认证方式

| 方式 | 算法 | Status | 说明 |
|------|------|--------|------|
| MD5 密码认证 | md5({password}{user}) | ✅ | 默认认证方式 |
| 明文密码认证 | — | ❌ | 未实现 |
| SCRAM-SHA-256 | — | ❌ | Phase 2 |

默认凭据: `username = "roris"`, `password = "roris-secret"`

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
| JSON / JSONB | STRING | ✅ 映射到 STRING |
| INT[] / TEXT[] | INT / STRING | ✅ 剥离数组标记 |
| UUID | UUID | ✅ 透传 |
| TIME | TIME | ✅ 透传 |
| TIME WITH TIME ZONE / TIMETZ | TIME | ✅ 透传 |
| INTERVAL | INTERVAL | ✅ 透传 |
| MONEY | MONEY | ✅ 透传 |
| BIT(n) / BIT VARYING(n) | BIT | ✅ 透传 |
| INET / CIDR / MACADDR | INET | ✅ 透传 |
| POINT / LINE / LSEG / BOX | POINT | ✅ 透传 |
| TSVECTOR / TSQUERY | TSVECTOR | ✅ 透传 |
| HLL / ROARINGBITMAP | HLL | ✅ Hologres 特有类型透传 |
| SERIAL | INT | ⚠️ 无自增 |
| BIGSERIAL | BIGINT | ⚠️ 无自增 |

### Hologres 特有 DDL

| Hologres 语法 | 处理方式 | 状态 |
|--------------|---------|------|
| `CREATE TABLE (...) WITH (orientation='column', ...)` | 剥离 WITH 子句 | ✅ 静默忽略 |
| `CREATE TABLE t WITH (...) AS SELECT ...` (CTAS) | 剥离 WITH 子句 | ✅ 静默忽略 |
| `CALL set_table_property(...)` | 忽略 | ✅ 静默忽略 |
| `CALL set_table_group(...)` | 忽略 | ✅ 静默忽略 |
| `PARTITION BY LIST(col)` | 剥离 | ✅ |
| `CREATE TABLE child PARTITION OF parent` | 忽略 | ✅ |
| `CREATE INDEX idx ON t USING bitmap(col)` | 转为 `CREATE INDEX idx ON t(col)` | ✅ |
| `ALTER TABLE t SET (orientation='column')` | 剥离 SET 子句 | ✅ 静默忽略 |
| `CREATE FOREIGN TABLE ...` | 透传 + 类型映射 | ✅ |
| `INSERT OVERWRITE TABLE t SELECT ...` | 转为 INSERT INTO | ✅ 自动转换 |
| `COPY t FROM STDIN / TO STDOUT` | 透传 | ✅ |
| `DROP TABLE [IF EXISTS]` | 直接执行 | ✅ |

### 不支持的 PG 特性

| 功能 | 状态 | 错误信息 |
|------|------|---------|
| `CREATE TRIGGER` | ❌ | Hologres 不支持触发器 |
| `CREATE FUNCTION` | ❌ | Phase 1 不支持 |
| `WITH RECURSIVE` | ❌ | Hologres 不支持递归 CTE |
| `SELECT ... FOR UPDATE` | ❌ | Hologres 无行级锁 |
| `CREATE EXTENSION` | ⚠️ | 静默忽略 |
| `CREATE DOMAIN` | ❌ | 不支持 |
| `LISTEN/NOTIFY` | ❌ | 不支持 |
| `EXPLAIN ANALYZE` | ⚠️ | 转为 EXPLAIN |
| `DISTINCT ON` | ❌ | 不支持 |
| `INSERT ON CONFLICT` | ⚠️ | 剥离 ON CONFLICT |

### pg_catalog 兼容

| 系统表/视图 | 状态 | 说明 |
|------------|------|------|
| `pg_tables` | ✅ | 模拟返回 |
| `pg_class` | ✅ | 模拟返回主要列 |
| `pg_namespace` | ✅ | 映射到 database |
| `pg_attribute` | ✅ | 映射到 column |
| `pg_user` / `pg_roles` | ✅ | 模拟返回 |
| `hg_stat_activity` | ✅ | Hologres 特有视图已实现 |
| `version()` | ✅ | 返回 "PostgreSQL 15.x (RorisDB)" |
| `current_schema()` | ✅ | 返回当前 schema |
| `current_database()` | ✅ | 返回当前 database |
| `pg_typeof()` | ✅ | 类型推断 |

---
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
./target/release/roris-fe --mysql-port 9030 --hologres-port 15432

# 使用 psql 连接
psql -h 127.0.0.1 -p 15432 -U roris -d default

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
| (TCP :9030)    |  | Protocol (HTTP)  |  | (TCP :15432)        |
| [已支持]       |  | :9031            |  | Hologres 兼容       |
+----------------+  +------------------+  +---------------------+
                           |                       |
                    MC SQL Translator       Hologres SQL Translator
                    (strip MC-specific)     (strip PG-unsupported,
                                             add set_table_property)
```

## 认证架构

```
MySQL:
  Native Password / Caching SHA2 → Challenge-response handshake

MaxCompute:
  V2 Signing:  HMAC-SHA1(method + path + query + headers + body)
  V4 Signing:  HMAC-SHA256(region + service + signed_headers + payload_hash)

Hologres / PG:
  MD5:  md5(password + user) stored, challenge-response: md5(md5(pw+user) + salt)
```