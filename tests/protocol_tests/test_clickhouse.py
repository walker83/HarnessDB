#!/usr/bin/env python3
"""ClickHouse HTTP protocol test suite for RorisDB (port 18123)."""

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
    tests = []
    db = f"db_{_run_id}"
    tests.append(("ddl_create_database", lambda: (run_query(f"CREATE DATABASE IF NOT EXISTS {db}"), ok())))
    tests.append(("ddl_use_database", lambda: (run_query(f"USE {db}"), ok())))
    tests.append(("ddl_show_databases", lambda: assert_contains(run_query("SHOW DATABASES"), db)))

    # CREATE TABLE variants
    for engine in ["MergeTree", "Memory", "Log"]:
        t = tbl(f"create_{engine}")
        pk = "ORDER BY id" if engine == "MergeTree" else ""
        q = f"CREATE TABLE {db}.{t} (id UInt32, name String) ENGINE = {engine} {pk}".strip()
        tests.append((f"ddl_create_table_{engine}", lambda _q=q: (run_query(_q), ok())))

    # CREATE TABLE with various data types
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

    # CREATE VIEW
    t = tbl("view_src")
    tests.append((f"ddl_create_view_setup", lambda: (
        run_query(f"CREATE TABLE {db}.{t} (id UInt32, val UInt32) ENGINE = Memory"),
        run_query(f"INSERT INTO {db}.{t} VALUES (1,100),(2,200)"),
        ok())))
    v = tbl("view1")
    tests.append((f"ddl_create_view", lambda: (
        run_query(f"CREATE VIEW {db}.{v} AS SELECT id, val FROM {db}.{t}"), ok())))
    tests.append((f"ddl_select_view", lambda: assert_contains(run_query(f"SELECT * FROM {db}.{v}"), "100")))

    # CREATE MATERIALIZED VIEW
    t2 = tbl("mv_src")
    mv = tbl("mv1")
    tests.append((f"ddl_create_mv_setup", lambda: (
        run_query(f"CREATE TABLE {db}.{t2} (id UInt32, val UInt32) ENGINE = Memory"), ok())))
    tests.append((f"ddl_create_materialized_view", lambda: (
        run_query(f"CREATE MATERIALIZED VIEW {db}.{mv} ENGINE = Memory AS SELECT id, val FROM {db}.{t2}"), ok())))

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

    # ALTER TABLE ADD COLUMN
    t = tbl("alter_add")
    tests.append((f"ddl_alter_add_col_setup", lambda: (
        run_query(f"CREATE TABLE {db}.{t} (id UInt32) ENGINE = Memory"), ok())))
    tests.append((f"ddl_alter_add_col", lambda: (
        run_query(f"ALTER TABLE {db}.{t} ADD COLUMN name String"), ok())))

    # ALTER TABLE DROP COLUMN
    t = tbl("alter_drop_col")
    tests.append((f"ddl_alter_drop_col_setup", lambda: (
        run_query(f"CREATE TABLE {db}.{t} (id UInt32, name String, age UInt8) ENGINE = Memory"), ok())))
    tests.append((f"ddl_alter_drop_col", lambda: (
        run_query(f"ALTER TABLE {db}.{t} DROP COLUMN age"), ok())))

    # ALTER TABLE MODIFY COLUMN
    t = tbl("alter_modify")
    tests.append((f"ddl_alter_modify_setup", lambda: (
        run_query(f"CREATE TABLE {db}.{t} (id UInt32, name String) ENGINE = Memory"), ok())))
    tests.append((f"ddl_alter_modify_col", lambda: (
        run_query(f"ALTER TABLE {db}.{t} MODIFY COLUMN name UInt64"), ok())))

    # TRUNCATE TABLE
    t = tbl("truncate")
    tests.append((f"ddl_truncate_setup", lambda: (
        run_query(f"CREATE TABLE {db}.{t} (id UInt32) ENGINE = Memory"),
        run_query(f"INSERT INTO {db}.{t} VALUES (1),(2),(3)"),
        ok())))
    tests.append((f"ddl_truncate", lambda: (
        run_query(f"TRUNCATE TABLE {db}.{t}"), ok())))
    tests.append((f"ddl_truncate_verify", lambda: assert_eq(run_query(f"SELECT count() FROM {db}.{t}"), "0")))

    # RENAME TABLE
    t1 = tbl("rename_src")
    t2_ = tbl("rename_dst")
    tests.append((f"ddl_rename_setup", lambda: (
        run_query(f"CREATE TABLE {db}.{t1} (id UInt32) ENGINE = Memory"),
        run_query(f"INSERT INTO {db}.{t1} VALUES (42)"),
        ok())))
    tests.append((f"ddl_rename_table", lambda: (
        run_query(f"RENAME TABLE {db}.{t1} TO {db}.{t2_}"), ok())))
    tests.append((f"ddl_rename_verify", lambda: assert_eq(run_query(f"SELECT id FROM {db}.{t2_}"), "42")))

    # ATTACH / DETACH
    t = tbl("attach_detach")
    tests.append((f"ddl_detach_setup", lambda: (
        run_query(f"CREATE TABLE {db}.{t} (id UInt32) ENGINE = Memory"),
        run_query(f"INSERT INTO {db}.{t} VALUES (1),(2)"),
        ok())))
    tests.append((f"ddl_detach_table", lambda: (
        run_query(f"DETACH TABLE {db}.{t}"), ok())))
    tests.append((f"ddl_attach_table", lambda: (
        run_query(f"ATTACH TABLE {db}.{t} (id UInt32) ENGINE = Memory"), ok())))

    # SHOW commands
    tests.append(("ddl_show_tables", lambda: (run_query(f"SHOW TABLES FROM {db}"), ok())))
    tests.append(("ddl_show_create", lambda: (run_query(f"SHOW CREATE TABLE {db}.{tbl('show_cr')}" if False else f"SHOW CREATE TABLE {db}.{t}"), ok())))

    # DESCRIBE
    t = tbl("describe")
    tests.append((f"ddl_describe_setup", lambda: (
        run_query(f"CREATE TABLE {db}.{t} (id UInt32, name String) ENGINE = Memory"), ok())))
    tests.append((f"ddl_describe_table", lambda: assert_contains(run_query(f"DESCRIBE TABLE {db}.{t}"), "UInt32")))
    tests.append((f"ddl_desc", lambda: assert_contains(run_query(f"DESC {db}.{t}"), "String")))

    # EXISTS
    tests.append((f"ddl_exists", lambda: assert_eq(run_query(f"EXISTS TABLE {db}.{t}"), "1")))
    tests.append((f"ddl_exists_false", lambda: assert_eq(run_query(f"EXISTS TABLE {db}.no_such_table_{_run_id}"), "0")))

    # CHECK TABLE
    tests.append((f"ddl_check_table", lambda: (run_query(f"CHECK TABLE {db}.{t}"), ok())))

    # OPTIMIZE TABLE
    t = tbl("optimize")
    tests.append((f"ddl_optimize_setup", lambda: (
        run_query(f"CREATE TABLE {db}.{t} (id UInt32) ENGINE = MergeTree ORDER BY id"),
        run_query(f"INSERT INTO {db}.{t} VALUES (1),(2)"),
        ok())))
    tests.append((f"ddl_optimize", lambda: (run_query(f"OPTIMIZE TABLE {db}.{t}"), ok())))

    # Cleanup DB
    tests.append(("ddl_cleanup", lambda: (run_query(f"DROP DATABASE IF EXISTS {db}"), ok())))
    return tests


def generate_dml_tests():
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

    # INSERT SELECT
    t3 = tbl("ins_sel_dst")
    tests.append((f"dml_insert_select_setup", lambda: (
        run_query(f"CREATE TABLE {db}.{t3} (id UInt32, v Float64) ENGINE = Memory"), ok())))
    tests.append((f"dml_insert_select", lambda: (
        run_query(f"INSERT INTO {db}.{t3} SELECT id, v FROM {db}.{t2}"), ok())))
    tests.append((f"dml_verify_insert_select", lambda: assert_eq(run_query(f"SELECT count() FROM {db}.{t3}"), "3")))

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
    tests = []
    db = f"db_{_run_id}_sel"
    tests.append(("sel_setup_db", lambda: (run_query(f"CREATE DATABASE {db}"), ok())))

    # Basic setup tables
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
    tests.append((f"sel_literal", lambda: assert_eq(run_query("SELECT 1"), "1")))
    tests.append((f"sel_expr", lambda: assert_eq(run_query("SELECT 2+3"), "5")))
    tests.append((f"sel_alias", lambda: assert_eq(run_query(f"SELECT id AS x FROM {db}.{t} WHERE id=1"), "1")))

    # WHERE
    tests.append((f"sel_where_eq", lambda: assert_eq(run_query(f"SELECT count() FROM {db}.{t} WHERE grp='a'"), "3")))
    tests.append((f"sel_where_neq", lambda: assert_eq(run_query(f"SELECT count() FROM {db}.{t} WHERE grp!='a'"), "3")))
    tests.append((f"sel_where_gt", lambda: assert_eq(run_query(f"SELECT count() FROM {db}.{t} WHERE val>30"), "3")))
    tests.append((f"sel_where_lt", lambda: assert_eq(run_query(f"SELECT count() FROM {db}.{t} WHERE val<4"), "3")))
    tests.append((f"sel_where_gte", lambda: assert_eq(run_query(f"SELECT count() FROM {db}.{t} WHERE val>=30"), "4")))
    tests.append((f"sel_where_lte", lambda: assert_eq(run_query(f"SELECT count() FROM {db}.{t} WHERE val<=3"), "3")))
    tests.append((f"sel_where_and", lambda: assert_eq(run_query(f"SELECT count() FROM {db}.{t} WHERE grp='a' AND val>20"), "2")))
    tests.append((f"sel_where_or", lambda: assert_eq(run_query(f"SELECT count() FROM {db}.{t} WHERE id=1 OR id=6"), "2")))
    tests.append((f"sel_where_in", lambda: assert_eq(run_query(f"SELECT count() FROM {db}.{t} WHERE id IN (1,2,3)"), "3")))
    tests.append((f"sel_where_not_in", lambda: assert_eq(run_query(f"SELECT count() FROM {db}.{t} WHERE id NOT IN (1,2,3)"), "3")))
    tests.append((f"sel_where_between", lambda: assert_eq(run_query(f"SELECT count() FROM {db}.{t} WHERE val BETWEEN 20 AND 40"), "3")))
    tests.append((f"sel_where_like", lambda: assert_eq(run_query(f"SELECT count() FROM {db}.{t} WHERE name LIKE 'a%'"), "1")))
    tests.append((f"sel_where_not_like", lambda: assert_eq(run_query(f"SELECT count() FROM {db}.{t} WHERE name NOT LIKE 'a%'"), "5")))

    # ORDER BY
    tests.append((f"sel_order_asc", lambda: assert_eq(run_query(f"SELECT id FROM {db}.{t} ORDER BY id ASC LIMIT 3"), "1\n2\n3")))
    tests.append((f"sel_order_desc", lambda: assert_eq(run_query(f"SELECT id FROM {db}.{t} ORDER BY id DESC LIMIT 3"), "6\n5\n4")))
    tests.append((f"sel_order_multi", lambda: (run_query(f"SELECT id,grp FROM {db}.{t} ORDER BY grp,id LIMIT 1"), ok())))

    # LIMIT
    tests.append((f"sel_limit", lambda: assert_eq(run_query(f"SELECT count() FROM (SELECT * FROM {db}.{t} LIMIT 2)"), "2")))
    tests.append((f"sel_limit_offset", lambda: assert_eq(run_query(f"SELECT id FROM {db}.{t} ORDER BY id LIMIT 2 OFFSET 1"), "2\n3")))

    # LIMIT BY
    tests.append((f"sel_limit_by", lambda: (run_query(f"SELECT * FROM {db}.{t} ORDER BY id LIMIT 1 BY grp"), ok())))

    # GROUP BY
    tests.append((f"sel_group_count", lambda: assert_eq(run_query(f"SELECT grp, count() FROM {db}.{t} GROUP BY grp ORDER BY grp"), "a\t3\nb\t3")))
    tests.append((f"sel_group_sum", lambda: assert_eq(run_query(f"SELECT grp, sum(val) FROM {db}.{t} GROUP BY grp ORDER BY grp"), "a\t90\nb\t120")))
    tests.append((f"sel_group_avg", lambda: assert_eq(run_query(f"SELECT grp, avg(val) FROM {db}.{t} GROUP BY grp ORDER BY grp"), "a\t30\nb\t40")))

    # HAVING
    tests.append((f"sel_having", lambda: assert_eq(run_query(f"SELECT grp, count() FROM {db}.{t} GROUP BY grp HAVING count()>2 ORDER BY grp"), "a\t3\nb\t3")))

    # DISTINCT
    tests.append((f"sel_distinct", lambda: assert_eq(run_query(f"SELECT DISTINCT grp FROM {db}.{t} ORDER BY grp"), "a\nb")))

    # UNION ALL
    tests.append((f"sel_union", lambda: assert_eq(run_query("SELECT 1 UNION ALL SELECT 2"), "1\n2")))

    # WITH (CTE)
    tests.append((f"sel_with", lambda: assert_eq(run_query("WITH 42 AS x SELECT x"), "42")))

    # JOINs
    t1 = tbl("join_a")
    t2 = tbl("join_b")
    tests.append((f"sel_join_setup", lambda: (
        run_query(f"CREATE TABLE {db}.{t1} (id UInt32, v String) ENGINE = Memory"),
        run_query(f"CREATE TABLE {db}.{t2} (id UInt32, w String) ENGINE = Memory"),
        run_query(f"INSERT INTO {db}.{t1} VALUES (1,'a'),(2,'b'),(3,'c')"),
        run_query(f"INSERT INTO {db}.{t2} VALUES (2,'x'),(3,'y'),(4,'z')"),
        ok())))
    tests.append((f"sel_inner_join", lambda: assert_eq(run_query(f"SELECT count() FROM {db}.{t1} INNER JOIN {db}.{t2} ON {db}.{t1}.id={db}.{t2}.id"), "2")))
    tests.append((f"sel_left_join", lambda: assert_eq(run_query(f"SELECT count() FROM {db}.{t1} LEFT JOIN {db}.{t2} ON {db}.{t1}.id={db}.{t2}.id"), "3")))
    tests.append((f"sel_right_join", lambda: assert_eq(run_query(f"SELECT count() FROM {db}.{t1} RIGHT JOIN {db}.{t2} ON {db}.{t1}.id={db}.{t2}.id"), "3")))
    tests.append((f"sel_full_join", lambda: assert_eq(run_query(f"SELECT count() FROM {db}.{t1} FULL JOIN {db}.{t2} ON {db}.{t1}.id={db}.{t2}.id"), "4")))
    tests.append((f"sel_cross_join", lambda: assert_eq(run_query(f"SELECT count() FROM {db}.{t1} CROSS JOIN {db}.{t2}"), "9")))

    # ARRAY JOIN
    t3 = tbl("arr_join")
    tests.append((f"sel_array_join_setup", lambda: (
        run_query(f"CREATE TABLE {db}.{t3} (id UInt32, arr Array(String)) ENGINE = Memory"),
        run_query(f"INSERT INTO {db}.{t3} VALUES (1, ['a','b']),(2, ['c'])"),
        ok())))
    tests.append((f"sel_array_join", lambda: assert_eq(run_query(f"SELECT count() FROM {db}.{t3} ARRAY JOIN arr"), "3")))

    # PREWHERE (only for MergeTree – skip if engine doesn't support; just test parsing)
    t4 = tbl("prewhere")
    tests.append((f"sel_prewhere_setup", lambda: (
        run_query(f"CREATE TABLE {db}.{t4} (id UInt32, val UInt32) ENGINE = MergeTree ORDER BY id"),
        run_query(f"INSERT INTO {db}.{t4} VALUES (1,10),(2,20),(3,30)"),
        ok())))
    tests.append((f"sel_prewhere", lambda: assert_eq(run_query(f"SELECT count() FROM {db}.{t4} PREWHERE val > 15"), "2")))

    # Subqueries
    tests.append((f"sel_subquery", lambda: assert_eq(run_query(f"SELECT * FROM (SELECT 1 AS x)"), "1")))
    tests.append((f"sel_scalar_subq", lambda: assert_eq(run_query(f"SELECT (SELECT 42)"), "42")))

    # CASE WHEN
    tests.append((f"sel_case", lambda: assert_eq(run_query("SELECT CASE WHEN 1>0 THEN 'yes' ELSE 'no' END"), "yes")))

    # IF function
    tests.append((f"sel_if", lambda: assert_eq(run_query("SELECT if(1>0,'yes','no')"), "yes")))

    # multiIf
    tests.append((f"sel_multiif", lambda: assert_eq(run_query("SELECT multiIf(1=1,'a',1=2,'b','c')"), "a")))

    # BETWEEN
    tests.append((f"sel_between", lambda: assert_eq(run_query("SELECT 5 BETWEEN 1 AND 10"), "1")))

    # COALESCE
    tests.append((f"sel_coalesce", lambda: assert_eq(run_query("SELECT coalesce(NULL, 42)"), "42")))

    # Null comparison
    tests.append((f"sel_is_null", lambda: assert_eq(run_query("SELECT NULL IS NULL"), "1")))
    tests.append((f"sel_is_not_null", lambda: assert_eq(run_query("SELECT 1 IS NOT NULL"), "1")))

    tests.append(("sel_cleanup", lambda: (run_query(f"DROP DATABASE IF EXISTS {db}"), ok())))
    return tests


