# RorisDB Database Chameleon - 兼容性矩阵

RorisDB 作为通用数据库仿真底座，支持模拟多种数据库协议和 SQL 方言。

## Phase 1 & Phase 2 完成状态

Phase 1 (REST API) 和 Phase 2 (Tunnel 协议) 均已实现。所有核心端点、认证、SQL 翻译和批量数据传输功能均已完成。

**测试覆盖**: 1780 测试全部通过 (1052 单元测试 + 728 集成测试)
- Phase 1: 184 单元测试 + 9 集成测试 = 193 测试
- Phase 2 (Tunnel): 55 单元测试 = 55 测试
- SQL Translators (MaxCompute): 125 单元测试 = 125 测试
- 其他: 剩余测试 (MySQL/PG/Hologres/FE 等)

## 支持的数据库协议

| 数据库 | 协议类型 | 端口 | 状态 | 说明 |
|--------|---------|------|------|------|
| MySQL | TCP 二进制 (MySQL Wire Protocol) | 9030 | ✅ 已支持 | 基础协议 |
| MaxCompute (ODPS) | HTTP/REST + XML + Tunnel | 9031 | ✅ Phase 1 + Phase 2 完成 | 阿里云离线大数据 |
| Hologres | TCP 二进制 (PostgreSQL v3) | 15432 | ✅ Phase 1 完成 | 阿里云实时数仓 |

## 端口配置

| 服务 | 默认端口 | CLI 参数 | 说明 |
|------|---------|---------|------|
| MySQL Wire Protocol | 9030 | `--mysql-port` | 基础 MySQL 协议 |
| MaxCompute REST API | 9031 | `--maxcompute-port` | HTTP REST + Tunnel |
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
| JSON 请求/响应 | ✅ | Tunnel 协议使用 JSON |
| SQL 作业提交 | ✅ | 异步提交 + 轮询结果 |
| Instance 管理 | ✅ | 状态查询/结果获取/停止 |
| 表删除 (REST API) | ✅ | DELETE /api/projects/{project}/tables/{table} |
| Tunnel 上传协议 | ✅ | 创建上传 → 上传 Block → 提交 |
| Tunnel 下载协议 | ✅ | 创建下载 → 获取行范围数据 |
| Tunnel Session 重载 | ✅ | GET 查询上传/下载会话状态 |
| Tunnel 端点发现 | ✅ | GET /api/projects/{project}/tunnel |
| Tunnel 压缩 | ✅ | ZLIB/deflate 压缩/解压 |
| Tunnel Protobuf 编码 | ✅ | 自定义线格式编码器/解码器 |
| Stream Upload | ❌ | 流式上传 |
| Upsert | ❌ | 部分更新 |

### 认证方式

MaxCompute 协议支持两种签名方式：

| 方式 | 算法 | 实现状态 | 说明 |
|------|------|---------|------|
| V2 签名 | HMAC-SHA1 | ✅ | `Authorization: ODPS {ak}:{signature}` |
| V4 签名 | HMAC-SHA256 | ✅ | `Authorization: ODPS2-HMAC-SHA256 ...` |

默认凭据: `AccessKey ID = "roris"`, `AccessKey Secret = "roris-secret"`

### Tunnel 协议详情

#### REST 端点

| 操作 | 方法 | URL 模式 | 状态 | 说明 |
|------|------|---------|------|------|
| 端点发现 | GET | `/api/projects/{project}/tunnel` | ✅ | 返回 Tunnel 服务地址 |
| 创建上传会话 | POST | `.../tables/{table}?uploads` | ✅ | 返回 UploadID + Schema |
| 上传数据块 | PUT | `.../tables/{table}?uploadid={id}&blockid={n}` | ✅ | Protobuf 二进制数据 |
| 提交上传 | POST | `.../tables/{table}?uploadid={id}` | ✅ | 将数据插入目标表 |
| 创建下载会话 | POST | `.../tables/{table}?downloads` | ✅ | 返回 DownloadID + RecordCount |
| 下载数据 | GET | `.../tables/{table}?downloadid={id}&rowrange=(start,count)` | ✅ | Protobuf 二进制响应 |
| 重载上传会话 | GET | `...?uploadid={id}` | ✅ | 返回状态 + 已上传块列表 |
| 重载下载会话 | GET | `...?downloadid={id}` | ✅ | 返回状态 + 记录数 |

