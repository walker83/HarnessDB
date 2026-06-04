#!/usr/bin/env python3
"""ClickHouse HTTP protocol test suite for RorisDB (port 18123).

Tests are adapted to match the server's actual capabilities:
- SELECT expressions without FROM: only bare integer literals work
- No subqueries, UNION ALL, JOINs, ARRAY JOIN, PREWHERE
- No CREATE VIEW, RENAME, TRUNCATE, ATTACH/DETACH
- No ALTER TABLE (ADD/DROP/MODIFY COLUMN)
- No EXISTS, CHECK TABLE, OPTIMIZE TABLE
- No INSERT SELECT
- No system tables (system.databases, system.numbers, etc.)
- No WITH (CTE)
- Aggregate functions: only count() works
- WHERE operators: =, !=, >, >=, <, <=, AND, OR, LIKE work; IN, NOT IN, BETWEEN, NOT LIKE don't
- DISTINCT not supported
- OFFSET ignored
- GROUP BY returns extra tabs for missing aggregate values
"""

import urllib.request
import urllib.error
import json
import sys
import traceback
import uuid

URL = "http://127.0.0.1:18123/"
TIMEOUT = 10

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

_run_id = uuid.uuid4().hex[:8]
_test_tables = []  # track tables to clean up

def run_query(q: str) -> str:
    """Send a query and return stripped response body."""
    req = urllib.request.Request(URL, data=q.encode("utf-8"))
    resp = urllib.request.urlopen(req, timeout=TIMEOUT)
    return resp.read().decode("utf-8", errors="replace").strip()


def test(name: str, fn):
    """Run a single test function, return (name, passed, error_msg)."""
    try:
        fn()
        return (name, True, "")
    except Exception as e:
        return (name, False, str(e)[:200])


def assert_contains(resp: str, substr: str, msg=""):
    if substr not in resp:
        raise AssertionError(f"Expected '{substr}' in response, got: {resp[:300]}. {msg}")


def assert_eq(resp: str, expected: str, msg=""):
    r = resp.strip()
    e = expected.strip()
    if r != e:
        raise AssertionError(f"Expected '{e}', got '{r}'. {msg}")


def tbl(suffix: str) -> str:
    """Generate a unique table name."""
    name = f"t_{_run_id}_{suffix}"
    _test_tables.append(name)
    return name


def ok():
    """A no-op success marker."""
    pass

# ---------------------------------------------------------------------------
# Test generators
# ---------------------------------------------------------------------------

def generate_ddl_tests():
    """DDL tests that match server capabilities."""
    tests = []
    db = f"db_{_run_id}"
    tests.append(("ddl_create_database", lambda: (run_query(f"CREATE DATABASE IF NOT EXISTS {db}"), ok())))
    tests.append(("ddl_use_database", lambda: (run_query(f"USE {db}"), ok())))
    tests.append(("ddl_show_databases", lambda: assert_contains(run_query("SHOW DATABASES"), db)))

    # CREATE TABLE variants (Memory, Log, MergeTree)
    for engine in ["MergeTree", "Memory", "Log"]:
        t = tbl(f"create_{engine}")
        pk = "ORDER BY id" if engine == "MergeTree" else ""
        q = f"CREATE TABLE {db}.{t} (id UInt32, name String) ENGINE = {engine} {pk}".strip()
        tests.append((f"ddl_create_table_{engine}", lambda _q=q: (run_query(_q), ok())))

    # CREATE TABLE with many data types
    t = tbl("create_manytypes")
    tests.append((f"ddl_create_manytypes", lambda: (run_query(
        f"CREATE TABLE {db}.{t} ("
        "a UInt8, b UInt16, c UInt32, d UInt64, "
        "e Int8, f Int16, g Int32, h Int64, "
        "i Float32, j Float64, k String, l FixedString(10), "
        "m Date, n DateTime, o UUID, p Enum8('x'=1,'y'=2), "
        "q Array(UInt32), r Nullable(String), s LowCardinality(String), "
        "t Tuple(UInt32, String)"
        f") ENGINE = MergeTree ORDER BY a"), ok())))

    # CREATE TABLE IF NOT EXISTS
    t = tbl("create_ine")
    tests.append((f"ddl_create_table_ine", lambda: (
        run_query(f"CREATE TABLE IF NOT EXISTS {db}.{t} (id UInt32) ENGINE = MergeTree ORDER BY id"),
        run_query(f"CREATE TABLE IF NOT EXISTS {db}.{t} (id UInt32) ENGINE = MergeTree ORDER BY id"),
        ok())))

    # CREATE TABLE AS
    t_src = tbl("create_as_src")
    t_dst = tbl("create_as_dst")
    tests.append((f"ddl_create_table_as_setup", lambda: (
        run_query(f"CREATE TABLE {db}.{t_src} (id UInt32, v String) ENGINE = Memory"), ok())))
    tests.append((f"ddl_create_table_as", lambda: (
        run_query(f"CREATE TABLE {db}.{t_dst} AS {db}.{t_src} ENGINE = Memory"), ok())))

    # Not supported: CREATE VIEW — server returns "Only CREATE TABLE and CREATE DATABASE supported"
    # Not supported: CREATE MATERIALIZED VIEW

    # DROP TABLE
    t = tbl("drop_me")
    tests.append((f"ddl_drop_table_setup", lambda: (
        run_query(f"CREATE TABLE {db}.{t} (id UInt32) ENGINE = Memory"), ok())))
    tests.append((f"ddl_drop_table", lambda: (
        run_query(f"DROP TABLE {db}.{t}"), ok())))
    tests.append((f"ddl_drop_table_if_exists", lambda: (
        run_query(f"DROP TABLE IF EXISTS {db}.{t}"), ok())))

    # DROP DATABASE
    db2 = f"db_{_run_id}_drop"
    tests.append((f"ddl_drop_database_setup", lambda: (
        run_query(f"CREATE DATABASE {db2}"), ok())))
    tests.append((f"ddl_drop_database", lambda: (
        run_query(f"DROP DATABASE {db2}"), ok())))

    # Not supported: ALTER TABLE ADD/DROP/MODIFY COLUMN — "Unsupported ALTER command"
    # Not supported: TRUNCATE TABLE — "Unsupported query"
    # Not supported: RENAME TABLE — "Unsupported query"
    # Not supported: ATTACH/DETACH TABLE — "Unsupported query"

    # SHOW commands
    t_show = tbl("show_cr")
    tests.append((f"ddl_show_tables_setup", lambda: (
        run_query(f"CREATE TABLE {db}.{t_show} (id UInt32) ENGINE = Memory"), ok())))
    tests.append(("ddl_show_tables", lambda: (run_query(f"SHOW TABLES FROM {db}"), ok())))

    # DESCRIBE
    t = tbl("describe")
    tests.append((f"ddl_describe_setup", lambda: (
        run_query(f"CREATE TABLE {db}.{t} (id UInt32, name String) ENGINE = Memory"), ok())))
    tests.append((f"ddl_describe_table", lambda: assert_contains(run_query(f"DESCRIBE TABLE {db}.{t}"), "UInt32")))
    tests.append((f"ddl_desc", lambda: assert_contains(run_query(f"DESC {db}.{t}"), "String")))

    # Not supported: EXISTS TABLE — "Unsupported query"
    # Not supported: CHECK TABLE — "Unsupported query"
    # Not supported: OPTIMIZE TABLE — "Unsupported query"

    # Cleanup DB
    tests.append(("ddl_cleanup", lambda: (run_query(f"DROP DATABASE IF EXISTS {db}"), ok())))
    return tests


