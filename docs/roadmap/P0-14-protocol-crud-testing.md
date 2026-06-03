# P0-14-protocol-crud-testing.md

# P0: 14种数据库协议 CRUD 综合测试

## 背景

项目当前引入了14种数据库协议，但测试非常不充分。现有集成测试全部只走MySQL协议（端口9030），其余13种协议虽有完整实现但未被测试覆盖。

## 14种协议清单

| # | 协议 | 端口 | 成熟度 | 当前状态 |
|---|------|------|--------|----------|
| 1 | mysql-protocol | 9030 | ✅ 生产级 | 已启动，有集成测试 |
| 2 | pg-protocol | 15432 | ✅ 生产级 | 已启动，无独立测试 |
| 3 | maxcompute-protocol | 9031 | ✅ 生产级 | 已启动，无独立测试 |
| 4 | redis-protocol | 6379 | ✅ 功能完整 | 未启动，无测试 |
| 5 | mongodb-protocol | 27017 | ✅ 功能完整 | 未启动，无测试 |
| 6 | clickhouse-protocol | 8123 | ✅ 功能完整 | 未启动，无测试 |
| 7 | elasticsearch-protocol | 9200 | ✅ 功能完整 | 未启动，无测试 |
| 8 | influxdb-protocol | 8086 | ✅ 功能完整 | 未启动，无测试 |
| 9 | cassandra-protocol | 9042 | 🚧 简化版 | 未启动，无测试 |
| 10 | oracle-protocol | 1521 | 🚧 只读模拟 | 未启动，无测试 |
| 11 | tablestore-protocol | 8087 | ✅ 功能完整 | 未启动，无测试 |
| 12 | adb-mysql-protocol | 8124 | 🚧 简化版 | 未启动，无测试 |
| 13 | lindorm-protocol | 7070 | 🚧 简化版 | 未启动，无测试 |
| 14 | vector-protocol | 9032 | 🚧 简化版 | 未启动，无测试 |

## 测试策略

### 阶段一：真实应用测试（MySQL/PG协议）

选取GitHub上5个流行应用，覆盖3种开发语言，通过MySQL和PostgreSQL协议连接HarnessDB进行全量CRUD测试：

| 应用 | 语言 | 协议 | 测试重点 |
|------|------|------|----------|
| **WordPress** | PHP | MySQL | CREATE TABLE, INSERT, SELECT WHERE, UPDATE, DELETE |
| **Grafana** | Go | MySQL + PG | SHOW VARIABLES, SET, SELECT with time filter |
| **Apache Superset** | Python | MySQL + PG | SHOW DATABASES, complex SELECT, JOIN |
| **Airbyte** | Python/Java | MySQL | ETL CRUD: CREATE, INSERT, CDC SELECT, DROP |
| **Metabase** | Clojure | MySQL + PG | 数据库探测, SELECT, CRUD operations |

### 阶段二：协议专用CRUD测试

对剩余11种协议，编写专用测试脚本验证基础CRUD：

**Redis** (`tests/real_world_scenarios/redis_crud_test.sh`):
- String: SET/GET/DEL/INCR/DECR/MSET/MGET
- Hash: HSET/HGET/HDEL/HGETALL
- List: LPUSH/RPUSH/LPOP/RPOP/LRANGE
- Set: SADD/SMEMBERS/SISMEMBER/SREM
- Key: KEYS/EXISTS/TTL/EXPIRE/RENAME
- Server: PING/INFO/DBSIZE/FLUSHDB

**MongoDB** (`tests/real_world_scenarios/mongodb_crud_test.sh`):
- Connection: ping, ismaster/hello, buildInfo
- CRUD: insertOne/findOne/updateOne/deleteOne
- Collection: createCollection, drop, listCollections
- Query: $eq, $gt, $lt, $in, $regex
- Database: listDatabases, createDatabase

**ClickHouse** (`tests/real_world_scenarios/clickhouse_crud_test.sh`):
- DDL: CREATE TABLE, DROP TABLE, DESCRIBE
- DML: INSERT, SELECT with WHERE/ORDER BY/LIMIT/GROUP BY
- System: SELECT version(), SHOW DATABASES, SHOW TABLES

**Elasticsearch** (`tests/real_world_scenarios/elasticsearch_crud_test.sh`):
- Index: PUT/GET/DELETE index
- Document: POST/GET/PUT/DELETE document
- Search: _search with query, _cluster/health, _cat/indices

**InfluxDB** (`tests/real_world_scenarios/influxdb_crud_test.sh`):
- Write: line protocol (measurement,tags field=value timestamp)
- Query: SELECT, SHOW DATABASES, CREATE/DROP DATABASE
- Retention: CREATE/DROP RETENTION POLICY

**Cassandra** (`tests/real_world_scenarios/cassandra_crud_test.sh`):
- DDL: CREATE KEYSPACE, CREATE TABLE
- DML: INSERT, SELECT, DELETE
- System: DESCRIBE KEYSPACES, DESCRIBE TABLES

**TableStore** (`tests/real_world_scenarios/tablestore_crud_test.sh`):
- Table: CreateTable, ListTables, DeleteTable
- Row: PutRow, GetRow, DeleteRow, GetRange

**Oracle** (`tests/real_world_scenarios/oracle_crud_test.sh`):
- SELECT FROM DUAL, SYSDATE, USER, VERSION

**PostgreSQL** (`tests/real_world_scenarios/pg_crud_test.sh`):
- DDL: CREATE DATABASE, CREATE TABLE, DROP TABLE
- DML: INSERT, SELECT, UPDATE, DELETE
- System: SELECT version(), SHOW server_version, SHOW ALL

**MaxCompute** (`tests/real_world_scenarios/maxcompute_crud_test.sh`):
- REST: ListProject, CreateTable, CreateInstance
- Tunnel: Upload, Download
- SQL: SELECT, INSERT via SQLTask

### 阶段三：集成验证

将所有协议服务器集成到 `fe_main.rs` 的启动流程，确保：
1. 所有14种协议可同时启动
2. 每种协议响应基础健康检查（ping）
3. 跨协议CRUD数据一致性验证

## 成功标准

- ✅ 5个真实应用通过MySQL/PG协议的完整CRUD测试
- ✅ 14种协议全部通过各自的CRUD测试脚本
- ✅ 所有协议服务器可在 `fe_main.rs` 中一键启动
- ✅ 测试覆盖率报告（每个协议的API endpoint覆盖率）