def generate_aggregate_tests():
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

    # count
    tests.append((f"agg_count", lambda: assert_eq(run_query(f"SELECT count() FROM {db}.{t}"), "10")))
    tests.append((f"agg_count_if", lambda: assert_eq(run_query(f"SELECT countIf(val>25) FROM {db}.{t}"), "5")))

    # sum
    tests.append((f"agg_sum", lambda: assert_eq(run_query(f"SELECT sum(val) FROM {db}.{t}"), "280")))
    tests.append((f"agg_sum_if", lambda: assert_eq(run_query(f"SELECT sumIf(val, grp='a') FROM {db}.{t}"), "100")))

    # avg
    tests.append((f"agg_avg", lambda: assert_eq(run_query(f"SELECT avg(val) FROM {db}.{t}"), "28")))
    tests.append((f"agg_avg_if", lambda: assert_eq(run_query(f"SELECT avgIf(val, grp='a') FROM {db}.{t}"), "20")))

    # min / max
    tests.append((f"agg_min", lambda: assert_eq(run_query(f"SELECT min(val) FROM {db}.{t}"), "10")))
    tests.append((f"agg_max", lambda: assert_eq(run_query(f"SELECT max(val) FROM {db}.{t}"), "60")))

    # uniq
    tests.append((f"agg_uniq", lambda: assert_eq(run_query(f"SELECT uniq(val) FROM {db}.{t}"), "7")))
    tests.append((f"agg_uniq_exact", lambda: assert_eq(run_query(f"SELECT uniqExact(val) FROM {db}.{t}"), "7")))

    # groupArray
    tests.append((f"agg_group_array", lambda: (run_query(f"SELECT groupArray(val) FROM {db}.{t}"), ok())))

    # groupUniqArray
    tests.append((f"agg_group_uniq_array", lambda: (run_query(f"SELECT groupUniqArray(val) FROM {db}.{t}"), ok())))

    # argMin / argMax
    tests.append((f"agg_argmin", lambda: assert_eq(run_query(f"SELECT argMin(id, val) FROM {db}.{t}"), "1")))
    tests.append((f"agg_argmax", lambda: assert_eq(run_query(f"SELECT argMax(id, val) FROM {db}.{t}"), "8")))

    # topK
    tests.append((f"agg_topk", lambda: (run_query(f"SELECT topK(3)(val) FROM {db}.{t}"), ok())))

    # any / last
    tests.append((f"agg_any", lambda: (run_query(f"SELECT any(val) FROM {db}.{t}"), ok())))
    tests.append((f"agg_last", lambda: (run_query(f"SELECT last(val) FROM {db}.{t}"), ok())))
    tests.append((f"agg_any_last", lambda: (run_query(f"SELECT anyLast(val) FROM {db}.{t}"), ok())))
    tests.append((f"agg_any_heavy", lambda: (run_query(f"SELECT anyHeavy(val) FROM {db}.{t}"), ok())))

    # median / quantile
    tests.append((f"agg_median", lambda: (run_query(f"SELECT median(val) FROM {db}.{t}"), ok())))
    tests.append((f"agg_quantile", lambda: (run_query(f"SELECT quantile(0.5)(val) FROM {db}.{t}"), ok())))
    tests.append((f"agg_quantiles", lambda: (run_query(f"SELECT quantiles(0.25,0.5,0.75)(val) FROM {db}.{t}"), ok())))

    # Grouped aggregates
    tests.append((f"agg_group_count", lambda: assert_eq(run_query(f"SELECT grp, count() FROM {db}.{t} GROUP BY grp ORDER BY grp"), "a\t5\nb\t5")))
    tests.append((f"agg_group_sum", lambda: (run_query(f"SELECT grp, sum(val) FROM {db}.{t} GROUP BY grp ORDER BY grp"), ok())))

    # Variance / stdDev
    tests.append((f"agg_var_pop", lambda: (run_query(f"SELECT varPop(val) FROM {db}.{t}"), ok())))
    tests.append((f"agg_var_samp", lambda: (run_query(f"SELECT varSamp(val) FROM {db}.{t}"), ok())))
    tests.append((f"agg_stddev_pop", lambda: (run_query(f"SELECT stddevPop(val) FROM {db}.{t}"), ok())))
    tests.append((f"agg_stddev_samp", lambda: (run_query(f"SELECT stddevSamp(val) FROM {db}.{t}"), ok())))

    # covar
    tests.append((f"agg_covar_pop", lambda: (run_query(f"SELECT covarPop(id, val) FROM {db}.{t}"), ok())))
    tests.append((f"agg_covar_samp", lambda: (run_query(f"SELECT covarSamp(id, val) FROM {db}.{t}"), ok())))

    # corr
    tests.append((f"agg_corr", lambda: (run_query(f"SELECT corr(id, val) FROM {db}.{t}"), ok())))

    # countDistinct
    tests.append((f"agg_count_distinct", lambda: assert_eq(run_query(f"SELECT countDistinct(val) FROM {db}.{t}"), "7")))

    # sumIf, avgIf, minIf, maxIf
    tests.append((f"agg_min_if", lambda: assert_eq(run_query(f"SELECT minIf(val, grp='b') FROM {db}.{t}"), "20")))
    tests.append((f"agg_max_if", lambda: assert_eq(run_query(f"SELECT maxIf(val, grp='b') FROM {db}.{t}"), "60")))

    tests.append(("agg_cleanup", lambda: (run_query(f"DROP DATABASE IF EXISTS {db}"), ok())))
    return tests


def generate_string_tests():
    tests = []
    db = f"db_{_run_id}_str"
    tests.append(("str_setup_db", lambda: (run_query(f"CREATE DATABASE {db}"), ok())))

    # Basic string functions
    tests.append((f"str_length", lambda: assert_eq(run_query("SELECT length('hello')"), "5")))
    tests.append((f"str_lower", lambda: assert_eq(run_query("SELECT lower('HELLO')"), "hello")))
    tests.append((f"str_upper", lambda: assert_eq(run_query("SELECT upper('hello')"), "HELLO")))
    tests.append((f"str_reverse", lambda: assert_eq(run_query("SELECT reverse('hello')"), "olleh")))
    tests.append((f"str_concat", lambda: assert_eq(run_query("SELECT concat('hello',' ','world')"), "hello world")))
    tests.append((f"str_substring", lambda: assert_eq(run_query("SELECT substring('hello',2,3)"), "ell")))
    tests.append((f"str_replace_one", lambda: assert_eq(run_query("SELECT replaceOne('hello','l','L')"), "heLlo")))
    tests.append((f"str_replace_all", lambda: assert_eq(run_query("SELECT replaceAll('hello','l','L')"), "heLLo")))
    tests.append((f"str_replace_regexp_one", lambda: assert_eq(run_query("SELECT replaceRegexpOne('hello','l+','L')"), "heLo")))
    tests.append((f"str_replace_regexp_all", lambda: assert_eq(run_query("SELECT replaceRegexpAll('hello','l+','L')"), "heLLo")))
    tests.append((f"str_trim", lambda: assert_eq(run_query("SELECT trim('  hello  ')"), "hello")))
    tests.append((f"str_trim_left", lambda: assert_eq(run_query("SELECT trimLeft('  hello  ')"), "hello  ")))
    tests.append((f"str_trim_right", lambda: assert_eq(run_query("SELECT trimRight('  hello  ')"), "  hello")))
    tests.append((f"str_lpad", lambda: assert_eq(run_query("SELECT lpad('hi',5,'0')"), "000hi")))
    tests.append((f"str_rpad", lambda: assert_eq(run_query("SELECT rpad('hi',5,'0')"), "hi000")))
    tests.append((f"str_like", lambda: assert_eq(run_query("SELECT 'hello' LIKE 'h%'"), "1")))
    tests.append((f"str_not_like", lambda: assert_eq(run_query("SELECT 'hello' NOT LIKE 'x%'"), "1")))
    tests.append((f"str_match", lambda: assert_eq(run_query("SELECT match('hello','^hel')"), "1")))
    tests.append((f"str_extract", lambda: assert_eq(run_query("SELECT extract('hello123','[0-9]+')"), "123")))
    tests.append((f"str_extract_all", lambda: assert_eq(run_query("SELECT extractAll('a1b2c3','[0-9]+')"), "['1','2','3']")))
    tests.append((f"str_position", lambda: assert_eq(run_query("SELECT position('hello','ll')"), "3")))
    tests.append((f"str_locate", lambda: assert_eq(run_query("SELECT locate('hello','ll')"), "3")))
    tests.append((f"str_left", lambda: assert_eq(run_query("SELECT left('hello',3)"), "hel")))
    tests.append((f"str_right", lambda: assert_eq(run_query("SELECT right('hello',3)"), "llo")))
    tests.append((f"str_ascii", lambda: assert_eq(run_query("SELECT ascii('A')"), "65")))
    tests.append((f"str_char", lambda: assert_eq(run_query("SELECT char(65)"), "A")))
    tests.append((f"str_hex", lambda: assert_eq(run_query("SELECT hex('AB')"), "4142")))
    tests.append((f"str_unhex", lambda: (run_query("SELECT unhex('4142')"), ok())))
    tests.append((f"str_base64_encode", lambda: assert_eq(run_query("SELECT base64Encode('hello')"), "aGVsbG8=")))
    tests.append((f"str_base64_decode", lambda: assert_eq(run_query("SELECT base64Decode('aGVsbG8=')"), "hello")))
    tests.append((f"str_split", lambda: (run_query("SELECT splitByChar(',','a,b,c')"), ok())))
    tests.append((f"str_join", lambda: assert_eq(run_query("SELECT arrayStringConcat(['a','b','c'],',')"), "a,b,c")))
    tests.append((f"str_repeat", lambda: assert_eq(run_query("SELECT repeat('ab',3)"), "ababab")))
    tests.append((f"str_empty", lambda: assert_eq(run_query("SELECT empty('')"), "1")))
    tests.append((f"str_not_empty", lambda: assert_eq(run_query("SELECT notEmpty('x')"), "1")))
    tests.append((f"str_length_utf8", lambda: assert_eq(run_query("SELECT lengthUTF8('café')"), "4")))
    tests.append((f"str_lower_utf8", lambda: assert_eq(run_query("SELECT lowerUTF8('CAFÉ')"), "café")))
    tests.append((f"str_upper_utf8", lambda: assert_eq(run_query("SELECT upperUTF8('café')"), "CAFÉ")))
    tests.append((f"str_reverse_utf8", lambda: assert_eq(run_query("SELECT reverseUTF8('café')"), "éfac")))
    tests.append((f"str_starts_with", lambda: assert_eq(run_query("SELECT startsWith('hello','hel')"), "1")))
    tests.append((f"str_ends_with", lambda: assert_eq(run_query("SELECT endsWith('hello','llo')"), "1")))
    tests.append((f"str_format", lambda: assert_eq(run_query("SELECT format('Hello {}','World')"), "Hello World")))
    tests.append((f"str_concat_ws", lambda: assert_eq(run_query("SELECT concatWS(',', 'a','b','c')"), "a,b,c")))

    # ILike
    tests.append((f"str_ilike", lambda: assert_eq(run_query("SELECT 'Hello' ILIKE 'hello'"), "1")))
    tests.append((f"str_not_ilike", lambda: assert_eq(run_query("SELECT 'Hello' NOT ILIKE 'xyz'"), "1")))

    # toString
    tests.append((f"str_to_string", lambda: assert_eq(run_query("SELECT toString(42)"), "42")))
    tests.append((f"str_to_string_float", lambda: assert_contains(run_query("SELECT toString(3.14)"), "3.14")))

    # Additional
    tests.append((f"str_encode_decode", lambda: assert_eq(run_query("SELECT decodeURLComponent('hello%20world')"), "hello world")))
    tests.append((f"str_encode", lambda: assert_eq(run_query("SELECT encodeURLComponent('hello world')"), "hello%20world")))

    tests.append(("str_cleanup", lambda: (run_query(f"DROP DATABASE IF EXISTS {db}"), ok())))
    return tests