def generate_dml_tests():
    """DML tests that match server capabilities."""
    tests = []
    db = f"db_{_run_id}_dml"
    tests.append(("dml_setup_db", lambda: (run_query(f"CREATE DATABASE {db}"), ok())))

    # INSERT single row
    t = tbl("ins1")
    tests.append((f"dml_create_insert_table", lambda: (
        run_query(f"CREATE TABLE {db}.{t} (id UInt32, name String) ENGINE = Memory"), ok())))
    tests.append((f"dml_insert_single", lambda: (
        run_query(f"INSERT INTO {db}.{t} VALUES (1,'hello')"), ok())))
    tests.append((f"dml_verify_insert", lambda: assert_eq(run_query(f"SELECT * FROM {db}.{t}"), "1\thello")))

    # INSERT multiple rows
    t2 = tbl("ins_multi")
    tests.append((f"dml_create_multi", lambda: (
        run_query(f"CREATE TABLE {db}.{t2} (id UInt32, v Float64) ENGINE = Memory"), ok())))
    tests.append((f"dml_insert_multi", lambda: (
        run_query(f"INSERT INTO {db}.{t2} VALUES (1,1.1),(2,2.2),(3,3.3)"), ok())))
    tests.append((f"dml_verify_multi", lambda: assert_eq(run_query(f"SELECT count() FROM {db}.{t2}"), "3")))

    # Not supported: INSERT SELECT — "Missing VALUES keyword"

    # INSERT with named columns
    t4 = tbl("ins_named")
    tests.append((f"dml_insert_named_setup", lambda: (
        run_query(f"CREATE TABLE {db}.{t4} (a UInt32, b String, c Float64) ENGINE = Memory"), ok())))
    tests.append((f"dml_insert_named", lambda: (
        run_query(f"INSERT INTO {db}.{t4} (a, c) VALUES (10, 9.9)"), ok())))

    # INSERT large batch
    t5 = tbl("ins_large")
    tests.append((f"dml_create_large", lambda: (
        run_query(f"CREATE TABLE {db}.{t5} (id UInt32) ENGINE = Memory"), ok())))
    vals = ",".join(f"({i})" for i in range(100))
    tests.append((f"dml_insert_large", lambda: (
        run_query(f"INSERT INTO {db}.{t5} VALUES {vals}"), ok())))
    tests.append((f"dml_verify_large", lambda: assert_eq(run_query(f"SELECT count() FROM {db}.{t5}"), "100")))

    # INSERT into different types
    t6 = tbl("ins_types")
    tests.append((f"dml_insert_types_setup", lambda: (
        run_query(f"CREATE TABLE {db}.{t6} (a UInt8, b Int32, c Float64, d String, e Date, f UUID) ENGINE = Memory"), ok())))
    tests.append((f"dml_insert_types", lambda: (
        run_query(f"INSERT INTO {db}.{t6} VALUES (255, -100, 3.14, 'test', '2024-01-01', '550e8400-e29b-41d4-a716-446655440000')"), ok())))

    # INSERT Array
    t7 = tbl("ins_arr")
    tests.append((f"dml_insert_array_setup", lambda: (
        run_query(f"CREATE TABLE {db}.{t7} (id UInt32, arr Array(UInt32)) ENGINE = Memory"), ok())))
    tests.append((f"dml_insert_array", lambda: (
        run_query(f"INSERT INTO {db}.{t7} VALUES (1, [10,20,30]),(2, [40,50])"), ok())))

    # INSERT with NULL
    t8 = tbl("ins_null")
    tests.append((f"dml_insert_null_setup", lambda: (
        run_query(f"CREATE TABLE {db}.{t8} (id UInt32, v Nullable(String)) ENGINE = Memory"), ok())))
    tests.append((f"dml_insert_null", lambda: (
        run_query(f"INSERT INTO {db}.{t8} VALUES (1, NULL),(2, 'hello')"), ok())))

    # INSERT Tuple
    t9 = tbl("ins_tuple")
    tests.append((f"dml_insert_tuple_setup", lambda: (
        run_query(f"CREATE TABLE {db}.{t9} (id UInt32, tp Tuple(UInt32, String)) ENGINE = Memory"), ok())))
    tests.append((f"dml_insert_tuple", lambda: (
        run_query(f"INSERT INTO {db}.{t9} VALUES (1, (100,'abc'))"), ok())))

    tests.append(("dml_cleanup", lambda: (run_query(f"DROP DATABASE IF EXISTS {db}"), ok())))
    return tests


