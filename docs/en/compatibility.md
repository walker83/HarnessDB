# HarnessDB — Doris Compatibility

> Version 0.3.0

HarnessDB aims for **SQL-level compatibility** with Apache Doris while using a completely different internal architecture (DataFusion + Parquet instead of custom storage engine).

## SQL Compatibility Summary

| Category | Doris | HarnessDB | Compatibility |
|----------|-------|---------|---------------|
| DDL | Full MySQL DDL | Core DDL | ~80% |
| DML | Full MySQL DML | INSERT/UPDATE/DELETE | ~70% |
| Queries | Full MySQL SELECT | DataFusion-powered | ~90% |
| Functions | 200+ functions | 50+ functions | ~25% |
| Data Types | 20+ types | 17 types | ~85% |
| Protocol | MySQL wire | MySQL wire | ~90% |

## Key Differences from Apache Doris

| Aspect | Apache Doris | HarnessDB |
|--------|-------------|---------|
| Architecture | Distributed MPP (FE + BE cluster) | Single-node (DataFusion) |
| Storage | Custom Tablet/Rowset/Segment | Apache Parquet files |
| Query Engine | Custom vectorized executor | Apache DataFusion |
| Columnar Format | Custom Block/Vector | Apache Arrow |
| Language | C++ | Rust |
| Deployment | Multi-node cluster | Single binary |
| Consensus | BDBJE (master/follower) | N/A (single node) |
| Compaction | Cumulative + Base background | N/A (single file per table) |
| Indexes | ZoneMap, BloomFilter, Inverted | Parquet page statistics |

## DDL Compatibility

| Statement | Doris | HarnessDB | Notes |
|-----------|-------|---------|-------|
| `CREATE DATABASE` | ✅ | ✅ | Compatible |
| `CREATE TABLE` | ✅ | ✅ | Supports Doris `KEYS` type syntax |
| `PARTITION BY RANGE` | ✅ | ⚠️ | Parsed, not enforced |
| `PARTITION BY LIST` | ✅ | ⚠️ | Parsed, not enforced |
| `DISTRIBUTED BY HASH` | ✅ | ⚠️ | Parsed, not enforced |
| `PROPERTIES (...)` | ✅ | ⚠️ | Parsed, stored but not applied |
| `ROLLUP` | ✅ | ⚠️ | Parsed, not executed |
| `COLOCATE WITH` | ✅ | ⚠️ | Parsed, not enforced |
| `CREATE MATERIALIZED VIEW` | ✅ | ⚠️ | Framework only |
| `CREATE EXTERNAL TABLE` | ✅ | ⚠️ | Framework only |

## DML Compatibility

| Statement | Doris | HarnessDB | Notes |
|-----------|-------|---------|-------|
| `INSERT INTO ... VALUES` | ✅ | ✅ | Multi-row, partial column |
| `INSERT INTO ... SELECT` | ✅ | ⚠️ | Not yet executed |
| `INSERT ... ON DUPLICATE KEY UPDATE` | ✅ | ⚠️ | Parsed, not executed |
| `INSERT ... SET col=val` | ✅ | ⚠️ | Parsed, not executed |
| `UPDATE` | ✅ | ✅ | All Arrow types |
| `DELETE` | ✅ | ✅ | With WHERE (AND/OR) |
| `DELETE ... ORDER BY ... LIMIT` | ✅ | ❌ | Not supported |
| Stream Load (CSV/JSON) | ✅ | ❌ | Not implemented |
| Export | ✅ | ❌ | Not implemented |

## Query Compatibility

HarnessDB delegates all query execution to **Apache DataFusion**, which provides excellent SQL coverage:

| Feature | Doris | HarnessDB | Notes |
|---------|-------|---------|-------|
| `SELECT` / `WHERE` / `ORDER BY` / `LIMIT` | ✅ | ✅ | Full support via DataFusion |
| `GROUP BY` + aggregates | ✅ | ✅ | |
| `HAVING` | ✅ | ✅ | |
| `JOIN` (all types) | ✅ | ✅ | INNER/LEFT/RIGHT/FULL/CROSS |
| Subqueries (`IN`, `EXISTS`) | ✅ | ✅ | |
| CTEs (`WITH`) | ✅ | ✅ | Including recursive |
| `UNION` / `UNION ALL` | ✅ | ✅ | |
| `INTERSECT` / `EXCEPT` | ✅ | ✅ | |
| Window functions | ✅ | ✅ | ROW_NUMBER, RANK, LAG, LEAD |
| `EXPLAIN` | ✅ | ✅ | Shows DataFusion plan |
| Query hints | ✅ | ❌ | DataFusion doesn't use hints |
| Lateral view | ✅ | ❌ | |

## Data Type Compatibility

