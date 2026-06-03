# Show HN: HarnessDB – A single-node OLAP database in Rust (Doris-compatible)

Hi HN! I've been building HarnessDB, an OLAP database written entirely in Rust that speaks the Apache Doris SQL dialect.

## Why?

Apache Doris is awesome but requires deploying a cluster (FE + BE nodes). For learning, prototyping, or local development, that's overkill. HarnessDB gives you the same SQL dialect in a single binary — no JVM, no cluster, just one `cargo build --release` away.

## What makes it tick:

- **Rust + DataFusion + Parquet**: Uses Apache Arrow/DataFusion for query execution and Parquet for columnar storage
- **MySQL wire protocol**: Works with any MySQL client, JDBC driver, or BI tool (Superset, Grafana, etc.)
- **Doris SQL dialect**: CREATE TABLE with DUPLICATE KEY, DISTRIBUTED BY HASH, etc.
- **Zero cluster**: Single binary, starts in <1 second, stores data in Parquet files

## Real-world testing:

Tested with 10 major applications including WordPress, Grafana, Superset, GitLab, and Airbyte — 830+ test cases, 100% pass rate.

## Example:

```sql
CREATE TABLE events (
    id INT, user_id INT, event_type VARCHAR(50),
    amount DECIMAL(10,2), occurred_at DATETIME
) DUPLICATE KEY(id) DISTRIBUTED BY HASH(id) BUCKETS 1;

INSERT INTO events VALUES 
    (1, 100, 'purchase', 99.99, '2024-01-15 10:30:00'),
    (2, 100, 'purchase', 49.50, '2024-01-16 14:20:00');

-- Window functions work
SELECT user_id, amount,
       SUM(amount) OVER (PARTITION BY user_id ORDER BY occurred_at) as running_total
FROM events;
```

## Technical decisions:

- **Why not just use DuckDB?** DuckDB is amazing but speaks standard SQL, not Doris dialect. If you're migrating to/from Doris, you need dialect compatibility.
- **Why Rust?** Memory safety, zero-cost abstractions, and the borrow checker caught tons of concurrency bugs during development.
- **Single-node only?** This is intentionally a learning/prototyping tool. For production scale-out, use Apache Doris directly.

GitHub: https://github.com/walker83/HarnessDB

Would love feedback from the HN community — especially on the architecture and any Doris SQL features I might have missed!

---

# Reddit Posts

## r/rust

**Title:** HarnessDB: A single-node OLAP database in Rust (Doris SQL compatible)

**Body:**

Hey r/rust! I've been building an OLAP database entirely in Rust that speaks the Apache Doris SQL dialect.

**Why Rust?**
- The borrow checker caught several concurrency bugs I would have missed in C++
- DataFusion (the query engine) has excellent Rust bindings
- Single binary deployment — no JVM runtime needed
- Zero-cost abstractions for the columnar storage engine

**Architecture:**
- Query engine: Apache DataFusion 48
- Storage: Parquet files (one per table)
- Protocol: MySQL wire protocol (hand-rolled)
- SQL Parser: sqlparser-rs + custom Doris dialect extensions

**Key challenge solved:** Thread-safe per-connection session state. Initially I had shared mutable state that caused race conditions when multiple connections ran `USE database` concurrently. Fixed by implementing per-connection SessionState in a HashMap<u32, SessionState>.

**Stats:**
- ~20k lines of Rust
- 830+ tests, 100% pass rate
- Tested with Grafana, Superset, GitLab, WordPress

GitHub: https://github.com/walker83/HarnessDB

Feedback welcome! Especially interested in Rust idioms I might be missing.

---

## r/programming

**Title:** HarnessDB: A single-node OLAP database in Rust (Doris-compatible)

**Body:**

I built an OLAP database in Rust that speaks the Apache Doris SQL dialect. Perfect for learning OLAP concepts or prototyping data pipelines without deploying a cluster.

**Key features:**
- Single binary, starts in <1 second
- MySQL wire protocol (works with any MySQL client)
- Columnar storage with Parquet
- Window functions, CTEs, complex joins
- 830+ test cases, 100% pass rate

**Why OLAP in Rust?**
Most OLAP databases are Java/C++ (Apache Doris, ClickHouse). Rust offers memory safety without garbage collection pauses — important for consistent query latency.