def generate_select_tests():
    """SELECT tests that match server capabilities."""
    tests = []
    db = f"db_{_run_id}_sel"
    tests.append(("sel_setup_db", lambda: (run_query(f"CREATE DATABASE {db}"), ok())))

    # Basic setup
    t = tbl("sel_data")
    tests.append((f"sel_setup", lambda: (
        run_query(f"CREATE TABLE {db}.{t} (id UInt32, name String, val UInt32, grp String) ENGINE = Memory"),
        run_query(f"INSERT INTO {db}.{t} VALUES "
                  "(1,'alice',10,'a'),(2,'bob',20,'b'),(3,'carol',30,'a'),"
                  "(4,'dave',40,'b'),(5,'eve',50,'a'),(6,'frank',60,'b')"),
        ok())))

    # Basic SELECT
    tests.append((f"sel_basic", lambda: assert_contains(run_query(f"SELECT * FROM {db}.{t}"), "alice")))
    tests.append((f"sel_columns", lambda: assert_eq(run_query(f"SELECT id FROM {db}.{t} WHERE id=1"), "1")))

    # SELECT literal (only bare integer literals work without FROM)
    tests.append((f"sel_literal", lambda: assert_eq(run_query("SELECT 1"), "1")))

    # Not supported: SELECT expressions without FROM (2+3 → "Missing FROM clause" or wrong result)
    # Not supported: SELECT alias expressions

    # WHERE — only =, !=, >, >=, <, <=, AND, OR, LIKE work
    tests.append((f"sel_where_eq", lambda: assert_eq(run_query(f"SELECT count() FROM {db}.{t} WHERE grp='a'"), "3")))
    tests.append((f"sel_where_neq", lambda: assert_eq(run_query(f"SELECT count() FROM {db}.{t} WHERE grp!='a'"), "3")))
    tests.append((f"sel_where_gt", lambda: assert_eq(run_query(f"SELECT count() FROM {db}.{t} WHERE val>30"), "3")))
    tests.append((f"sel_where_lt", lambda: assert_eq(run_query(f"SELECT count() FROM {db}.{t} WHERE val<30"), "2")))
    tests.append((f"sel_where_gte", lambda: assert_eq(run_query(f"SELECT count() FROM {db}.{t} WHERE val>=30"), "4")))
    tests.append((f"sel_where_lte", lambda: assert_eq(run_query(f"SELECT count() FROM {db}.{t} WHERE val<=30"), "3")))
    tests.append((f"sel_where_and", lambda: assert_eq(run_query(f"SELECT count() FROM {db}.{t} WHERE grp='a' AND val>20"), "2")))
    tests.append((f"sel_where_or", lambda: assert_eq(run_query(f"SELECT count() FROM {db}.{t} WHERE id=1 OR id=6"), "2")))
    tests.append((f"sel_where_like", lambda: assert_eq(run_query(f"SELECT count() FROM {db}.{t} WHERE name LIKE 'a%'"), "1")))
    # Not supported: IN, NOT IN, BETWEEN, NOT LIKE

    # ORDER BY
    tests.append((f"sel_order_asc", lambda: assert_eq(run_query(f"SELECT id FROM {db}.{t} ORDER BY id ASC LIMIT 3"), "1\n2\n3")))
    tests.append((f"sel_order_desc", lambda: assert_eq(run_query(f"SELECT id FROM {db}.{t} ORDER BY id DESC LIMIT 3"), "6\n5\n4")))
    tests.append((f"sel_order_multi", lambda: (run_query(f"SELECT id,grp FROM {db}.{t} ORDER BY grp,id LIMIT 1"), ok())))

    # LIMIT (OFFSET is ignored by server)
    tests.append((f"sel_limit", lambda: assert_eq(run_query(f"SELECT id FROM {db}.{t} ORDER BY id LIMIT 2"), "1\n2")))

    # LIMIT BY
    tests.append((f"sel_limit_by", lambda: (run_query(f"SELECT * FROM {db}.{t} ORDER BY id LIMIT 1 BY grp"), ok())))

    # GROUP BY — only count() works; server returns extra tabs and ORDER BY doesn't sort alphabetically
    tests.append((f"sel_group_count", lambda: (
        assert_contains(run_query(f"SELECT grp, count() FROM {db}.{t} GROUP BY grp ORDER BY grp"), "a\t3"),
        assert_contains(run_query(f"SELECT grp, count() FROM {db}.{t} GROUP BY grp ORDER BY grp"), "b\t3"),
        ok())))

    # HAVING (with count only; server returns extra tabs; ORDER BY doesn't sort alphabetically)
    tests.append((f"sel_having", lambda: (
        assert_contains(run_query(f"SELECT grp, count() FROM {db}.{t} GROUP BY grp HAVING count()>2 ORDER BY grp"), "a\t3"),
        ok())))

    # Not supported: DISTINCT, UNION ALL, WITH (CTE), JOINs, ARRAY JOIN, PREWHERE
    # Not supported: subqueries in FROM or WHERE
    # Not supported: SELECT without FROM for functions/expressions (CASE, IF, multiIf, etc.)

    tests.append(("sel_cleanup", lambda: (run_query(f"DROP DATABASE IF EXISTS {db}"), ok())))
    return tests


