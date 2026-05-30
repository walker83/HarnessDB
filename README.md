<div align="center">

# RorisDB

### The Universal Database Chameleon

**One binary. Three protocols. Zero infrastructure.**

**✅ Alibaba Cloud Compatible — MaxCompute & Hologres**

[![Apache-2.0 License](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-2024--edition-orange.svg)](https://www.rust-lang.org)
[![Version](https://img.shields.io/badge/Version-0.3.0-green.svg)]()
[![Stars](https://img.shields.io/github/stars/walker83/RorisDB.svg?style=social&label=Star)](https://github.com/walker83/RorisDB)

[Quick Start](#-quick-start) · [Supported Protocols](#-supported-protocols) · [Architecture](#-architecture) · [Documentation](#-documentation) · [Contributing](#-contributing)

</div>

---

## What is RorisDB?

RorisDB is a **universal database simulation platform** built in Rust with Apache DataFusion. The core SQL engine is **Doris-compatible** — accepting all four Doris table model syntaxes, Doris-specific functions, and the full Doris DDL/DML grammar. On top of this foundation, RorisDB simultaneously speaks **MaxCompute (ODPS)** and **Hologres (PostgreSQL)** protocols, translating their vendor-specific syntax into the common Doris-based engine.

**Alibaba Cloud Compatible:**
- **MaxCompute (ODPS)** — Full protocol support with HMAC-SHA1/SHA256 authentication, SQL submission, instance management
- **Hologres** — PostgreSQL v3 wire protocol with `pg_catalog` system tables and Hologres-specific DDL

**One binary replaces your entire dependency matrix:**

- Replace a Doris cluster for local development with native Doris SQL
- Mock MaxCompute (ODPS) APIs for offline data pipeline testing
- Simulate Hologres for real-time analytics development
- Run any MySQL-compatible application without provisioning infrastructure

No containers. No clusters. No cloud bills. Just `./roris-fe` and go.

## Key Capabilities

| Capability | Description |
|------------|-------------|
| **Doris SQL Compatible** | Full Doris grammar: `DUPLICATE/AGGREGATE/UNIQUE/PRIMARY KEY`, `DISTRIBUTED BY HASH`, 35 Doris UDFs |
| **Alibaba Cloud Compatible** | Full MaxCompute (ODPS) and Hologres protocol support |
| **Multi-Protocol** | MySQL (:9030), MaxCompute (:9031), Hologres (:15432) — simultaneously on a single instance |
| **Columnar Engine** | Apache DataFusion query engine with Parquet storage, ZSTD compression |
| **Protocol Fidelity** | Real wire protocols — works with `mysql`, `psql`, `pyodps`, JDBC, and BI tools |
| **SQL Translation** | MaxCompute/Hologres syntax auto-normalized into Doris-based engine |
| **Embedded Web UI** | Browser-based SQL editor at `http://localhost:8080` with schema exploration |
| **Single Binary** | ~100MB RAM footprint, 60-second setup, zero external dependencies |
| **Backup & Restore** | Full database backup with manifest tracking |
| **Audit Logging** | Async audit log with slow query tracking |

## Supported Protocols

### MySQL Wire Protocol — Port 9030

Connect with any MySQL client, driver, ORM, or BI tool. **Full Doris SQL grammar** — the native engine accepts all four Doris table models (`DUPLICATE KEY`, `AGGREGATE KEY`, `UNIQUE KEY`, `PRIMARY KEY`), `DISTRIBUTED BY HASH`, `PARTITION BY`, Doris built-in functions, and `ON DUPLICATE KEY UPDATE`. Note: MySQL protocol currently does not enforce password authentication — any non-empty username is accepted.

```bash
mysql -h 127.0.0.1 -P 9030 -uroot
```

### MaxCompute (ODPS) REST API — Port 9031

**✅ Alibaba Cloud MaxCompute Compatible** — Simulate Alibaba Cloud MaxCompute for data pipeline development. Supports HMAC-SHA1 (V2) and HMAC-SHA256 (V4) authentication, SQL job submission, instance management, and the full ODPS type system.

```python
from odps import ODPS
o = ODPS('roris', 'roris-secret', 'default',
         endpoint='http://127.0.0.1:9031/api')
o.execute_sql('SELECT * FROM my_table').wait_for_success()
```

### Hologres (PostgreSQL) — Port 15432

**✅ Alibaba Cloud Hologres Compatible** — Simulate Alibaba Cloud Hologres with PostgreSQL v3 wire protocol. Supports Simple Query, Extended Query (Parse/Bind/Execute), `pg_catalog` system tables, and Hologres-specific DDL (`WITH (orientation='column', ...)`, `CALL set_table_property`).

```bash
psql -h 127.0.0.1 -p 15432 -U roris -d default
```

### Protocol Comparison

| Feature | MySQL | MaxCompute | Hologres |
|---------|-------|------------|----------|
| Wire Protocol | TCP Binary | HTTP/REST + XML | TCP (PostgreSQL v3) |
| Authentication | Handshake (no password enforcement) | HMAC-SHA1 / HMAC-SHA256 (verified) | MD5 (verified) |
| Default Credentials | Any username / no password | `roris` / `roris-secret` | `roris` / `roris-secret` |
| SQL Dialect | Doris/MySQL (native) | ODPS SQL (translated) | PostgreSQL (translated) |
| DDL Extensions | `DUPLICATE KEY`, `DISTRIBUTED BY` | `PARTITIONED BY`, `LIFECYCLE` | `WITH (orientation=...)`, `set_table_property` |
| Clients | `mysql`, JDBC, DBeaver | `pyodps`, DataWorks SDK | `psql`, JDBC, pg-driver |
| Status | Stable | Phase 1 complete | Phase 1 complete |

## Quick Start

```bash
# Build (requires Rust 2024 edition)
git clone https://github.com/walker83/RorisDB.git
cd RorisDB
cargo build --release

# Start with all protocols enabled
./target/release/roris-fe --mysql-port 9030 --maxcompute-port 9031 --hologres-port 15432
```

### Doris SQL Example (MySQL Protocol)

```bash
mysql -h 127.0.0.1 -P 9030 -uroot
```

```sql
CREATE DATABASE analytics;
USE analytics;

-- Doris Duplicate Key model with distribution
CREATE TABLE events (
    event_id INT,
    user_id INT,
    event_type VARCHAR(50),
    amount DECIMAL(10,2),
    occurred_at DATETIME
) DUPLICATE KEY(event_id)
DISTRIBUTED BY HASH(event_id) BUCKETS 1;

-- Doris Aggregate Key model (syntax accepted, modifiers stored as metadata)
CREATE TABLE daily_stats (
    stat_date DATE,
    channel VARCHAR(50),
    pv BIGINT SUM,
    uv BIGINT SUM,
    amount DECIMAL(10,2) SUM
) AGGREGATE KEY(stat_date, channel)
DISTRIBUTED BY HASH(stat_date) BUCKETS 1;

INSERT INTO events VALUES
    (1, 100, 'purchase', 99.99, '2024-01-15 10:30:00'),
    (2, 100, 'purchase', 49.50, '2024-01-16 14:20:00'),
    (3, 200, 'view', 0.00, '2024-01-15 11:00:00');

-- Doris built-in functions
SELECT date_trunc('month', occurred_at) AS month,
       COUNT(*) AS cnt,
       SUM(amount) AS total
FROM events
GROUP BY date_trunc('month', occurred_at);
```

### MaxCompute Example

```python
from odps import ODPS

o = ODPS('roris', 'roris-secret', 'default',
         endpoint='http://127.0.0.1:9031/api')

# Create table with ODPS syntax — LIFECYCLE, PARTITIONED BY all work
o.execute_sql("""
CREATE TABLE user_events (
    user_id BIGINT,
    action STRING,
    amount DOUBLE
) PARTITIONED BY (ds STRING) LIFECYCLE 365
""").wait_for_success()

# INSERT OVERWRITE is auto-converted to INSERT INTO
o.execute_sql("INSERT OVERWRITE TABLE user_events VALUES (1, 'click', 1.0)").wait_for_success()
```

### Hologres Example

```bash
psql -h 127.0.0.1 -p 15432 -U roris -d default
```

```sql
-- Hologres DDL with WITH clause — silently normalized
CREATE TABLE orders (
    id BIGINT NOT NULL,
    user_id BIGINT,
    amount DOUBLE PRECISION,
    created_at TIMESTAMP,
    PRIMARY KEY (id)
) WITH (
    orientation = 'column',
    distribution_key = 'id'
);

INSERT INTO orders VALUES (1, 100, 99.99, now());
SELECT * FROM orders WHERE user_id = 100;
```

## Architecture

```
                          +-----------------------------------+
                          |         Client Applications       |
                          | mysql | psql | pyodps | JDBC | ...|
                          +------+-------+--------+-----+----+
                                 |       |        |     |
                   MySQL Wire    |       |        |     |  PostgreSQL v3
                   Protocol      |       |        |     |  Wire Protocol
                                 v       |        |     v
                    +-----------+ +------+------+ +--+-----------+
                    |  MySQL    | | MaxCompute  | |  Hologres    |
                    |  Protocol | | Protocol    | |  (PG)        |
                    |  :9030    | | :9031       | |  :15432      |
                    +-----+----+ +------+------+ +-+----+-------+
                          |            |              |
                          |     SQL Translator  SQL Translator
                          |     (strip ODPS     (strip Hologres-
                          |      syntax)         specific DDL)
                          |            |              |
                          +------+-----+------+-------+
                                 |            |
                                 v            v
                    +-----------------------------------+
                    |      Doris SQL Core Engine         |
                    |  DDL Handler | DML Handler | SELECT|
                    |  (DataFusion SessionContext)       |
                    +----------------+------------------+
                                     |
                       +-------------+-------------+
                       |             |             |
                       v             v             v
                  +---------+  +----------+  +----------+
                  |fe-catalog|  |fe-storage|  |fe-monitor|
                  |(metadata)|  | (Parquet)|  | (audit)  |
                  +----------+  +----+-----+  +----------+
                                     |
                                     v
                              +-------------+
                              |   Parquet   |
                              |   Files     |
                              +-------------+
```

### SQL Translation Pipeline

All protocol adapters translate vendor-specific syntax into RorisDB's **Doris-compatible core engine**:

```
MaxCompute:  INSERT OVERWRITE TABLE t SELECT ...  →  INSERT INTO t SELECT ...
             PARTITIONED BY (ds STRING)           →  column `ds` added to schema
             LIFECYCLE 365                        →  (stripped)
             DISTRIBUTE BY col SORT BY col        →  ORDER BY col

Hologres:    CREATE TABLE ... WITH (orientation='column')  →  CREATE TABLE ...
             CALL set_table_property(...)                  →  (no-op)
             CREATE INDEX idx USING bitmap(col)            →  CREATE INDEX idx(col)
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

## Use Cases

### Alibaba Cloud Data Pipeline Development

Develop and test MaxCompute / Hologres pipelines locally before deploying to Alibaba Cloud. RorisDB accepts the same SQL dialect and protocols, so your `pyodps` scripts and Hologres queries work without modification.

**Key Benefits:**
- Test MaxCompute SQL jobs locally without cloud costs
- Validate Hologres queries before deployment
- Mock Alibaba Cloud APIs for CI/CD pipelines
- Development and debugging without internet access

### Application Integration Testing

Validate that your application works against MySQL-compatible databases (Doris, StarRocks, TiDB) without provisioning a cluster. Tested against 17 real-world application scenarios including WordPress, Grafana, Superset, GitLab, Airbyte, DBeaver, and phpMyAdmin.

### Multi-Cloud Compatibility Testing

Test that your SQL works across MySQL, MaxCompute, and PostgreSQL-family databases from a single deployment. Identify vendor-specific syntax early.

### Local Analytics Workbench

Run ad-hoc analytical queries on Parquet files with a familiar SQL interface. The built-in web UI at `:8080` provides an interactive environment for exploration.

## SQL Compatibility

RorisDB's core SQL engine is **Doris-compatible**. MaxCompute and Hologres protocols translate their vendor-specific syntax into this common Doris-based engine.

### Doris Table Models

All four Doris table model syntaxes are accepted and stored in metadata:

| Table Model | Syntax | Execution Status |
|------------|--------|-----------------|
| **Duplicate** | `DUPLICATE KEY(col1, ...)` | Fully functional (append semantics) |
| **Aggregate** | `AGGREGATE KEY(col1, ...) + col SUM/MAX/MIN/REPLACE` | Syntax accepted; auto-aggregation on insert not yet implemented |
| **Unique** | `UNIQUE KEY(col1, ...)` | Syntax accepted; dedup on insert not yet implemented |
| **Primary** | `PRIMARY KEY(col1, ...)` | Syntax accepted; constraint enforcement not yet implemented |

**Distribution:** `DISTRIBUTED BY HASH(col1, ...) BUCKETS N`
**Partition:** `PARTITION BY RANGE/LIST(col)`

### Data Types

Boolean, Int8-64, Float32/64, Decimal, Date, DateTime, Timestamp, String, Binary, Array, Map, Struct, JSON

### Queries

- `SELECT` with `JOIN` (INNER/LEFT/RIGHT/FULL/CROSS)
- Subqueries and CTEs (`WITH`, `WITH RECURSIVE`)
- Window functions (`ROW_NUMBER`, `RANK`, `DENSE_RANK`, `LAG`, `LEAD`, `NTILE`)
- Aggregates (`COUNT`, `SUM`, `AVG`, `MIN`, `MAX`, `GROUP_CONCAT`, `BITMAP_COUNT`)
- `GROUPING SETS`, `ROLLUP`, `CUBE`
- `UNION`, `EXCEPT`, `INTERSECT`
- `ORDER BY`, `GROUP BY`, `HAVING`, `LIMIT`

### Doris Built-in Functions

- **Date/Time:** `date_trunc`, `date_add`, `date_sub`, `months_add`, `days_add`, `hours_add`, `datediff`, `date_format`, `str_to_date`, `from_unixtime`, `unix_timestamp`, `year`, `month`, `day`, `hour`, `minute`, `second`, `dayofweek`, `dayofyear`, `last_day`, `curdate`, `curtime`
- **String:** `concat`, `concat_ws`, `substr`, `substring`, `substring_index`, `length`, `replace`, `trim`, `upper`, `lower`, `hex`, `unhex`
- **Math:** `truncate`, `abs`, `ceil`, `floor`, `round`, `log`, `pow`, `sqrt`, `mod`
- **Conditional:** `if`, `ifnull`, `case when`, `coalesce`, `nullif`
- **Utility:** `uuid`, `version`, `database`

### Operations

- `SHOW PROCESSLIST` — real-time connection and query info
- `SHOW STATUS` — server metrics (uptime, queries, threads, etc.)
- `KILL QUERY / KILL CONNECTION`
- `SHOW DATABASES / TABLES / COLUMNS`
- `SHOW VARIABLES` (global/session, 31 system variables)

### DML

- `INSERT INTO ... VALUES` (single and multi-row)
- `INSERT INTO ... SELECT`
- `INSERT INTO ... ON DUPLICATE KEY UPDATE` (syntax accepted; upsert execution not yet implemented)
- `INSERT OVERWRITE TABLE` (MaxCompute syntax, auto-converted)
- `UPDATE` with `WHERE`
- `DELETE` with `WHERE`

### DDL

- `CREATE/DROP DATABASE`
- `CREATE/DROP TABLE` with full Doris extensions (`DUPLICATE/AGGREGATE/UNIQUE/PRIMARY KEY`, `DISTRIBUTED BY HASH`, `PARTITION BY`, `PROPERTIES`)
- `ALTER TABLE` (ADD/DROP/MODIFY COLUMN, ADD/DROP PARTITION)
- `TRUNCATE TABLE`
- `CREATE INDEX` (metadata-only, stored as table properties)

### Vendor-Specific Syntax Handling

**Doris native syntax (MySQL protocol) — directly executed:**

| Syntax | Behavior |
|--------|----------|
| `DUPLICATE/AGGREGATE/UNIQUE/PRIMARY KEY` | Parsed and stored as table model metadata; only Duplicate semantics fully enforced |
| `DISTRIBUTED BY HASH(col) BUCKETS N` | Parsed and stored (BUCKETS optional, defaults to 1) |
| `PARTITION BY RANGE/LIST(col)` | Parsed and stored in metadata; partition pruning not yet implemented |
| `PROPERTIES ("key" = "value")` | Stored as table properties |
| `col TYPE SUM/MAX/MIN/REPLACE` | Accepted and stripped during parsing (aggregate modifiers) |
| `INSERT ... ON DUPLICATE KEY UPDATE` | Syntax accepted; upsert execution not yet implemented |
| `date_trunc`, `months_add`, etc. | Doris built-in functions (35 UDFs) |

**MaxCompute protocol — translated to Doris engine:**

| Syntax | Behavior |
|--------|----------|
| `PARTITIONED BY (col TYPE)` | Partition column added to schema |
| `LIFECYCLE N` | Stripped |
| `STORED AS ORC/PARQUET/...` | Stripped (unified Parquet internally) |
| `INSERT OVERWRITE TABLE` | Converted to `INSERT INTO` |
| `DISTRIBUTE BY ... SORT BY` | Converted to `ORDER BY` |
| `CLUSTER BY col` | Converted to `ORDER BY` |
| `LATERAL VIEW explode(col)` | Converted to `CROSS JOIN UNNEST` |
| `SET key=value` | No-op (accepted silently) |

**Hologres (PostgreSQL) protocol — translated to Doris engine:**

| Syntax | Behavior |
|--------|----------|
| `WITH (orientation='column', ...)` | Stripped |
| `CALL set_table_property(...)` | No-op |
| `CREATE INDEX ... USING bitmap` | Converted to standard index |
| `CREATE TRIGGER / DOMAIN / EXTENSION` | No-op (accepted silently) |
| `GRANT / REVOKE` | No-op (accepted silently) |
| `SELECT ... FOR UPDATE` | `FOR UPDATE` clause stripped |

## Building from Source

```bash
# Prerequisites: Rust 2024 edition (rustup update)
git clone https://github.com/walker83/RorisDB.git
cd RorisDB

# Build
cargo build --release

# Run tests
cargo test --workspace

# Binary: target/release/roris-fe
```

## Configuration

```bash
# Start with default ports
./target/release/roris-fe

# Custom ports and data directory
./target/release/roris-fe \
    --mysql-port 9030 \
    --maxcompute-port 9031 \
    --hologres-port 15432 \
    --data-dir /path/to/data \
    --meta-dir /path/to/meta

# TOML config file (30+ system variables)
./target/release/roris-fe --config-file config.toml
```

| Service | Default Port | CLI Flag |
|---------|-------------|----------|
| MySQL Wire Protocol | 9030 | `--mysql-port` |
| MaxCompute REST API | 9031 | `--maxcompute-port` |
| Hologres (PostgreSQL) | 15432 | `--hologres-port` |
| Web SQL Editor | 8080 | config: `server.http_port` |
| Metadata Directory | `data/fe/doris-meta` | `--meta-dir` |
| Data Directory | `data/fe/storage` | `--data-dir` |
| Config File | `roris.toml` | `--config-file` |

## Project Stats

- **Language:** Rust (~68,000 lines)
- **Crates:** 20
- **Protocols:** 3 (MySQL, MaxCompute, Hologres)
- **SQL Dialect:** Doris-compatible core
- **Test Coverage:** 1,440 unit tests + 19 integration test suites + 17 real-world scenarios + TPC-H benchmarks
- **License:** Apache 2.0

## Known Limitations

As a simulation platform, RorisDB prioritizes protocol compatibility and SQL grammar acceptance over production-grade enforcement. Key limitations:

**Storage:**
- Single Parquet file per table — all DML (INSERT/UPDATE/DELETE) reads the entire file, modifies in memory, and writes back. O(N) per operation.
- No multi-segment storage or compaction yet (planned for v0.4.0).

**Query Engine:**
- Filter pushdown is limited to simple `column op literal` patterns (with AND combinations). Complex expressions (OR, IN, IS NULL, function predicates) are not pushed down.
- Partition metadata is stored but partition pruning is not yet implemented at query time.
- `information_schema.tables` scans all Parquet files to compute row counts on every query.

**Doris Semantics:**
- `AGGREGATE KEY` auto-aggregation on insert is not implemented (syntax accepted, modifiers stored as metadata).
- `UNIQUE KEY` dedup on insert is not enforced.
- `PRIMARY KEY` constraint enforcement is not implemented.
- `ON DUPLICATE KEY UPDATE` is parsed but upsert execution is not implemented.

**Security:**
- MySQL protocol accepts any non-empty username without password validation.
- MaxCompute and Hologres protocols properly verify HMAC and MD5 authentication respectively.

**Metadata Durability:**
- EditLog (catalog change log) is flushed asynchronously every 10 seconds. DDL changes within this window may be lost on crash.

## Roadmap

### v0.4.0
- Multi-segment storage (append writes + compaction)
- Real transactions (MVCC)
- Parquet predicate pushdown (row group pruning)
- Partition table execution

### v0.5.0
- Replace `types` crate with native Arrow types
- Arrow-native QueryResult (eliminate string conversion)
- Streaming bulk load (CSV/JSON)
- Materialized views

### v1.0.0
- Production-ready stability
- Full protocol fidelity across all three adapters
- Performance optimization
- Comprehensive documentation

## Documentation

- [SQL Reference](docs/en/sql-reference.md)
- [Configuration Guide](docs/en/configuration.md)
- [Architecture Deep Dive](docs/en/architecture.md)
- [Alibaba Cloud Compatibility Matrix](docs/alibaba-cloud-compatibility.md)
- [Roadmap](docs/roadmap/README.md)

## Contributing

Contributions welcome:

1. **Star the repo** — helps discovery
2. **Report bugs** — open an issue with reproduction steps
3. **Suggest features** — share your use case
4. **Submit PRs** — fix bugs or add features
5. **Write docs** — improve documentation

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

Apache License 2.0. See [LICENSE](LICENSE).

## Acknowledgments

- **[Apache Doris](https://doris.apache.org)** — OLAP inspiration and SQL dialect reference
- **[Apache DataFusion](https://github.com/apache/arrow-datafusion)** — Query engine
- **[Apache Arrow](https://arrow.apache.org)** / **[Apache Parquet](https://parquet.apache.org)** — Columnar ecosystem
- **[sqlparser-rs](https://github.com/sqlparser-rs/sqlparser-rs)** — SQL parsing foundation
