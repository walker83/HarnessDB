# HarnessDB Features

> Version 0.3.0

## SQL Language Support

### DDL (Data Definition)

| Statement | Status | Notes |
|-----------|--------|-------|
| `CREATE DATABASE` | ✅ | With `IF NOT EXISTS` |
| `DROP DATABASE` | ✅ | With `IF EXISTS` |
| `CREATE TABLE` | ✅ | Column definitions, `KEYS` type, `PARTITION BY`, `DISTRIBUTED BY` |
| `DROP TABLE` | ✅ | With `IF EXISTS` |
| `ALTER TABLE` | ✅ | `ADD COLUMN`, `DROP COLUMN`, `RENAME COLUMN`, `MODIFY COLUMN` |
| `TRUNCATE TABLE` | ✅ | Deletes data, keeps schema |
| `CREATE VIEW` | ✅ | Stores query definition |
| `DROP VIEW` | ✅ | |
| `CREATE INDEX` | ⚠️ | Parsed, not enforced |
| `CREATE MATERIALIZED VIEW` | ⚠️ | Parsed, framework only |

### DML (Data Manipulation)

| Statement | Status | Notes |
|-----------|--------|-------|
| `INSERT INTO ... VALUES` | ✅ | Multi-row, partial column, type-aware |
| `INSERT INTO ... SELECT` | ⚠️ | Parsed, not executed |
| `UPDATE ... SET ... WHERE` | ✅ | All Arrow types supported |
| `DELETE FROM ... WHERE` | ✅ | Supports `AND`/`OR` conditions |

### Queries

| Feature | Status | Notes |
|---------|--------|-------|
| `SELECT` | ✅ | Via DataFusion |
| `WHERE` | ✅ | Full DataFusion predicate support |
| `ORDER BY` | ✅ | |
| `LIMIT` / `OFFSET` | ✅ | |
| `GROUP BY` | ✅ | |
| `HAVING` | ✅ | |
| `JOIN` (INNER) | ✅ | |
| `JOIN` (LEFT/RIGHT/FULL/CROSS) | ✅ | |
| Subqueries | ✅ | `IN`, `EXISTS`, correlated |
| CTEs (`WITH`) | ✅ | Recursive supported |
| `UNION` / `UNION ALL` | ✅ | |
| `INTERSECT` / `EXCEPT` | ✅ | |
| `EXPLAIN` | ✅ | Shows DataFusion execution plan |

### Aggregate Functions

| Function | Status |
|----------|--------|
| `COUNT(*)` | ✅ |
| `COUNT(expr)` | ✅ |
| `COUNT(DISTINCT expr)` | ✅ |
| `SUM` | ✅ |
| `AVG` | ✅ |
| `MIN` / `MAX` | ✅ |
| `GROUP_CONCAT` | ✅ |

### Window Functions

| Function | Status |
|----------|--------|
| `ROW_NUMBER()` | ✅ |
| `RANK()` | ✅ |
| `DENSE_RANK()` | ✅ |
| `LAG(expr, n)` | ✅ |
| `LEAD(expr, n)` | ✅ |

### Built-in Functions

| Category | Examples | Count |
|----------|---------|-------|
| Math | `abs`, `ceil`, `floor`, `round`, `sqrt`, `pow`, `log`, `sin`, `cos`, `tan`, `rand` | 30+ |
| String | `concat`, `concat_ws`, `length`, `upper`, `lower`, `trim`, `substring`, `replace`, `substring_index` | 15+ |
| Date/Time | `date_trunc`, `months_add`, `days_add`, `hours_add`, `now`, `curdate` | 8+ |
| Aggregate | `bitmap_count` | 1 |
| Type conversion | `CAST(expr AS type)` | All Arrow types |

## Data Types

| HarnessDB Type | SQL Syntax | Arrow Type | Notes |
|-------------|-----------|-----------|-------|
| `Boolean` | `BOOLEAN`, `BOOL` | Boolean | |
| `Int8` | `TINYINT` | Int8 | |
| `Int16` | `SMALLINT` | Int16 | |
| `Int32` | `INT`, `INTEGER` | Int32 | |
| `Int64` | `BIGINT` | Int64 | |
| `Float32` | `FLOAT` | Float32 | |
| `Float64` | `DOUBLE` | Float64 | |
| `Decimal(p,s)` | `DECIMAL(10,2)` | Decimal128 | Full precision |
| `Date` | `DATE` | Date32 | Days since epoch |
| `DateTime` | `DATETIME`, `TIMESTAMP` | Timestamp(Second) | |
| `String` | `VARCHAR`, `CHAR`, `TEXT`, `STRING` | Utf8 | |
| `Binary` | `BINARY`, `BLOB` | Binary | |
| `Array(T)` | `ARRAY<INT>` | List | Nested type |
| `Map(K,V)` | `MAP<STRING, INT>` | Map | Nested type |
| `Struct` | `STRUCT<...>` | Struct | Nested type |
| `Json` | `JSON` | Utf8 | Stored as string |

## SHOW / DESCRIBE Commands

| Command | Status |
|---------|--------|
| `SHOW DATABASES` | ✅ |
| `SHOW TABLES` | ✅ | With `LIKE` pattern |
| `SHOW CREATE TABLE` | ✅ | |
| `SHOW CREATE DATABASE` | ✅ | |
| `DESCRIBE table` / `DESC table` | ✅ | |
| `SHOW TABLE STATUS` | ✅ | |
| `SHOW VARIABLES` | ✅ | Returns empty |
| `SHOW PROCESSLIST` | ✅ | Returns current connection |

## Transaction Support

| Feature | Status | Notes |
|---------|--------|-------|
| `BEGIN` / `START TRANSACTION` | ⚠️ | Parsed, state tracked |
| `COMMIT` | ⚠️ | State tracked, no MVCC |
| `ROLLBACK` | ⚠️ | State tracked, no actual rollback |
| `SAVEPOINT` | ⚠️ | Parsed, state tracked |
| Isolation levels | ⚠️ | `SET TRANSACTION ISOLATION LEVEL` parsed, not enforced |

> Transactions are **syntactically supported** but not enforced — there is no MVCC or WAL. Operations are individually atomic (Parquet atomic write).

## MySQL Protocol Compatibility

| Feature | Status |
|---------|--------|
| Wire protocol | ✅ MySQL text protocol |
| Authentication | ✅ `mysql_native_password` (any password accepted) |
| `COM_QUERY` | ✅ |
| `COM_INIT_DB` | ✅ (USE database) |
| `COM_FIELD_LIST` | ✅ |
| `COM_QUIT` | ✅ |
| Prepared statements | ⚠️ Framework only |
| SSL/TLS | ❌ Not implemented |

## Monitoring & Observability

| Feature | Status | Notes |
|---------|--------|-------|
| Audit log | ✅ | Query logging |

## User Management

| Feature | Status | Notes |
|---------|--------|-------|
| `CREATE USER` | ⚠️ | Parsed, stub execution |
| `DROP USER` | ⚠️ | Parsed, stub execution |
| `GRANT` / `REVOKE` | ⚠️ | Parsed, stub execution |
| `SET PASSWORD` | ⚠️ | Parsed, stub execution |

> User management SQL is **parsed and accepted** for compatibility, but not enforced — all connections have full access.