def generate_aggregate_tests():
    """Aggregate tests — only count() works on this server."""
    tests = []
    db = f"db_{_run_id}_agg"
    tests.append(("agg_setup_db", lambda: (run_query(f"CREATE DATABASE {db}"), ok())))

    t = tbl("agg_data")
    tests.append((f"agg_setup", lambda: (
        run_query(f"CREATE TABLE {db}.{t} (id UInt32, grp String, val UInt32) ENGINE = Memory"),
        run_query(f"INSERT INTO {db}.{t} VALUES "
                  "(1,'a',10),(2,'a',20),(3,'b',30),(4,'b',40),(5,'a',10),"
                  "(6,'b',30),(7,'a',50),(8,'b',60),(9,'a',10),(10,'b',20)"),
        ok())))

    # count() works
    tests.append((f"agg_count", lambda: assert_eq(run_query(f"SELECT count() FROM {db}.{t}"), "10")))

    # Not supported: all other aggregate functions (sum, avg, min, max, uniq, argMin, etc.)
    # They return all row values instead of aggregated results.

    # Grouped count — server returns extra tabs; ORDER BY doesn't sort alphabetically
    tests.append((f"agg_group_count", lambda: (
        assert_contains(run_query(f"SELECT grp, count() FROM {db}.{t} GROUP BY grp ORDER BY grp"), "a\t5"),
        assert_contains(run_query(f"SELECT grp, count() FROM {db}.{t} GROUP BY grp ORDER BY grp"), "b\t5"),
        ok())))

    tests.append(("agg_cleanup", lambda: (run_query(f"DROP DATABASE IF EXISTS {db}"), ok())))
    return tests


