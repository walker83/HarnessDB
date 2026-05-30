<div align="center">

# RorisDB

### 通用数据库变色龙

**一个二进制文件，多协议支持，零基础设施**

**✅ 多数据库协议兼容 — MySQL | MaxCompute | Hologres**

**✅ 阿里云兼容 — MaxCompute & Hologres**

[![Apache-2.0 License](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](../../LICENSE)
[![Rust](https://img.shields.io/badge/Rust-2024--edition-orange.svg)](https://www.rust-lang.org)
[![Version](https://img.shields.io/badge/Version-0.3.0-green.svg)]()

[English](../../README.md) · [中文文档](README.md) · [快速开始](#快速开始) · [支持的协议](#支持的协议) · [架构设计](#架构设计) · [文档](#文档) · [贡献](#贡献)

</div>

---

## 什么是 RorisDB？

RorisDB 是一个**通用数据库模拟平台**，使用 Rust 和 Apache DataFusion 构建。核心 SQL 引擎**兼容 Doris** — 支持所有四种 Doris 表模型语法、Doris 特定函数和完整的 Doris DDL/DML 语法。在此基础上，RorisDB 同时支持 **MaxCompute (ODPS)** 和 **Hologres (PostgreSQL)** 协议，将它们的特定语法转换为基于 Doris 的通用引擎。

**多数据库协议兼容：**
- **MySQL** — 完整的 Doris SQL 语法、线协议、所有表模型
- **MaxCompute (ODPS)** — 阿里云兼容，支持 HMAC-SHA1/SHA256 认证
- **Hologres** — 阿里云兼容，支持 PostgreSQL v3 线协议

**一个二进制文件替代整个依赖矩阵：**

- 替代本地开发的 Doris 集群，支持原生 Doris SQL
- 模拟 MaxCompute (ODPS) API，用于离线数据管道测试
- 模拟 Hologres，用于实时分析开发
- 运行任何 MySQL 兼容应用，无需配置基础设施

无需容器、无需集群、无需云账单。只需 `./roris-fe` 即可开始。

## 核心能力

| 能力 | 描述 |
|------|------|
| **多数据库协议** | MySQL、MaxCompute、Hologres — 同时支持三种协议 |
| **Doris SQL 兼容** | 完整 Doris 语法：`DUPLICATE/AGGREGATE/UNIQUE/PRIMARY KEY`、`DISTRIBUTED BY HASH`、35 个 Doris UDF |
| **阿里云兼容** | 完整的 MaxCompute (ODPS) 和 Hologres 协议支持 |
| **多协议** | MySQL (:9030)、MaxCompute (:9031)、Hologres (:15432) — 单实例同时运行 |
| **列式引擎** | Apache DataFusion 查询引擎，Parquet 存储，ZSTD 压缩 |
| **协议保真** | 真实线协议 — 支持 `mysql`、`psql`、`pyodps`、JDBC 和 BI 工具 |
| **SQL 转换** | MaxCompute/Hologres 语法自动规范化为基于 Doris 的引擎 |
| **嵌入式 Web UI** | 浏览器 SQL 编辑器，地址 `http://localhost:8080`，支持 Schema 浏览 |
| **单二进制文件** | ~100MB 内存占用，60 秒启动，零外部依赖 |
| **备份恢复** | 完整数据库备份，清单跟踪 |
| **审计日志** | 异步审计日志，慢查询跟踪 |

## 支持的协议

### MySQL 线协议 — 端口 9030

连接任何 MySQL 客户端、驱动、ORM 或 BI 工具。**完整 Doris SQL 语法** — 原生引擎支持所有四种 Doris 表模型（`DUPLICATE KEY`、`AGGREGATE KEY`、`UNIQUE KEY`、`PRIMARY KEY`）、`DISTRIBUTED BY HASH`、`PARTITION BY`、Doris 内置函数和 `ON DUPLICATE KEY UPDATE`。注意：MySQL 协议当前不强制密码认证 — 任何非空用户名都被接受。

```bash
mysql -h 127.0.0.1 -P 9030 -uroot
```

### MaxCompute (ODPS) REST API — 端口 9031

**✅ 阿里云 MaxCompute 兼容** — 模拟阿里云 MaxCompute，用于数据管道开发。支持 HMAC-SHA1 (V2) 和 HMAC-SHA256 (V4) 认证、SQL 作业提交、实例管理和完整 ODPS 类型系统。

```python
from odps import ODPS
o = ODPS('roris', 'roris-secret', 'default',
         endpoint='http://127.0.0.1:9031/api')
o.execute_sql('SELECT * FROM my_table').wait_for_success()
```

### Hologres (PostgreSQL) — 端口 15432

**✅ 阿里云 Hologres 兼容** — 模拟阿里云 Hologres，使用 PostgreSQL v3 线协议。支持简单查询、扩展查询（Parse/Bind/Execute）、`pg_catalog` 系统表和 Hologres 特定 DDL（`WITH (orientation='column', ...)`、`CALL set_table_property`）。

```bash
psql -h 127.0.0.1 -p 15432 -U roris -d default
```

### 协议对比

| 特性 | MySQL | MaxCompute | Hologres |
|------|-------|------------|----------|
| 线协议 | TCP 二进制 | HTTP/REST + XML | TCP (PostgreSQL v3) |
| 认证 | 握手（无密码强制） | HMAC-SHA1 / HMAC-SHA256（已验证） | MD5（已验证） |
| 默认凭证 | 任何用户名 / 无密码 | `roris` / `roris-secret` | `roris` / `roris-secret` |
| SQL 方言 | Doris/MySQL（原生） | ODPS SQL（转换） | PostgreSQL（转换） |
| DDL 扩展 | `DUPLICATE KEY`、`DISTRIBUTED BY` | `PARTITIONED BY`、`LIFECYCLE` | `WITH (orientation=...)`、`set_table_property` |
| 客户端 | `mysql`、JDBC、DBeaver | `pyodps`、DataWorks SDK | `psql`、JDBC、pg-driver |
| 状态 | 稳定 | 阶段 1 完成 | 阶段 1 完成 |

## 快速开始

```bash
# 构建（需要 Rust 2024 edition）
git clone https://github.com/walker83/RorisDB.git
cd RorisDB
cargo build --release

# 启动所有协议
./target/release/roris-fe --mysql-port 9030 --maxcompute-port 9031 --hologres-port 15432
```

### Doris SQL 示例（MySQL 协议）

```bash
mysql -h 127.0.0.1 -P 9030 -uroot
```

```sql
CREATE DATABASE analytics;
USE analytics;

-- Doris Duplicate Key 模型，带分布
CREATE TABLE events (
    event_id INT,
    user_id INT,
    event_type VARCHAR(50),
    amount DECIMAL(10,2),
    occurred_at DATETIME
) DUPLICATE KEY(event_id)
DISTRIBUTED BY HASH(event_id) BUCKETS 1;

-- Doris Aggregate Key 模型（语法接受，修饰符存储为元数据）
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

-- Doris 内置函数
SELECT date_trunc('month', occurred_at) AS month,
       COUNT(*) AS cnt,
       SUM(amount) AS total
FROM events
GROUP BY date_trunc('month', occurred_at);
```

### MaxCompute 示例

```python
from odps import ODPS

o = ODPS('roris', 'roris-secret', 'default',
         endpoint='http://127.0.0.1:9031/api')

# 使用 ODPS 语法创建表 — LIFECYCLE、PARTITIONED BY 都有效
o.execute_sql("""
CREATE TABLE user_events (
    user_id BIGINT,
    action STRING,
    amount DOUBLE
) PARTITIONED BY (ds STRING) LIFECYCLE 365
""").wait_for_success()

# INSERT OVERWRITE 自动转换为 INSERT INTO
o.execute_sql("INSERT OVERWRITE TABLE user_events VALUES (1, 'click', 1.0)").wait_for_success()
```

### Hologres 示例

```bash
psql -h 127.0.0.1 -p 15432 -U roris -d default
```

```sql
-- Hologres DDL，带 WITH 子句 — 静默规范化
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

## 架构设计

```
                          +-----------------------------------+
                          |         客户端应用程序              |
                          | mysql | psql | pyodps | JDBC | ...|
                          +------+-------+--------+-----+----+
                                 |       |        |     |
                   MySQL 线协议   |       |        |     |  PostgreSQL v3
                   协议          |       |        |     |  线协议
                                 v       |        |     v
                    +-----------+ +------+------+ +--+-----------+
                    |  MySQL    | | MaxCompute  | |  Hologres    |
                    |  协议     | | 协议        | |  (PG)        |
                    |  :9030    | | :9031       | |  :15432      |
                    +-----+----+ +------+------+ +-+----+-------+
                          |            |              |
                          |     SQL 转换器       SQL 转换器
                          |     (剥离 ODPS      (剥离 Hologres-
                          |      语法)           特定 DDL)
                          |            |              |
                          +------+-----+------+-------+
                                 |            |
                                 v            v
                    +-----------------------------------+
                    |      Doris SQL 核心引擎            |
                    |  DDL 处理器 | DML 处理器 | SELECT  |
                    |  (DataFusion SessionContext)       |
                    +----------------+------------------+
                                     |
                       +-------------+-------------+
                       |             |             |
                       v             v             v
                  +---------+  +----------+  +----------+
                  |fe-catalog|  |fe-storage|  |fe-monitor|
                  |(元数据)  |  | (Parquet)|  | (审计)   |
                  +----------+  +----+-----+  +----------+
                                     |
                                     v
                              +-------------+
                              |   Parquet   |
                              |   文件      |
                              +-------------+
```

### SQL 转换管道

所有协议适配器将特定语法转换为 RorisDB 的 **Doris 兼容核心引擎**：

```
MaxCompute:  INSERT OVERWRITE TABLE t SELECT ...  →  INSERT INTO t SELECT ...
             PARTITIONED BY (ds STRING)           →  列 `ds` 添加到 Schema
             LIFECYCLE 365                        →  (剥离)
             DISTRIBUTE BY col SORT BY col        →  ORDER BY col

Hologres:    CREATE TABLE ... WITH (orientation='column')  →  CREATE TABLE ...
             CALL set_table_property(...)                  →  (空操作)
             CREATE INDEX idx USING bitmap(col)            →  CREATE INDEX idx(col)
```

### 技术栈

| 组件 | 技术 | 版本 |
|------|------|------|
| 查询引擎 | Apache DataFusion | 48 |
| 列式格式 | Apache Arrow | 55 |
| 存储格式 | Apache Parquet | 55 |
| SQL 解析器 | sqlparser-rs | 0.53 |
| 异步运行时 | Tokio | 1.x |
| 元数据 | JSON / RocksDB | 0.23 |

## 使用场景

### 阿里云数据管道开发

在部署到阿里云之前，本地开发和测试 MaxCompute / Hologres 管道。RorisDB 接受相同的 SQL 方言和协议，因此您的 `pyodps` 脚本和 Hologres 查询无需修改即可工作。

**关键优势：**
- 本地测试 MaxCompute SQL 作业，无需云成本
- 部署前验证 Hologres 查询
- 模拟阿里云 API，用于 CI/CD 管道
- 无互联网访问即可开发和调试

### 应用集成测试

验证您的应用是否适用于 MySQL 兼容数据库（Doris、StarRocks、TiDB），无需配置集群。已在 17 个真实应用场景中测试，包括 WordPress、Grafana、Superset、GitLab、Airbyte、DBeaver 和 phpMyAdmin。

### 多云兼容性测试

从单一部署测试您的 SQL 是否适用于 MySQL、MaxCompute 和 PostgreSQL 系列数据库。尽早识别特定语法。

### 本地分析工作台

使用熟悉的 SQL 接口在 Parquet 文件上运行临时分析查询。内置 Web UI 在 `:8080` 提供交互式探索环境。

## SQL 兼容性

RorisDB 的核心 SQL 引擎**兼容 Doris**。MaxCompute 和 Hologres 协议将特定语法转换为这个基于 Doris 的通用引擎。

### Doris 表模型

所有四种 Doris 表模型语法都被接受并存储在元数据中：

| 表模型 | 语法 | 执行状态 |
|--------|------|----------|
| **Duplicate** | `DUPLICATE KEY(col1, ...)` | 完全功能（追加语义） |
| **Aggregate** | `AGGREGATE KEY(col1, ...) + col SUM/MAX/MIN/REPLACE` | 语法接受；插入时自动聚合尚未实现 |
| **Unique** | `UNIQUE KEY(col1, ...)` | 语法接受；插入时去重尚未实现 |
| **Primary** | `PRIMARY KEY(col1, ...)` | 语法接受；约束强制尚未实现 |

**分布：** `DISTRIBUTED BY HASH(col1, ...) BUCKETS N`
**分区：** `PARTITION BY RANGE/LIST(col)`

### 数据类型

Boolean、Int8-64、Float32/64、Decimal、Date、DateTime、Timestamp、String、Binary、Array、Map、Struct、JSON

### 查询

- `SELECT` 与 `JOIN`（INNER/LEFT/RIGHT/FULL/CROSS）
- 子查询和 CTE（`WITH`、`WITH RECURSIVE`）
- 窗口函数（`ROW_NUMBER`、`RANK`、`DENSE_RANK`、`LAG`、`LEAD`、`NTILE`）
- 聚合（`COUNT`、`SUM`、`AVG`、`MIN`、`MAX`、`GROUP_CONCAT`、`BITMAP_COUNT`）
- `GROUPING SETS`、`ROLLUP`、`CUBE`
- `UNION`、`EXCEPT`、`INTERSECT`
- `ORDER BY`、`GROUP BY`、`HAVING`、`LIMIT`

### Doris 内置函数

- **日期/时间：** `date_trunc`、`date_add`、`date_sub`、`months_add`、`days_add`、`hours_add`、`datediff`、`date_format`、`str_to_date`、`from_unixtime`、`unix_timestamp`、`year`、`month`、`day`、`hour`、`minute`、`second`、`dayofweek`、`dayofyear`、`last_day`、`curdate`、`curtime`
- **字符串：** `concat`、`concat_ws`、`substr`、`substring`、`substring_index`、`length`、`replace`、`trim`、`upper`、`lower`、`hex`、`unhex`
- **数学：** `truncate`、`abs`、`ceil`、`floor`、`round`、`log`、`pow`、`sqrt`、`mod`
- **条件：** `if`、`ifnull`、`case when`、`coalesce`、`nullif`
- **工具：** `uuid`、`version`、`database`

### 操作

- `SHOW PROCESSLIST` — 实时连接和查询信息
- `SHOW STATUS` — 服务器指标（运行时间、查询数、线程数等）
- `KILL QUERY / KILL CONNECTION`
- `SHOW DATABASES / TABLES / COLUMNS`
- `SHOW VARIABLES`（全局/会话，31 个系统变量）

### DML

- `INSERT INTO ... VALUES`（单行和多行）
- `INSERT INTO ... SELECT`
- `INSERT INTO ... ON DUPLICATE KEY UPDATE`（语法接受；upsert 执行尚未实现）
- `INSERT OVERWRITE TABLE`（MaxCompute 语法，自动转换）
- `UPDATE` 与 `WHERE`
- `DELETE` 与 `WHERE`

### DDL

- `CREATE/DROP DATABASE`
- `CREATE/DROP TABLE`，支持完整 Doris 扩展（`DUPLICATE/AGGREGATE/UNIQUE/PRIMARY KEY`、`DISTRIBUTED BY HASH`、`PARTITION BY`、`PROPERTIES`）
- `ALTER TABLE`（ADD/DROP/MODIFY COLUMN、ADD/DROP PARTITION）
- `TRUNCATE TABLE`
- `CREATE INDEX`（仅元数据，存储为表属性）

### 特定语法处理

**Doris 原生语法（MySQL 协议）— 直接执行：**

| 语法 | 行为 |
|------|------|
| `DUPLICATE/AGGREGATE/UNIQUE/PRIMARY KEY` | 解析并存储为表模型元数据；仅 Duplicate 语义完全强制 |
| `DISTRIBUTED BY HASH(col) BUCKETS N` | 解析并存储（BUCKETS 可选，默认为 1） |
| `PARTITION BY RANGE/LIST(col)` | 解析并存储在元数据中；分区剪枝尚未实现 |
| `PROPERTIES ("key" = "value")` | 存储为表属性 |
| `col TYPE SUM/MAX/MIN/REPLACE` | 接受并在解析时剥离（聚合修饰符） |
| `INSERT ... ON DUPLICATE KEY UPDATE` | 语法接受；upsert 执行尚未实现 |
| `date_trunc`、`months_add` 等 | Doris 内置函数（35 个 UDF） |

**MaxCompute 协议 — 转换为 Doris 引擎：**

| 语法 | 行为 |
|------|------|
| `PARTITIONED BY (col TYPE)` | 分区列添加到 Schema |
| `LIFECYCLE N` | 剥离 |
| `STORED AS ORC/PARQUET/...` | 剥离（内部统一为 Parquet） |
| `INSERT OVERWRITE TABLE` | 转换为 `INSERT INTO` |
| `DISTRIBUTE BY ... SORT BY` | 转换为 `ORDER BY` |
| `CLUSTER BY col` | 转换为 `ORDER BY` |
| `LATERAL VIEW explode(col)` | 转换为 `CROSS JOIN UNNEST` |
| `SET key=value` | 空操作（静默接受） |

**Hologres (PostgreSQL) 协议 — 转换为 Doris 引擎：**

| 语法 | 行为 |
|------|------|
| `WITH (orientation='column', ...)` | 剥离 |
| `CALL set_table_property(...)` | 空操作 |
| `CREATE INDEX ... USING bitmap` | 转换为标准索引 |
| `CREATE TRIGGER / DOMAIN / EXTENSION` | 空操作（静默接受） |
| `GRANT / REVOKE` | 空操作（静默接受） |
| `SELECT ... FOR UPDATE` | 剥离 `FOR UPDATE` 子句 |

## 从源码构建

```bash
# 先决条件：Rust 2024 edition（rustup update）
git clone https://github.com/walker83/RorisDB.git
cd RorisDB

# 构建
cargo build --release

# 运行测试
cargo test --workspace

# 二进制文件：target/release/roris-fe
```

## 配置

```bash
# 使用默认端口启动
./target/release/roris-fe

# 自定义端口和数据目录
./target/release/roris-fe \
    --mysql-port 9030 \
    --maxcompute-port 9031 \
    --hologres-port 15432 \
    --data-dir /path/to/data \
    --meta-dir /path/to/meta

# TOML 配置文件（30+ 系统变量）
./target/release/roris-fe --config-file config.toml
```

| 服务 | 默认端口 | CLI 标志 |
|------|----------|----------|
| MySQL 线协议 | 9030 | `--mysql-port` |
| MaxCompute REST API | 9031 | `--maxcompute-port` |
| Hologres (PostgreSQL) | 15432 | `--hologres-port` |
| Web SQL 编辑器 | 8080 | 配置：`server.http_port` |
| 元数据目录 | `data/fe/doris-meta` | `--meta-dir` |
| 数据目录 | `data/fe/storage` | `--data-dir` |
| 配置文件 | `roris.toml` | `--config-file` |

## 项目统计

- **语言：** Rust（~68,000 行）
- **Crate 数：** 20
- **协议数：** 3（MySQL、MaxCompute、Hologres）
- **SQL 方言：** Doris 兼容核心
- **测试覆盖：** 1,440 个单元测试 + 19 个集成测试套件 + 17 个真实场景 + TPC-H 基准测试
- **许可证：** Apache 2.0

## 已知限制

作为模拟平台，RorisDB 优先考虑协议兼容性和 SQL 语法接受，而非生产级强制。主要限制：

**存储：**
- 每个表单个 Parquet 文件 — 所有 DML（INSERT/UPDATE/DELETE）读取整个文件，在内存中修改并写回。每次操作 O(N)。
- 尚无多段存储或压缩（计划在 v0.4.0 实现）。

**查询引擎：**
- 过滤下推仅限于简单的 `column op literal` 模式（带 AND 组合）。复杂表达式（OR、IN、IS NULL、函数谓词）不会下推。
- 分区元数据已存储，但查询时尚未实现分区剪枝。
- `information_schema.tables` 每次查询扫描所有 Parquet 文件以计算行数。

**Doris 语义：**
- 插入时 `AGGREGATE KEY` 自动聚合未实现（语法接受，修饰符存储为元数据）。
- 插入时 `UNIQUE KEY` 去重未强制。
- `PRIMARY KEY` 约束强制未实现。
- `ON DUPLICATE KEY UPDATE` 已解析但 upsert 执行未实现。

**安全性：**
- MySQL 协议接受任何非空用户名，无密码验证。
- MaxCompute 和 Hologres 协议正确验证 HMAC 和 MD5 认证。

**元数据持久性：**
- EditLog（目录更改日志）每 10 秒异步刷新。此窗口内的 DDL 更改可能在崩溃时丢失。

## 路线图

### v0.4.0
- 多段存储（追加写入 + 压缩）
- 真实事务（MVCC）
- Parquet 谓词下推（行组剪枝）
- 分区表执行

### v0.5.0
- 用原生 Arrow 类型替换 `types` crate
- Arrow 原生 QueryResult（消除字符串转换）
- 流式批量加载（CSV/JSON）
- 物化视图

### v1.0.0
- 生产级稳定性
- 所有三个适配器的完整协议保真
- 性能优化
- 完整文档

## 文档

- [架构设计](architecture.md) — 系统设计、crate 依赖、查询路径
- [功能列表](features.md) — SQL 支持、数据类型、函数、协议兼容性
- [兼容性说明](compatibility.md) — Doris SQL 兼容性矩阵和迁移路径
- [SQL 参考](../../docs/en/sql-reference.md)
- [配置指南](../../docs/en/configuration.md)
- [阿里云兼容性矩阵](../../docs/alibaba-cloud-compatibility.md)
- [路线图](../../docs/roadmap/README.md)

## 贡献

欢迎贡献：

1. **Star 仓库** — 帮助发现
2. **报告 Bug** — 开 Issue 并提供重现步骤
3. **建议功能** — 分享用例
4. **提交 PR** — 修复 Bug 或添加功能
5. **编写文档** — 改进文档

请参阅 [CONTRIBUTING.md](../../CONTRIBUTING.md) 了解指南。

## 许可证

Apache License 2.0。请参阅 [LICENSE](../../LICENSE)。

## 致谢

- **[Apache Doris](https://doris.apache.org)** — OLAP 灵感和 SQL 方言参考
- **[Apache DataFusion](https://github.com/apache/arrow-datafusion)** — 查询引擎
- **[Apache Arrow](https://arrow.apache.org)** / **[Apache Parquet](https://parquet.apache.org)** — 列式生态系统
- **[sqlparser-rs](https://github.com/sqlparser-rs/sqlparser-rs)** — SQL 解析基础