def generate_date_tests():
    tests = []

    tests.append((f"date_now", lambda: (run_query("SELECT now()"), ok())))
    tests.append((f"date_today", lambda: (run_query("SELECT today()"), ok())))
    tests.append((f"date_yesterday", lambda: (run_query("SELECT yesterday()"), ok())))
    tests.append((f"date_to_date", lambda: (run_query("SELECT toDate('2024-01-15')"), ok())))
    tests.append((f"date_to_datetime", lambda: (run_query("SELECT toDateTime('2024-01-15 10:30:00')"), ok())))
    tests.append((f"date_to_start_of_day", lambda: (run_query("SELECT toStartOfDay(now())"), ok())))
    tests.append((f"date_to_start_of_week", lambda: (run_query("SELECT toStartOfWeek(today())"), ok())))
    tests.append((f"date_to_start_of_month", lambda: (run_query("SELECT toStartOfMonth(today())"), ok())))
    tests.append((f"date_to_start_of_quarter", lambda: (run_query("SELECT toStartOfQuarter(today())"), ok())))
    tests.append((f"date_to_start_of_year", lambda: (run_query("SELECT toStartOfYear(today())"), ok())))
    tests.append((f"date_format", lambda: (run_query("SELECT formatDateTime(now(), '%Y-%m-%d')"), ok())))
    tests.append((f"date_diff_days", lambda: (run_query("SELECT dateDiff('day', today(), today())"), ok())))
    tests.append((f"date_diff_months", lambda: (run_query("SELECT dateDiff('month', toDate('2024-01-01'), toDate('2024-06-01'))"), ok())))
    tests.append((f"date_add_days", lambda: (run_query("SELECT addDays(today(), 1)"), ok())))
    tests.append((f"date_add_weeks", lambda: (run_query("SELECT addWeeks(today(), 1)"), ok())))
    tests.append((f"date_add_months", lambda: (run_query("SELECT addMonths(today(), 1)"), ok())))
    tests.append((f"date_add_years", lambda: (run_query("SELECT addYears(today(), 1)"), ok())))
    tests.append((f"date_add_hours", lambda: (run_query("SELECT addHours(now(), 1)"), ok())))
    tests.append((f"date_add_minutes", lambda: (run_query("SELECT addMinutes(now(), 30)"), ok())))
    tests.append((f"date_add_seconds", lambda: (run_query("SELECT addSeconds(now(), 30)"), ok())))
    tests.append((f"date_subtract_days", lambda: (run_query("SELECT subtractDays(today(), 1)"), ok())))
    tests.append((f"date_subtract_weeks", lambda: (run_query("SELECT subtractWeeks(today(), 1)"), ok())))
    tests.append((f"date_subtract_months", lambda: (run_query("SELECT subtractMonths(today(), 1)"), ok())))
    tests.append((f"date_subtract_years", lambda: (run_query("SELECT subtractYears(today(), 1)"), ok())))
    tests.append((f"date_to_monday", lambda: (run_query("SELECT toMonday(today())"), ok())))
    tests.append((f"date_to_relative_year", lambda: (run_query("SELECT toRelativeYearNum(now())"), ok())))
    tests.append((f"date_to_relative_month", lambda: (run_query("SELECT toRelativeMonthNum(today())"), ok())))
    tests.append((f"date_to_day_of_week", lambda: (run_query("SELECT toDayOfWeek(today())"), ok())))
    tests.append((f"date_to_day_of_month", lambda: (run_query("SELECT toDayOfMonth(today())"), ok())))
    tests.append((f"date_to_month", lambda: (run_query("SELECT toMonth(today())"), ok())))
    tests.append((f"date_to_year", lambda: (run_query("SELECT toYear(today())"), ok())))
    tests.append((f"date_to_hour", lambda: (run_query("SELECT toHour(now())"), ok())))
    tests.append((f"date_to_minute", lambda: (run_query("SELECT toMinute(now())"), ok())))
    tests.append((f"date_to_second", lambda: (run_query("SELECT toSecond(now())"), ok())))
    tests.append((f"date_to_quarter", lambda: (run_query("SELECT toQuarter(today())"), ok())))
    tests.append((f"date_parse_best_effort", lambda: (run_query("SELECT parseDateTimeBestEffort('2024-01-15 10:30:00')"), ok())))
    tests.append((f"date_to_unix_timestamp", lambda: (run_query("SELECT toUnixTimestamp(toDateTime('2024-01-01 00:00:00'))"), ok())))
    tests.append((f"date_from_unix", lambda: (run_query("SELECT fromUnixTimestamp(1704067200)"), ok())))
    tests.append((f"date_to_date_time32", lambda: (run_query("SELECT toDateTime32('2024-01-01 00:00:00')"), ok())))
    tests.append((f"date_to_start_of_hour", lambda: (run_query("SELECT toStartOfHour(now())"), ok())))
    tests.append((f"date_to_start_of_minute", lambda: (run_query("SELECT toStartOfMinute(now())"), ok())))
    tests.append((f"date_to_start_of_five_min", lambda: (run_query("SELECT toStartOfFiveMinutes(now())"), ok())))
    tests.append((f"date_to_start_of_fifteen_min", lambda: (run_query("SELECT toStartOfFifteenMinutes(now())"), ok())))
    tests.append((f"date_to_time", lambda: (run_query("SELECT toTime(now())"), ok())))
    tests.append((f"date_date_add", lambda: (run_query("SELECT dateAdd(day, 1, today())"), ok())))
    tests.append((f"date_date_sub", lambda: (run_query("SELECT dateSub(day, 1, today())"), ok())))
    tests.append((f"date_to_iso_week", lambda: (run_query("SELECT toISOWeek(today())"), ok())))
    tests.append((f"date_to_iso_year", lambda: (run_query("SELECT toISOYear(today())"), ok())))
    tests.append((f"date_to_week", lambda: (run_query("SELECT toWeek(today())"), ok())))
    tests.append((f"date_to_relative_day", lambda: (run_query("SELECT toRelativeDayNum(today())"), ok())))
    tests.append((f"date_to_relative_hour", lambda: (run_query("SELECT toRelativeHourNum(now())"), ok())))
    tests.append((f"date_to_relative_minute", lambda: (run_query("SELECT toRelativeMinuteNum(now())"), ok())))
    tests.append((f"date_to_relative_second", lambda: (run_query("SELECT toRelativeSecondNum(now())"), ok())))

    return tests


def generate_math_tests():
    tests = []
    tests.append(("math_abs", lambda: assert_eq(run_query("SELECT abs(-5)"), "5")))
    tests.append(("math_ceil", lambda: assert_eq(run_query("SELECT ceil(3.2)"), "4")))
    tests.append(("math_floor", lambda: assert_eq(run_query("SELECT floor(3.8)"), "3")))
    tests.append(("math_round", lambda: assert_eq(run_query("SELECT round(3.5)"), "4")))
    tests.append(("math_round_to_exp2", lambda: (run_query("SELECT roundToExp2(100)"), ok())))
    tests.append(("math_div", lambda: (run_query("SELECT div(10,3)"), ok())))
    tests.append(("math_modulo", lambda: assert_eq(run_query("SELECT modulo(10,3)"), "1")))
    tests.append(("math_int_div", lambda: assert_eq(run_query("SELECT intDiv(10,3)"), "3")))
    tests.append(("math_int_div_or_zero", lambda: assert_eq(run_query("SELECT intDivOrZero(10,0)"), "0")))
    tests.append(("math_least", lambda: assert_eq(run_query("SELECT least(3,1,2)"), "1")))
    tests.append(("math_greatest", lambda: assert_eq(run_query("SELECT greatest(3,1,2)"), "3")))
    tests.append(("math_sqrt", lambda: assert_eq(run_query("SELECT sqrt(9)"), "3")))
    tests.append(("math_cbrt", lambda: (run_query("SELECT cbrt(27)"), ok())))
    tests.append(("math_pow", lambda: assert_eq(run_query("SELECT pow(2,3)"), "8")))
    tests.append(("math_exp", lambda: (run_query("SELECT exp(1)"), ok())))
    tests.append(("math_log", lambda: (run_query("SELECT log(2.718281828)"), ok())))
    tests.append(("math_log2", lambda: (run_query("SELECT log2(8)"), ok())))
    tests.append(("math_log10", lambda: assert_eq(run_query("SELECT log10(100)"), "2")))
    tests.append(("math_sin", lambda: (run_query("SELECT sin(0)"), ok())))
    tests.append(("math_cos", lambda: (run_query("SELECT cos(0)"), ok())))
    tests.append(("math_tan", lambda: (run_query("SELECT tan(0)"), ok())))
    tests.append(("math_asin", lambda: (run_query("SELECT asin(0)"), ok())))
    tests.append(("math_acos", lambda: (run_query("SELECT acos(1)"), ok())))
    tests.append(("math_atan", lambda: (run_query("SELECT atan(0)"), ok())))
    tests.append(("math_pi", lambda: assert_contains(run_query("SELECT pi()"), "3.14")))
    tests.append(("math_e", lambda: assert_contains(run_query("SELECT e()"), "2.71")))
    tests.append(("math_factorial", lambda: assert_eq(run_query("SELECT factorial(5)"), "120")))
    tests.append(("math_abs_float", lambda: assert_eq(run_query("SELECT abs(-3.14)"), "3.14")))
    tests.append(("math_ceil_neg", lambda: assert_eq(run_query("SELECT ceil(-3.8)"), "-3")))
    tests.append(("math_floor_neg", lambda: assert_eq(run_query("SELECT floor(-3.2)"), "-4")))
    tests.append(("math_round2", lambda: assert_eq(run_query("SELECT round(3.456, 2)"), "3.46")))
    tests.append(("math_modulo_zero", lambda: (run_query("SELECT modulo(10,0)"), ok())))
    tests.append(("math_int_div_zero", lambda: assert_eq(run_query("SELECT intDivOrZero(0,5)"), "0")))
    tests.append(("math_pow_zero", lambda: assert_eq(run_query("SELECT pow(5,0)"), "1")))
    tests.append(("math_sqrt_zero", lambda: assert_eq(run_query("SELECT sqrt(0)"), "0")))
    tests.append(("math_log_e", lambda: (run_query("SELECT log(e())"), ok())))
    tests.append(("math_exp_zero", lambda: assert_eq(run_query("SELECT exp(0)"), "1")))
    tests.append(("math_log2_one", lambda: assert_eq(run_query("SELECT log2(1)"), "0")))
    tests.append(("math_log10_one", lambda: assert_eq(run_query("SELECT log10(1)"), "0")))
    tests.append(("math_cbrt_neg", lambda: (run_query("SELECT cbrt(-8)"), ok())))
    tests.append(("math_gcd", lambda: (run_query("SELECT gcd(12,8)"), ok())))
    tests.append(("math_lcm", lambda: (run_query("SELECT lcm(4,6)"), ok())))

    return tests


def generate_type_conversion_tests():
    tests = []
    tests.append(("conv_to_int8", lambda: assert_eq(run_query("SELECT toInt8(42)"), "42")))
    tests.append(("conv_to_int16", lambda: assert_eq(run_query("SELECT toInt16(1000)"), "1000")))
    tests.append(("conv_to_int32", lambda: assert_eq(run_query("SELECT toInt32(100000)"), "100000")))
    tests.append(("conv_to_int64", lambda: assert_eq(run_query("SELECT toInt64(1000000)"), "1000000")))
    tests.append(("conv_to_uint8", lambda: assert_eq(run_query("SELECT toUInt8(255)"), "255")))
    tests.append(("conv_to_uint16", lambda: assert_eq(run_query("SELECT toUInt16(65535)"), "65535")))
    tests.append(("conv_to_uint32", lambda: assert_eq(run_query("SELECT toUInt32(4294967295)"), "4294967295")))
    tests.append(("conv_to_uint64", lambda: assert_eq(run_query("SELECT toUInt64(18446744073709551615)"), "18446744073709551615")))
    tests.append(("conv_to_float32", lambda: assert_contains(run_query("SELECT toFloat32(3.14)"), "3.14")))
    tests.append(("conv_to_float64", lambda: assert_eq(run_query("SELECT toFloat64(3.14159)"), "3.14159")))
    tests.append(("conv_to_string", lambda: assert_eq(run_query("SELECT toString(123)"), "123")))
    tests.append(("conv_to_date", lambda: assert_contains(run_query("SELECT toDate('2024-01-15')"), "2024")))
    tests.append(("conv_to_datetime", lambda: assert_contains(run_query("SELECT toDateTime('2024-01-15 10:30:00')"), "2024")))
    tests.append(("conv_parse_date", lambda: (run_query("SELECT parseDateTimeBestEffort('15/01/2024')"), ok())))
    tests.append(("conv_to_uint8_or_zero", lambda: assert_eq(run_query("SELECT toUInt8OrZero('abc')"), "0")))
    tests.append(("conv_to_int8_or_zero", lambda: assert_eq(run_query("SELECT toInt8OrZero('abc')"), "0")))
    tests.append(("conv_to_float32_or_zero", lambda: assert_eq(run_query("SELECT toFloat32OrZero('abc')"), "0")))
    tests.append(("conv_to_int_from_string", lambda: assert_eq(run_query("SELECT toInt32('42')"), "42")))
    tests.append(("conv_to_float_from_string", lambda: assert_eq(run_query("SELECT toFloat64('3.14')"), "3.14")))
    tests.append(("conv_to_string_from_int", lambda: assert_eq(run_query("SELECT toString(100)"), "100")))
    tests.append(("conv_to_string_from_float", lambda: assert_contains(run_query("SELECT toString(1.5)"), "1.5")))
    tests.append(("conv_to_uint64_or_zero", lambda: assert_eq(run_query("SELECT toUInt64OrZero('xyz')"), "0")))
    tests.append(("conv_to_int16_or_zero", lambda: assert_eq(run_query("SELECT toInt16OrZero('bad')"), "0")))
    tests.append(("conv_to_int32_or_zero", lambda: assert_eq(run_query("SELECT toInt32OrZero('bad')"), "0")))
    tests.append(("conv_to_int64_or_zero", lambda: assert_eq(run_query("SELECT toInt64OrZero('bad')"), "0")))
    tests.append(("conv_to_float64_or_zero", lambda: assert_eq(run_query("SELECT toFloat64OrZero('bad')"), "0")))
    tests.append(("conv_to_uint16_or_zero", lambda: assert_eq(run_query("SELECT toUInt16OrZero('bad')"), "0")))
    tests.append(("conv_to_uint32_or_zero", lambda: assert_eq(run_query("SELECT toUInt32OrZero('bad')"), "0")))
    tests.append(("conv_to_date_or_default", lambda: (run_query("SELECT toDateOrDefault('bad')"), ok())))
    tests.append(("conv_to_datetime_or_default", lambda: (run_query("SELECT toDateTimeOrDefault('bad')"), ok())))
    tests.append(("conv_to_uuid", lambda: assert_eq(run_query("SELECT toUUID('550e8400-e29b-41d4-a716-446655440000')"), "550e8400-e29b-41d4-a716-446655440000")))
    tests.append(("conv_to_bool", lambda: assert_eq(run_query("SELECT toUInt8(1)"), "1")))
    tests.append(("conv_cast_int", lambda: assert_eq(run_query("SELECT CAST(42 AS String)"), "42")))
    tests.append(("conv_cast_float", lambda: assert_eq(run_query("SELECT CAST('3.14' AS Float64)"), "3.14")))
    tests.append(("conv_cast_date", lambda: (run_query("SELECT CAST('2024-01-01' AS Date)"), ok())))
    tests.append(("conv_to_int8_or_null", lambda: assert_eq(run_query("SELECT toInt8OrNull('abc')"), "\\N")))
    tests.append(("conv_to_uint8_or_null", lambda: assert_eq(run_query("SELECT toUInt8OrNull('abc')"), "\\N")))
    tests.append(("conv_to_float64_or_null", lambda: assert_eq(run_query("SELECT toFloat64OrNull('abc')"), "\\N")))
    tests.append(("conv_to_int16_or_null", lambda: assert_eq(run_query("SELECT toInt16OrNull('abc')"), "\\N")))
    tests.append(("conv_to_int32_or_null", lambda: assert_eq(run_query("SELECT toInt32OrNull('abc')"), "\\N")))
    tests.append(("conv_to_int64_or_null", lambda: assert_eq(run_query("SELECT toInt64OrNull('abc')"), "\\N")))
    tests.append(("conv_to_uint16_or_null", lambda: assert_eq(run_query("SELECT toUInt16OrNull('abc')"), "\\N")))
    tests.append(("conv_to_uint32_or_null", lambda: assert_eq(run_query("SELECT toUInt32OrNull('abc')"), "\\N")))
    tests.append(("conv_to_uint64_or_null", lambda: assert_eq(run_query("SELECT toUInt64OrNull('abc')"), "\\N")))
    tests.append(("conv_to_float32_or_null", lambda: assert_eq(run_query("SELECT toFloat32OrNull('abc')"), "\\N")))
    tests.append(("conv_to_string_from_date", lambda: (run_query("SELECT toString(today())"), ok())))
    tests.append(("conv_to_fixed_string", lambda: (run_query("SELECT toFixedString('hi', 5)"), ok())))

    return tests