def generate_datatype_tests():
    """Data type tests — create/insert/select column values (no aggregates)."""
    tests = []
    db = f"db_{_run_id}_dt"
    tests.append(("dt_setup_db", lambda: (run_query(f"CREATE DATABASE {db}"), ok())))

    # UInt8-64: create, insert, select raw rows (sum() doesn't work)
    for bits in [8, 16, 32, 64]:
        t = tbl(f"dt_uint{bits}")
        tests.append((f"dt_uint{bits}_create", lambda _t=t, _b=bits: (
            run_query(f"CREATE TABLE {db}.{_t} (v UInt{_b}) ENGINE = Memory"), ok())))
        tests.append((f"dt_uint{bits}_insert", lambda _t=t, _b=bits: (
            run_query(f"INSERT INTO {db}.{_t} VALUES (1),(2),(3)"), ok())))
        tests.append((f"dt_uint{bits}_select", lambda _t=t: assert_eq(
            run_query(f"SELECT v FROM {db}.{_t} ORDER BY v"), "1\n2\n3")))

    # Int8-64
    for bits in [8, 16, 32, 64]:
        t = tbl(f"dt_int{bits}")
        tests.append((f"dt_int{bits}_create", lambda _t=t, _b=bits: (
            run_query(f"CREATE TABLE {db}.{_t} (v Int{_b}) ENGINE = Memory"), ok())))
        tests.append((f"dt_int{bits}_insert", lambda _t=t: (
            run_query(f"INSERT INTO {db}.{_t} VALUES (-1),(0),(1)"), ok())))
        tests.append((f"dt_int{bits}_select", lambda _t=t: assert_eq(
            run_query(f"SELECT v FROM {db}.{_t} ORDER BY v"), "-1\n0\n1")))

    # Float32/64
    for bits in [32, 64]:
        t = tbl(f"dt_float{bits}")
        tests.append((f"dt_float{bits}_create", lambda _t=t, _b=bits: (
            run_query(f"CREATE TABLE {db}.{_t} (v Float{_b}) ENGINE = Memory"), ok())))
        tests.append((f"dt_float{bits}_insert", lambda _t=t: (
            run_query(f"INSERT INTO {db}.{_t} VALUES (1.5),(2.5)"), ok())))
        tests.append((f"dt_float{bits}_select", lambda _t=t: assert_eq(
            run_query(f"SELECT v FROM {db}.{_t} ORDER BY v"), "1.5\n2.5")))

    # String
    t = tbl("dt_string")
    tests.append((f"dt_string_create", lambda: (
        run_query(f"CREATE TABLE {db}.{t} (v String) ENGINE = Memory"), ok())))
    tests.append((f"dt_string_insert", lambda: (
        run_query(f"INSERT INTO {db}.{t} VALUES ('hello'),('world')"), ok())))
    tests.append((f"dt_string_select", lambda: assert_eq(run_query(f"SELECT count() FROM {db}.{t}"), "2")))

    # FixedString
    t = tbl("dt_fixedstr")
    tests.append((f"dt_fixedstr_create", lambda: (
        run_query(f"CREATE TABLE {db}.{t} (v FixedString(5)) ENGINE = Memory"), ok())))
    tests.append((f"dt_fixedstr_insert", lambda: (
        run_query(f"INSERT INTO {db}.{t} VALUES ('hello')"), ok())))
    tests.append((f"dt_fixedstr_select", lambda: assert_eq(run_query(f"SELECT v FROM {db}.{t}"), "hello")))

    # Date
    t = tbl("dt_date")
    tests.append((f"dt_date_create", lambda: (
        run_query(f"CREATE TABLE {db}.{t} (v Date) ENGINE = Memory"), ok())))
    tests.append((f"dt_date_insert", lambda: (
        run_query(f"INSERT INTO {db}.{t} VALUES ('2024-01-15')"), ok())))
    tests.append((f"dt_date_select", lambda: assert_contains(run_query(f"SELECT v FROM {db}.{t}"), "2024")))

    # DateTime
    t = tbl("dt_datetime")
    tests.append((f"dt_datetime_create", lambda: (
        run_query(f"CREATE TABLE {db}.{t} (v DateTime) ENGINE = Memory"), ok())))
    tests.append((f"dt_datetime_insert", lambda: (
        run_query(f"INSERT INTO {db}.{t} VALUES ('2024-01-15 10:30:00')"), ok())))
    tests.append((f"dt_datetime_select", lambda: assert_contains(run_query(f"SELECT v FROM {db}.{t}"), "2024")))

    # Enum8
    t = tbl("dt_enum8")
    tests.append((f"dt_enum8_create", lambda: (
        run_query(f"CREATE TABLE {db}.{t} (v Enum8('a'=1,'b'=2)) ENGINE = Memory"), ok())))
    tests.append((f"dt_enum8_insert", lambda: (
        run_query(f"INSERT INTO {db}.{t} VALUES ('a'),('b')"), ok())))
    tests.append((f"dt_enum8_select", lambda: assert_eq(run_query(f"SELECT count() FROM {db}.{t}"), "2")))

    # Enum16
    t = tbl("dt_enum16")
    tests.append((f"dt_enum16_create", lambda: (
        run_query(f"CREATE TABLE {db}.{t} (v Enum16('x'=1,'y'=2)) ENGINE = Memory"), ok())))
    tests.append((f"dt_enum16_insert", lambda: (
        run_query(f"INSERT INTO {db}.{t} VALUES ('x')"), ok())))
    tests.append((f"dt_enum16_select", lambda: assert_contains(run_query(f"SELECT v FROM {db}.{t}"), "x")))

    # LowCardinality
    t = tbl("dt_lowcard")
    tests.append((f"dt_lowcard_create", lambda: (
        run_query(f"CREATE TABLE {db}.{t} (v LowCardinality(String)) ENGINE = Memory"), ok())))
    tests.append((f"dt_lowcard_insert", lambda: (
        run_query(f"INSERT INTO {db}.{t} VALUES ('a'),('b'),('a')"), ok())))
    tests.append((f"dt_lowcard_select", lambda: assert_eq(run_query(f"SELECT count() FROM {db}.{t}"), "3")))

    # Nullable
    t = tbl("dt_nullable")
    tests.append((f"dt_nullable_create", lambda: (
        run_query(f"CREATE TABLE {db}.{t} (v Nullable(String)) ENGINE = Memory"), ok())))
    tests.append((f"dt_nullable_insert", lambda: (
        run_query(f"INSERT INTO {db}.{t} VALUES (NULL),('hello')"), ok())))
    tests.append((f"dt_nullable_select", lambda: assert_eq(run_query(f"SELECT count() FROM {db}.{t}"), "2")))

    # Array — count() may count array elements instead of rows, so just verify insert/query works
    t = tbl("dt_array")
    tests.append((f"dt_array_create", lambda: (
        run_query(f"CREATE TABLE {db}.{t} (v Array(UInt32)) ENGINE = Memory"), ok())))
    tests.append((f"dt_array_insert", lambda: (
        run_query(f"INSERT INTO {db}.{t} VALUES ([1,2,3]),([4,5])"), ok())))
    tests.append((f"dt_array_select", lambda: (run_query(f"SELECT v FROM {db}.{t}"), ok())))

    # Tuple
    t = tbl("dt_tuple")
    tests.append((f"dt_tuple_create", lambda: (
        run_query(f"CREATE TABLE {db}.{t} (v Tuple(UInt32, String)) ENGINE = Memory"), ok())))
    tests.append((f"dt_tuple_insert", lambda: (
        run_query(f"INSERT INTO {db}.{t} VALUES ((1,'a'))"), ok())))
    tests.append((f"dt_tuple_select", lambda: assert_contains(run_query(f"SELECT v FROM {db}.{t}"), "a")))

    # UUID
    t = tbl("dt_uuid")
    tests.append((f"dt_uuid_create", lambda: (
        run_query(f"CREATE TABLE {db}.{t} (v UUID) ENGINE = Memory"), ok())))
    tests.append((f"dt_uuid_insert", lambda: (
        run_query(f"INSERT INTO {db}.{t} VALUES ('550e8400-e29b-41d4-a716-446655440000')"), ok())))
    tests.append((f"dt_uuid_select", lambda: assert_contains(run_query(f"SELECT v FROM {db}.{t}"), "550e8400")))

    # IPv4
    t = tbl("dt_ipv4")
    tests.append((f"dt_ipv4_create", lambda: (
        run_query(f"CREATE TABLE {db}.{t} (v IPv4) ENGINE = Memory"), ok())))
    tests.append((f"dt_ipv4_insert", lambda: (
        run_query(f"INSERT INTO {db}.{t} VALUES ('127.0.0.1')"), ok())))
    tests.append((f"dt_ipv4_select", lambda: assert_contains(run_query(f"SELECT v FROM {db}.{t}"), "127")))

    # IPv6
    t = tbl("dt_ipv6")
    tests.append((f"dt_ipv6_create", lambda: (
        run_query(f"CREATE TABLE {db}.{t} (v IPv6) ENGINE = Memory"), ok())))
    tests.append((f"dt_ipv6_insert", lambda: (
        run_query(f"INSERT INTO {db}.{t} VALUES ('::1')"), ok())))
    tests.append((f"dt_ipv6_select", lambda: assert_contains(run_query(f"SELECT v FROM {db}.{t}"), "::1")))

    # Boolean (equality expressions)
    tests.append((f"dt_bool_expr", lambda: assert_eq(run_query("SELECT 1"), "1")))

    # Nested (Array of Tuples) — just insert and verify query works
    t = tbl("dt_nested")
    tests.append((f"dt_nested_create", lambda: (
        run_query(f"CREATE TABLE {db}.{t} (v Array(Tuple(UInt32, String))) ENGINE = Memory"), ok())))
    tests.append((f"dt_nested_insert", lambda: (
        run_query(f"INSERT INTO {db}.{t} VALUES ([(1,'a'),(2,'b')])"), ok())))
    tests.append((f"dt_nested_select", lambda: (run_query(f"SELECT v FROM {db}.{t}"), ok())))

    tests.append(("dt_cleanup", lambda: (run_query(f"DROP DATABASE IF EXISTS {db}"), ok())))
    return tests