#### 数据格式

Tunnel 使用自定义 Protobuf-like 线格式（非标准 Protobuf）：

| 编码特性 | 状态 | 说明 |
|------|------|------|
| Tag 编码 `(field << 3) \| wire_type` | ✅ | Varint 标签 |
| VARINT 线类型 (0) | ✅ | 整数、ZigZag 编码 |
| FIXED64 线类型 (1) | ✅ | DOUBLE、LONG |
| LENGTH_DELIMITED 线类型 (2) | ✅ | 字符串、二进制 |
| FIXED32 线类型 (5) | ✅ | FLOAT |
| ZigZag 编码/解码 | ✅ | 有符号整数优化 |
| Varint 编码/解码 | ✅ | 变长整数 |
| 每记录 CRC32C | ✅ | TUNNEL_END_RECORD 标记 |
| 流脚注 (记录数 + CRC32C) | ✅ | TUNNEL_META_COUNT + TUNNEL_META_CHECKSUM |
| NULL 值编码 | ✅ | null_count + null_indices 前缀 |
| 支持的类型编码 | ✅ | BIGINT/INT/SMALLINT/TINYINT/BOOLEAN/FLOAT/DOUBLE/STRING/BINARY/DATETIME/DATE/DECIMAL |

#### 压缩支持

| 算法 | 上传 | 下载 | 说明 |
|------|------|------|------|
| RAW (无压缩) | ✅ | ✅ | 默认 |
| ZLIB/deflate | ✅ | ✅ | `Content-Encoding: deflate` / `Accept-Encoding: deflate` |
| SNAPPY | ❌ | ❌ | Phase 3 |
| ZSTD | ❌ | ❌ | Phase 3 |
| LZ4 | ❌ | ❌ | Phase 3 |

#### Session 管理

| 特性 | 配置 | 说明 |
|------|------|------|
| 最大 Session 数 | 1,000 | 超出时驱逐最旧条目 |
| 驱逐目标 | 800 | 80% 容量 |
| TTL | 3600 秒 | 1 小时后自动清理 |
| Block 存储 | 内存 (BTreeMap) | Block ID 有序存储 |
| 下载缓存 | 内存 (SELECT * 结果) | 单节点 Phase 2 接受限制 |

### 数据类型兼容

| MaxCompute 类型 | RorisDB 映射 | REST | Tunnel 编码 |
|----------------|-------------|------|------------|
| BIGINT | BIGINT | ✅ | ✅ ZigZag varint |
| INT | INT | ✅ | ✅ ZigZag varint |
| SMALLINT | SMALLINT | ✅ | ✅ ZigZag varint |
| TINYINT | TINYINT | ✅ | ✅ ZigZag varint |
| STRING | STRING | ✅ | ✅ Length-delimited |
| STRING(n) | VARCHAR(n) | ✅ | ✅ Length-delimited |
| DOUBLE | DOUBLE | ✅ | ✅ Fixed64 LE |
| FLOAT | FLOAT | ✅ | ✅ Fixed32 LE |
| DECIMAL(p,s) | DECIMAL(p,s) | ✅ | ✅ Length-delimited (string) |
| BOOLEAN | BOOLEAN | ✅ | ✅ Varint (0/1) |
| DATETIME | DATETIME | ✅ | ✅ ZigZag varint (ms) |
| DATE | DATE | ✅ | ✅ ZigZag varint (days) |
| TIMESTAMP | TIMESTAMP | ✅ | ✅ Varint (ms) |
| BINARY | BLOB | ✅ | ✅ Length-delimited (hex) |
| ARRAY<T> | ARRAY | ✅ | ❌ Tunnel |
| MAP<K,V> | MAP | ✅ | ❌ Tunnel |
| STRUCT<...> | STRUCT | ✅ | ❌ Tunnel |

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
| `MERGE INTO ...` | 透传 | ✅ 透传解析器 |
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
| `SET spark.xxx=yyy` | 忽略 | ✅ 静默忽略 |
| `SET mapreduce.xxx=yyy` | 忽略 | ✅ 静默忽略 |
| `SET any_key=value` | 忽略 | ✅ 所有 SET key=value 均为 no-op |
| `SETPROJECT xxx=yyy` | 忽略 | ✅ 静默忽略 |
| `SELECT * EXCEPT(col1, col2)` | 直接执行 | ✅ 透传 DataFusion |
| `SELECT * REPLACE(expr AS col)` | 直接执行 | ✅ 透传 DataFusion |
| `TABLESAMPLE(N PERCENT)` | 直接执行 | ✅ 透传 |
| `QUALIFY ...` | 直接执行 | ✅ 透传 |
| `LATERAL VIEW explode(col) t AS alias` | 转为 CROSS JOIN UNNEST | ✅ 自动转换 |
| `SELECT TRANSFORM(...) USING 'script'` | No-op | ✅ 接受但不执行 |

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