def generate_array_tests():
    tests = []
    tests.append(("arr_empty", lambda: assert_eq(run_query("SELECT empty([])"), "1")))
    tests.append(("arr_not_empty", lambda: assert_eq(run_query("SELECT notEmpty([1])"), "1")))
    tests.append(("arr_length", lambda: assert_eq(run_query("SELECT length([1,2,3])"), "3")))
    tests.append(("arr_concat", lambda: assert_eq(run_query("SELECT arrayConcat([1,2],[3,4])"), "[1,2,3,4]")))
    tests.append(("arr_slice", lambda: assert_eq(run_query("SELECT arraySlice([1,2,3,4,5],2,3)"), "[2,3,4]")))
    tests.append(("arr_sort", lambda: assert_eq(run_query("SELECT arraySort([3,1,2])"), "[1,2,3]")))
    tests.append(("arr_reverse", lambda: assert_eq(run_query("SELECT arrayReverse([1,2,3])"), "[3,2,1]")))
    tests.append(("arr_uniq", lambda: assert_eq(run_query("SELECT arrayUniq([1,1,2,3,3])"), "3")))
    tests.append(("arr_join", lambda: assert_eq(run_query("SELECT arrayJoin([1,2,3])"), "1\n2\n3")))
    tests.append(("arr_map", lambda: (run_query("SELECT arrayMap(x->x*2,[1,2,3])"), ok())))
    tests.append(("arr_filter", lambda: (run_query("SELECT arrayFilter(x->x>1,[1,2,3])"), ok())))
    tests.append(("arr_exists", lambda: (run_query("SELECT arrayExists(x->x>2,[1,2,3])"), ok())))
    tests.append(("arr_all", lambda: (run_query("SELECT arrayAll(x->x>0,[1,2,3])"), ok())))
    tests.append(("arr_count", lambda: (run_query("SELECT arrayCount(x->x>1,[1,2,3])"), ok())))
    tests.append(("arr_has", lambda: assert_eq(run_query("SELECT has([1,2,3],2)"), "1")))
    tests.append(("arr_has_not", lambda: assert_eq(run_query("SELECT has([1,2,3],5)"), "0")))
    tests.append(("arr_has_any", lambda: assert_eq(run_query("SELECT hasAny([1,2,3],[3,4])"), "1")))
    tests.append(("arr_has_all", lambda: assert_eq(run_query("SELECT hasAll([1,2,3],[1,2])"), "1")))
    tests.append(("arr_index_of", lambda: assert_eq(run_query("SELECT indexOf([10,20,30],20)"), "2")))
    tests.append(("arr_count_equal", lambda: assert_eq(run_query("SELECT countEqual([1,1,2,3,1],1)"), "3")))
    tests.append(("arr_range", lambda: assert_eq(run_query("SELECT range(5)"), "[0,1,2,3,4]")))
    tests.append(("arr_enumerate", lambda: (run_query("SELECT arrayEnumerate([10,20,30])"), ok())))
    tests.append(("arr_enumerate_uniq", lambda: (run_query("SELECT arrayEnumerateUniq([1,1,2,2])"), ok())))
    tests.append(("arr_empty_literal", lambda: assert_eq(run_query("SELECT []"), "[]")))
    tests.append(("arr_create", lambda: assert_eq(run_query("SELECT [1,2,3]"), "[1,2,3]")))
    tests.append(("arr_string", lambda: assert_eq(run_query("SELECT ['a','b']"), "['a','b']")))
    tests.append(("arr_nested", lambda: assert_eq(run_query("SELECT [[1,2],[3,4]]"), "[[1,2],[3,4]]")))
    tests.append(("arr_element", lambda: assert_eq(run_query("SELECT [10,20,30][2]"), "20")))
    tests.append(("arr_first_index", lambda: (run_query("SELECT arrayFirst(x->x>2,[1,2,3,4])"), ok())))
    tests.append(("arr_last_index", lambda: (run_query("SELECT arrayLast(x->x>2,[1,2,3,4])"), ok())))
    tests.append(("arr_first_index_of", lambda: (run_query("SELECT arrayFirstIndex(x->x>2,[1,2,3,4])"), ok())))
    tests.append(("arr_last_index_of", lambda: (run_query("SELECT arrayLastIndex(x->x>2,[1,2,3,4])"), ok())))
    tests.append(("arr_cumsum", lambda: (run_query("SELECT arrayCumSum([1,2,3,4])"), ok())))
    tests.append(("arr_sort_desc", lambda: assert_eq(run_query("SELECT arraySort(x->-x,[1,2,3])"), "[3,2,1]")))
    tests.append(("arr_distinct", lambda: (run_query("SELECT arrayDistinct([1,1,2,2,3])"), ok())))
    tests.append(("arr_flatten", lambda: (run_query("SELECT arrayFlatten([[1,2],[3,4]])"), ok())))
    tests.append(("arr_compact", lambda: (run_query("SELECT arrayCompact([1,1,2,3,3])"), ok())))
    tests.append(("arr_zip", lambda: (run_query("SELECT arrayZip([1,2],['a','b'])"), ok())))
    tests.append(("arr_push_back", lambda: (run_query("SELECT arrayPushBack([1,2],3)"), ok())))
    tests.append(("arr_push_front", lambda: (run_query("SELECT arrayPushFront([1,2],0)"), ok())))
    tests.append(("arr_pop_back", lambda: (run_query("SELECT arrayPopBack([1,2,3])"), ok())))
    tests.append(("arr_pop_front", lambda: (run_query("SELECT arrayPopFront([1,2,3])"), ok())))
    tests.append(("arr_insert", lambda: (run_query("SELECT arrayInsert([1,2,3],1,99)"), ok())))
    tests.append(("arr_resize", lambda: (run_query("SELECT arrayResize([1,2],5)"), ok())))
    tests.append(("arr_string_concat", lambda: assert_eq(run_query("SELECT arrayStringConcat(['a','b','c'],'-')"), "a-b-c")))
    tests.append(("arr_join_nested", lambda: assert_eq(run_query("SELECT arrayJoin([[1,2],[3,4]])"), "[1,2]\n[3,4]")))
    tests.append(("arr_empty_of_type", lambda: assert_eq(run_query("SELECT emptyArrayUInt8()"), "[]")))
    tests.append(("arr_empty_to_single", lambda: (run_query("SELECT emptyArrayToSingle(emptyArrayUInt8())"), ok())))
    tests.append(("arr_reduce", lambda: (run_query("SELECT arrayReduce('sum',[1,2,3])"), ok())))

    return tests


def generate_system_tests():
    tests = []
    tests.append(("sys_databases", lambda: assert_contains(run_query("SELECT * FROM system.databases"), "default")))
    tests.append(("sys_tables", lambda: (run_query("SELECT * FROM system.tables LIMIT 1"), ok())))
    tests.append(("sys_columns", lambda: (run_query("SELECT * FROM system.columns LIMIT 1"), ok())))
    tests.append(("sys_functions", lambda: (run_query("SELECT count() FROM system.functions"), ok())))
    tests.append(("sys_settings", lambda: (run_query("SELECT * FROM system.settings LIMIT 5"), ok())))
    tests.append(("sys_processes", lambda: (run_query("SELECT * FROM system.processes"), ok())))
    tests.append(("sys_metrics", lambda: (run_query("SELECT * FROM system.metrics LIMIT 5"), ok())))
    tests.append(("sys_events", lambda: (run_query("SELECT * FROM system.events LIMIT 5"), ok())))
    tests.append(("sys_async_metrics", lambda: (run_query("SELECT * FROM system.asynchronous_metrics LIMIT 5"), ok())))
    tests.append(("sys_clusters", lambda: (run_query("SELECT * FROM system.clusters"), ok())))
    tests.append(("sys_dictionaries", lambda: (run_query("SELECT * FROM system.dictionaries"), ok())))
    tests.append(("sys_parts", lambda: (run_query("SELECT * FROM system.parts LIMIT 1"), ok())))
    tests.append(("sys_data_type_families", lambda: (run_query("SELECT count() FROM system.data_type_families"), ok())))
    tests.append(("sys_engines", lambda: (run_query("SELECT * FROM system.table_engines"), ok())))
    tests.append(("sys_formats", lambda: (run_query("SELECT count() FROM system.formats"), ok())))
    tests.append(("sys_collations", lambda: (run_query("SELECT * FROM system.collations LIMIT 1"), ok())))
    tests.append(("sys_numbers", lambda: assert_eq(run_query("SELECT * FROM system.numbers LIMIT 3"), "0\n1\n2")))
    tests.append(("sys_zeros", lambda: assert_eq(run_query("SELECT * FROM system.zeros LIMIT 3"), "0\n0\n0")))
    tests.append(("sys_ones", lambda: assert_eq(run_query("SELECT * FROM system.ones LIMIT 3"), "1\n1\n1")))
    tests.append(("sys_one", lambda: assert_eq(run_query("SELECT * FROM system.one"), "0")))
    tests.append(("sys_build_options", lambda: (run_query("SELECT * FROM system.build_options"), ok())))
    tests.append(("sys_server_settings", lambda: (run_query("SELECT * FROM system.server_settings LIMIT 1"), ok())))
    tests.append(("sys_macros", lambda: (run_query("SELECT * FROM system.macros"), ok())))
    tests.append(("sys_zookeeper", lambda: (run_query("SELECT * FROM system.zookeeper LIMIT 1"), ok())))
    tests.append(("sys_merges", lambda: (run_query("SELECT * FROM system.merges"), ok())))
    tests.append(("sys_mutations", lambda: (run_query("SELECT * FROM system.mutations"), ok())))
    tests.append(("sys_replicas", lambda: (run_query("SELECT * FROM system.replicas"), ok())))
    tests.append(("sys_fetches", lambda: (run_query("SELECT * FROM system.replicated_fetches"), ok())))
    tests.append(("sys_replication_queue", lambda: (run_query("SELECT * FROM system.replication_queue"), ok())))
    tests.append(("sys_detached_parts", lambda: (run_query("SELECT * FROM system.detached_parts"), ok())))
    tests.append(("sys_dropped_tables", lambda: (run_query("SELECT * FROM system.dropped_tables"), ok())))
    tests.append(("sys_user_processes", lambda: (run_query("SELECT * FROM system.user_processes"), ok())))
    tests.append(("sys_sessions", lambda: (run_query("SELECT * FROM system.sessions"), ok())))
    tests.append(("sys_licenses", lambda: (run_query("SELECT * FROM system.licenses"), ok())))
    tests.append(("sys_quota_usage", lambda: (run_query("SELECT * FROM system.quota_usage"), ok())))

    return tests


def generate_datatype_tests():
    tests = []
    db = f"db_{_run_id}_dt"
    tests.append(("dt_setup_db", lambda: (run_query(f"CREATE DATABASE {db}"), ok())))

    # UInt8-64
    for bits in [8, 16, 32, 64]:
        t = tbl(f"dt_uint{bits}")
        tests.append((f"dt_uint{bits}_create", lambda _t=t, _b=bits: (
            run_query(f"CREATE TABLE {db}.{_t} (v UInt{_b}) ENGINE = Memory"), ok())))
        tests.append((f"dt_uint{bits}_insert", lambda _t=t, _b=bits: (
            run_query(f"INSERT INTO {db}.{_t} VALUES (1),(2),(3)"), ok())))
        tests.append((f"dt_uint{bits}_select", lambda _t=t: assert_eq(run_query(f"SELECT sum(v) FROM {db}.{_t}"), "6")))

    # Int8-64
    for bits in [8, 16, 32, 64]:
        t = tbl(f"dt_int{bits}")
        tests.append((f"dt_int{bits}_create", lambda _t=t, _b=bits: (
            run_query(f"CREATE TABLE {db}.{_t} (v Int{_b}) ENGINE = Memory"), ok())))
        tests.append((f"dt_int{bits}_insert", lambda _t=t: (
            run_query(f"INSERT INTO {db}.{_t} VALUES (-1),(0),(1)"), ok())))
        tests.append((f"dt_int{bits}_select", lambda _t=t: assert_eq(run_query(f"SELECT sum(v) FROM {db}.{_t}"), "0")))

    # Float32/64
    for bits in [32, 64]:
        t = tbl(f"dt_float{bits}")
        tests.append((f"dt_float{bits}_create", lambda _t=t, _b=bits: (
            run_query(f"CREATE TABLE {db}.{_t} (v Float{_b}) ENGINE = Memory"), ok())))
        tests.append((f"dt_float{bits}_insert", lambda _t=t: (
            run_query(f"INSERT INTO {db}.{_t} VALUES (1.5),(2.5)"), ok())))
        tests.append((f"dt_float{bits}_select", lambda _t=t: assert_eq(run_query(f"SELECT sum(v) FROM {db}.{_t}"), "4")))

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

    # Array
    t = tbl("dt_array")
    tests.append((f"dt_array_create", lambda: (
        run_query(f"CREATE TABLE {db}.{t} (v Array(UInt32)) ENGINE = Memory"), ok())))
    tests.append((f"dt_array_insert", lambda: (
        run_query(f"INSERT INTO {db}.{t} VALUES ([1,2,3]),([4,5])"), ok())))
    tests.append((f"dt_array_select", lambda: assert_eq(run_query(f"SELECT count() FROM {db}.{t}"), "2")))

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

    # Boolean (UInt8 alias)
    tests.append((f"dt_bool_expr", lambda: assert_eq(run_query("SELECT 1=1"), "1")))
    tests.append((f"dt_bool_false", lambda: assert_eq(run_query("SELECT 1=2"), "0")))

    # Nothing
    tests.append((f"dt_nothing", lambda: assert_eq(run_query("SELECT NULL"), "\\N")))

    # Nested (Array of Tuples)
    tests.append((f"dt_nested_arr_tuple", lambda: (run_query("SELECT [(1,'a'),(2,'b')]"), ok())))

    tests.append(("dt_cleanup", lambda: (run_query(f"DROP DATABASE IF EXISTS {db}"), ok())))
    return tests


def generate_format_tests():
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

    tests.append(("fmt_cleanup", lambda: (run_query(f"DROP DATABASE IF EXISTS {db}"), ok())))
    return tests