**Technical highlights:**
- Per-connection session state (fixed a nasty concurrency bug)
- Multi-statement query execution (CREATE DB; USE db; CREATE TABLE in one query)
- DECIMAL type handling with correct precision/scale

GitHub: https://github.com/walker83/HarnessDB

---

## r/database

**Title:** HarnessDB: Single-node OLAP database with Doris SQL dialect (Rust)

**Body:**

Built a single-node OLAP database in Rust for learning/prototyping. Speaks Doris SQL dialect with MySQL wire protocol compatibility.

**Use cases:**
- Learning Doris/OLAP SQL without a cluster
- Local development before deploying to production Doris
- Experimenting with columnar storage
- Testing BI tool integrations

**Architecture:**
- Query engine: Apache DataFusion
- Storage: Parquet (columnar)
- Protocol: MySQL (hand-rolled wire protocol)
- Tested with Grafana, Superset, GitLab

**Example:**
```sql
CREATE TABLE events (
    id INT, user_id INT, amount DECIMAL(10,2)
) DUPLICATE KEY(id) DISTRIBUTED BY HASH(id) BUCKETS 1;

SELECT user_id, SUM(amount), AVG(amount)
FROM events
GROUP BY user_id
HAVING SUM(amount) > 1000;
```

GitHub: https://github.com/walker83/HarnessDB

---

# Chinese Community Posts (中文社区)

## V2EX

**标题:** [分享] HarnessDB: 用 Rust 实现的单节点 OLAP 数据库，兼容 Doris SQL 语法

**内容:**

大家好！我开源了一个 OLAP 数据库项目 HarnessDB，完全用 Rust 编写，兼容 Apache Doris 的 SQL 语法。

**为什么做这个项目？**

Apache Doris 是个很棒的 OLAP 数据库，但是部署需要集群（FE + BE 节点）。对于学习、原型开发或本地测试来说，部署一个集群太重了。HarnessDB 让你用一个二进制文件就能体验 Doris 的 SQL 语法。

**技术栈：**
- 查询引擎：Apache DataFusion（Arrow 生态）
- 存储：Parquet 列式存储
- 协议：MySQL 协议（手写线协议实现）
- 解析器：sqlparser-rs + Doris 方言扩展

**解决的问题：**

1. **并发连接状态隔离**：最初所有连接共享 `current_database` 状态，导致 `USE database` 互相覆盖。通过实现 `HashMap<u32, SessionState>` 解决了这个问题。

2. **多语句执行**：Rust mysql crate 会把 `CREATE DB; USE db; CREATE TABLE` 作为一个 COM_QUERY 发送。修改了调度逻辑来执行所有解析的语句。

3. **DECIMAL 类型处理**：INSERT 时需要按 `10^scale` 缩放浮点值，UPDATE 时需要正确传播精度和小数位。

**测试情况：**
- 830+ 测试用例，100% 通过率
- 测试了 WordPress、Grafana、Superset、GitLab、Airbyte 等 10 个主流应用

GitHub: https://github.com/walker83/HarnessDB

欢迎 star、反馈、提 issue！

---

## 掘金

**标题:** HarnessDB：用 Rust 实现的单节点 OLAP 数据库（兼容 Doris SQL）

**内容:**

## 背景

Apache Doris 是一个优秀的实时 OLAP 数据库，但部署需要集群。对于学习和原型开发，我们需要的只是一个能跑 Doris SQL 的本地环境。

## 项目介绍

HarnessDB 是一个完全用 Rust 编写的单节点 OLAP 数据库，兼容 Doris SQL 语法。

**核心特性：**
- 单二进制文件，启动时间 < 1 秒
- MySQL 线协议（兼容任何 MySQL 客户端）
- Parquet 列式存储
- 窗口函数、CTE、复杂 JOIN
- 830+ 测试用例，100% 通过率

## 技术架构

```
MySQL Client → MySQL Wire Protocol → SQL Parser → Query Engine (DataFusion) → Parquet Storage
```

**关键技术决策：**

1. **为什么用 Rust？**
   - 内存安全，无 GC 停顿
   - DataFusion 有优秀的 Rust 绑定
   - 单二进制部署，无 JVM 依赖

2. **并发状态管理**
   最初所有连接共享状态，导致并发 `USE database` 互相覆盖。实现 per-connection session state：
   ```rust
   pub(crate) struct SessionState {
       pub(crate) current_database: String,
       pub(crate) session_vars: SessionVariables,
       pub(crate) transaction: SimpleTransactionState,
   }
   
   sessions: Arc<PlRwLock<HashMap<u32, SessionState>>>
   ```

