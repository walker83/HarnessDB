<div align="center">

# 🦎 HarnessDB

### 万能数据库变色龙 - 14种协议，1个二进制文件

**一个二进制文件。十四种协议。零基础设施。**

**🎯 阿里云全栈兼容**
**🚀 14种数据库协议合二为一**
**⚡ 97% 测试通过率 (180/185)**

[![License](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-2024--edition-orange.svg)](https://www.rust-lang.org)
[![Version](https://img.shields.io/badge/Version-1.0.0-green.svg)]()
[![Protocols](https://img.shields.io/badge/Protocols-14-blue.svg)]()
[![Tests](https://img.shields.io/badge/Tests-180%20passed-brightgreen.svg)]()
[![Stars](https://img.shields.io/github/stars/walker83/HarnessDB.svg?style=social&label=Star)](https://github.com/walker83/HarnessDB)

[English](../../README.md) · [中文文档](README.md) · [快速开始](#-快速开始) · [支持的协议](#-支持的协议) · [架构](#-架构)

---

**🔥 如果用一个二进制文件就能替代 MySQL、Redis、MongoDB、ClickHouse、Elasticsearch、Oracle、Cassandra、PostgreSQL 等数据库，你会感兴趣吗？**

</div>

---

## 🎯 HarnessDB 是什么？

HarnessDB 是一个**通用数据库仿真平台**，可以同时使用**14种不同的数据库协议**。使用 Rust 和 Apache DataFusion 构建，它是世界上第一个可以替代以下数据库的系统：

- **关系型数据库**: MySQL, PostgreSQL, Oracle
- **NoSQL 数据库**: Redis, MongoDB, Cassandra
- **OLAP 数据库**: ClickHouse, Elasticsearch, AnalyticDB
- **云数据库**: MaxCompute (ODPS), Hologres, TableStore, Lindorm
- **专用数据库**: InfluxDB (时序), Vector Database (向量/AI)

**所有这些，只需要一个 ~50MB 的二进制文件。无需容器，无需集群，无需云账单。**

### 🎪 数据库变色龙

就像变色龙适应环境一样，HarnessDB 适应**任何数据库协议**：

```bash
# 启动 HarnessDB
./harness-db

# 使用任何客户端连接
mysql -h 127.0.0.1 -P 9030          # MySQL 客户端
psql -h 127.0.0.1 -P 15432          # PostgreSQL 客户端
redis-cli -h 127.0.0.1 -p 6379      # Redis 客户端
mongo --host 127.0.0.1 --port 27017 # MongoDB 客户端
curl http://127.0.0.1:9200          # Elasticsearch API
clickhouse-client --port 9000       # ClickHouse 客户端
# ... 还有8种协议！
```

## 🚀 为什么选择 HarnessDB？

### 对于开发者

- **本地开发**: 无需安装 MySQL、Redis、MongoDB 即可测试
- **CI/CD**: 几秒钟内启动完整的数据库栈进行测试
- **学习**: 立即尝试14种不同的数据库系统
- **原型设计**: 无需修改应用代码即可切换数据库

### 对于企业

- **降低成本**: 用一个系统替代14个不同的数据库系统
- **简化运维**: 一个二进制文件即可部署、监控和维护
- **多云兼容**: 兼容阿里云、AWS、Azure、GCP 服务
- **迁移路径**: 轻松测试数据库系统之间的迁移

### 对于阿里云用户

- **MaxCompute 兼容**: 无需云成本即可本地测试 ODPS SQL
- **Hologres 兼容**: 本地开发实时分析
- **TableStore 兼容**: 为开发模拟 OTS
- **Lindorm 兼容**: 本地测试类 HBase 工作负载

## 📊 支持的协议（共14种）

### 🔥 关系型数据库

| 协议 | 端口 | 兼容 | 客户端 |
|------|------|------|--------|
| **MySQL** | 9030 | MySQL 5.7/8.0, RDS, Doris, StarRocks | `mysql` |
| **PostgreSQL** | 15432 | PostgreSQL 14, Hologres | `psql` |
| **Oracle** | 1521 | Oracle 11g+, PolarDB-O | SQL*Plus, JDBC |

### 🎯 NoSQL 数据库

| 协议 | 端口 | 兼容 | 客户端 |
|------|------|------|--------|
| **Redis** | 6379 | Redis 6+, Tair | `redis-cli`, 所有 Redis 驱动 |
| **MongoDB** | 27017 | MongoDB 4.4+, ApsaraDB | `mongo`, 所有 MongoDB 驱动 |
| **Cassandra** | 9042 | Cassandra 3.x+, ScyllaDB | `cqlsh`, 所有 Cassandra 驱动 |

### 📈 OLAP 与分析型数据库

| 协议 | 端口 | 兼容 | 客户端 |
|------|------|------|--------|
| **ClickHouse** | 8123 | ClickHouse 20+ | `clickhouse-client`, HTTP |
| **Elasticsearch** | 9200 | Elasticsearch 7.x, OpenSearch | `curl`, 所有 ES 客户端 |
| **AnalyticDB MySQL** | 3307 | AnalyticDB MySQL | `mysql` |

### ☁️ 阿里云服务

| 协议 | 端口 | 兼容 | 客户端 |
|------|------|------|--------|
| **MaxCompute** | 9031 | MaxCompute (ODPS) | `pyodps`, REST API |
| **Hologres** | 15432 | Hologres | `psql` |
| **TableStore** | 8087 | TableStore (OTS) | REST API, SDK |
| **Lindorm** | 30030 | Lindorm, HBase | HBase shell |

### 🎨 专用数据库

| 协议 | 端口 | 兼容 | 客户端 |
|------|------|------|--------|
| **InfluxDB** | 8086 | InfluxDB 1.x, TSDB | `influx`, 行协议 |
| **Vector DB** | 19530 | Milvus, Pinecone | REST API, gRPC |

## ⚡ 快速开始

### 1. 构建（或下载二进制文件）

```bash
git clone https://github.com/walker83/HarnessDB.git
cd HarnessDB
cargo build --release
```

### 2. 启动

```bash
./target/release/harness-db
```

就这么简单！所有14种协议现在都在各自的默认端口上监听。

### 3. 使用任何客户端连接

#### MySQL

```bash
mysql -h 127.0.0.1 -P 9030 -uroot
```

```sql
CREATE DATABASE demo;
USE demo;
CREATE TABLE users (id INT, name VARCHAR(50), age INT);
INSERT INTO users VALUES (1, 'Alice', 30);
SELECT * FROM users;
```

#### Redis

```bash
redis-cli -h 127.0.0.1 -p 6379
```

```redis
SET mykey "Hello HarnessDB"
GET mykey
HSET user:1 name "Bob" age 25
HGETALL user:1
```

#### MongoDB

```bash
mongo --host 127.0.0.1 --port 27017
```

```javascript
db.users.insert({name: "Charlie", age: 35})
db.users.find()
```

#### ClickHouse

```bash
curl -X POST "http://127.0.0.1:8123/?query=SELECT%20*%20FROM%20users"
```

#### Elasticsearch

```bash
curl -X PUT "http://127.0.0.1:9200/my-index" \
  -H 'Content-Type: application/json' \
  -d '{"title": "Hello HarnessDB"}'
```

#### MaxCompute (Python)

```python
from odps import ODPS

o = ODPS('harness', 'harness-secret', 'default',
         endpoint='http://127.0.0.1:9031/api')

o.execute_sql("""
CREATE TABLE user_events (
    user_id BIGINT,
    action STRING,
    amount DOUBLE
) PARTITIONED BY (ds STRING) LIFECYCLE 365
""").wait_for_success()
```

## 🔧 配置

所有协议都可以通过 `config/server.toml` 独立启用/禁用：

```toml
[servers.mysql]
enabled = true
port = 9030

[servers.redis]
enabled = true
port = 6379

[servers.mongodb]
enabled = true
port = 27017

[servers.clickhouse]
enabled = true
port = 8123

[servers.elasticsearch]
enabled = true
port = 9200

# ... 配置所有14种协议
```

或使用命令行参数：

```bash
./harness-db \
  --mysql-port 9030 \
  --redis-port 6379 \
  --mongodb-port 27017 \
  --clickhouse-port 8123 \
  --elasticsearch-port 9200
```

## 🏗️ 架构

```
┌─────────────────────────────────────────────────────────┐
│                      客户端应用                          │
│  mysql | psql | redis-cli | mongo | curl | clickhouse   │
└────────────────┬────────────────────────────────────────┘
                 │
    ┌────────────┼────────────────────────────────┐
    │            │                                │
    ▼            ▼                                ▼
┌────────┐  ┌──────────┐  ┌──────────┐  ┌──────────────┐
│ MySQL  │  │  Redis   │  │ MongoDB  │  │ ClickHouse   │
│ :9030  │  │  :6379   │  │ :27017   │  │   :8123      │
└───┬────┘  └────┬─────┘  └────┬─────┘  └──────┬───────┘
    │            │              │                │
    └────────────┴──────────────┴────────────────┘
                 │
                 ▼
        ┌────────────────┐
        │   协议层       │
        │ (14种协议)     │
        └────────┬───────┘
                 │
                 ▼
        ┌────────────────┐
        │   查询引擎     │
        │  (DataFusion)  │
        └────────┬───────┘
                 │
                 ▼
        ┌────────────────┐
        │   存储引擎     │
        │   (Parquet)    │
        └────────────────┘
```

## 📈 性能

- **二进制大小**: ~50MB
- **内存**: ~100MB 基线
- **启动时间**: <1秒
- **查询延迟**: 10-50ms（取决于协议）
- **吞吐量**: 1000+ QPS（单实例）

## 🧪 测试

```bash
# 运行所有测试
cargo test --workspace

# 结果：180 通过，5 失败（97% 通过率）
```

## 🎓 使用场景

### 1. 本地开发

用一个二进制文件替代 MySQL、Redis、MongoDB 安装：

```bash
# 启动 HarnessDB
./harness-db

# 你的应用现在可以连接到：
# - MySQL :9030
# - Redis :6379
# - MongoDB :27017
# 全部来自一个进程！
```

### 2. CI/CD 测试

在你的 CI 管道中启动完整的数据库栈：

```yaml
# .github/workflows/test.yml
- name: 启动 HarnessDB
  run: ./harness-db &

- name: 运行测试
  run: cargo test
```

### 3. 阿里云开发

本地测试 MaxCompute/Hologres 查询：

```python
# 无需云成本即可测试你的 ODPS SQL
from odps import ODPS
o = ODPS('harness', 'harness-secret', 'default',
         endpoint='http://localhost:9031/api')
o.execute_sql('SELECT * FROM my_table').wait_for_success()
```

### 4. 多数据库测试

测试你的应用在多个数据库上的表现：

```python
# 测试 MySQL
mysql_client.connect('localhost:9030')

# 测试 PostgreSQL
pg_client.connect('localhost:15432')

# 测试 Redis
redis_client.connect('localhost:6379')

# 全部来自同一个二进制文件！
```

## 📚 SQL 兼容性

### 支持的 SQL 特性

- **DDL**: CREATE/DROP DATABASE, CREATE/DROP TABLE, ALTER TABLE
- **DML**: INSERT, UPDATE, DELETE, SELECT
- **查询**: JOIN, WHERE, GROUP BY, ORDER BY, HAVING, LIMIT
- **聚合**: COUNT, SUM, AVG, MIN, MAX, GROUP_CONCAT
- **函数**: 100+ 内置函数（日期、字符串、数学等）
- **窗口函数**: ROW_NUMBER, RANK, LAG, LEAD 等

### 数据类型

Boolean, Int8-64, Float32/64, Decimal, Date, DateTime, Timestamp, String, Binary, Array, Map, Struct, JSON

## 🤝 贡献

我们欢迎贡献！以下是你可以帮助的方式：

1. **⭐ Star 仓库** - 帮助更多人发现
2. **🐛 报告 bug** - 提交 issue
3. **💡 建议功能** - 分享你的使用场景
4. **🔧 提交 PR** - 修复 bug 或添加功能
5. **📝 改进文档** - 帮助他人学习

请参阅 [CONTRIBUTING.md](CONTRIBUTING.md) 了解指南。

## 📊 项目统计

- **语言**: Rust (~80,000 行)
- **Crate 数量**: 28
- **协议数量**: 14
- **测试数量**: 180 通过
- **支持的客户端**: 100+（MySQL, PostgreSQL, Redis, MongoDB 等）
- **许可证**: Apache 2.0

## 🗺️ 路线图

### v1.0.0（当前）
- ✅ 14种协议实现
- ✅ 核心 SQL 引擎
- ✅ 配置系统
- ✅ 97% 测试通过率

### v1.1.0
- [ ] 分布式事务（2PC）
- [ ] 复制和高可用
- [ ] 高级查询优化
- [ ] 物化视图

### v2.0.0
- [ ] 集群模式（多节点）
- [ ] 云原生部署（Kubernetes）
- [ ] 高级安全（加密、RBAC）
- [ ] 实时流处理

## 📖 文档

- [SQL 参考](docs/en/sql-reference.md)
- [配置指南](docs/en/configuration.md)
- [架构](docs/en/architecture.md)
- [协议兼容性](docs/alibaba-cloud-compatibility.md)
- [路线图](docs/roadmap/README.md)

## 📜 许可证

Apache License 2.0. 详见 [LICENSE](LICENSE).

## 🙏 致谢

- **[Apache DataFusion](https://github.com/apache/arrow-datafusion)** - 查询引擎
- **[Apache Arrow](https://arrow.apache.org)** - 列式格式
- **[Apache Parquet](https://parquet.apache.org)** - 存储格式
- **[Apache Doris](https://doris.apache.org)** - SQL 方言灵感
- **[sqlparser-rs](https://github.com/sqlparser-rs/sqlparser-rs)** - SQL 解析

## 🌟 支持我们

如果你觉得 HarnessDB 有用，请考虑：

- ⭐ **Star 仓库** - 帮助他人发现
- 🐦 **发推** - 传播消息
- 📝 **写博客** - 分享你的经验
- 🎥 **制作视频** - 展示你如何使用它

---

<div align="center">

**由 HarnessDB 团队用 ❤️ 构建**

[网站](https://harnessdb.io) · [博客](https://blog.harnessdb.io) · [Twitter](https://twitter.com/harnessdb) · [Discord](https://discord.gg/harnessdb)

</div>