| Doris Type | HarnessDB Type | Status |
|-----------|-------------|--------|
| `BOOLEAN` | `Boolean` | ✅ |
| `TINYINT` | `Int8` | ✅ |
| `SMALLINT` | `Int16` | ✅ |
| `INT` | `Int32` | ✅ |
| `BIGINT` | `Int64` | ✅ |
| `LARGEINT` | `Int128` (Decimal128) | ✅ |
| `FLOAT` | `Float32` | ✅ |
| `DOUBLE` | `Float64` | ✅ |
| `DECIMAL(p,s)` | `Decimal(p,s)` | ✅ |
| `DATE` | `Date` (Date32) | ✅ |
| `DATETIME` | `DateTime` (Timestamp) | ✅ |
| `VARCHAR(n)` | `String` (Utf8) | ✅ |
| `CHAR(n)` | `String` (Utf8) | ✅ |
| `STRING` | `String` (Utf8) | ✅ |
| `JSON` | `Json` (Utf8) | ✅ Stored as string |
| `ARRAY<T>` | `Array(T)` | ✅ Type mapping |
| `MAP<K,V>` | `Map(K,V)` | ✅ Type mapping |
| `STRUCT<...>` | `Struct(...)` | ✅ Type mapping |
| `BITMAP` | ❌ | Not implemented |
| `HLL` | ❌ | Not implemented |
| `QUANTILE_STATE` | ❌ | Not implemented |

## Function Compatibility

### Doris UDFs Implemented in HarnessDB

| Function | Category | Status |
|----------|---------|--------|
| `date_trunc(precision, date)` | Date/Time | ✅ |
| `months_add(date, n)` | Date/Time | ✅ |
| `days_add(date, n)` | Date/Time | ✅ |
| `hours_add(datetime, n)` | Date/Time | ✅ |
| `concat_ws(sep, s1, s2, ...)` | String | ✅ |
| `substring_index(str, delim, count)` | String | ✅ |
| `bitmap_count(expr)` | Aggregate | ✅ |

### Functions via DataFusion (built-in)

DataFusion provides 100+ built-in functions including:

- **Math**: `abs`, `ceil`, `floor`, `round`, `sqrt`, `power`, `log`, `ln`, `exp`, `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `pi`, `random`
- **String**: `concat`, `length`, `lower`, `upper`, `trim`, `ltrim`, `rtrim`, `substring`, `replace`, `reverse`, `repeat`, `lpad`, `rpad`, `split_part`, `starts_with`, `ends_with`, `contains`
- **Date/Time**: `now`, `current_date`, `current_timestamp`, `date_part`, `date_trunc`, `to_char`, `to_date`, `to_timestamp`
- **Aggregate**: `count`, `sum`, `avg`, `min`, `max`, `count(distinct)`, `array_agg`, `string_agg`
- **Window**: `row_number`, `rank`, `dense_rank`, `lag`, `lead`, `first_value`, `last_value`, `nth_value`

### Doris Functions NOT Yet Implemented

| Function | Category | Priority |
|----------|---------|----------|
| `bitmap_union`, `bitmap_intersect` | Bitmap | Low |
| `hll_union`, `hll_cardinality` | HLL | Low |
| `json_query`, `json_value` | JSON | Medium |
| `array_contains`, `array_length` | Array | Medium |
| `grouping_id`, `grouping` | Aggregate | Low |
| `collect_set`, `collect_list` | Aggregate | Medium |

## Protocol Compatibility

| Feature | Doris | HarnessDB | Notes |
|---------|-------|---------|-------|
| MySQL wire protocol | ✅ | ✅ | |
| `mysql_native_password` | ✅ | ✅ | Any password accepted |
| `COM_QUERY` | ✅ | ✅ | |
| `COM_INIT_DB` | ✅ | ✅ | |
| `COM_FIELD_LIST` | ✅ | ✅ | |
| `COM_STMT_PREPARE` | ✅ | ⚠️ | Framework only |
| SSL/TLS | ✅ | ❌ | |
| Compression | ✅ | ❌ | |
| Connection pooling | ✅ | ❌ | |

## Migration Path

For users coming from Apache Doris:

1. **SQL**: Most SELECT queries work as-is. DDL needs simplification (no partition enforcement).
2. **Data**: Export from Doris as Parquet, copy to `data/{db}/{table}/data.parquet`.
3. **Clients**: Any MySQL client works — no driver changes needed.
4. **Functions**: Most DataFusion functions match Doris behavior. Check UDF list above.

## Version History

| Version | Date | Highlights |
|---------|------|-----------|
| 0.3.0 | 2026-05-23 | DataFusion 48 upgrade, 481 E2E tests, SQL bug fixes, startup simplification (-1833 lines) |
| 0.2.0 | 2026-05-21 | DataFusion/Arrow migration, type system completion, pushdown |
| 0.1.5 | 2026-05-21 | Massive code cleanup (-5500 lines), bug fixes |
| 0.1.0–0.1.4 | 2026-05 | Initial development, parser, protocol, catalog |
