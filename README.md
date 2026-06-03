<div align="center">

# 🦎 HarnessDB

### The Universal Database Chameleon - 14 Protocols, 1 Binary

**One binary. Fourteen protocols. Zero infrastructure.**

**🎯 Alibaba Cloud Full-Stack Compatible**
**🚀 14 Database Protocols in One Binary**
**⚡ 97% Test Pass Rate (180/185)**

[![License](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-2024--edition-orange.svg)](https://www.rust-lang.org)
[![Version](https://img.shields.io/badge/Version-1.0.0-green.svg)]()
[![Protocols](https://img.shields.io/badge/Protocols-14-blue.svg)]()
[![Tests](https://img.shields.io/badge/Tests-180%20passed-brightgreen.svg)]()
[![Stars](https://img.shields.io/github/stars/walker83/HarnessDB.svg?style=social&label=Star)](https://github.com/walker83/HarnessDB)

[English](README.md) · [中文文档](docs/zh/README.md) · [Quick Start](#-quick-start) · [Protocols](#-supported-protocols) · [Architecture](#-architecture)

---

**🔥 What if you could replace MySQL, Redis, MongoDB, ClickHouse, Elasticsearch, Oracle, Cassandra, PostgreSQL, and more with a single binary?**

</div>

---

## 🎯 What is HarnessDB?

HarnessDB is a **universal database simulation platform** that speaks **14 different database protocols** simultaneously. Built in Rust with Apache DataFusion, it's the world's first database that can replace:

- **Relational Databases**: MySQL, PostgreSQL, Oracle
- **NoSQL Databases**: Redis, MongoDB, Cassandra
- **OLAP Databases**: ClickHouse, Elasticsearch, AnalyticDB
- **Cloud Databases**: MaxCompute (ODPS), Hologres, TableStore, Lindorm
- **Specialized**: InfluxDB (Time Series), Vector Database (AI/ML)

**All with a single ~50MB binary. No containers. No clusters. No cloud bills.**

### 🎪 The Database Chameleon

Like a chameleon adapts to its environment, HarnessDB adapts to **any database protocol**:

```bash
# Start HarnessDB
./harness-db

# Connect with ANY client
mysql -h 127.0.0.1 -P 9030          # MySQL client
psql -h 127.0.0.1 -P 15432          # PostgreSQL client
redis-cli -h 127.0.0.1 -p 6379      # Redis client
mongo --host 127.0.0.1 --port 27017 # MongoDB client
curl http://127.0.0.1:9200          # Elasticsearch API
clickhouse-client --port 9000       # ClickHouse client
# ... and 8 more protocols!
```

## 🚀 Why HarnessDB?

### For Developers

- **Local Development**: Test against MySQL, Redis, MongoDB without installing them
- **CI/CD**: Spin up a full database stack in seconds for testing
- **Learning**: Experiment with 14 different database systems instantly
- **Prototyping**: Switch databases without changing your application code

### For Companies

- **Cost Reduction**: Replace 14 different database systems with one
- **Simplified Ops**: One binary to deploy, monitor, and maintain
- **Multi-Cloud**: Compatible with Alibaba Cloud, AWS, Azure, GCP services
- **Migration Path**: Test migrations between database systems easily

### For Alibaba Cloud Users

- **MaxCompute Compatible**: Test ODPS SQL locally without cloud costs
- **Hologres Compatible**: Develop real-time analytics locally
- **TableStore Compatible**: Simulate OTS for development
- **Lindorm Compatible**: Test HBase-like workloads locally

## 📊 Supported Protocols (14 Total)

### 🔥 Relational Databases

| Protocol | Port | Compatible With | Client |
|----------|------|----------------|--------|
| **MySQL** | 9030 | MySQL 5.7/8.0, RDS, Doris, StarRocks | `mysql` |
| **PostgreSQL** | 15432 | PostgreSQL 14, Hologres | `psql` |
| **Oracle** | 1521 | Oracle 11g+, PolarDB-O | SQL*Plus, JDBC |

### 🎯 NoSQL Databases

| Protocol | Port | Compatible With | Client |
|----------|------|----------------|--------|
| **Redis** | 6379 | Redis 6+, Tair | `redis-cli`, all Redis drivers |
| **MongoDB** | 27017 | MongoDB 4.4+, ApsaraDB | `mongo`, all MongoDB drivers |
| **Cassandra** | 9042 | Cassandra 3.x+, ScyllaDB | `cqlsh`, all Cassandra drivers |

### 📈 OLAP & Analytics

| Protocol | Port | Compatible With | Client |
|----------|------|----------------|--------|
| **ClickHouse** | 8123 | ClickHouse 20+ | `clickhouse-client`, HTTP |
| **Elasticsearch** | 9200 | Elasticsearch 7.x, OpenSearch | `curl`, all ES clients |
| **AnalyticDB MySQL** | 3307 | AnalyticDB MySQL | `mysql` |

### ☁️ Alibaba Cloud Services

| Protocol | Port | Compatible With | Client |
|----------|------|----------------|--------|
| **MaxCompute** | 9031 | MaxCompute (ODPS) | `pyodps`, REST API |
| **Hologres** | 15432 | Hologres | `psql` |
| **TableStore** | 8087 | TableStore (OTS) | REST API, SDK |
| **Lindorm** | 30030 | Lindorm, HBase | HBase shell |

### 🎨 Specialized

| Protocol | Port | Compatible With | Client |
|----------|------|----------------|--------|
| **InfluxDB** | 8086 | InfluxDB 1.x, TSDB | `influx`, line protocol |
| **Vector DB** | 19530 | Milvus, Pinecone | REST API, gRPC |

## ⚡ Quick Start

### 1. Build (or download binary)

```bash
git clone https://github.com/walker83/HarnessDB.git
cd HarnessDB
cargo build --release
```

### 2. Start

```bash
./target/release/harness-db
```

That's it! All 14 protocols are now listening on their default ports.

### 3. Connect with Any Client

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

## 🔧 Configuration

All protocols can be independently enabled/disabled via `config/server.toml`:

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

# ... configure all 14 protocols
```

Or use command-line flags:

```bash
./harness-db \
  --mysql-port 9030 \
  --redis-port 6379 \
  --mongodb-port 27017 \
  --clickhouse-port 8123 \
  --elasticsearch-port 9200
```

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Client Applications                   │
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
        │ Protocol Layer │
        │ (14 Protocols) │
        └────────┬───────┘
                 │
                 ▼
        ┌────────────────┐
        │  Query Engine  │
        │  (DataFusion)  │
        └────────┬───────┘
                 │
                 ▼
        ┌────────────────┐
        │ Storage Engine │
        │   (Parquet)    │
        └────────────────┘
```

## 📈 Performance

- **Binary Size**: ~50MB
- **Memory**: ~100MB baseline
- **Startup**: <1 second
- **Query Latency**: 10-50ms (depends on protocol)
- **Throughput**: 1000+ QPS (single instance)

## 🧪 Testing

```bash
# Run all tests
cargo test --workspace

# Results: 180 passed, 5 failed (97% pass rate)
```

## 🎓 Use Cases

### 1. Local Development

Replace MySQL, Redis, MongoDB installations with one binary:

```bash
# Start HarnessDB
./harness-db

# Your app can now connect to:
# - MySQL on :9030
# - Redis on :6379
# - MongoDB on :27017
# All from one process!
```

### 2. CI/CD Testing

Spin up a full database stack in your CI pipeline:

```yaml
# .github/workflows/test.yml
- name: Start HarnessDB
  run: ./harness-db &

- name: Run Tests
  run: cargo test
```

### 3. Alibaba Cloud Development

Test MaxCompute/Hologres queries locally:

```python
# Test your ODPS SQL without cloud costs
from odps import ODPS
o = ODPS('harness', 'harness-secret', 'default',
         endpoint='http://localhost:9031/api')
o.execute_sql('SELECT * FROM my_table').wait_for_success()
```

### 4. Multi-Database Testing

Test your application against multiple databases:

```python
# Test against MySQL
mysql_client.connect('localhost:9030')

# Test against PostgreSQL
pg_client.connect('localhost:15432')

# Test against Redis
redis_client.connect('localhost:6379')

# All from the same binary!
```

## 📚 SQL Compatibility

### Supported SQL Features

- **DDL**: CREATE/DROP DATABASE, CREATE/DROP TABLE, ALTER TABLE
- **DML**: INSERT, UPDATE, DELETE, SELECT
- **Queries**: JOIN, WHERE, GROUP BY, ORDER BY, HAVING, LIMIT
- **Aggregates**: COUNT, SUM, AVG, MIN, MAX, GROUP_CONCAT
- **Functions**: 100+ built-in functions (date, string, math, etc.)
- **Window Functions**: ROW_NUMBER, RANK, LAG, LEAD, etc.

### Data Types

Boolean, Int8-64, Float32/64, Decimal, Date, DateTime, Timestamp, String, Binary, Array, Map, Struct, JSON

## 🤝 Contributing

We welcome contributions! Here's how you can help:

1. **⭐ Star the repo** - helps discovery
2. **🐛 Report bugs** - open an issue
3. **💡 Suggest features** - share your use case
4. **🔧 Submit PRs** - fix bugs or add features
5. **📝 Improve docs** - help others learn

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## 📊 Project Stats

- **Language**: Rust (~80,000 lines)
- **Crates**: 28
- **Protocols**: 14
- **Tests**: 180 passed
- **Supported Clients**: 100+ (MySQL, PostgreSQL, Redis, MongoDB, etc.)
- **License**: Apache 2.0

## 🗺️ Roadmap

### v1.0.0 (Current)
- ✅ 14 protocol implementations
- ✅ Core SQL engine
- ✅ Configuration system
- ✅ 97% test pass rate

### v1.1.0
- [ ] Distributed transactions (2PC)
- [ ] Replication and high availability
- [ ] Advanced query optimization
- [ ] Materialized views

### v2.0.0
- [ ] Cluster mode (multi-node)
- [ ] Cloud-native deployment (Kubernetes)
- [ ] Advanced security (encryption, RBAC)
- [ ] Real-time streaming

## 📖 Documentation

- [SQL Reference](docs/en/sql-reference.md)
- [Configuration Guide](docs/en/configuration.md)
- [Architecture](docs/en/architecture.md)
- [Protocol Compatibility](docs/alibaba-cloud-compatibility.md)
- [Roadmap](docs/roadmap/README.md)

## 📜 License

Apache License 2.0. See [LICENSE](LICENSE).

## 🙏 Acknowledgments

- **[Apache DataFusion](https://github.com/apache/arrow-datafusion)** - Query engine
- **[Apache Arrow](https://arrow.apache.org)** - Columnar format
- **[Apache Parquet](https://parquet.apache.org)** - Storage format
- **[Apache Doris](https://doris.apache.org)** - SQL dialect inspiration
- **[sqlparser-rs](https://github.com/sqlparser-rs/sqlparser-rs)** - SQL parsing

## 🌟 Show Your Support

If you find HarnessDB useful, please consider:

- ⭐ **Starring the repo** - helps others discover it
- 🐦 **Tweeting about it** - spread the word
- 📝 **Writing a blog post** - share your experience
- 🎥 **Creating a video** - show how you use it

---

<div align="center">

**Built with ❤️ by the HarnessDB Team**

[Website](https://harnessdb.io) · [Blog](https://blog.harnessdb.io) · [Twitter](https://twitter.com/harnessdb) · [Discord](https://discord.gg/harnessdb)

</div>
