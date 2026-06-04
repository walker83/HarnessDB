# Spark JDBC 兼容性对接

## 状态: ✅ Done

## 问题
Spark 3.5.x 通过 MySQL Connector/J 8.0.x 连接 RorisDB 时失败，因为连接初始化发送的部分 SQL 未被正确处理。

## Spark 连接 RorisDB 的完整流程

```
Spark executor → JDBC → Connector/J 8.0.x → RorisDB

连接初始化序列:
  1. Wire protocol 握手 (V10)                    → ✅ 已支持
  2. mysql_native_password 认证                  → ✅ 已支持
  3. SET NAMES utf8mb4                          → ✅ 已支持
  4. SHOW VARIABLES                             → ✅ 已支持
  5. SHOW COLLATION                             → ❌→✅ 新增
  6. SELECT @@max_allowed_packet                → ✅ 已支持
  7. SELECT @@auto_increment_increment          → ❌→✅ 新增

Schema发现 (Driver端):
  8. SELECT TABLE_NAME FROM INFORMATION_SCHEMA.TABLES WHERE ... → ✅
  9. SELECT * FROM table WHERE 1=0                             → ✅ via DataFusion

数据读写 (Executor端):
  10. COM_STMT_PREPARE → INSERT INTO ... VALUES (?, ?, ?)      → ✅
  11. COM_STMT_EXECUTE (batch)                                 → ✅
  12. COM_STMT_CLOSE                                           → ✅
```

## 修改内容

### 1. SHOW COLLATION 支持
- `crates/fe-sql-parser/src/ast.rs` — 添加 `ShowCollation { pattern }` variant
- `crates/fe-sql-parser/src/parser.rs` — 添加 `SHOW COLLATION [LIKE 'pattern']` 解析
- `harness-server/src/query_executor.rs` — 返回 13 个 MySQL 兼容 collation:
  - utf8mb4: general_ci, unicode_ci, bin, 0900_ai_ci
  - utf8: general_ci, unicode_ci, bin
  - latin1: swedish_ci, general_ci, bin
  - binary
  - gbk: chinese_ci, bin

### 2. 缺失的 @@session 变量
- `crates/mysql-protocol/src/connection.rs` — 新增:
  - `auto_increment_increment` → `"1"`
  - `auto_increment_offset` → `"1"`
  - `tx_read_only` / `transaction_read_only` → `"0"`
  - `tx_isolation` / `transaction_isolation` → `"REPEATABLE-READ"`
  - `character_set_server` / `character_set_database` → `"utf8mb4"`
  - `collation_database` → `"utf8mb4_general_ci"`
  - `version_compile_os` → `"Linux"`
  - `version_compile_machine` → `"x86_64"`
  - `init_connect` → `""`

### 3. 版本号统一
- `crates/mysql-protocol/src/packet.rs` — handshake: `"5.7.44-RovisDB"` → `"8.0.33-HarnessDB"`
- `crates/fe-config/src/variables.rs` — SystemVariableManager: `"5.7.42"` → `"8.0.33"`
- `crates/mysql-protocol/src/connection.rs` — `@@version` → `"8.0.33"`, `@@version_comment` → `"HarnessDB"`

### 4. CLIENT_CONNECT_ATTRS 能力位
- `crates/mysql-protocol/src/packet.rs` — DEFAULT_CAPABILITIES 添加 CONNECT_ATTRS
- Connector/J 8.0 发送的 connect attrs 在包末尾，被安全忽略

## Spark 使用示例

```python
from pyspark.sql import SparkSession

spark = SparkSession.builder \
    .appName("RorisDB Test") \
    .getOrCreate()

# 读取表
df = spark.read.format("jdbc") \
    .option("url", "jdbc:mysql://127.0.0.1:9030/mydb") \
    .option("driver", "com.mysql.cj.jdbc.Driver") \
    .option("dbtable", "my_table") \
    .option("user", "root") \
    .option("password", "") \
    .load()

# 推荐 JDBC URL 参数优化 Spark 性能:
# jdbc:mysql://host:9030/db?cacheServerConfiguration=true&rewriteBatchedStatements=true&useServerPrepStmts=false
```

## 推荐的 Connector/J 配置

| 参数 | 推荐值 | 原因 |
|------|--------|------|
| `cacheServerConfiguration` | `true` | 缓存 SHOW VARIABLES/COLLATION，避免每个 partition 重复查询 |
| `rewriteBatchedStatements` | `true` | 批量 INSERT 合并为多值 INSERT，10-100x 提速 |
| `useServerPrepStmts` | `false` | 避免服务端 prepared statement 开销 |
| `allowPublicKeyRetrieval` | `true` | 支持 caching_sha2_password fallback |
| `useSSL` | `false` | 内网环境避免 SSL 开销 |