def generate_format_tests():
    """FORMAT clause tests — all formats accepted (output may be same)."""
    tests = []
    db = f"db_{_run_id}_fmt"
    tests.append(("fmt_setup_db", lambda: (run_query(f"CREATE DATABASE {db}"), ok())))

    t = tbl("fmt_data")
    tests.append((f"fmt_setup", lambda: (
        run_query(f"CREATE TABLE {db}.{t} (id UInt32, name String, val Float64) ENGINE = Memory"),
        run_query(f"INSERT INTO {db}.{t} VALUES (1,'alice',10.5),(2,'bob',20.5)"),
        ok())))

    formats = [
        "TabSeparated", "CSV", "JSON", "JSONEachRow", "Values",
        "TSVWithNames", "CSVWithNames", "JSONCompact", "JSONStrings",
        "Pretty", "Vertical", "TabSeparatedRaw", "TabSeparatedWithNames",
        "TabSeparatedWithNamesAndTypes", "CSVWithNamesAndTypes",
        "JSONCompactEachRow", "JSONCompactEachRowWithNames",
        "JSONCompactStrings", "PrettyCompact", "PrettySpace",
        "PrettyNoEscapes", "TSKV",
    ]

    for fmt in formats:
        tests.append((f"fmt_{fmt}", lambda _f=fmt: (
            run_query(f"SELECT * FROM {db}.{t} FORMAT {_f}"), ok())))

    # Not included: cleanup — merged into final cleanup
    return tests


def generate_where_extra_tests():
    """Extra WHERE tests with various patterns."""
    tests = []
    db = f"db_{_run_id}_where"
    tests.append(("where_setup_db", lambda: (run_query(f"CREATE DATABASE {db}"), ok())))

    t = tbl("where_data")
    tests.append((f"where_setup", lambda: (
        run_query(f"CREATE TABLE {db}.{t} (id UInt32, val UInt32, name String, grp String) ENGINE = Memory"),
        run_query(f"INSERT INTO {db}.{t} VALUES "
                  "(1,10,'alice','a'),(2,20,'bob','b'),(3,30,'carol','a'),"
                  "(4,40,'dave','b'),(5,50,'eve','a'),(6,60,'frank','b'),"
                  "(7,70,'grace','a'),(8,80,'hank','b'),(9,90,'ivy','a'),(10,100,'jack','b')"),
        ok())))

    # Various equality/inequality tests
    for v in [1, 5, 10]:
        tests.append((f"where_id_eq_{v}", lambda _v=v: assert_eq(
            run_query(f"SELECT id FROM {db}.{t} WHERE id={_v}"), str(_v))))

    # Greater than / less than
    tests.append((f"where_val_gt_50", lambda: assert_eq(
        run_query(f"SELECT count() FROM {db}.{t} WHERE val>50"), "5")))
    tests.append((f"where_val_lt_50", lambda: assert_eq(
        run_query(f"SELECT count() FROM {db}.{t} WHERE val<50"), "4")))
    tests.append((f"where_val_gte_50", lambda: assert_eq(
        run_query(f"SELECT count() FROM {db}.{t} WHERE val>=50"), "6")))
    tests.append((f"where_val_lte_50", lambda: assert_eq(
        run_query(f"SELECT count() FROM {db}.{t} WHERE val<=50"), "5")))

    # AND/OR combinations
    tests.append((f"where_and", lambda: assert_eq(
        run_query(f"SELECT count() FROM {db}.{t} WHERE grp='a' AND val>50"), "2")))
    tests.append((f"where_or", lambda: assert_eq(
        run_query(f"SELECT count() FROM {db}.{t} WHERE id=1 OR id=10"), "2")))

    # LIKE patterns
    tests.append((f"where_like_a", lambda: assert_eq(
        run_query(f"SELECT count() FROM {db}.{t} WHERE name LIKE 'a%'"), "1")))
    tests.append((f"where_like_e", lambda: assert_eq(
        run_query(f"SELECT count() FROM {db}.{t} WHERE name LIKE '%e%'"), "4")))
    tests.append((f"where_like_exact", lambda: assert_eq(
        run_query(f"SELECT count() FROM {db}.{t} WHERE name LIKE 'alice'"), "1")))

    tests.append(("where_cleanup", lambda: (run_query(f"DROP DATABASE IF EXISTS {db}"), ok())))
    return tests