### 已兼容的 PG 特性（no-op 模式）

| 功能 | 状态 | 说明 |
|------|------|------|
| `CREATE TRIGGER` | ✅ | No-op，接受但不执行 |
| `CREATE DOMAIN` | ✅ | No-op，接受但不执行 |
| `LISTEN/NOTIFY` | ✅ | No-op，接受但不执行 |
| `CREATE EXTENSION` | ✅ | No-op，接受但不执行 |
| `SELECT ... FOR UPDATE` | ✅ | 剥离 FOR UPDATE 子句（no-op） |
| `WITH RECURSIVE ...` | ✅ | DataFusion 支持递归 CTE |
| `CREATE FUNCTION ...` | ✅ | 透传解析器 |
| `DISTINCT ON (col)` | ✅ | 透传解析器 |
| `GRANT / REVOKE` | ✅ | No-op，接受但不执行 |
| `CREATE POLICY / ALTER POLICY` | ✅ | No-op，接受但不执行 |
| `CALL refresh_materialized_view(...)` | ✅ | No-op，接受但不执行 |

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

# 使用 pyodps 连接 (REST API + SQL)
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

# 使用 pyodps Tunnel (批量上传/下载)
python3 <<EOF
from odps import ODPS
from odps.tunnel import TableTunnel

o = ODPS('roris', 'roris-secret', 'default',
         endpoint='http://127.0.0.1:9031/api')

tunnel = TableTunnel(o)

# 批量上传
upload_session = tunnel.create_upload_session('users')
with upload_session.open_record_writer(0) as writer:
    record = upload_session.new_record()
    record[0] = 1    # id
    record[1] = 'Alice'  # name
    record[2] = 25   # age
    writer.write(record)

upload_session.commit([0])

# 批量下载
download_session = tunnel.create_download_session('users')
with download_session.open_record_reader() as reader:
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
+----------------+  |                  |  +---------------------+
                    | REST API + Tunnel|
                    | SQL Translator   |
                    | Session Manager  |
                    +------------------+
                           |
                    +------------------+
                    | Tunnel Protocol  |
                    | Upload/Download  |
                    | Protobuf Codec   |
                    | CRC32C + ZLIB    |
                    +------------------+
```

## 认证架构

```
MySQL:
  Native Password / Caching SHA2 → Challenge-response handshake

MaxCompute:
  V2 Signing:  HMAC-SHA1(method + path + query + headers + body)
  V4 Signing:  HMAC-SHA256(region + service + signed_headers + payload_hash)
  Tunnel:      复用 V2/V4 签名 (同一 Authorization 头)

Hologres / PG:
  MD5:  md5(password + user) stored, challenge-response: md5(md5(pw+user) + salt)
