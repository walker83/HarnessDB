# E2E SQL Test Suite

## Overview

MySQL-compatible SQL test scripts for end-to-end testing of HarnessDB via the MySQL wire protocol. Each file covers a specific domain and can be executed independently against a running `harness-db` instance.

Connect: `mysql -h 127.0.0.1 -P 9030 -uroot < tests/integration/sql/e2e/01_ddl_data_types.sql`

## Files

| File | Domain | Test Cases | SQL Statements |
|------|--------|-----------|----------------|
| `01_ddl_data_types.sql` | DDL & Data Types | 259 | 930 |
| `02_dml_insert.sql` | INSERT Operations | 220 | 1,170 |
| `03_dml_update_delete.sql` | UPDATE & DELETE | 225 | 1,400 |
| `04_select_queries.sql` | SELECT Queries | 403 | 719 |
| `05_join_aggregate_window.sql` | JOIN / Aggregate / Window | 230 | 280 |
| `06_builtin_functions.sql` | Built-in Functions | 383 | 480 |
| **Total** | | **1,720** | **4,979** |

## Coverage

### 01 - DDL & Data Types
- All supported types: BOOLEAN, TINYINT, SMALLINT, INT, BIGINT, FLOAT, DOUBLE, DECIMAL(p,s), VARCHAR(n), CHAR(n), STRING, TEXT, DATE, DATETIME
- CREATE/DROP DATABASE, CREATE/DROP TABLE (IF [NOT] EXISTS)
- ALTER TABLE ADD/DROP COLUMN, ALTER TABLE RENAME TO
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
- DELETE: all comparison operators, AND/OR/NOT (nested), IN, BETWEEN, LIKE, IS NULL, ORDER BY LIMIT
- Cross-table subqueries in UPDATE/DELETE WHERE
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
- **String**: UPPER, LOWER, LENGTH, CHAR_LENGTH, CONCAT, CONCAT_WS, SUBSTRING, TRIM, LTRIM, RTRIM, REPLACE, REVERSE, LEFT, RIGHT, LPAD, RPAD, REPEAT, SPACE, LOCATE/INSTR, SUBSTRING_INDEX, HEX, UNHEX
- **Math**: ABS, CEIL, FLOOR, ROUND, truncate, MOD, POWER, SQRT, LOG, LOG2, LOG10, EXP, SIGN, PI, random, GREATEST, LEAST
- **Date/Time**: NOW, CURRENT_DATE, CURRENT_TIMESTAMP, date_part, date_trunc, days_add, months_add, hours_add, CAST AS DATE/TIMESTAMP, DATEDIFF, STR_TO_DATE, FROM_UNIXTIME, UNIX_TIMESTAMP, MAKEDATE, MAKETIME, LAST_DAY
- **Conditional**: IF, IFNULL, NULLIF, CASE WHEN, COALESCE, GREATEST, LEAST
- **Conversion**: CAST (INT, DOUBLE, VARCHAR, DECIMAL, DATE, TIMESTAMP, BOOLEAN, BIGINT)
- **Other**: UUID, VERSION, DATABASE, GROUP_CONCAT

## Known Limitations

The following items have known issues or use DataFusion-native alternatives:

### Not Supported Syntax
| Feature | Status |
|---------|--------|
| `PRIMARY KEY` | Parsed but not enforced; use DUPLICATE KEY instead |
| `LARGEINT` type | Use BIGINT instead (Int128 mapping issues) |

### Server Limitations
| Issue | Workaround in Tests |
|-------|-------------------|
| UPDATE SET arithmetic (`col = col + 1`) | Tests kept with warning comments |

### Function Alternatives
| MySQL Function | DataFusion Equivalent |
|----------|------------|
| `YEAR()` / `MONTH()` / `DAY()` | `date_part('year', CAST(col AS DATE))` |
| `HOUR()` / `MINUTE()` / `SECOND()` | `date_part('hour', CAST(col AS TIMESTAMP))` |
| `DATE_ADD(d, INTERVAL n DAY)` | `days_add(CAST(d AS DATE), n)` |
| `DATE_SUB(d, INTERVAL n DAY)` | `days_add(CAST(d AS DATE), -n)` |
| `CURDATE()` / `CURTIME()` | `CURRENT_DATE` / `CURRENT_TIMESTAMP` |

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