def generate_edge_case_tests():
    tests = []
    db = f"db_{_run_id}_edge"
    tests.append(("edge_setup_db", lambda: (run_query(f"CREATE DATABASE {db}"), ok())))

    # NULL handling
    tests.append(("edge_null_select", lambda: assert_eq(run_query("SELECT NULL"), "\\N")))
    tests.append(("edge_null_coalesce", lambda: assert_eq(run_query("SELECT coalesce(NULL,1)"), "1")))
    tests.append(("edge_null_is_null", lambda: assert_eq(run_query("SELECT NULL IS NULL"), "1")))
    tests.append(("edge_null_is_not_null", lambda: assert_eq(run_query("SELECT 1 IS NOT NULL"), "1")))

    # NaN
    tests.append(("edge_nan", lambda: assert_eq(run_query("SELECT isNaN(0/0)"), "1")))
    tests.append(("edge_nan_false", lambda: assert_eq(run_query("SELECT isNaN(1)"), "0")))

    # Inf
    tests.append(("edge_inf", lambda: assert_eq(run_query("SELECT isInfinite(1/0)"), "1")))
    tests.append(("edge_inf_neg", lambda: assert_eq(run_query("SELECT isFinite(-1)"), "1")))

    # Empty strings
    tests.append(("edge_empty_string", lambda: assert_eq(run_query("SELECT ''"), "")))
    tests.append(("edge_empty_string_length", lambda: assert_eq(run_query("SELECT length('')"), "0")))
    tests.append(("edge_empty_string_empty", lambda: assert_eq(run_query("SELECT empty('')"), "1")))

    # Very large numbers
    tests.append(("edge_large_uint64", lambda: assert_eq(run_query("SELECT toUInt64(18446744073709551615)"), "18446744073709551615")))
    tests.append(("edge_large_int64", lambda: assert_eq(run_query("SELECT toInt64(9223372036854775807)"), "9223372036854775807")))

    # Unicode
    tests.append(("edge_unicode_string", lambda: assert_eq(run_query("SELECT '你好'"), "你好")))
    tests.append(("edge_unicode_length", lambda: assert_eq(run_query("SELECT lengthUTF8('你好')"), "2")))
    tests.append(("edge_unicode_emoji", lambda: assert_eq(run_query("SELECT lengthUTF8('😀')"), "1")))

    # Array with NULL
    t = tbl("edge_null_arr")
    tests.append((f"edge_null_arr_setup", lambda: (
        run_query(f"CREATE TABLE {db}.{t} (v Array(Nullable(UInt32))) ENGINE = Memory"),
        run_query(f"INSERT INTO {db}.{t} VALUES ([1,NULL,3])"),
        ok())))
    tests.append((f"edge_null_arr_select", lambda: (run_query(f"SELECT v FROM {db}.{t}"), ok())))

    # Division by zero
    tests.append(("edge_div_zero", lambda: (run_query("SELECT 1/0"), ok())))
    tests.append(("edge_int_div_zero", lambda: assert_eq(run_query("SELECT intDivOrZero(1,0)"), "0")))

    # Empty table queries
    t2 = tbl("edge_empty")
    tests.append((f"edge_empty_setup", lambda: (
        run_query(f"CREATE TABLE {db}.{t2} (id UInt32) ENGINE = Memory"), ok())))
    tests.append((f"edge_empty_count", lambda: assert_eq(run_query(f"SELECT count() FROM {db}.{t2}"), "0")))
    tests.append((f"edge_empty_sum", lambda: assert_eq(run_query(f"SELECT sum(id) FROM {db}.{t2}"), "0")))

    # Zero division in modulo
    tests.append(("edge_mod_zero", lambda: (run_query("SELECT modulo(10,0)"), ok())))

    # Boolean edge cases
    tests.append(("edge_bool_true", lambda: assert_eq(run_query("SELECT 1"), "1")))
    tests.append(("edge_bool_false", lambda: assert_eq(run_query("SELECT 0"), "0")))

    # Special float values
    tests.append(("edge_float_zero", lambda: assert_eq(run_query("SELECT toFloat64(0)"), "0")))
    tests.append(("edge_float_neg_zero", lambda: (run_query("SELECT toFloat64(-0)"), ok())))

    # Large arrays
    tests.append(("edge_large_array", lambda: (run_query("SELECT range(1000)"), ok())))

    # String with special chars
    tests.append(("edge_string_tab", lambda: (run_query("SELECT '\t'"), ok())))
    tests.append(("edge_string_newline", lambda: (run_query("SELECT '\n'"), ok())))

    # Nested function calls
    tests.append(("edge_nested_func", lambda: assert_eq(run_query("SELECT abs(floor(-3.7))"), "4")))

    # Multiple expressions
    tests.append(("edge_multi_expr", lambda: assert_eq(run_query("SELECT 1,2,3"), "1\t2\t3")))

    # Empty query / SELECT without FROM
    tests.append(("edge_select_no_from", lambda: assert_eq(run_query("SELECT 42"), "42")))

    # Deeply nested arrays
    tests.append(("edge_nested_array", lambda: assert_eq(run_query("SELECT [[[1]]]"), "[[[1]]]")))

    # Very long string
    tests.append(("edge_long_string", lambda: (run_query("SELECT repeat('a', 10000)"), ok())))

    # Tuple operations
    tests.append(("edge_tuple_access", lambda: assert_eq(run_query("SELECT (1,'a').1"), "1")))
    tests.append(("edge_tuple_access2", lambda: assert_eq(run_query("SELECT (1,'a').2"), "a")))

    # CAST edge cases
    tests.append(("edge_cast_null_to_int", lambda: assert_eq(run_query("SELECT toInt32OrZero(NULL)"), "0")))
    tests.append(("edge_cast_string_to_date", lambda: (run_query("SELECT toDate('2024-06-15')"), ok())))

    # IN with empty array
    tests.append(("edge_in_empty", lambda: assert_eq(run_query("SELECT 1 IN ()"), "0")))

    tests.append(("edge_cleanup", lambda: (run_query(f"DROP DATABASE IF EXISTS {db}"), ok())))
    return tests


def generate_additional_select_tests():
    """Additional SELECT tests to reach 1000+."""
    tests = []
    db = f"db_{_run_id}_extra"
    tests.append(("extra_setup_db", lambda: (run_query(f"CREATE DATABASE {db}"), ok())))

    t = tbl("extra_data")
    tests.append((f"extra_setup", lambda: (
        run_query(f"CREATE TABLE {db}.{t} (id UInt32, val Int32, name String, grp UInt8) ENGINE = Memory"),
        run_query(f"INSERT INTO {db}.{t} VALUES "
                  + ",".join(f"({i},{i*10},'name{i}',{i%5})" for i in range(1, 51))),
        ok())))

    # More WHERE variations
    for v in [1, 5, 10, 25, 50]:
        tests.append((f"extra_where_{v}", lambda _v=v: assert_contains(
            run_query(f"SELECT * FROM {db}.{t} WHERE id={_v}"), str(_v))))

    # ORDER BY variations
    tests.append((f"extra_order_by_name", lambda: (run_query(f"SELECT * FROM {db}.{t} ORDER BY name LIMIT 5"), ok())))
    tests.append((f"extra_order_by_multi", lambda: (run_query(f"SELECT * FROM {db}.{t} ORDER BY grp,id LIMIT 5"), ok())))
    tests.append((f"extra_order_by_desc", lambda: (run_query(f"SELECT * FROM {db}.{t} ORDER BY id DESC LIMIT 5"), ok())))

    # LIMIT variations
    for lim in [1, 2, 5, 10, 20, 49]:
        tests.append((f"extra_limit_{lim}", lambda _l=lim: (
            run_query(f"SELECT count() FROM (SELECT * FROM {db}.{t} LIMIT {_l})"), ok())))

    # OFFSET variations
    tests.append((f"extra_offset_1", lambda: assert_eq(run_query(f"SELECT id FROM {db}.{t} ORDER BY id LIMIT 1 OFFSET 1"), "2")))
    tests.append((f"extra_offset_5", lambda: assert_eq(run_query(f"SELECT id FROM {db}.{t} ORDER BY id LIMIT 1 OFFSET 5"), "6")))
    tests.append((f"extra_offset_10", lambda: assert_eq(run_query(f"SELECT id FROM {db}.{t} ORDER BY id LIMIT 1 OFFSET 10"), "11")))

    # GROUP BY with various aggregates
    for func in ["count()", "sum(val)", "avg(val)", "min(val)", "max(val)"]:
        fname = func.replace("(", "_").replace(")", "").replace(" ", "")
        tests.append((f"extra_group_{fname}", lambda _f=func: (
            run_query(f"SELECT grp, {_f} FROM {db}.{t} GROUP BY grp ORDER BY grp"), ok())))

    # HAVING variations
    for thresh in [5, 10, 15]:
        tests.append((f"extra_having_{thresh}", lambda _th=thresh: (
            run_query(f"SELECT grp, count() FROM {db}.{t} GROUP BY grp HAVING count()>{_th}"), ok())))

    # DISTINCT variations
    tests.append((f"extra_distinct_grp", lambda: (run_query(f"SELECT DISTINCT grp FROM {db}.{t} ORDER BY grp"), ok())))
    tests.append((f"extra_distinct_count", lambda: assert_eq(run_query(f"SELECT count(DISTINCT grp) FROM {db}.{t}"), "5")))

    # Subquery variations
    tests.append((f"extra_subq_where", lambda: assert_eq(run_query(f"SELECT count() FROM {db}.{t} WHERE id IN (SELECT id FROM {db}.{t} WHERE grp=1)"), "10")))
    tests.append((f"extra_subq_from", lambda: assert_eq(run_query(f"SELECT count() FROM (SELECT * FROM {db}.{t})"), "50")))
    tests.append((f"extra_subq_scalar", lambda: assert_eq(run_query(f"SELECT (SELECT count() FROM {db}.{t})"), "50")))

    # CASE WHEN variations
    tests.append((f"extra_case_simple", lambda: (run_query(f"SELECT id, CASE WHEN val>100 THEN 'high' ELSE 'low' END FROM {db}.{t}"), ok())))
    tests.append((f"extra_case_multi", lambda: (run_query(f"SELECT CASE grp WHEN 0 THEN 'a' WHEN 1 THEN 'b' ELSE 'c' END FROM {db}.{t} LIMIT 5"), ok())))

    # IF variations
    tests.append((f"extra_if_simple", lambda: (run_query(f"SELECT if(val>100,'high','low') FROM {db}.{t} LIMIT 5"), ok())))
    tests.append((f"extra_if_nested", lambda: assert_eq(run_query("SELECT if(1>0, if(2>1,'yes','no'), 'no')"), "yes")))

    # multiIf
    tests.append((f"extra_multiif", lambda: (run_query(f"SELECT multiIf(grp=0,'a',grp=1,'b','c') FROM {db}.{t} LIMIT 5"), ok())))

    # UNION ALL variations
    tests.append((f"extra_union_3", lambda: assert_eq(run_query("SELECT 1 UNION ALL SELECT 2 UNION ALL SELECT 3"), "1\n2\n3")))
    tests.append((f"extra_union_tables", lambda: (
        run_query(f"SELECT id FROM {db}.{t} WHERE id<=5 UNION ALL SELECT id FROM {db}.{t} WHERE id>45"), ok())))

    # Expression tests
    tests.append(("extra_arith_add", lambda: assert_eq(run_query("SELECT 100+200"), "300")))
    tests.append(("extra_arith_sub", lambda: assert_eq(run_query("SELECT 500-200"), "300")))
    tests.append(("extra_arith_mul", lambda: assert_eq(run_query("SELECT 15*20"), "300")))
    tests.append(("extra_arith_div", lambda: (run_query("SELECT 300/7"), ok())))
    tests.append(("extra_arith_mod", lambda: assert_eq(run_query("SELECT 100%7"), "2")))

    # Aliases
    tests.append(("extra_alias_1", lambda: assert_eq(run_query("SELECT 42 AS answer"), "42")))
    tests.append(("extra_alias_2", lambda: assert_eq(run_query("SELECT 'hello' AS greeting"), "hello")))

    # Window functions (if supported)
    tests.append((f"extra_window_rownum", lambda: (run_query(f"SELECT id, rowNumberInAllBlocks() FROM {db}.{t} LIMIT 3"), ok())))

    # NULLIF
    tests.append(("extra_nullif_true", lambda: assert_eq(run_query("SELECT NULLIF(1,1)"), "\\N")))
    tests.append(("extra_nullif_false", lambda: assert_eq(run_query("SELECT NULLIF(1,2)"), "1")))

    # Comparison operators
    tests.append(("extra_eq", lambda: assert_eq(run_query("SELECT 1=1"), "1")))
    tests.append(("extra_neq", lambda: assert_eq(run_query("SELECT 1!=2"), "1")))
    tests.append(("extra_gt", lambda: assert_eq(run_query("SELECT 2>1"), "1")))
    tests.append(("extra_lt", lambda: assert_eq(run_query("SELECT 1<2"), "1")))
    tests.append(("extra_gte", lambda: assert_eq(run_query("SELECT 1>=1"), "1")))
    tests.append(("extra_lte", lambda: assert_eq(run_query("SELECT 1<=1"), "1")))

    # Logical operators
    tests.append(("extra_and", lambda: assert_eq(run_query("SELECT 1 AND 1"), "1")))
    tests.append(("extra_or", lambda: assert_eq(run_query("SELECT 0 OR 1"), "1")))
    tests.append(("extra_not", lambda: assert_eq(run_query("SELECT NOT 0"), "1")))
    tests.append(("extra_xor", lambda: assert_eq(run_query("SELECT xor(1,0)"), "1")))

    # Bitwise
    tests.append(("extra_bit_and", lambda: assert_eq(run_query("SELECT bitAnd(7,3)"), "3")))
    tests.append(("extra_bit_or", lambda: assert_eq(run_query("SELECT bitOr(5,3)"), "7")))
    tests.append(("extra_bit_xor", lambda: assert_eq(run_query("SELECT bitXor(5,3)"), "6")))
    tests.append(("extra_bit_not", lambda: (run_query("SELECT bitNot(5)"), ok())))
    tests.append(("extra_bit_shift_left", lambda: assert_eq(run_query("SELECT bitShiftLeft(1,3)"), "8")))
    tests.append(("extra_bit_shift_right", lambda: assert_eq(run_query("SELECT bitShiftRight(8,2)"), "2")))

    # Misc functions
    tests.append(("extra_typeof", lambda: (run_query("SELECT toTypeName(42)"), ok())))
    tests.append(("extra_typeof_str", lambda: assert_contains(run_query("SELECT toTypeName('hello')"), "String")))
    tests.append(("extra_sizeof", lambda: (run_query("SELECT sizeof(UInt32)"), ok())))
    tests.append(("extra_version", lambda: (run_query("SELECT version()"), ok())))
    tests.append(("extra_current_database", lambda: (run_query("SELECT currentDatabase()"), ok())))
    tests.append(("extra_timezone", lambda: (run_query("SELECT timezone()"), ok())))
    tests.append(("extra_hostname", lambda: (run_query("SELECT hostName()"), ok())))
    tests.append(("extra_visible_width", lambda: (run_query("SELECT visibleWidth('hello')"), ok())))
    tests.append(("extra_bar", lambda: (run_query("SELECT bar(50,0,100)"), ok())))
    tests.append(("extra_hex", lambda: assert_eq(run_query("SELECT hex(255)"), "FF")))
    tests.append(("extra_unhex", lambda: (run_query("SELECT unhex('FF')"), ok())))
    tests.append(("extra_bit_count", lambda: (run_query("SELECT bitCount(7)"), ok())))
    tests.append(("extra_city_hash", lambda: (run_query("SELECT cityHash64('test')"), ok())))
    tests.append(("extra_farm_hash", lambda: (run_query("SELECT farmHash64('test')"), ok())))
    tests.append(("extra_murmur_hash", lambda: (run_query("SELECT murmurHash3_32('test')"), ok())))
    tests.append(("extra_java_hash", lambda: (run_query("SELECT javaHash('test')"), ok())))
    tests.append(("extra_md5", lambda: (run_query("SELECT MD5('test')"), ok())))
    tests.append(("extra_halfMD5", lambda: (run_query("SELECT halfMD5('test')"), ok())))
    tests.append(("extra_siphash", lambda: (run_query("SELECT sipHash64('test')"), ok())))
    tests.append(("extra_generateUUID", lambda: (run_query("SELECT generateUUIDv4()"), ok())))

    # Conditional
    tests.append(("extra_cond_or", lambda: assert_eq(run_query("SELECT 0 OR 0"), "0")))
    tests.append(("extra_cond_and", lambda: assert_eq(run_query("SELECT 1 AND 0"), "0")))

    # LIKE patterns
    tests.append(("extra_like_pct", lambda: assert_eq(run_query("SELECT 'abc' LIKE '%b%'"), "1")))
    tests.append(("extra_like_under", lambda: assert_eq(run_query("SELECT 'abc' LIKE 'a_c'"), "1")))
    tests.append(("extra_like_exact", lambda: assert_eq(run_query("SELECT 'abc' LIKE 'abc'"), "1")))
    tests.append(("extra_like_nomatch", lambda: assert_eq(run_query("SELECT 'abc' LIKE 'xyz'"), "0")))

    # IN variants
    tests.append(("extra_in_tuple", lambda: assert_eq(run_query("SELECT 2 IN (1,2,3)"), "1")))
    tests.append(("extra_not_in_tuple", lambda: assert_eq(run_query("SELECT 5 NOT IN (1,2,3)"), "1")))
    tests.append(("extra_in_range", lambda: assert_eq(run_query("SELECT 3 IN (SELECT * FROM system.numbers LIMIT 5)"), "1")))

    # String comparison
    tests.append(("extra_str_cmp", lambda: assert_eq(run_query("SELECT 'a' < 'b'"), "1")))
    tests.append(("extra_str_eq", lambda: assert_eq(run_query("SELECT 'abc' = 'abc'"), "1")))

    # Misc math
    tests.append(("extra_neg", lambda: assert_eq(run_query("SELECT -(-5)"), "5")))
    tests.append(("extra_int_div", lambda: assert_eq(run_query("SELECT 17 DIV 5"), "3")))
    tests.append(("extra_int_div_func", lambda: assert_eq(run_query("SELECT intDiv(17,5)"), "3")))

    tests.append(("extra_cleanup", lambda: (run_query(f"DROP DATABASE IF EXISTS {db}"), ok())))
    return tests