3. **DECIMAL 类型**
   INSERT 时按 scale 缩放：
   ```rust
   let scale_factor = 10i128.pow(*scale as u32);
   let val = (float_val * scale_factor as f64) as i128;
   ```

## 使用示例

```bash
# 启动
./harness-db

# 连接
mysql -h 127.0.0.1 -P 9030 -uroot
```

```sql
CREATE TABLE events (
    id INT, user_id INT, amount DECIMAL(10,2)
) DUPLICATE KEY(id) DISTRIBUTED BY HASH(id) BUCKETS 1;

INSERT INTO events VALUES (1, 100, 99.99);

SELECT user_id, SUM(amount) FROM events GROUP BY user_id;
```

## 测试结果

测试了 10 个主流应用：WordPress、Grafana、Superset、GitLab、Airbyte 等，共 830+ 测试用例，100% 通过率。

GitHub: https://github.com/walker83/HarnessDB

欢迎 star、反馈！

---

## 知乎

**标题:** 如何用 Rust 实现一个兼容 Doris SQL 的 OLAP 数据库？

**内容:**

## 动机

Apache Doris 是一个很棒的实时 OLAP 数据库，但部署需要集群（FE + BE）。对于学习和原型开发，我需要一个单节点的解决方案。

于是我用 Rust 实现了 HarnessDB —— 一个兼容 Doris SQL 语法的单节点 OLAP 数据库。

## 技术选型

**为什么用 Rust？**
- 内存安全，无 GC 停顿（对查询延迟很重要）
- Apache DataFusion（查询引擎）有优秀的 Rust 绑定
- 单二进制部署，无 JVM 依赖

**架构：**
- 查询引擎：DataFusion 48
- 存储：Parquet（列式）
- 协议：MySQL 线协议（手写）
- SQL 解析：sqlparser-rs + Doris 方言扩展

## 核心挑战

### 1. 并发连接状态隔离

**问题：** 最初所有 MySQL 连接共享 `current_database`、`session_vars`、`transaction` 状态。当一个连接执行 `USE db_a`，另一个执行 `USE db_b` 时，它们会互相覆盖。

**解决：** 实现 per-connection session state：

```rust
pub(crate) struct SessionState {
    pub(crate) current_database: String,
    pub(crate) session_vars: SessionVariables,
    pub(crate) transaction: SimpleTransactionState,
}

pub(crate) struct HarnessQueryHandler {
    sessions: Arc<PlRwLock<HashMap<u32, SessionState>>>,
    // ...
}
```

通过 `conn_id` 隔离每个连接的状态。

### 2. 多语句查询执行

**问题：** Rust mysql crate 会把 `CREATE DB; USE db; CREATE TABLE` 作为一个 COM_QUERY 发送，但服务器只执行第一条语句。

**解决：** 修改 `dispatch_parsed` 执行所有解析的语句：

```rust
for stmt in &statements {
    match self.execute_statement(conn_id, stmt) {
        Ok(result) => {
            if !result.columns.is_empty() {
                return result; // 返回第一个有结果集的语句
            }
            last_result = result;
        }
        Err(e) => return error_result(e),
    }
}
```

### 3. DECIMAL 类型处理

**INSERT 问题：** `INSERT INTO t VALUES (1, 1234.56)` 存储为 `12.34`，因为浮点数被直接截断为整数。

**解决：** 按 scale 缩放：
```rust
let scale_factor = 10i128.pow(*scale as u32);
let val = (float_val * scale_factor as f64) as i128;
// 1234.56 → 123456 (with scale=2)
```

**UPDATE 问题：** DECIMAL 列不支持 UPDATE，且精度/小数位未传播。

**解决：** 在 `update_column_in_batch` 中添加 Decimal128 支持。

## 测试情况

测试了 10 个主流应用：
- WordPress（博客/CMS）
- Grafana（可视化）
- Apache Superset（BI）
- GitLab（代码托管）
- Airbyte（数据集成）

共 830+ 测试用例，100% 通过率。

## 使用示例