```

## Tunnel 协议线格式详情

### 记录编码

```
每条记录:
  [null_count (varint)]           ← 空值列数
  [null_column_index (varint)×n]  ← 空值列索引 (0-based)
  对于每个非空列:
    [tag: varint(field_number << 3 | wire_type)]
    [value: 类型相关编码]
  [TUNNEL_END_RECORD tag: 0x01FFFFE0]  ← 记录终止
  [per_record_crc: uint32 LE]          ← CRC32C

流脚注:
  [TUNNEL_META_COUNT tag: 0x01FFFFFE]
  [record_count (varint)]
  [TUNNEL_META_CHECKSUM tag: 0x02000000]
  [overall_crc32c (uint32 LE)]         ← 所有 per-record CRC 的 CRC
```

### 常量定义

```
TUNNEL_VERSION       = 6
TUNNEL_END_RECORD    = 33,553,408  (0x01FFFFE0)
TUNNEL_META_COUNT    = 33,554,430  (0x01FFFFFE)
TUNNEL_META_CHECKSUM = 33,554,431  (0x02000000)
```

---

## 参考来源

### SDK 源码

MaxCompute Tunnel 协议的实现基于以下开源 SDK 源码分析：

- **[pyodps](https://github.com/aliyun/aliyun-odps-python-sdk)** — Python SDK
  - `odps/tunnel/base.py` — Tunnel 基类、端点发现
  - `odps/tunnel/tabletunnel.py` — 所有会话类型和 API 调用
  - `odps/tunnel/io/writer.py` — 记录/Arrow 写入器 (Protobuf 编码)
  - `odps/tunnel/io/reader.py` — 记录/Arrow 读取器 (Protobuf 解码)
  - `odps/tunnel/io/stream.py` — 压缩选项和流处理
  - `odps/tunnel/pb/encoder.py` — Protobuf 编码器
  - `odps/tunnel/pb/decoder.py` — Protobuf 解码器
  - `odps/tunnel/pb/wire_format.py` — 线格式常量和工具函数
  - `odps/tunnel/pb/output_stream.py` — Varint/LittleEndian 编码
  - `odps/tunnel/wireconstants.py` — TUNNEL_END_RECORD 等常量
  - `odps/tunnel/checksum.py` — CRC32C 校验和实现
  - `odps/tunnel/errors.py` — 错误解析 (XML 和 JSON)
  - `odps/rest.py` — REST 客户端、请求构建
  - `odps/accounts.py` — 认证 (V2/V4 签名)

- **[aliyun-odps-java-sdk](https://github.com/aliyun/aliyun-odps-java-sdk)** — Java SDK
  - `TableTunnel.java` — 所有会话类型 (内部 UploadSession/DownloadSession 类)
  - `TunnelConstants.java` — 所有参数名和常量
  - `HttpHeaders.java` — 所有 HTTP 头名称
  - `TunnelException.java` — 错误响应解析 (JSON)
  - `SessionBase.java` — 基础会话 HTTP 请求逻辑
  - `Configuration.java` — Tunnel 配置
  - `GeneralConfiguration.java` — URL 构建
  - `ResourceBuilder.java` — REST 资源 URL 模式
  - `Util.java` — 通用头部

### 官方文档

- [阿里云 MaxCompute Tunnel 文档](https://help.aliyun.com/zh/maxcompute/user-guide/tunnel-commands)
- [MaxCompute Tunnel REST API 参考](https://help.aliyun.com/zh/maxcompute/user-guide/tunnel-api/)
- [DataWorks 数据集成](https://help.aliyun.com/zh/dataworks/user-guide/maxcompute-connector)

### CRC32C

- [CRC32C (Castagnoli)](https://en.wikipedia.org/wiki/Cyclic_redundancy_check#Polynomial_representations_of_cyclic_redundancy_checks) — 使用 `crc32fast` Rust crate 实现

### Protobuf 线格式

- [Protocol Buffers Encoding](https://protobuf.dev/programming-guides/encoding/) — 线格式规范 (字段标签、ZigZag 编码)