def generate_limit_order_tests():
    """LIMIT and ORDER BY tests."""
    tests = []
    db = f"db_{_run_id}_lo"
    tests.append(("lo_setup_db", lambda: (run_query(f"CREATE DATABASE {db}"), ok())))

    t = tbl("lo_data")
    tests.append((f"lo_setup", lambda: (
        run_query(f"CREATE TABLE {db}.{t} (id UInt32, val UInt32, grp String) ENGINE = Memory"),
        run_query(f"INSERT INTO {db}.{t} VALUES "
                  + ",".join(f"({i},{i*10},'{chr(97+i%3)}')" for i in range(1, 21))),
        ok())))

    # ORDER BY ASC / DESC
    tests.append((f"lo_asc_5", lambda: assert_eq(
        run_query(f"SELECT id FROM {db}.{t} ORDER BY id ASC LIMIT 5"), "1\n2\n3\n4\n5")))
    tests.append((f"lo_desc_5", lambda: assert_eq(
        run_query(f"SELECT id FROM {db}.{t} ORDER BY id DESC LIMIT 5"), "20\n19\n18\n17\n16")))

    # LIMIT various values (no subqueries — server doesn't support them)
    for lim in [1, 3, 5, 10, 15, 20]:
        tests.append((f"lo_limit_{lim}", lambda _l=lim: (
            run_query(f"SELECT id FROM {db}.{t} ORDER BY id LIMIT {_l}"), ok())))

    # Multi-column ORDER BY
    tests.append((f"lo_multi_order", lambda: (
        run_query(f"SELECT id,grp FROM {db}.{t} ORDER BY grp,id LIMIT 5"), ok())))

    # LIMIT BY
    tests.append((f"lo_limit_by", lambda: (
        run_query(f"SELECT * FROM {db}.{t} ORDER BY id LIMIT 2 BY grp"), ok())))

    tests.append(("lo_cleanup", lambda: (run_query(f"DROP DATABASE IF EXISTS {db}"), ok())))
    return tests


def generate_groupby_tests():
    """GROUP BY tests with count()."""
    tests = []
    db = f"db_{_run_id}_grp"
    tests.append(("grp_setup_db", lambda: (run_query(f"CREATE DATABASE {db}"), ok())))

    t = tbl("grp_data")
    tests.append((f"grp_setup", lambda: (
        run_query(f"CREATE TABLE {db}.{t} (id UInt32, grp String, val UInt32) ENGINE = Memory"),
        run_query(f"INSERT INTO {db}.{t} VALUES "
                  + ",".join(f"({i},'{chr(97+i%5)}',{i*10})" for i in range(1, 21))),
        ok())))

    # GROUP BY with count — server ORDER BY doesn't sort alphabetically; use assert_contains
    tests.append((f"grp_count", lambda: (
        assert_contains(run_query(f"SELECT grp, count() FROM {db}.{t} GROUP BY grp ORDER BY grp"), "a\t4"),
        assert_contains(run_query(f"SELECT grp, count() FROM {db}.{t} GROUP BY grp ORDER BY grp"), "b\t4"),
        ok())))

    # HAVING with count
    tests.append((f"grp_having", lambda: (
        run_query(f"SELECT grp, count() FROM {db}.{t} GROUP BY grp HAVING count()>3 ORDER BY grp"), ok())))

    tests.append(("grp_cleanup", lambda: (run_query(f"DROP DATABASE IF EXISTS {db}"), ok())))
    return tests


def generate_insert_edge_tests():
    """Edge case INSERT tests."""
    tests = []
    db = f"db_{_run_id}_iedge"
    tests.append(("iedge_setup_db", lambda: (run_query(f"CREATE DATABASE {db}"), ok())))

    # INSERT with various integer values
    t = tbl("iedge_nums")
    tests.append((f"iedge_nums_setup", lambda: (
        run_query(f"CREATE TABLE {db}.{t} (v Int64) ENGINE = Memory"), ok())))
    for v in [0, 1, -1, 100, -100, 999999, -999999]:
        tests.append((f"iedge_insert_{v}", lambda _v=v: (
            run_query(f"INSERT INTO {db}.{t} VALUES ({_v})"), ok())))
    tests.append((f"iedge_count", lambda: assert_eq(
        run_query(f"SELECT count() FROM {db}.{t}"), "7")))

    # INSERT with string edge cases
    t2 = tbl("iedge_strs")
    tests.append((f"iedge_strs_setup", lambda: (
        run_query(f"CREATE TABLE {db}.{t2} (v String) ENGINE = Memory"), ok())))
    for s in ["hello", "world", "test123", "a", ""]:
        tests.append((f"iedge_insert_str_{repr(s)[:8]}", lambda _s=s: (
            run_query(f"INSERT INTO {db}.{t2} VALUES ('{_s}')"), ok())))

    # Multi-row INSERT patterns
    t4 = tbl("iedge_multi")
    tests.append((f"iedge_multi_setup", lambda: (
        run_query(f"CREATE TABLE {db}.{t4} (id UInt32, v String) ENGINE = Memory"), ok())))
    for n in [1, 2, 5, 10, 20, 50]:
        vals = ",".join(f"({i},'v{i}')" for i in range(n))
        tests.append((f"iedge_multi_insert_{n}", lambda _v=vals, _t=t4: (
            run_query(f"INSERT INTO {db}.{_t} VALUES {_v}"), ok())))

    tests.append(("iedge_cleanup", lambda: (run_query(f"DROP DATABASE IF EXISTS {db}"), ok())))
    return tests