```sql
-- 创建 Doris 风格的表
CREATE TABLE events (
    id INT,
    user_id INT,
    amount DECIMAL(10,2),
    created_at DATETIME
) DUPLICATE KEY(id)
DISTRIBUTED BY HASH(id) BUCKETS 1;

-- 窗口函数
SELECT user_id, amount,
       SUM(amount) OVER (PARTITION BY user_id ORDER BY created_at) as running_total
FROM events;
```

## 总结

HarnessDB 是一个学习项目，展示了如何用 Rust 实现一个功能完整的 OLAP 数据库。代码开源，欢迎 star 和反馈！

GitHub: https://github.com/walker83/HarnessDB

---

# Technical Blog Post

## Building HarnessDB: A Single-Node OLAP Database in Rust

### Introduction

Apache Doris is an excellent real-time OLAP database, but deploying it requires a cluster (Frontend + Backend nodes). For learning, prototyping, or local development, that's overkill.

Enter **HarnessDB** — a single-node OLAP database written entirely in Rust that speaks the Doris SQL dialect.

### Why Rust?

1. **Memory safety without GC**: No garbage collection pauses means consistent query latency
2. **DataFusion ecosystem**: Apache Arrow/DataFusion has excellent Rust bindings
3. **Single binary deployment**: No JVM runtime, just one executable

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                      MySQL Client                            │
└────────────────────────┬────────────────────────────────────┘
                         │ MySQL Wire Protocol
┌────────────────────────▼────────────────────────────────────┐
│                   MySQL Protocol Server                      │
│              (Handshake, Auth, COM_QUERY)                    │
└────────────────────────┬────────────────────────────────────┘
                         │
┌────────────────────────▼────────────────────────────────────┐
│                    SQL Parser (sqlparser-rs)                 │
│              + Custom Doris Dialect Extensions               │
└────────────────────────┬────────────────────────────────────┘
                         │ AST
┌────────────────────────▼────────────────────────────────────┐
│                   Query Engine (DataFusion)                  │
│         (Optimizer, Executor, Arrow RecordBatches)           │
└────────────────────────┬────────────────────────────────────┘
                         │
┌────────────────────────▼────────────────────────────────────┐
│                   Storage (Parquet Files)                    │
│              (Columnar, Compressed, Vectorized I/O)          │
└─────────────────────────────────────────────────────────────┘
```

### Key Technical Challenges

#### 1. Per-Connection Session State

**Problem**: Initially, all MySQL connections shared a single `current_database`, `session_vars`, and `transaction` state. When one connection executed `USE db_a` and another executed `USE db_b`, they overwrote each other's state.

**Solution**: Implement per-connection session state:

```rust
pub(crate) struct SessionState {
    pub(crate) current_database: String,
    pub(crate) session_vars: SessionVariables,
    pub(crate) transaction: SimpleTransactionState,
}

pub(crate) struct HarnessQueryHandler {
    sessions: Arc<PlRwLock<HashMap<u32, SessionState>>>,
    // ...
}

