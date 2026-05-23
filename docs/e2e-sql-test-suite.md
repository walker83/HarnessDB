# E2E SQL Test Suite

## Overview

MySQL-compatible SQL test scripts for end-to-end testing of RorisDB via the MySQL wire protocol. Each file covers a specific domain and can be executed independently against a running `roris-fe` instance.

Connect: `mysql -h 127.0.0.1 -P 9030 -uroot < tests/integration/sql/e2e/01_ddl_data_types.sql`

## Files

| File | Domain | Test Cases | SQL Statements |
|------|--------|-----------|----------------|
| `01_ddl_data_types.sql` | DDL & Data Types | 249 | 885 |
| `02_dml_insert.sql` | INSERT Operations | 215 | 1,125 |
| `03_dml_update_delete.sql` | UPDATE & DELETE | 206 | 1,326 |
| `04_select_queries.sql` | SELECT Queries | 403 | 719 |
| `05_join_aggregate_window.sql` | JOIN / Aggregate / Window | 230 | 280 |
| `06_builtin_functions.sql` | Built-in Functions | 341 | 402 |
| **Total** | | **1,644** | **4,737** |

## Coverage

### 01 - DDL & Data Types
- All supported types: BOOLEAN, TINYINT, SMALLINT, INT, BIGINT, FLOAT, DOUBLE, DECIMAL(p,s), VARCHAR(n), CHAR(n), STRING, TEXT, DATE, DATETIME
- CREATE/DROP DATABASE, CREATE/DROP TABLE (IF [NOT] EXISTS)
- ALTER TABLE ADD COLUMN
- DUPLICATE KEY, DISTRIBUTED BY HASH, BUCKETS
- DEFAULT values, NULL/NOT NULL constraints
- Boundary values, special characters in names (backtick-quoted), long names

### 02 - INSERT Operations
- Single-row INSERT, batch INSERT (2-35 rows)
- Column-specified INSERT, reordered columns
- NULL in every type position, DEFAULT values
- Expression values (arithmetic, string concatenation)
- INSERT INTO SELECT: basic, WHERE, ORDER BY LIMIT, GROUP BY, HAVING, JOINs, subqueries, CASE, DISTINCT
- Type conversion (string-to-int, int-to-varchar, etc.)
- Wide tables (up to 16 columns)

### 03 - UPDATE & DELETE
- UPDATE: single/multiple columns, expressions, all WHERE clause types
- DELETE: all comparison operators, AND/OR/NOT (nested), IN, BETWEEN, LIKE, IS NULL
- Subqueries replaced with literal values (cross-table subqueries not supported in DML WHERE)
- Chained operations: INSERT -> UPDATE -> DELETE -> verify cycles
- Complex WHERE conditions (multi-level nesting, combined operators)

### 04 - SELECT Queries
- Column selection, aliases, DISTINCT
- WHERE: all comparison operators, AND/OR/NOT, LIKE, IN, BETWEEN, IS NULL
- ORDER BY: ASC/DESC, multiple columns, expressions, NULL handling
- LIMIT/OFFSET: boundaries, LIMIT 0, large OFFSET
- CASE WHEN (simple and searched), CAST, COALESCE
- Subqueries: IN, scalar, EXISTS, correlated, derived tables
- Combined multi-feature queries

### 05 - JOIN / Aggregate / Window Functions
- JOINs: INNER, LEFT, RIGHT, FULL OUTER, CROSS, self-join, multi-table (3+)
- Aggregates: COUNT, SUM, AVG, MIN, MAX, GROUP BY, HAVING, COUNT DISTINCT
- Window: ROW_NUMBER, RANK, DENSE_RANK, LAG, LEAD, running SUM/AVG/COUNT
- Frame clauses: ROWS BETWEEN ... AND ...
- Complex real-world scenarios: sales reports, rankings, moving averages, top-N per group

### 06 - Built-in Functions
- **String**: UPPER, LOWER, LENGTH, CHAR_LENGTH, CONCAT, CONCAT_WS, SUBSTRING, TRIM, LTRIM, RTRIM, REPLACE, REVERSE, LEFT, RIGHT, LPAD, RPAD, REPEAT, SPACE, LOCATE/INSTR, SUBSTRING_INDEX
- **Math**: ABS, CEIL, FLOOR, ROUND, trunc, MOD, POWER, SQRT, LOG, LOG2, LOG10, EXP, SIGN, PI, random, GREATEST, LEAST
- **Date/Time**: NOW, CURRENT_DATE, CURRENT_TIMESTAMP, date_part, date_trunc, days_add, months_add, hours_add, CAST AS DATE/TIMESTAMP
- **Conditional**: IF, IFNULL, NULLIF, CASE WHEN, COALESCE, GREATEST, LEAST
- **Conversion**: CAST (INT, DOUBLE, VARCHAR, DECIMAL, DATE, TIMESTAMP, BOOLEAN, BIGINT)

## Known Limitations (Documented in Scripts)

65 `NOT SUPPORTED` markers document features that are not yet implemented:

### Not Supported Syntax
| Feature | Reason |
|---------|--------|
| `ALTER TABLE DROP COLUMN` | Parser recognizes syntax but handler silently ignores |
| `ALTER TABLE RENAME TO` | Parsed but not reliably executed |
| `DELETE ... ORDER BY ... LIMIT` | ORDER BY/LIMIT parsed but ignored by delete handler |
| `INSERT INTO SELECT ... UNION` | Custom parser silently discards UNION right side |
| `PRIMARY KEY` | Not handled by parser or storage |
| `LARGEINT` type | Int128/Decimal128 mapping causes issues; use BIGINT instead |

### Server Limitations
| Issue | Workaround in Tests |
|-------|-------------------|
| DATE/DATETIME columns return empty on INSERT | Use `VARCHAR(30)` columns instead |
| UPDATE SET arithmetic (`col = col + 1`) | Tests kept with warning comments |
| Cross-table subqueries in UPDATE/DELETE WHERE | Replaced with literal values |

### Unsupported Functions (documented with markers)
| Function | Alternative |
|----------|------------|
| `YEAR()` / `MONTH()` / `DAY()` | `date_part('year', CAST(col AS DATE))` |
| `HOUR()` / `MINUTE()` / `SECOND()` | `date_part('hour', CAST(col AS TIMESTAMP))` |
| `DATE_ADD(d, INTERVAL n DAY)` | `days_add(CAST(d AS DATE), n)` |
| `DATE_SUB(d, INTERVAL n DAY)` | `days_add(CAST(d AS DATE), -n)` |
| `DATEDIFF()` | Not available |
| `STR_TO_DATE()` | Not available |
| `FROM_UNIXTIME()` / `UNIX_TIMESTAMP()` | Not available |
| `MAKEDATE()` / `MAKETIME()` / `LAST_DAY()` | Not available |
| `CURDATE()` / `CURTIME()` | Use `CURRENT_DATE` / `CURRENT_TIMESTAMP` |
| `TRUNCATE(n, d)` | Use `trunc(n, d)` |
| `HEX()` / `UNHEX()` | Not available |

## Test Convention

Each test follows this pattern:
```sql
-- Test X.Y: description
CREATE TABLE t_test (...) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_test VALUES (...);
SELECT * FROM t_test ORDER BY id;
-- Expected: ...
DROP TABLE IF EXISTS t_test;
```

Each file is self-contained with its own database:
```sql
DROP DATABASE IF EXISTS e2e_xxx_test;
CREATE DATABASE e2e_xxx_test;
USE e2e_xxx_test;
-- ... tests ...
DROP DATABASE e2e_xxx_test;
```