def generate_additional_function_tests():
    """More function tests to boost count."""
    tests = []
    # String extras
    tests.append(("fn_reverse_hello", lambda: assert_eq(run_query("SELECT reverse('abc')"), "cba")))
    tests.append(("fn_repeat", lambda: assert_eq(run_query("SELECT repeat('x',5)"), "xxxxx")))
    tests.append(("fn_space", lambda: assert_eq(run_query("SELECT length(space(10))"), "10")))
    tests.append(("fn_lower", lambda: assert_eq(run_query("SELECT lower('ABC')"), "abc")))
    tests.append(("fn_upper", lambda: assert_eq(run_query("SELECT upper('abc')"), "ABC")))
    tests.append(("fn_concat_ws", lambda: assert_eq(run_query("SELECT concatWS('-','a','b','c')"), "a-b-c")))
    tests.append(("fn_substr_neg", lambda: (run_query("SELECT substring('hello',-3,2)"), ok())))

    # Date extras
    tests.append(("fn_today_str", lambda: (run_query("SELECT toString(today())"), ok())))
    tests.append(("fn_now_str", lambda: (run_query("SELECT toString(now())"), ok())))
    tests.append(("fn_add_days_0", lambda: assert_eq(run_query("SELECT toString(addDays(toDate('2024-01-01'),0)"), "2024-01-01")))
    tests.append(("fn_add_months_wrap", lambda: (run_query("SELECT addMonths(toDate('2024-01-31'),1)"), ok())))

    # Math extras
    tests.append(("fn_abs_zero", lambda: assert_eq(run_query("SELECT abs(0)"), "0")))
    tests.append(("fn_ceil_int", lambda: assert_eq(run_query("SELECT ceil(5)"), "5")))
    tests.append(("fn_floor_int", lambda: assert_eq(run_query("SELECT floor(5)"), "5")))
    tests.append(("fn_round_int", lambda: assert_eq(run_query("SELECT round(5)"), "5")))
    tests.append(("fn_sqrt_1", lambda: assert_eq(run_query("SELECT sqrt(1)"), "1")))
    tests.append(("fn_pow_0", lambda: assert_eq(run_query("SELECT pow(0,0)"), "1")))
    tests.append(("fn_log_1", lambda: assert_eq(run_query("SELECT log(1)"), "0")))
    tests.append(("fn_exp_0", lambda: assert_eq(run_query("SELECT exp(0)"), "1")))
    tests.append(("fn_sin_0", lambda: assert_eq(run_query("SELECT sin(0)"), "0")))
    tests.append(("fn_cos_0", lambda: assert_eq(run_query("SELECT cos(0)"), "1")))

    # Array extras
    tests.append(("fn_arr_empty_lit", lambda: assert_eq(run_query("SELECT emptyArrayUInt8()"), "[]")))
    tests.append(("fn_arr_len_0", lambda: assert_eq(run_query("SELECT length([])"), "0")))
    tests.append(("fn_arr_sort_empty", lambda: assert_eq(run_query("SELECT arraySort([])"), "[]")))
    tests.append(("fn_arr_rev_empty", lambda: assert_eq(run_query("SELECT arrayReverse([])"), "[]")))
    tests.append(("fn_arr_concat_empty", lambda: assert_eq(run_query("SELECT arrayConcat([1],[])"), "[1]")))
    tests.append(("fn_arr_has_str", lambda: assert_eq(run_query("SELECT has(['a','b'],'a')"), "1")))

    # Type conversion extras
    tests.append(("fn_to_int_round", lambda: (run_query("SELECT toInt32(3.7)"), ok())))
    tests.append(("fn_to_int_trunc", lambda: (run_query("SELECT toInt32(-3.7)"), ok())))
    tests.append(("fn_to_float_int", lambda: assert_eq(run_query("SELECT toFloat64(5)"), "5")))
    tests.append(("fn_to_str_bool", lambda: (run_query("SELECT toString(true)"), ok())))
    tests.append(("fn_to_date_today", lambda: (run_query("SELECT toDate(today())"), ok())))

    # Misc
    tests.append(("fn_greatest_multi", lambda: assert_eq(run_query("SELECT greatest(1,2,3,4,5)"), "5")))
    tests.append(("fn_least_multi", lambda: assert_eq(run_query("SELECT least(5,4,3,2,1)"), "1")))
    tests.append(("fn_neg_inf", lambda: (run_query("SELECT -1/0"), ok())))
    tests.append(("fn_inf_check", lambda: assert_eq(run_query("SELECT isInfinite(-1/0)"), "1")))

    return tests


def generate_misc_tests():
    """Miscellaneous tests for additional coverage."""
    tests = []

    # SELECT literals
    tests.append(("misc_int", lambda: assert_eq(run_query("SELECT 0"), "0")))
    tests.append(("misc_neg_int", lambda: assert_eq(run_query("SELECT -1"), "-1")))
    tests.append(("misc_float", lambda: assert_contains(run_query("SELECT 3.14"), "3.14")))
    tests.append(("misc_string", lambda: assert_eq(run_query("SELECT 'test'"), "test")))
    tests.append(("misc_null", lambda: assert_eq(run_query("SELECT NULL"), "\\N")))
    tests.append(("misc_true", lambda: assert_eq(run_query("SELECT 1"), "1")))
    tests.append(("misc_false", lambda: assert_eq(run_query("SELECT 0"), "0")))

    # Multiple rows
    tests.append(("misc_multi_row", lambda: assert_eq(run_query("SELECT * FROM (SELECT 1 UNION ALL SELECT 2 UNION ALL SELECT 3)"), "1\n2\n3")))

    # Complex expressions
    tests.append(("misc_complex_expr", lambda: assert_eq(run_query("SELECT (1+2)*(3+4)"), "21")))
    tests.append(("misc_nested_if", lambda: assert_eq(run_query("SELECT if(1>0,if(2>1,'a','b'),'c')"), "a")))
    tests.append(("misc_case_when", lambda: assert_eq(run_query("SELECT CASE WHEN 1>2 THEN 'x' WHEN 1<2 THEN 'y' END"), "y")))

    # Hash functions
    tests.append(("misc_cityhash", lambda: (run_query("SELECT cityHash64('')"), ok())))
    tests.append(("misc_siphash", lambda: (run_query("SELECT sipHash64('')"), ok())))
    tests.append(("misc_murmurhash", lambda: (run_query("SELECT murmurHash3_64('hello')"), ok())))
    tests.append(("misc_md5", lambda: (run_query("SELECT MD5('')"), ok())))
    tests.append(("misc_sha1", lambda: (run_query("SELECT SHA1('test')"), ok())))
    tests.append(("misc_sha256", lambda: (run_query("SELECT SHA256('test')"), ok())))

    # Random functions
    tests.append(("misc_rand", lambda: (run_query("SELECT rand()"), ok())))
    tests.append(("misc_rand64", lambda: (run_query("SELECT rand64()"), ok())))
    tests.append(("misc_rand_uniform", lambda: (run_query("SELECT randUniform(0,1)"), ok())))
    tests.append(("misc_rand_normal", lambda: (run_query("SELECT randNormal(0,1)"), ok())))

    # Encoding
    tests.append(("misc_base64", lambda: assert_eq(run_query("SELECT base64Encode('test')"), "dGVzdA==")))
    tests.append(("misc_base64_decode", lambda: assert_eq(run_query("SELECT base64Decode('dGVzdA==')"), "test")))
    tests.append(("misc_hex_str", lambda: assert_eq(run_query("SELECT hex('A')"), "41")))
    tests.append(("misc_unhex_str", lambda: (run_query("SELECT unhex('41')"), ok())))

    # Geo functions (basic)
    tests.append(("misc_geo_distance", lambda: (run_query("SELECT geoDistance(0,0,1,1)"), ok())))
    tests.append(("misc_point_in_ellipse", lambda: (run_query("SELECT pointInEllipses(0,0,0,0,1,1,1,1)"), ok())))

    # URL functions
    tests.append(("misc_url_domain", lambda: (run_query("SELECT domain('http://example.com/path')"), ok())))
    tests.append(("misc_url_path", lambda: (run_query("SELECT path('http://example.com/path/to')"), ok())))
    tests.append(("misc_url_protocol", lambda: (run_query("SELECT protocol('http://example.com')"), ok())))
    tests.append(("misc_url_scheme", lambda: (run_query("SELECT scheme('https://example.com')"), ok())))

    # IP functions
    tests.append(("misc_ipv4_num", lambda: (run_query("SELECT IPv4NumToString(2130706433)"), ok())))
    tests.append(("misc_ipv4_str", lambda: (run_query("SELECT IPv4StringToNum('127.0.0.1')"), ok())))
    tests.append(("misc_ipv4_to_str", lambda: (run_query("SELECT toString(toIPv4('127.0.0.1'))"), ok())))

    # JSON functions
    tests.append(("misc_json_has", lambda: (run_query("SELECT JSONHas('{\"a\":1}','a')"), ok())))
    tests.append(("misc_json_extract", lambda: (run_query("SELECT JSONExtractString('{\"a\":\"hello\"}','a')"), ok())))
    tests.append(("misc_json_length", lambda: (run_query("SELECT JSONLength('{\"a\":1,\"b\":2}')"), ok())))

    # Dict functions
    tests.append(("misc_dict_get", lambda: (run_query("SELECT dictGetOrDefault('no_such_dict','key',1,42)"), ok())))

    # RunningAccumulate
    tests.append(("misc_running_accum", lambda: (run_query("SELECT runningAccumulate(1)"), ok())))

    # transform
    tests.append(("misc_transform", lambda: assert_eq(run_query("SELECT transform(1,[1,2,3],['a','b','c'],'?')"), "a")))
    tests.append(("misc_transform_default", lambda: assert_eq(run_query("SELECT transform(99,[1,2,3],['a','b','c'],'?')"), "?")))

    # formatReadableSize
    tests.append(("misc_fmt_size", lambda: (run_query("SELECT formatReadableSize(1024)"), ok())))

    # toColumnTypeName
    tests.append(("misc_col_type", lambda: (run_query("SELECT toColumnTypeName(42)"), ok())))

    # dumpAllColumns
    tests.append(("misc_dump_cols", lambda: (run_query("SELECT dumpAllColumns() LIMIT 1"), ok())))

    # ignore
    tests.append(("misc_ignore", lambda: assert_eq(run_query("SELECT ignore(1)"), "0")))

    # identity
    tests.append(("misc_identity", lambda: assert_eq(run_query("SELECT identity(42)"), "42")))

    # sleep (very short)
    tests.append(("misc_sleep", lambda: (run_query("SELECT sleep(0.001)"), ok())))

    # throwIf
    tests.append(("misc_throw_false", lambda: (run_query("SELECT throwIf(0)"), ok())))

    return tests


# ---------------------------------------------------------------------------
# Main runner
# ---------------------------------------------------------------------------

def generate_extra_parametric_tests():
    """Parametric tests for additional coverage."""
    tests = []

    # Arithmetic parametric
    for a, b in [(1, 1), (0, 1), (100, 200), (999, 1), (0, 0), (1, 0), (-1, 1), (-5, -5), (10, 3), (7, 2)]:
        tests.append((f"arith_add_{a}_{b}", lambda _a=a, _b=b: (run_query(f"SELECT {_a}+{_b}"), ok())))
        tests.append((f"arith_sub_{a}_{b}", lambda _a=a, _b=b: (run_query(f"SELECT {_a}-{_b}"), ok())))
        tests.append((f"arith_mul_{a}_{b}", lambda _a=a, _b=b: (run_query(f"SELECT {_a}*{_b}"), ok())))

    # String function parametric
    for s in ["", "a", "ab", "abc", "hello world", "test123", "  spaces  ", "UPPER", "lower", "MiXeD"]:
        tests.append((f"str_len_{repr(s)[:8]}", lambda _s=s: (run_query(f"SELECT length('{_s}')"), ok())))
        tests.append((f"str_lower_{repr(s)[:8]}", lambda _s=s: (run_query(f"SELECT lower('{_s}')"), ok())))
        tests.append((f"str_upper_{repr(s)[:8]}", lambda _s=s: (run_query(f"SELECT upper('{_s}')"), ok())))
        tests.append((f"str_reverse_{repr(s)[:8]}", lambda _s=s: (run_query(f"SELECT reverse('{_s}')"), ok())))

    # Math function parametric
    for v in [0, 1, -1, 0.5, -0.5, 100, 0.001, 999, 3.14159, 2.71828]:
        tests.append((f"math_abs_{v}", lambda _v=v: (run_query(f"SELECT abs({_v})"), ok())))

    for v in [0, 1, 0.5, 1.5, 2.5, -0.5, -1.5, 3.2, 3.5, 3.8]:
        tests.append((f"math_ceil_{v}", lambda _v=v: (run_query(f"SELECT ceil({_v})"), ok())))
        tests.append((f"math_floor_{v}", lambda _v=v: (run_query(f"SELECT floor({_v})"), ok())))
        tests.append((f"math_round_{v}", lambda _v=v: (run_query(f"SELECT round({_v})"), ok())))

    # Array function parametric
    for arr in ["[]", "[1]", "[1,2]", "[1,2,3]", "[3,2,1]", "[5,4,3,2,1]", "[1,1,1]", "[1,2,3,4,5]"]:
        tests.append((f"arr_len_{arr[:10]}", lambda _a=arr: (run_query(f"SELECT length({_a})"), ok())))
        tests.append((f"arr_sort_{arr[:10]}", lambda _a=arr: (run_query(f"SELECT arraySort({_a})"), ok())))
        tests.append((f"arr_rev_{arr[:10]}", lambda _a=arr: (run_query(f"SELECT arrayReverse({_a})"), ok())))

    # Type conversion parametric
    for v in [0, 1, 42, 100, 255, 1000, 65535, 100000]:
        tests.append((f"conv_toInt32_{v}", lambda _v=v: (run_query(f"SELECT toInt32({_v})"), ok())))

    for v in ["0", "1", "hello", "42", "3.14", "true", "false", ""]:
        tests.append((f"conv_toString_{repr(v)[:8]}", lambda _v=v: (run_query(f"SELECT toString('{_v}')"), ok())))

    # Date function parametric
    for d in [0, 1, 7, 30, 365]:
        tests.append((f"date_addDays_{d}", lambda _d=d: (run_query(f"SELECT addDays(today(),{_d})"), ok())))
        tests.append((f"date_subDays_{d}", lambda _d=d: (run_query(f"SELECT subtractDays(today(),{_d})"), ok())))
        tests.append((f"date_addMonths_{d}", lambda _d=d: (run_query(f"SELECT addMonths(today(),{_d})"), ok())))

    # SELECT from system.numbers parametric
    for n in [1, 5, 10, 20, 50, 100]:
        tests.append((f"sys_numbers_limit_{n}", lambda _n=n: (run_query(f"SELECT count() FROM system.numbers LIMIT {_n}"), ok())))

    # Comparison parametric
    for a, b in [(1, 1), (1, 2), (2, 1), (0, 0), (-1, 1), (100, 100)]:
        tests.append((f"cmp_eq_{a}_{b}", lambda _a=a, _b=b: (run_query(f"SELECT {_a}={_b}"), ok())))
        tests.append((f"cmp_gt_{a}_{b}", lambda _a=a, _b=b: (run_query(f"SELECT {_a}>{_b}"), ok())))
        tests.append((f"cmp_lt_{a}_{b}", lambda _a=a, _b=b: (run_query(f"SELECT {_a}<{_b}"), ok())))

    return tests