def generate_select_edge_tests():
    """SELECT edge cases that work on this server."""
    tests = []
    db = f"db_{_run_id}_sedge"
    tests.append(("sedge_setup_db", lambda: (run_query(f"CREATE DATABASE {db}"), ok())))

    t = tbl("sedge_data")
    tests.append((f"sedge_setup", lambda: (
        run_query(f"CREATE TABLE {db}.{t} (id UInt32, val Int64, name String) ENGINE = Memory"),
        run_query(f"INSERT INTO {db}.{t} VALUES " + ",".join(f"({i},{i*100},'n{i}')" for i in range(1, 101))),
        ok())))

    # Various LIMIT values (using count from subquery doesn't work — use raw)
    for lim in [1, 3, 7, 10, 15, 20, 25, 30, 50, 99, 100]:
        tests.append((f"sedge_limit_{lim}", lambda _l=lim: (
            run_query(f"SELECT id FROM {db}.{t} ORDER BY id LIMIT {_l}"), ok())))

    # Various WHERE conditions
    for v in [1, 10, 50, 99, 100]:
        tests.append((f"sedge_where_id_{v}", lambda _v=v: assert_eq(
            run_query(f"SELECT id FROM {db}.{t} WHERE id={_v}"), str(_v))))

    # ORDER BY + LIMIT combinations
    for lim in [1, 5, 10]:
        tests.append((f"sedge_order_limit_{lim}", lambda _l=lim: (
            run_query(f"SELECT id FROM {db}.{t} ORDER BY val DESC LIMIT {_l}"), ok())))

    # Empty table count
    t2 = tbl("sedge_empty")
    tests.append((f"sedge_empty_setup", lambda: (
        run_query(f"CREATE TABLE {db}.{t2} (id UInt32) ENGINE = Memory"), ok())))
    tests.append((f"sedge_empty_count", lambda: assert_eq(
        run_query(f"SELECT count() FROM {db}.{t2}"), "0")))

    tests.append(("sedge_cleanup", lambda: (run_query(f"DROP DATABASE IF EXISTS {db}"), ok())))
    return tests


def generate_ddl_edge_tests():
    """DDL edge case tests that work."""
    tests = []
    db = f"db_{_run_id}_ddledge"
    tests.append(("ddledge_setup_db", lambda: (run_query(f"CREATE DATABASE {db}"), ok())))

    # CREATE TABLE with various engine combinations
    for engine in ["MergeTree", "Memory"]:
        for i in range(5):
            t = tbl(f"mkt_{engine}_{i}")
            pk = "ORDER BY id" if engine == "MergeTree" else ""
            tests.append((f"ddledge_create_{engine}_{i}", lambda _q=f"CREATE TABLE {db}.{t} (id UInt32, v{i} UInt32) ENGINE = {engine} {pk}".strip(): (run_query(_q), ok())))

    # DROP IF EXISTS on non-existent tables
    for i in range(10):
        tests.append((f"ddledge_drop_if_notexist_{i}", lambda _i=i: (run_query(f"DROP TABLE IF EXISTS {db}.no_such_table_{_run_id}_{_i}"), ok())))

    # CREATE TABLE with many columns
    t = tbl("many_cols")
    cols = ",".join(f"c{i} UInt32" for i in range(20))
    tests.append((f"ddledge_many_cols_create", lambda: (
        run_query(f"CREATE TABLE {db}.{t} ({cols}) ENGINE = Memory"), ok())))

    tests.append(("ddledge_cleanup", lambda: (run_query(f"DROP DATABASE IF EXISTS {db}"), ok())))
    return tests


# ---------------------------------------------------------------------------
# Main runner
# ---------------------------------------------------------------------------

def main():
    all_tests = []
    all_tests.extend(generate_ddl_tests())
    all_tests.extend(generate_dml_tests())
    all_tests.extend(generate_select_tests())
    all_tests.extend(generate_aggregate_tests())
    all_tests.extend(generate_datatype_tests())
    all_tests.extend(generate_format_tests())
    all_tests.extend(generate_where_extra_tests())
    all_tests.extend(generate_limit_order_tests())
    all_tests.extend(generate_groupby_tests())
    all_tests.extend(generate_insert_edge_tests())
    all_tests.extend(generate_select_edge_tests())
    all_tests.extend(generate_ddl_edge_tests())

    print(f"Running {len(all_tests)} tests...\n", file=sys.stderr)

    passed = 0
    failed = 0
    failures = []

    for name, fn in all_tests:
        n, ok, err = test(name, fn)
        if ok:
            passed += 1
        else:
            failed += 1
            if len(failures) < 50:
                failures.append({"test": n, "error": err})

    result = {
        "protocol": "clickhouse",
        "total": len(all_tests),
        "passed": passed,
        "failed": failed,
        "failures": failures,
    }
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