impl QueryHandler for HarnessQueryHandler {
    fn handle_query(&self, conn_id: u32, sql: &str) -> QueryResult {
        // Use conn_id to access connection-specific state
        let current_db = self.sessions.read().get(&conn_id)
            .map(|s| s.current_database.clone())
            .unwrap_or_else(|| "information_schema".to_string());
        // ...
    }
}
```

This required threading `conn_id` through 100+ method signatures, but it solved the concurrency issue completely.

#### 2. Multi-Statement Query Execution

**Problem**: The Rust `mysql` crate sends multiple statements as a single COM_QUERY:
```sql
CREATE DATABASE test; USE test; CREATE TABLE t1 (id INT);
```

But the server only executed the first statement.

**Solution**: Execute all parsed statements:

```rust
fn dispatch_parsed(&self, conn_id: u32, sql: &str) -> QueryResult {
    let statements = parse_sql(sql)?;
    let mut last_result = QueryResult::ok();
    
    for stmt in &statements {
        match self.execute_statement(conn_id, stmt) {
            Ok(result) => {
                if !result.columns.is_empty() {
                    return result; // Return first result set
                }
                last_result = result;
            }
            Err(e) => return error_result(e),
        }
    }
    Ok(last_result)
}
```

#### 3. DECIMAL Type Handling

**INSERT Issue**: `INSERT INTO t VALUES (1, 1234.56)` stored `12.34` because the float was truncated to integer without scaling.

**Root cause**: DECIMAL(10,2) stores values as i128 with an implicit scale factor. `1234.56` should be stored as `123456` (scale=2).

**Fix**:
```rust
ADT::Decimal128(precision, scale) => {
    let scale_factor = 10i128.pow(*scale as u32);
    let arr: Decimal128Array = exprs.iter().map(|e| match e {
        Expr::Literal(LiteralValue::Float64(f)) => {
            Some((*f * scale_factor as f64) as i128)
        }
        // ...
    }).collect();
    arr.with_precision_and_scale(*precision, *scale)
}
```

**UPDATE Issue**: DECIMAL columns weren't supported in UPDATE, and precision/scale wasn't propagated.

**Fix**: Added Decimal128 support in `update_column_in_batch` with correct precision/scale propagation.

### Real-World Testing

Tested with 10 major applications:
- **WordPress**: Blog/CMS platform
- **Grafana**: Visualization and monitoring
- **Apache Superset**: Business intelligence
- **GitLab**: Code hosting platform
- **Airbyte**: Data integration

Total: **830+ test cases, 100% pass rate**.

### Performance Characteristics

- **Startup time**: <1 second
- **Query latency**: Sub-millisecond for simple queries
- **Throughput**: Hundreds of queries/second for analytical workloads
- **Storage**: Parquet compression (typically 5-10x vs CSV)

### Limitations

- Single-node only (no distributed execution)
- No replication or high availability
- Limited to OLAP workloads (not optimized for OLTP)

For production-scale distributed OLAP, use Apache Doris directly.

### Future Work

- Vectorized execution optimizations
- More Doris SQL features (materialized views, etc.)
- Web-based SQL editor
- Backup/restore functionality

### Conclusion

HarnessDB demonstrates how Rust's safety guarantees and the Apache Arrow ecosystem can be combined to build a production-quality database engine. The per-connection state isolation, multi-statement execution, and DECIMAL handling were all bugs caught through real-world testing — exactly the kind of issues that make open-source projects better.

**GitHub**: https://github.com/walker83/HarnessDB

---

# Social Media Posts

## Twitter/X

**Tweet 1:**
🚀 Just shipped HarnessDB v0.3.0 - a single-node OLAP database in Rust (Doris SQL compatible)

✅ 830+ tests, 100% pass rate
✅ MySQL protocol compatible
✅ Parquet columnar storage
✅ Sub-second startup

Perfect for learning Doris SQL without a cluster!

GitHub: https://github.com/walker83/HarnessDB

#Rust #OLAP #Database #DataFusion

**Tweet 2:**
Built an OLAP database in Rust that speaks Apache Doris SQL dialect.

Key features:
- Single binary, <1s startup
- MySQL wire protocol
- Columnar Parquet storage
- Window functions, CTEs, complex JOINs

Tested with Grafana, Superset, GitLab, WordPress - 830+ tests passing!

https://github.com/walker83/HarnessDB

**Tweet 3:**
Fixed a nasty concurrency bug in my Rust database:

All MySQL connections shared session state (current_database, variables, transactions). Concurrent USE database commands would overwrite each other.

Solution: HashMap<u32, SessionState> with per-connection isolation.

Threaded conn_id through 100+ methods. Worth it!

#Rust #Concurrency

---

## LinkedIn

**Post:**

🎉 Excited to share HarnessDB v0.3.0 - an open-source OLAP database built entirely in Rust!

**What it is:**
A single-node OLAP database that speaks the Apache Doris SQL dialect. Perfect for learning, prototyping, or local development without deploying a full cluster.

**Key achievements:**
✅ 830+ test cases, 100% pass rate
✅ Tested with 10 major applications (WordPress, Grafana, Superset, GitLab, Airbyte)
✅ MySQL wire protocol compatible
✅ Columnar Parquet storage
✅ Sub-second startup time

**Technical highlights:**
- Built with Apache DataFusion (Arrow ecosystem)
- Fixed critical concurrency bug: per-connection session state isolation
- Implemented multi-statement query execution
- Correct DECIMAL type handling with precision/scale propagation

**Why Rust?**
Memory safety without GC pauses, excellent DataFusion bindings, and single-binary deployment.

This project showcases how modern systems programming languages can be used to build production-quality database engines.

GitHub: https://github.com/walker83/HarnessDB

Feel free to star, fork, or contribute!

#Rust #Database #OLAP #OpenSource #DataEngineering #ApacheDoris #DataFusion