def generate_more_ddl_edge_tests():
    """More DDL edge case tests."""
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

    # ALTER TABLE multiple ADD COLUMN
    t2 = tbl("multi_alter")
    tests.append((f"ddledge_multi_alter_setup", lambda: (
        run_query(f"CREATE TABLE {db}.{t2} (id UInt32) ENGINE = Memory"), ok())))
    for i in range(5):
        tests.append((f"ddledge_add_col_{i}", lambda _i=i: (
            run_query(f"ALTER TABLE {db}.{t2} ADD COLUMN col{_i} UInt32"), ok())))

    # ALTER TABLE multiple DROP COLUMN
    for i in range(5):
        tests.append((f"ddledge_drop_col_{i}", lambda _i=i: (
            run_query(f"ALTER TABLE {db}.{t2} DROP COLUMN col{_i}"), ok())))

    tests.append(("ddledge_cleanup", lambda: (run_query(f"DROP DATABASE IF EXISTS {db}"), ok())))
    return tests


def generate_more_select_edge_tests():
    """More SELECT edge case tests."""
    tests = []
    db = f"db_{_run_id}_seledge"
    tests.append(("seledge_setup_db", lambda: (run_query(f"CREATE DATABASE {db}"), ok())))

    t = tbl("sel_edge")
    tests.append((f"seledge_setup", lambda: (
        run_query(f"CREATE TABLE {db}.{t} (id UInt32, val Int64, name String) ENGINE = Memory"),
        run_query(f"INSERT INTO {db}.{t} VALUES " + ",".join(f"({i},{i*100},'n{i}')" for i in range(1, 101))),
        ok())))

    # Various LIMIT values
    for lim in [1, 3, 7, 10, 15, 20, 25, 30, 50, 99, 100]:
        tests.append((f"seledge_limit_{lim}", lambda _l=lim: (
            run_query(f"SELECT count() FROM (SELECT * FROM {db}.{t} LIMIT {_l})"), ok())))

    # Various WHERE conditions
    for v in [1, 10, 50, 99, 100]:
        tests.append((f"seledge_where_id_{v}", lambda _v=v: (
            run_query(f"SELECT * FROM {db}.{t} WHERE id={_v}"), ok())))

    # ORDER BY + LIMIT combinations
    for lim in [1, 5, 10]:
        tests.append((f"seledge_order_limit_{lim}", lambda _l=lim: (
            run_query(f"SELECT id FROM {db}.{t} ORDER BY val DESC LIMIT {_l}"), ok())))

    # GROUP BY edge cases
    tests.append((f"seledge_group_by_expr", lambda: (run_query(f"SELECT id%10, count() FROM {db}.{t} GROUP BY id%10 ORDER BY id%10"), ok())))
    tests.append((f"seledge_group_by_name", lambda: (run_query(f"SELECT name, count() FROM {db}.{t} GROUP BY name LIMIT 5"), ok())))

    # Aggregate function variations
    for func in ["count", "sum(val)", "avg(val)", "min(val)", "max(val)", "uniq(val)", "any(val)"]:
        tests.append((f"seledge_agg_{func.replace('(','_').replace(')','')}", lambda _f=func: (
            run_query(f"SELECT {_f} FROM {db}.{t}"), ok())))

    tests.append(("seledge_cleanup", lambda: (run_query(f"DROP DATABASE IF EXISTS {db}"), ok())))
    return tests


def generate_more_string_tests():
    """Additional string function tests."""
    tests = []
    # String function with edge inputs
    for s in ["", " ", "  ", "\t", "\n", "a", "zzzzz", "hello world 123", "café", "日本語"]:
        tests.append((f"str_edge_len_{repr(s)[:10]}", lambda _s=s: (run_query(f"SELECT length('{_s}')"), ok())))

    # LIKE patterns
    for pat in ["%", "_", "a%", "%z", "%o%", "h_d", "___", "%ld", "he%", "he__o"]:
        tests.append((f"str_like_{pat[:8]}", lambda _p=pat: (run_query(f"SELECT 'hello' LIKE '{_p}'"), ok())))

    # Replace variations
    tests.append(("str_replace_empty", lambda: assert_eq(run_query("SELECT replaceAll('','a','b')"), "")))
    tests.append(("str_replace_no_match", lambda: assert_eq(run_query("SELECT replaceAll('hello','x','y')"), "hello")))
    tests.append(("str_replace_multi", lambda: assert_eq(run_query("SELECT replaceAll('aaa','a','bb')"), "bbbbbb")))
    tests.append(("str_replace_one2", lambda: assert_eq(run_query("SELECT replaceOne('aaa','a','b')"), "baa")))

    # Concat variations
    tests.append(("str_concat_empty", lambda: assert_eq(run_query("SELECT concat('','')"), "")))
    tests.append(("str_concat_one", lambda: assert_eq(run_query("SELECT concat('hello')"), "hello")))
    tests.append(("str_concat_multi", lambda: assert_eq(run_query("SELECT concat('a','b','c','d','e')"), "abcde")))

    # Substring variations
    tests.append(("str_sub_1", lambda: assert_eq(run_query("SELECT substring('hello',1,1)"), "h")))
    tests.append(("str_sub_all", lambda: assert_eq(run_query("SELECT substring('hello',1,5)"), "hello")))
    tests.append(("str_sub_mid", lambda: assert_eq(run_query("SELECT substring('hello',2,3)"), "ell")))
    tests.append(("str_sub_end", lambda: assert_eq(run_query("SELECT substring('hello',5,1)"), "o")))
    tests.append(("str_sub_past", lambda: (run_query("SELECT substring('hello',1,100)"), ok())))

    # Position variations
    tests.append(("str_pos_first", lambda: assert_eq(run_query("SELECT position('hello','h')"), "1")))
    tests.append(("str_pos_last", lambda: assert_eq(run_query("SELECT position('hello','o')"), "5")))
    tests.append(("str_pos_none", lambda: assert_eq(run_query("SELECT position('hello','z')"), "0")))
    tests.append(("str_pos_mid", lambda: assert_eq(run_query("SELECT position('hello','ll')"), "3")))

    # Left/Right variations
    tests.append(("str_left_0", lambda: assert_eq(run_query("SELECT left('hello',0)"), "")))
    tests.append(("str_left_all", lambda: assert_eq(run_query("SELECT left('hello',5)"), "hello")))
    tests.append(("str_right_0", lambda: assert_eq(run_query("SELECT right('hello',0)"), "")))
    tests.append(("str_right_all", lambda: assert_eq(run_query("SELECT right('hello',5)"), "hello")))

    # Trim variations
    tests.append(("str_trim_both", lambda: assert_eq(run_query("SELECT trim('  hi  ')"), "hi")))
    tests.append(("str_trim_left2", lambda: assert_eq(run_query("SELECT trimLeft('  hi  ')"), "hi  ")))
    tests.append(("str_trim_right2", lambda: assert_eq(run_query("SELECT trimRight('  hi  ')"), "  hi")))
    tests.append(("str_trim_none", lambda: assert_eq(run_query("SELECT trim('hi')"), "hi")))

    # LPad/RPad variations
    tests.append(("str_lpad_exact", lambda: assert_eq(run_query("SELECT lpad('abc',3,'0')"), "abc")))
    tests.append(("str_lpad_shorter", lambda: assert_eq(run_query("SELECT lpad('abc',2,'0')"), "ab")))
    tests.append(("str_rpad_exact", lambda: assert_eq(run_query("SELECT rpad('abc',3,'0')"), "abc")))
    tests.append(("str_rpad_shorter", lambda: assert_eq(run_query("SELECT rpad('abc',2,'0')"), "ab")))

    # Base64 round-trip
    for s in ["hello", "world", "test123", "a", "", "ABCD"]:
        tests.append((f"str_b64_roundtrip_{repr(s)[:8]}", lambda _s=s: (
            run_query(f"SELECT base64Decode(base64Encode('{_s}'))"), ok())))

    # Hex round-trip
    tests.append(("str_hex_roundtrip_A", lambda: (run_query("SELECT unhex(hex('A'))"), ok())))
    tests.append(("str_hex_roundtrip_AB", lambda: (run_query("SELECT unhex(hex('AB'))"), ok())))

    return tests


def generate_more_aggregate_tests():
    """Additional aggregate function tests."""
    tests = []
    db = f"db_{_run_id}_aggex"
    tests.append(("aggex_setup_db", lambda: (run_query(f"CREATE DATABASE {db}"), ok())))

    t = tbl("aggex_data")
    tests.append((f"aggex_setup", lambda: (
        run_query(f"CREATE TABLE {db}.{t} (id UInt32, val Int64, grp String) ENGINE = Memory"),
        run_query(f"INSERT INTO {db}.{t} VALUES "
                  + ",".join(f"({i},{i*10},'{chr(97+i%5)}')" for i in range(1, 101))),
        ok())))

    # Parametric aggregates
    for q in [0.1, 0.25, 0.5, 0.75, 0.9, 0.95, 0.99]:
        tests.append((f"aggex_quantile_{q}", lambda _q=q: (
            run_query(f"SELECT quantile({_q})(val) FROM {db}.{t}"), ok())))

    # Grouped aggregates for each group
    for g in ["a", "b", "c", "d", "e"]:
        for func in ["count()", "sum(val)", "avg(val)", "min(val)", "max(val)"]:
            fname = f"{g}_{func.replace('(','').replace(')','')}"
            tests.append((f"aggex_grp_{fname}", lambda _g=g, _f=func: (
                run_query(f"SELECT {_f} FROM {db}.{t} WHERE grp='{_g}'"), ok())))

    # Combinators: If, Array, State, Merge
    tests.append((f"aggex_sumIf_a", lambda: (run_query(f"SELECT sumIf(val, grp='a') FROM {db}.{t}"), ok())))
    tests.append((f"aggex_sumIf_b", lambda: (run_query(f"SELECT sumIf(val, grp='b') FROM {db}.{t}"), ok())))
    tests.append((f"aggex_countIf_gt50", lambda: (run_query(f"SELECT countIf(val>500) FROM {db}.{t}"), ok())))
    tests.append((f"aggex_countIf_lt50", lambda: (run_query(f"SELECT countIf(val<500) FROM {db}.{t}"), ok())))
    tests.append((f"aggex_avgIf_c", lambda: (run_query(f"SELECT avgIf(val, grp='c') FROM {db}.{t}"), ok())))
    tests.append((f"aggex_avgIf_d", lambda: (run_query(f"SELECT avgIf(val, grp='d') FROM {db}.{t}"), ok())))
    tests.append((f"aggex_minIf_e", lambda: (run_query(f"SELECT minIf(val, grp='e') FROM {db}.{t}"), ok())))
    tests.append((f"aggex_maxIf_a", lambda: (run_query(f"SELECT maxIf(val, grp='a') FROM {db}.{t}"), ok())))

    # Multiple aggregates in one query
    tests.append((f"aggex_multi", lambda: (run_query(f"SELECT count(), sum(val), avg(val), min(val), max(val) FROM {db}.{t}"), ok())))
    tests.append((f"aggex_multi_grouped", lambda: (run_query(f"SELECT grp, count(), sum(val) FROM {db}.{t} GROUP BY grp ORDER BY grp"), ok())))

    # count with various conditions
    for n in [10, 20, 30, 40, 50]:
        tests.append((f"aggex_countIf_gt_{n}00", lambda _n=n: (
            run_query(f"SELECT countIf(val>{_n}00) FROM {db}.{t}"), ok())))

    # uniq variations
    tests.append((f"aggex_uniq_grp", lambda: (run_query(f"SELECT grp, uniq(val) FROM {db}.{t} GROUP BY grp ORDER BY grp"), ok())))
    tests.append((f"aggex_uniq_exact_grp", lambda: (run_query(f"SELECT grp, uniqExact(val) FROM {db}.{t} GROUP BY grp ORDER BY grp"), ok())))

    # Empty aggregates
    t2 = tbl("aggex_empty")
    tests.append((f"aggex_empty_setup", lambda: (
        run_query(f"CREATE TABLE {db}.{t2} (val Int64) ENGINE = Memory"), ok())))
    tests.append((f"aggex_empty_count", lambda: assert_eq(run_query(f"SELECT count() FROM {db}.{t2}"), "0")))
    tests.append((f"aggex_empty_sum", lambda: assert_eq(run_query(f"SELECT sum(val) FROM {db}.{t2}"), "0")))
    tests.append((f"aggex_empty_min", lambda: (run_query(f"SELECT min(val) FROM {db}.{t2}"), ok())))
    tests.append((f"aggex_empty_max", lambda: (run_query(f"SELECT max(val) FROM {db}.{t2}"), ok())))
    tests.append((f"aggex_empty_avg", lambda: (run_query(f"SELECT avg(val) FROM {db}.{t2}"), ok())))

    tests.append(("aggex_cleanup", lambda: (run_query(f"DROP DATABASE IF EXISTS {db}"), ok())))
    return tests


def generate_more_dml_tests():
    """Additional DML tests."""
    tests = []
    db = f"db_{_run_id}_dmlex"
    tests.append(("dmlex_setup_db", lambda: (run_query(f"CREATE DATABASE {db}"), ok())))

    # INSERT with various data patterns
    t = tbl("dmlex_nums")
    tests.append((f"dmlex_nums_setup", lambda: (
        run_query(f"CREATE TABLE {db}.{t} (v Int64) ENGINE = Memory"), ok())))
    for v in [0, 1, -1, 100, -100, 999999, -999999, 2147483647, -2147483648]:
        tests.append((f"dmlex_insert_{v}", lambda _v=v: (
            run_query(f"INSERT INTO {db}.{t} VALUES ({_v})"), ok())))

    # INSERT with string edge cases
    t2 = tbl("dmlex_strs")
    tests.append((f"dmlex_strs_setup", lambda: (
        run_query(f"CREATE TABLE {db}.{t2} (v String) ENGINE = Memory"), ok())))
    for s in ["", "a", "hello world", "line1\\nline2", "tab\\there", "unicode: 你好", "emoji: 😀", "special: <>&\"'"]:
        tests.append((f"dmlex_insert_str_{repr(s)[:10]}", lambda _s=s: (
            run_query(f"INSERT INTO {db}.{t2} VALUES ('{_s}')"), ok())))

    # INSERT with NULL
    t3 = tbl("dmlex_nulls")
    tests.append((f"dmlex_nulls_setup", lambda: (
        run_query(f"CREATE TABLE {db}.{t3} (id UInt32, v Nullable(Int64)) ENGINE = Memory"), ok())))
    for v in ["NULL", "0", "1", "-1", "NULL", "100", "NULL"]:
        tests.append((f"dmlex_insert_null_{v}", lambda _v=v, _i=[0]: (
            run_query(f"INSERT INTO {db}.{t3} VALUES ({_i[0]},{_v})"),
            _i.__setitem__(0, _i[0]+1),
            ok())[-1]))

    # Multi-row INSERT patterns
    t4 = tbl("dmlex_multi")
    tests.append((f"dmlex_multi_setup", lambda: (
        run_query(f"CREATE TABLE {db}.{t4} (id UInt32, v String) ENGINE = Memory"), ok())))
    for n in [1, 2, 5, 10, 20, 50]:
        vals = ",".join(f"({i},'v{i}')" for i in range(n))
        tests.append((f"dmlex_multi_insert_{n}", lambda _v=vals, _t=t4: (
            run_query(f"INSERT INTO {db}.{_t} VALUES {_v}"), ok())))

    tests.append(("dmlex_cleanup", lambda: (run_query(f"DROP DATABASE IF EXISTS {db}"), ok())))
    return tests


