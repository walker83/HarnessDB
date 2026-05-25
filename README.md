<div align="center">

# RorisDB

### The Doris-Compatible Single-Node OLAP Database

**Learn Doris SQL locally. No cluster required.**

[![Apache-2.0 License](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-2024--edition-orange.svg)](https://www.rust-lang.org)
[![Version](https://img.shields.io/badge/Version-0.3.0-green.svg)]()
[![Stars](https://img.shields.io/github/stars/walker83/RorisDB.svg?style=social&label=Star)](https://github.com/walker83/RorisDB)
[![Documentation](https://img.shields.io/badge/Docs-English-blue)](docs/en/)
[![中文文档](https://img.shields.io/badge/Docs-中文-green)](docs/zh/)

[Quick Start](#-quick-start) • [Features](#-features) • [Architecture](#-architecture) • [Examples](#-examples) • [Contributing](#-contributing)

</div>

---

## 🎯 What is RorisDB?

RorisDB is a **single-node OLAP database** that speaks **Doris SQL dialect** — built in Rust with Apache DataFusion and Parquet.

**Perfect for:**
- 🎓 Learning Doris/OLAP SQL without deploying a cluster
- 🧪 Experimenting with columnar storage and analytical queries
- 🚀 Prototyping data pipelines locally before production
- 📚 Studying modern database internals in readable Rust code
- 🔌 Connecting with any MySQL client, JDBC driver, or BI tool

**Real-World Compatibility:** Tested with 10 major applications including WordPress, Grafana, Superset, GitLab, and Airbyte — **137 test cases, 100% pass rate**.

## ⚡ Quick Start

Get up and running in 60 seconds:

```bash
# Install (requires Rust 2024 edition)
git clone https://github.com/walker83/RorisDB.git
cd RorisDB
cargo build --release

# Start server
./target/release/roris-fe

# Connect with MySQL client
mysql -h 127.0.0.1 -P 9030 -uroot
```

```sql
-- Create database and table
CREATE DATABASE analytics;
USE analytics;

CREATE TABLE events (
    id INT,
    user_id INT,
    event_type VARCHAR(50),
    amount DECIMAL(10,2),
    occurred_at DATETIME
) DUPLICATE KEY(id)
DISTRIBUTED BY HASH(id) BUCKETS 1;

-- Insert data
INSERT INTO events VALUES 
    (1, 100, 'purchase', 99.99, '2024-01-15 10:30:00'),
    (2, 100, 'purchase', 49.50, '2024-01-16 14:20:00'),
    (3, 200, 'view', 0.00, '2024-01-15 11:00:00');

-- Analytical queries
SELECT event_type, COUNT(*), SUM(amount) 
FROM events 
GROUP BY event_type;

-- Window functions
SELECT user_id, 
       amount,
       SUM(amount) OVER (PARTITION BY user_id ORDER BY occurred_at) as running_total
FROM events;
```

## 🎬 Demo

<div align="center">

*GIF: Terminal showing RorisDB startup, table creation, and analytical query*

![Demo](docs/assets/demo.gif)

</div>

## ✨ Features

### 🔧 Production-Ready Features

| Feature | Description |
|---------|-------------|
| **MySQL Compatible** | Full MySQL wire protocol — works with any MySQL client, driver, or ORM |
| **Doris SQL** | `DUPLICATE KEY`, `DISTRIBUTED BY HASH`, `date_trunc`, `months_add` |
| **Columnar Storage** | Apache Parquet with ZSTD compression and page-level statistics |
| **Web SQL Editor** | Built-in browser-based SQL editor at `http://localhost:8080` |
| **Backup & Restore** | Full database backup to local repository with manifest tracking |
| **Configuration System** | TOML config file with 30+ system variables |
| **Audit Logging** | Async audit log with slow query tracking |
| **Operations** | `SHOW PROCESSLIST`, `SHOW STATUS`, `KILL QUERY` |

### 📊 SQL Support

**Data Types:** Boolean, Int8-64, Float32/64, Decimal, Date, DateTime, String, Binary, Array, Map, Struct

**Queries:**
- ✅ `SELECT` with `JOIN` (INNER/LEFT/RIGHT/FULL/CROSS)
- ✅ Subqueries and CTEs (`WITH`)
- ✅ Window functions (`ROW_NUMBER`, `RANK`, `LAG`, `LEAD`)
- ✅ Aggregates (`COUNT`, `SUM`, `AVG`, `MIN`, `MAX`, `GROUP_CONCAT`)
- ✅ `UNION`, `EXCEPT`, `INTERSECT`
- ✅ `ORDER BY`, `GROUP BY`, `HAVING`, `LIMIT`

**DML:**
- ✅ `INSERT INTO ... VALUES` (single and multi-row)
- ✅ `INSERT INTO ... SELECT`
- ✅ `UPDATE` with `WHERE`
- ✅ `DELETE` with `WHERE`

**DDL:**
- ✅ `CREATE/DROP DATABASE`
- ✅ `CREATE/DROP TABLE` with Doris extensions
- ✅ `ALTER TABLE` (ADD/DROP/MODIFY COLUMN)
- ✅ `TRUNCATE TABLE`

### 🔌 Ecosystem Integration

Tested with real-world applications:

| Application | Category | Tests | Status |
|------------|----------|-------|--------|
| **WordPress** | CMS | 14 | ✅ Pass |
| **Discourse** | Forum | 14 | ✅ Pass |
| **Grafana** | Monitoring | 12 | ✅ Pass |
| **Apache Superset** | BI | 13 | ✅ Pass |
| **Metabase** | Analytics | 11 | ✅ Pass |
| **dbt** | Data Transform | 12 | ✅ Pass |
| **Nextcloud** | File Sync | 13 | ✅ Pass |
| **Mattermost** | Team Chat | 15 | ✅ Pass |
| **GitLab** | DevOps | 17 | ✅ Pass |
| **Airbyte** | ETL | 16 | ✅ Pass |

**Total: 137 real-world tests, 100% compatibility**

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      MySQL Client                            │
│              (any mysql CLI, JDBC, ORM, etc.)                │
└──────────────────────────┬──────────────────────────────────┘
                           │ MySQL wire protocol
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                    roris-server                               │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │
│  │  DDL     │  │  DML     │  │  SELECT  │  │  SHOW    │   │
│  │ handler  │  │ handler  │  │(DataFusion│  │ commands │   │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘   │
└───────┼──────────────┼──────────────┼──────────────┼────────┘
        │              │              │              │
        ▼              ▼              ▼              ▼
┌──────────────┐ ┌──────────┐ ┌──────────────┐ ┌──────────┐
│  fe-catalog  │ │fe-storage│ │ fe-datafusion│ │fe-monitor│
│  (metadata)  │ │ (Parquet)│ │  (UDFs +    │ │ (audit   │
│              │ │          │ │   types)    │ │   log)   │
└──────────────┘ └────┬─────┘ └──────────────┘ └──────────┘
                      │
                      ▼
              ┌──────────────┐
              │    Parquet   │
              │    files     │
              └──────────────┘
```

### Tech Stack

| Component | Technology | Version |
|-----------|-----------|---------|
| Query Engine | Apache DataFusion | 48 |
| Columnar Format | Apache Arrow | 55 |
| Storage Format | Apache Parquet | 55 |
| SQL Parser | sqlparser-rs | 0.53 |
| Async Runtime | Tokio | 1.x |
| Metadata | JSON / RocksDB | 0.23 |

## 📚 Examples

### Example 1: WordPress Analytics

```sql
-- Create WordPress-style tables
CREATE DATABASE wordpress;
USE wordpress;

CREATE TABLE wp_posts (
    ID INT,
    post_author INT,
    post_title TEXT,
    post_status VARCHAR(20),
    post_date DATETIME
);

CREATE TABLE wp_users (
    ID INT,
    user_login VARCHAR(60),
    user_email VARCHAR(100)
);

-- Analyze publishing patterns
SELECT u.user_login, COUNT(p.ID) as post_count
FROM wp_users u
JOIN wp_posts p ON u.ID = p.post_author
WHERE p.post_status = 'publish'
GROUP BY u.user_login
ORDER BY post_count DESC;
```

### Example 2: Event Tracking

```sql
-- Create event tracking table
CREATE TABLE events (
    event_id INT,
    user_id INT,
    event_type VARCHAR(50),
    properties TEXT,
    occurred_at DATETIME
);

-- Daily active users
SELECT DATE(occurred_at) as date, 
       COUNT(DISTINCT user_id) as dau
FROM events
WHERE occurred_at >= '2024-01-01'
GROUP BY DATE(occurred_at)
ORDER BY date;

-- Event funnel
SELECT event_type, COUNT(*) as count
FROM events
GROUP BY event_type
ORDER BY count DESC;
```

### Example 3: Web SQL Editor

Open your browser to `http://localhost:8080` for a built-in SQL editor:

- Database browser with schema exploration
- SQL editor with syntax highlighting
- Query results with export capability
- Query history

## 🔍 Why RorisDB?

### vs Apache Doris

| Feature | RorisDB | Apache Doris |
|---------|---------|--------------|
| Deployment | Single binary | Distributed cluster |
| Setup time | 60 seconds | 30+ minutes |
| Learning curve | Minimal | Steep |
| Resource usage | ~100MB RAM | GBs of RAM |
| Use case | Learning, prototyping | Production workloads |
| SQL compatibility | Doris dialect | Full Doris SQL |

**Choose RorisDB for:** Learning Doris SQL, local development, prototyping
**Choose Doris for:** Production analytics, large datasets, distributed processing

### vs SQLite

| Feature | RorisDB | SQLite |
|---------|---------|--------|
| Storage model | Columnar (Parquet) | Row-based |
| Query optimization | Analytical (OLAP) | Transactional (OLTP) |
| Compression | ZSTD | Minimal |
| Analytical queries | Fast | Slow on large data |
| Window functions | Full support | Limited |
| MySQL compatibility | Yes | No |

**Choose RorisDB for:** Analytics, aggregations, time series, large datasets
**Choose SQLite for:** Transactional apps, embedded databases, simple queries

### vs DuckDB

| Feature | RorisDB | DuckDB |
|---------|---------|--------|
| Language | Rust | C++ |
| MySQL compatibility | Yes | No (PostgreSQL wire protocol) |
| Doris SQL | Yes | No |
| Storage | Parquet | Custom format |
| Web UI | Built-in | No |
| Backup/Restore | Built-in | Manual |

**Choose RorisDB for:** MySQL ecosystem, Doris compatibility, production-like setup
**Choose DuckDB for:** Pure analytics, Python integration, academic research

## 🚀 Roadmap

### v0.4.0 (Q2 2025)
- [ ] Multi-segment storage (append writes + compaction)
- [ ] Real transactions (MVCC)
- [ ] Parquet predicate pushdown (row group pruning)
- [ ] Partition table execution

### v0.5.0 (Q3 2025)
- [ ] Replace `types` crate with native Arrow types
- [ ] Arrow-native QueryResult (eliminate string conversion)
- [ ] Streaming bulk load (CSV/JSON)
- [ ] Materialized views

### v1.0.0 (Q4 2025)
- [ ] Production-ready stability
- [ ] Full Doris SQL compatibility
- [ ] Performance optimization
- [ ] Comprehensive documentation

## 🛠️ Building from Source

```bash
# Prerequisites: Rust 2024 edition (rustup update)
git clone https://github.com/walker83/RorisDB.git
cd RorisDB

# Build
cargo build --release

# Run tests
cargo test --workspace

# Run integration tests
./tests/real_world_scenarios/run_tests.sh

# Binary: target/release/roris-fe
```

## 📖 Documentation

- [English Documentation](docs/en/)
- [中文文档](docs/zh/)
- [SQL Reference](docs/en/sql-reference.md)
- [Configuration Guide](docs/en/configuration.md)
- [Architecture Deep Dive](docs/en/architecture.md)

## 🤝 Contributing

We welcome contributions! Here's how you can help:

1. **Star the repo** ⭐ — It helps more people discover RorisDB
2. **Report bugs** — Open an issue with reproduction steps
3. **Suggest features** — Share your use cases
4. **Submit PRs** — Fix bugs or add features
5. **Write docs** — Improve documentation
6. **Share** — Blog posts, tweets, talks

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Contributors

Thanks to these wonderful people:

<!-- ALL-CONTRIBUTORS-LIST:START -->
*Your name could be here!*
<!-- ALL-CONTRIBUTORS-LIST:END -->

## 📊 Project Stats

- **Language:** Rust (~27,000 lines)
- **Crates:** 11
- **Test Coverage:** 137 real-world scenarios + 1,644 integration tests
- **Compatibility:** WordPress, Grafana, Superset, GitLab, and 6 more
- **License:** Apache 2.0

## 🔗 Links

- **Website:** [rorisdb.com](https://rorisdb.com) (coming soon)
- **Documentation:** [docs/](docs/)
- **Issues:** [GitHub Issues](https://github.com/walker83/RorisDB/issues)
- **Discussions:** [GitHub Discussions](https://github.com/walker83/RorisDB/discussions)
- **Twitter:** [@RorisDB](https://twitter.com/RorisDB) (coming soon)

## 📄 License

Apache License 2.0. See [LICENSE](LICENSE).

## 🙏 Acknowledgments

RorisDB is built on the shoulders of giants:

- **[Apache Doris](https://doris.apache.org)** — For pioneering real-time OLAP and inspiring this project
- **[Apache DataFusion](https://github.com/apache/arrow-datafusion)** — For the blazing-fast query engine
- **[Apache Arrow](https://arrow.apache.org)** — For the columnar format ecosystem
- **[Apache Parquet](https://parquet.apache.org)** — For the efficient storage format
- **[sqlparser-rs](https://github.com/sqlparser-rs/sqlparser-rs)** — For the SQL parsing foundation

## ❓ FAQ

**Q: Is this a fork of Apache Doris?**
A: No. RorisDB is an independent project that reimplements Doris concepts in Rust with DataFusion and Parquet.

**Q: Can I use RorisDB in production?**
A: RorisDB is currently in v0.3.0 and suitable for learning, prototyping, and small workloads. For production analytics, use Apache Doris.

**Q: Why Rust instead of C++/Java?**
A: Rust provides memory safety, zero-cost abstractions, and excellent performance — perfect for database systems.

**Q: How does it compare to DuckDB?**
A: RorisDB focuses on MySQL compatibility and Doris SQL dialect, while DuckDB uses PostgreSQL protocol. Both are excellent analytical databases.

**Q: What's the maximum dataset size?**
A: No hard limit, but single-file-per-table design means INSERT is O(N). For datasets >10GB, consider partitioning or wait for v0.4.0 multi-segment storage.

---

<div align="center">

**If RorisDB helps you learn OLAP or prototype faster, please consider [starring the repo](https://github.com/walker83/RorisDB)!** ⭐

Made with ❤️ by the RorisDB community

</div>