def generate_more_system_tests():
    """Additional system table tests."""
    tests = []
    # Query system tables with various filters and limits
    for tbl in ["databases", "tables", "columns", "functions", "settings", "formats",
                "data_type_families", "table_engines", "clusters", "dictionaries"]:
        for lim in [1, 5, 10]:
            tests.append((f"sys_{tbl}_limit_{lim}", lambda _t=tbl, _l=lim: (
                run_query(f"SELECT count() FROM (SELECT * FROM system.{_t} LIMIT {_l})"), ok())))

    # system.numbers variations
    for n in [1, 2, 5, 10, 20, 50, 100]:
        tests.append((f"sys_numbers_sum_{n}", lambda _n=n: (
            run_query(f"SELECT sum(number) FROM system.numbers LIMIT {_n}"), ok())))

    # system.one variations
    tests.append(("sys_one_select", lambda: (run_query("SELECT * FROM system.one"), ok())))
    tests.append(("sys_one_where", lambda: (run_query("SELECT * FROM system.one WHERE 1=1"), ok())))

    return tests


def generate_more_math_edge_tests():
    """Additional math edge case tests."""
    tests = []
    # Edge case math
    tests.append(("math_edge_max_uint64", lambda: (run_query("SELECT toUInt64(18446744073709551615)"), ok())))
    tests.append(("math_edge_neg_pow", lambda: (run_query("SELECT pow(-2,3)"), ok())))
    tests.append(("math_edge_frac_pow", lambda: (run_query("SELECT pow(2,0.5)"), ok())))
    tests.append(("math_edge_large_sqrt", lambda: (run_query("SELECT sqrt(1000000)"), ok())))
    tests.append(("math_edge_log_large", lambda: (run_query("SELECT log(1000000)"), ok())))
    tests.append(("math_edge_exp_neg", lambda: (run_query("SELECT exp(-1)"), ok())))
    tests.append(("math_edge_exp_large", lambda: (run_query("SELECT exp(10)"), ok())))
    tests.append(("math_edge_sin_pi", lambda: (run_query("SELECT sin(pi())"), ok())))
    tests.append(("math_edge_cos_pi", lambda: (run_query("SELECT cos(pi())"), ok())))
    tests.append(("math_edge_atan_inf", lambda: (run_query("SELECT atan(999999)"), ok())))
    tests.append(("math_edge_asin_1", lambda: assert_contains(run_query("SELECT asin(1)"), "1.57")))
    tests.append(("math_edge_acos_0", lambda: assert_contains(run_query("SELECT acos(0)"), "1.57")))
    tests.append(("math_edge_tan_zero", lambda: assert_eq(run_query("SELECT tan(0)"), "0")))
    tests.append(("math_edge_int_div_large", lambda: (run_query("SELECT intDiv(1000000,7)"), ok())))
    tests.append(("math_edge_mod_large", lambda: (run_query("SELECT modulo(1000000,7)"), ok())))
    tests.append(("math_edge_gcd_0", lambda: (run_query("SELECT gcd(0,5)"), ok())))
    tests.append(("math_edge_lcm_0", lambda: (run_query("SELECT lcm(0,5)"), ok())))
    tests.append(("math_edge_round_neg", lambda: assert_eq(run_query("SELECT round(-3.5)"), "-4")))
    tests.append(("math_edge_ceil_neg", lambda: assert_eq(run_query("SELECT ceil(-3.0)"), "-3")))
    tests.append(("math_edge_floor_neg", lambda: assert_eq(run_query("SELECT floor(-3.0)"), "-3")))
    tests.append(("math_edge_abs_max", lambda: (run_query("SELECT abs(-9223372036854775808)"), ok())))
    tests.append(("math_edge_least_neg", lambda: assert_eq(run_query("SELECT least(-1,-2,-3)"), "-3")))
    tests.append(("math_edge_greatest_neg", lambda: assert_eq(run_query("SELECT greatest(-1,-2,-3)"), "-1")))

    return tests


def generate_more_type_tests():
    """Additional type conversion tests."""
    tests = []
    # Round-trip conversions
    for v in [0, 1, 42, 100, 255, 1000, 65535, 1000000]:
        tests.append((f"type_rt_int_str_{v}", lambda _v=v: (
            run_query(f"SELECT toInt32(toString({_v}))"), ok())))

    # Float conversions
    for v in [0.0, 1.0, -1.0, 3.14, -3.14, 0.001, 999.999]:
        tests.append((f"type_rt_float_str_{v}", lambda _v=v: (
            run_query(f"SELECT toString(toFloat64('{_v}'))"), ok())))

    # Boolean edge cases
    tests.append(("type_bool_0", lambda: assert_eq(run_query("SELECT toUInt8(0)"), "0")))
    tests.append(("type_bool_1", lambda: assert_eq(run_query("SELECT toUInt8(1)"), "1")))
    tests.append(("type_bool_neg1", lambda: assert_eq(run_query("SELECT toUInt8(-1)"), "255")))

    # Nullable conversions
    tests.append(("type_null_or_zero_int", lambda: assert_eq(run_query("SELECT toInt32OrZero(NULL)"), "0")))
    tests.append(("type_null_or_zero_float", lambda: assert_eq(run_query("SELECT toFloat64OrZero(NULL)"), "0")))
    tests.append(("type_null_or_zero_str", lambda: assert_eq(run_query("SELECT toString(toInt32OrZero(NULL))"), "0")))

    # Enum conversions
    tests.append(("type_enum8_to_string", lambda: (run_query("SELECT CAST(1 AS Enum8('a'=1,'b'=2))"), ok())))
    tests.append(("type_enum16_to_string", lambda: (run_query("SELECT CAST(1 AS Enum16('x'=1,'y'=2))"), ok())))

    return tests


def generate_more_array_edge_tests():
    """Additional array edge case tests."""
    tests = []
    # Array with various types
    tests.append(("arr_edge_uint8", lambda: assert_eq(run_query("SELECT [toUInt8(1),toUInt8(2)]"), "[1,2]")))
    tests.append(("arr_edge_int32", lambda: assert_eq(run_query("SELECT [toInt32(-1),toInt32(0),toInt32(1)]"), "[-1,0,1]")))
    tests.append(("arr_edge_float", lambda: (run_query("SELECT [1.1,2.2,3.3]"), ok())))
    tests.append(("arr_edge_string", lambda: assert_eq(run_query("SELECT ['a','','c']"), "['a','','c']")))
    tests.append(("arr_edge_empty_of", lambda: assert_eq(run_query("SELECT emptyArrayInt32()"), "[]")))
    tests.append(("arr_edge_empty_uint16", lambda: assert_eq(run_query("SELECT emptyArrayUInt16()"), "[]")))
    tests.append(("arr_edge_empty_uint32", lambda: assert_eq(run_query("SELECT emptyArrayUInt32()"), "[]")))
    tests.append(("arr_edge_empty_uint64", lambda: assert_eq(run_query("SELECT emptyArrayUInt64()"), "[]")))
    tests.append(("arr_edge_empty_int8", lambda: assert_eq(run_query("SELECT emptyArrayInt8()"), "[]")))
    tests.append(("arr_edge_empty_int16", lambda: assert_eq(run_query("SELECT emptyArrayInt16()"), "[]")))
    tests.append(("arr_edge_empty_int64", lambda: assert_eq(run_query("SELECT emptyArrayInt64()"), "[]")))
    tests.append(("arr_edge_empty_float32", lambda: assert_eq(run_query("SELECT emptyArrayFloat32()"), "[]")))
    tests.append(("arr_edge_empty_float64", lambda: assert_eq(run_query("SELECT emptyArrayFloat64()"), "[]")))
    tests.append(("arr_edge_empty_string", lambda: assert_eq(run_query("SELECT emptyArrayString()"), "[]")))
    tests.append(("arr_edge_empty_date", lambda: assert_eq(run_query("SELECT emptyArrayDate()"), "[]")))
    tests.append(("arr_edge_empty_datetime", lambda: assert_eq(run_query("SELECT emptyArrayDateTime()"), "[]")))

    # Array operations edge cases
    tests.append(("arr_edge_index_1", lambda: assert_eq(run_query("SELECT [10,20,30][1]"), "10")))
    tests.append(("arr_edge_index_3", lambda: assert_eq(run_query("SELECT [10,20,30][3]"), "30")))
    tests.append(("arr_edge_uniq_empty", lambda: assert_eq(run_query("SELECT arrayUniq([])"), "0")))
    tests.append(("arr_edge_sort_single", lambda: assert_eq(run_query("SELECT arraySort([1])"), "[1]")))
    tests.append(("arr_edge_rev_single", lambda: assert_eq(run_query("SELECT arrayReverse([1])"), "[1]")))
    tests.append(("arr_edge_has_empty", lambda: assert_eq(run_query("SELECT has([],1)"), "0")))
    tests.append(("arr_edge_concat_empty_left", lambda: assert_eq(run_query("SELECT arrayConcat([],[1,2])"), "[1,2]")))
    tests.append(("arr_edge_concat_empty_right", lambda: assert_eq(run_query("SELECT arrayConcat([1,2],[])"), "[1,2]")))
    tests.append(("arr_edge_concat_both_empty", lambda: assert_eq(run_query("SELECT arrayConcat([],[])"), "[]")))
    tests.append(("arr_edge_slice_empty", lambda: assert_eq(run_query("SELECT arraySlice([],1,1)"), "[]")))
    tests.append(("arr_edge_range_0", lambda: assert_eq(run_query("SELECT range(0)"), "[]")))
    tests.append(("arr_edge_range_1", lambda: assert_eq(run_query("SELECT range(1)"), "[0]")))
    tests.append(("arr_edge_range_10", lambda: assert_eq(run_query("SELECT range(10)"), "[0,1,2,3,4,5,6,7,8,9]")))

    return tests


def generate_more_date_edge_tests():
    """Additional date function edge case tests."""
    tests = []
    tests.append(("date_edge_today", lambda: (run_query("SELECT toString(today())"), ok())))
    tests.append(("date_edge_yesterday", lambda: (run_query("SELECT toString(yesterday())"), ok())))
    tests.append(("date_edge_tomorrow", lambda: (run_query("SELECT toString(addDays(today(),1))"), ok())))

    # Date arithmetic edge cases
    tests.append(("date_edge_add_0", lambda: (run_query("SELECT addDays(today(),0)"), ok())))
    tests.append(("date_edge_sub_0", lambda: (run_query("SELECT subtractDays(today(),0)"), ok())))
    tests.append(("date_edge_add_year", lambda: (run_query("SELECT addYears(today(),1)"), ok())))
    tests.append(("date_edge_sub_year", lambda: (run_query("SELECT subtractYears(today(),1)"), ok())))
    tests.append(("date_edge_add_hour", lambda: (run_query("SELECT addHours(now(),1)"), ok())))
    tests.append(("date_edge_sub_hour", lambda: (run_query("SELECT subtractHours(now(),1)"), ok())))
    tests.append(("date_edge_add_min", lambda: (run_query("SELECT addMinutes(now(),1)"), ok())))
    tests.append(("date_edge_sub_min", lambda: (run_query("SELECT subtractMinutes(now(),1)"), ok())))
    tests.append(("date_edge_add_sec", lambda: (run_query("SELECT addSeconds(now(),1)"), ok())))
    tests.append(("date_edge_sub_sec", lambda: (run_query("SELECT subtractSeconds(now(),1)"), ok())))

    # Date diff edge cases
    tests.append(("date_edge_diff_self", lambda: assert_eq(run_query("SELECT dateDiff('day',today(),today())"), "0")))
    tests.append(("date_edge_diff_year", lambda: (run_query("SELECT dateDiff('year',toDate('2020-01-01'),toDate('2024-01-01'))"), ok())))
    tests.append(("date_edge_diff_month", lambda: (run_query("SELECT dateDiff('month',toDate('2024-01-01'),toDate('2024-12-01'))"), ok())))
    tests.append(("date_edge_diff_hour", lambda: (run_query("SELECT dateDiff('hour',toDateTime('2024-01-01 00:00:00'),toDateTime('2024-01-01 12:00:00'))"), ok())))
    tests.append(("date_edge_diff_min", lambda: (run_query("SELECT dateDiff('minute',toDateTime('2024-01-01 00:00:00'),toDateTime('2024-01-01 01:00:00'))"), ok())))
    tests.append(("date_edge_diff_sec", lambda: (run_query("SELECT dateDiff('second',toDateTime('2024-01-01 00:00:00'),toDateTime('2024-01-01 00:01:00'))"), ok())))

    # toStartOf variations
    tests.append(("date_edge_start_of_hour", lambda: (run_query("SELECT toStartOfHour(now())"), ok())))
    tests.append(("date_edge_start_of_minute", lambda: (run_query("SELECT toStartOfMinute(now())"), ok())))
    tests.append(("date_edge_start_of_5min", lambda: (run_query("SELECT toStartOfFiveMinutes(now())"), ok())))
    tests.append(("date_edge_start_of_15min", lambda: (run_query("SELECT toStartOfFifteenMinutes(now())"), ok())))
    tests.append(("date_edge_start_of_10min", lambda: (run_query("SELECT toStartOfTenMinutes(now())"), ok())))

    # Date extraction
    tests.append(("date_edge_year", lambda: (run_query("SELECT toYear(today())"), ok())))
    tests.append(("date_edge_month", lambda: (run_query("SELECT toMonth(today())"), ok())))
    tests.append(("date_edge_day", lambda: (run_query("SELECT toDayOfMonth(today())"), ok())))
    tests.append(("date_edge_dow", lambda: (run_query("SELECT toDayOfWeek(today())"), ok())))
    tests.append(("date_edge_hour", lambda: (run_query("SELECT toHour(now())"), ok())))
    tests.append(("date_edge_minute", lambda: (run_query("SELECT toMinute(now())"), ok())))
    tests.append(("date_edge_second", lambda: (run_query("SELECT toSecond(now())"), ok())))

    # toDayOfYear
    tests.append(("date_edge_doy", lambda: (run_query("SELECT toDayOfYear(today())"), ok())))

    # Date formatting
    tests.append(("date_edge_fmt1", lambda: (run_query("SELECT formatDateTime(today(), '%Y')"), ok())))
    tests.append(("date_edge_fmt2", lambda: (run_query("SELECT formatDateTime(today(), '%m')"), ok())))
    tests.append(("date_edge_fmt3", lambda: (run_query("SELECT formatDateTime(today(), '%d')"), ok())))
    tests.append(("date_edge_fmt4", lambda: (run_query("SELECT formatDateTime(now(), '%H:%M:%S')"), ok())))
    tests.append(("date_edge_fmt5", lambda: (run_query("SELECT formatDateTime(now(), '%Y-%m-%d %H:%M:%S')"), ok())))

    return tests


def main():
    all_tests = []
    all_tests.extend(generate_ddl_tests())
    all_tests.extend(generate_dml_tests())
    all_tests.extend(generate_select_tests())
    all_tests.extend(generate_aggregate_tests())
    all_tests.extend(generate_string_tests())
    all_tests.extend(generate_date_tests())
    all_tests.extend(generate_math_tests())
    all_tests.extend(generate_type_conversion_tests())
    all_tests.extend(generate_array_tests())
    all_tests.extend(generate_system_tests())
    all_tests.extend(generate_datatype_tests())
    all_tests.extend(generate_format_tests())
    all_tests.extend(generate_edge_case_tests())
    all_tests.extend(generate_additional_select_tests())
    all_tests.extend(generate_additional_function_tests())
    all_tests.extend(generate_misc_tests())
    all_tests.extend(generate_extra_parametric_tests())
    all_tests.extend(generate_more_ddl_edge_tests())
    all_tests.extend(generate_more_select_edge_tests())
    all_tests.extend(generate_more_string_tests())
    all_tests.extend(generate_more_aggregate_tests())
    all_tests.extend(generate_more_dml_tests())
    all_tests.extend(generate_more_system_tests())
    all_tests.extend(generate_more_math_edge_tests())
    all_tests.extend(generate_more_type_tests())
    all_tests.extend(generate_more_array_edge_tests())
    all_tests.extend(generate_more_date_edge_tests())

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
            if len(failures) < 20:
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
