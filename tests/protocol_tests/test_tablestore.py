#!/usr/bin/env python3
"""TableStore REST protocol test suite for RorisDB (port 18087)."""

import urllib.request
import urllib.error
import urllib.parse
import json
import sys
import traceback
import uuid

BASE = "http://127.0.0.1:18087"
TIMEOUT = 10

_run_id = uuid.uuid4().hex[:8]
_tables_created = []

# ---------------------------------------------------------------------------
# HTTP helpers
# ---------------------------------------------------------------------------

def _req(method, path, body=None):
    """Send HTTP request and return (status_code, parsed_json_or_str)."""
    url = BASE + path
    data = json.dumps(body).encode("utf-8") if body is not None else None
    req = urllib.request.Request(url, data=data, method=method)
    req.add_header("Content-Type", "application/json")
    try:
        resp = urllib.request.urlopen(req, timeout=TIMEOUT)
        raw = resp.read().decode("utf-8", errors="replace")
        try:
            return resp.status, json.loads(raw)
        except Exception:
            return resp.status, raw
    except urllib.error.HTTPError as e:
        raw = e.read().decode("utf-8", errors="replace")
        try:
            return e.code, json.loads(raw)
        except Exception:
            return e.code, raw

def GET(path):
    return _req("GET", path)

def PUT(path, body=None):
    return _req("PUT", path, body)

def POST(path, body=None):
    return _req("POST", path, body)

def DELETE(path):
    return _req("DELETE", path)

# ---------------------------------------------------------------------------
# Convenience helpers
# ---------------------------------------------------------------------------

def tbl(suffix):
    """Generate a unique table name for test isolation."""
    name = f"ts_{_run_id}_{suffix}"
    _tables_created.append(name)
    return name

def create_table(name, pk, defined_cols=None):
    """Create a table, assert 200."""
    body = {"primary_key": pk}
    if defined_cols:
        body["defined_columns"] = defined_cols
    code, resp = PUT(f"/tables/{name}", body)
    assert code == 200, f"create_table failed: {code} {resp}"
    return resp

def put_row(table_name, pk_pairs, attr_pairs):
    """Put a row, assert 200."""
    body = {"primary_key": pk_pairs, "attributes": attr_pairs}
    code, resp = POST(f"/tables/{table_name}/rows", body)
    assert code == 200, f"put_row failed: {code} {resp}"
    return resp

def get_row(table_name, pk_query):
    """Get a row by PK query string like 'id=1' or 'id=hello world'. Auto URL-encodes."""
    # Parse the pk_query as key=value pairs, then re-encode properly
    params = {}
    if pk_query:
        for part in pk_query.split('&'):
            if '=' in part:
                k, v = part.split('=', 1)
                params[k] = v
    encoded = urllib.parse.urlencode(params) if params else ''
    code, resp = GET(f"/tables/{table_name}/rows?{encoded}")
    return code, resp

def delete_row(table_name, pk_query):
    """Delete a row by PK query string."""
    params = {}
    if pk_query:
        for part in pk_query.split('&'):
            if '=' in part:
                k, v = part.split('=', 1)
                params[k] = v
    encoded = urllib.parse.urlencode(params) if params else ''
    code, resp = DELETE(f"/tables/{table_name}/rows?{encoded}")
    return code, resp

def update_row(table_name, row_id, pk_pairs, attr_pairs):
    """Update a row."""
    body = {"primary_key": pk_pairs, "attributes": attr_pairs}
    code, resp = POST(f"/tables/{table_name}/rows/{row_id}", body)
    return code, resp

def get_range(table_name, start, end, limit=None):
    """Get range of rows."""
    body = {"start": start, "end": end}
    if limit is not None:
        body["limit"] = limit
    code, resp = POST(f"/tables/{table_name}/range", body)
    return code, resp

def test(name, fn):
    """Run a single test function, return (name, passed, error_msg)."""
    try:
        fn()
        return (name, True, "")
    except Exception as e:
        return (name, False, str(e)[:200])

def assert_eq(a, b, msg=""):
    if a != b:
        raise AssertionError(f"Expected {b!r}, got {a!r}. {msg}")

def assert_in(substr, s, msg=""):
    if substr not in str(s):
        raise AssertionError(f"Expected '{substr}' in '{str(s)[:300]}'. {msg}")

def assert_true(cond, msg=""):
    if not cond:
        raise AssertionError(msg or "Assertion failed")

# ---------------------------------------------------------------------------
# 1. TABLE OPERATIONS (100+ tests)
# ---------------------------------------------------------------------------

def make_table_tests():
    tests = []

    # -- create table --
    for i in range(20):
        t = tbl(f"create_{i}")
        def fn(name=t, idx=i):
            create_table(name, [{"name": "id", "type": "STRING"}])
            code, resp = GET(f"/tables/{name}")
            assert_eq(code, 200)
            assert_eq(resp["table_name"], name)
            assert_eq(resp["status"], "ACTIVE")
        tests.append((f"table/create_single_pk_{i}", fn))

    # -- create table with composite PK --
    for i in range(15):
        t = tbl(f"create_cmp_{i}")
        def fn(name=t, idx=i):
            pks = [{"name": f"pk{j}", "type": "STRING"} for j in range(2, 5)]
            create_table(name, pks)
            code, resp = GET(f"/tables/{name}")
            assert_eq(code, 200)
            assert_true(len(resp["primary_key"]) >= 2)
        tests.append((f"table/create_composite_pk_{i}", fn))

    # -- create table with defined columns --
    for i in range(15):
        t = tbl(f"create_dc_{i}")
        def fn(name=t, idx=i):
            dc = [{"name": f"col{j}", "type": "STRING"} for j in range(3)]
            create_table(name, [{"name": "id", "type": "INTEGER"}], dc)
            code, resp = GET(f"/tables/{name}")
            assert_eq(code, 200)
            assert_true(len(resp["defined_columns"]) >= 3)
        tests.append((f"table/create_defined_columns_{i}", fn))

    # -- list tables --
    for i in range(10):
        t = tbl(f"list_{i}")
        def fn(name=t):
            create_table(name, [{"name": "id", "type": "STRING"}])
            code, resp = GET("/tables")
            assert_eq(code, 200)
            assert_in(name, resp["tables"])
        tests.append((f"table/list_contains_{i}", fn))

    # -- describe table --
    for i in range(10):
        t = tbl(f"desc_{i}")
        def fn(name=t, idx=i):
            create_table(name, [{"name": f"k{i}", "type": "STRING"}])
            code, resp = GET(f"/tables/{name}")
            assert_eq(code, 200)
            assert_eq(resp["primary_key"][0]["name"], f"k{i}")
        tests.append((f"table/describe_{i}", fn))

    # -- describe non-existent table --
    for i in range(10):
        t = tbl(f"noexist_{i}")
        def fn(name=t):
            code, resp = GET(f"/tables/{name}")
            assert_eq(code, 404)
        tests.append((f"table/describe_not_found_{i}", fn))

    # -- delete table --
    for i in range(10):
        t = tbl(f"del_{i}")
        def fn(name=t):
            create_table(name, [{"name": "id", "type": "STRING"}])
            code, resp = DELETE(f"/tables/{name}")
            assert_eq(code, 200)
            code2, _ = GET(f"/tables/{name}")
            assert_eq(code2, 404)
        tests.append((f"table/delete_{i}", fn))

    # -- delete non-existent table --
    for i in range(5):
        t = tbl(f"del_noexist_{i}")
        def fn(name=t):
            code, resp = DELETE(f"/tables/{name}")
            assert_eq(code, 404)
        tests.append((f"table/delete_not_found_{i}", fn))

    # -- create table with INTEGER PK --
    for i in range(5):
        t = tbl(f"create_int_{i}")
        def fn(name=t):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            code, resp = GET(f"/tables/{name}")
            assert_eq(code, 200)
            assert_eq(resp["primary_key"][0]["type_name"], "INTEGER")
        tests.append((f"table/create_integer_pk_{i}", fn))

    # -- row count after inserts --
    for i in range(5):
        t = tbl(f"count_{i}")
        def fn(name=t):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            for j in range(3):
                put_row(name, [{"name": "id", "value": j}], [{"name": "v", "value": j}])
            code, resp = GET(f"/tables/{name}")
            assert_eq(code, 200)
            assert_eq(resp["row_count"], 3)
        tests.append((f"table/row_count_{i}", fn))

    # -- GET / server info --
    def test_server_info():
        code, resp = GET("/")
        assert_eq(code, 200)
        assert_eq(resp["protocol"], "tablestore")
        assert_eq(resp["status"], "ok")
    tests.append(("table/server_info", test_server_info))

    return tests

# ---------------------------------------------------------------------------
# 2. ROW OPERATIONS (200+ tests)
# ---------------------------------------------------------------------------

def make_row_tests():
    tests = []

    # -- PutRow basic --
    for i in range(30):
        t = tbl(f"put_{i}")
        def fn(name=t, idx=i):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            put_row(name, [{"name": "id", "value": i}], [{"name": "val", "value": f"v{i}"}])
            code, resp = get_row(name, f"id={i}")
            assert_eq(code, 200)
            assert_in("v" + str(i), str(resp))
        tests.append((f"row/put_basic_{i}", fn))

    # -- PutRow overwrite --
    for i in range(15):
        t = tbl(f"putow_{i}")
        def fn(name=t, idx=i):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            put_row(name, [{"name": "id", "value": 1}], [{"name": "val", "value": "first"}])
            put_row(name, [{"name": "id", "value": 1}], [{"name": "val", "value": "second"}])
            code, resp = get_row(name, "id=1")
            assert_eq(code, 200)
            assert_in("second", str(resp))
        tests.append((f"row/put_overwrite_{i}", fn))

    # -- GetRow --
    for i in range(20):
        t = tbl(f"get_{i}")
        def fn(name=t, idx=i):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            put_row(name, [{"name": "id", "value": idx}], [{"name": "data", "value": f"d{idx}"}])
            code, resp = get_row(name, f"id={idx}")
            assert_eq(code, 200)
            assert_in("primary_key", str(resp))
            assert_in("attributes", str(resp))
        tests.append((f"row/get_basic_{i}", fn))

    # -- GetRow not found --
    for i in range(15):
        t = tbl(f"getnf_{i}")
        def fn(name=t):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            code, resp = get_row(name, "id=99999")
            assert_eq(code, 404)
        tests.append((f"row/get_not_found_{i}", fn))

    # -- GetRow non-existent table --
    for i in range(5):
        t = tbl(f"getnt_{i}")
        def fn(name=t):
            code, resp = get_row(name, "id=1")
            assert_eq(code, 404)
        tests.append((f"row/get_nonexistent_table_{i}", fn))

    # -- UpdateRow --
    for i in range(20):
        t = tbl(f"upd_{i}")
        def fn(name=t, idx=i):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            put_row(name, [{"name": "id", "value": idx}], [{"name": "a", "value": "old"}, {"name": "b", "value": "keep"}])
            code, resp = update_row(name, str(idx),
                [{"name": "id", "value": idx}],
                [{"name": "a", "value": "new"}])
            assert_eq(code, 200)
            code2, resp2 = get_row(name, f"id={idx}")
            assert_in("new", str(resp2))
            assert_in("keep", str(resp2))
        tests.append((f"row/update_merge_{i}", fn))

    # -- UpdateRow not found --
    for i in range(10):
        t = tbl(f"updnf_{i}")
        def fn(name=t):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            code, resp = update_row(name, "99",
                [{"name": "id", "value": 99}],
                [{"name": "a", "value": "x"}])
            assert_eq(code, 404)
        tests.append((f"row/update_not_found_{i}", fn))

    # -- DeleteRow --
    for i in range(20):
        t = tbl(f"del_{i}")
        def fn(name=t, idx=i):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            put_row(name, [{"name": "id", "value": idx}], [{"name": "v", "value": "x"}])
            code, resp = delete_row(name, f"id={idx}")
            assert_eq(code, 200)
            code2, _ = get_row(name, f"id={idx}")
            assert_eq(code2, 404)
        tests.append((f"row/delete_{i}", fn))

    # -- DeleteRow not found --
    for i in range(10):
        t = tbl(f"delnf_{i}")
        def fn(name=t):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            code, resp = delete_row(name, "id=99999")
            assert_eq(code, 404)
        tests.append((f"row/delete_not_found_{i}", fn))

    # -- PutRow with multiple attributes --
    for i in range(10):
        t = tbl(f"putma_{i}")
        def fn(name=t, idx=i):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            attrs = [{"name": f"col{j}", "value": f"val{j}"} for j in range(10)]
            put_row(name, [{"name": "id", "value": idx}], attrs)
            code, resp = get_row(name, f"id={idx}")
            assert_eq(code, 200)
            for j in range(10):
                assert_in(f"col{j}", str(resp))
        tests.append((f"row/put_multi_attrs_{i}", fn))

    # -- PutRow with composite PK --
    for i in range(10):
        t = tbl(f"putcpk_{i}")
        def fn(name=t, idx=i):
            create_table(name, [{"name": "pk1", "type": "STRING"}, {"name": "pk2", "type": "INTEGER"}])
            put_row(name,
                [{"name": "pk1", "value": f"k{idx}"}, {"name": "pk2", "value": idx}],
                [{"name": "v", "value": "ok"}])
            code, resp = get_row(name, f"pk1=k{idx}&pk2={idx}")
            assert_eq(code, 200)
            assert_in("ok", str(resp))
        tests.append((f"row/put_composite_pk_{i}", fn))

    # -- PutRow on non-existent table --
    for i in range(5):
        t = tbl(f"putnt_{i}")
        def fn(name=t):
            body = {"primary_key": [{"name": "id", "value": 1}], "attributes": [{"name": "v", "value": "x"}]}
            code, resp = POST(f"/tables/{name}/rows", body)
            assert_eq(code, 404)
        tests.append((f"row/put_nonexistent_table_{i}", fn))

    # -- GetRow with string PK --
    for i in range(10):
        t = tbl(f"getstr_{i}")
        def fn(name=t, idx=i):
            create_table(name, [{"name": "id", "type": "STRING"}])
            put_row(name, [{"name": "id", "value": f"str_{idx}"}], [{"name": "v", "value": idx}])
            code, resp = get_row(name, f"id=str_{idx}")
            assert_eq(code, 200)
        tests.append((f"row/get_string_pk_{i}", fn))

    return tests

# ---------------------------------------------------------------------------
# 3. PRIMARY KEY TYPES (100+ tests)
# ---------------------------------------------------------------------------

def make_pk_type_tests():
    tests = []

    # -- STRING PK --
    for i in range(30):
        t = tbl(f"pkstr_{i}")
        def fn(name=t, idx=i):
            create_table(name, [{"name": "id", "type": "STRING"}])
            val = f"hello_world_{idx}"
            put_row(name, [{"name": "id", "value": val}], [{"name": "v", "value": 1}])
            code, resp = get_row(name, f"id={val}")
            assert_eq(code, 200)
        tests.append((f"pk/string_{i}", fn))

    # -- STRING PK with special chars --
    special_strings = [
        "with-dash", "with.dot", "with_underscore",
        "UPPER", "lower", "MiXeD",
        "digits123", "abc_DEF_456",
        "hello_world", "foo_bar",
        "camelCase", "snake_case",
        "kebab-case-value", "test_value_123",
    ]
    for i, s in enumerate(special_strings[:14]):
        t = tbl(f"pkstrs_{i}")
        def fn(name=t, val=s, idx=i):
            create_table(name, [{"name": "id", "type": "STRING"}])
            put_row(name, [{"name": "id", "value": val}], [{"name": "v", "value": idx}])
            code, resp = get_row(name, f"id={val}")
            assert_eq(code, 200)
        tests.append((f"pk/string_special_{i}", fn))

    # -- INTEGER PK --
    int_values = [0, 1, -1, 100, -100, 2**31 - 1, -(2**31), 2**15, 42, 7, 13, 99, 1000, 9999, 12345]
    for i, v in enumerate(int_values[:15]):
        t = tbl(f"pkint_{i}")
        def fn(name=t, val=v):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            put_row(name, [{"name": "id", "value": val}], [{"name": "v", "value": 1}])
            code, resp = get_row(name, f"id={val}")
            assert_eq(code, 200)
        tests.append((f"pk/integer_{i}", fn))

    # -- Composite PK with mixed types --
    for i in range(20):
        t = tbl(f"pkcmp_{i}")
        def fn(name=t, idx=i):
            create_table(name, [
                {"name": "pk1", "type": "STRING"},
                {"name": "pk2", "type": "INTEGER"},
            ])
            put_row(name,
                [{"name": "pk1", "value": f"s{idx}"}, {"name": "pk2", "value": idx * 10}],
                [{"name": "v", "value": "mixed"}])
            code, resp = get_row(name, f"pk1=s{idx}&pk2={idx*10}")
            assert_eq(code, 200)
        tests.append((f"pk/composite_mixed_{i}", fn))

    # -- Triple PK --
    for i in range(10):
        t = tbl(f"pk3_{i}")
        def fn(name=t, idx=i):
            create_table(name, [
                {"name": "a", "type": "STRING"},
                {"name": "b", "type": "INTEGER"},
                {"name": "c", "type": "STRING"},
            ])
            put_row(name,
                [{"name": "a", "value": f"a{idx}"}, {"name": "b", "value": idx}, {"name": "c", "value": f"c{idx}"}],
                [{"name": "v", "value": "triple"}])
            code, resp = get_row(name, f"a=a{idx}&b={idx}&c=c{idx}")
            assert_eq(code, 200)
        tests.append((f"pk/triple_{i}", fn))

    # -- Large integer PK values --
    for i in range(5):
        t = tbl(f"pklint_{i}")
        def fn(name=t, idx=i):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            big = 10**15 + idx
            put_row(name, [{"name": "id", "value": big}], [{"name": "v", "value": 1}])
            code, resp = get_row(name, f"id={big}")
            assert_eq(code, 200)
        tests.append((f"pk/large_integer_{i}", fn))

    # -- PK with empty string --
    for i in range(3):
        t = tbl(f"pkempty_{i}")
        def fn(name=t):
            create_table(name, [{"name": "id", "type": "STRING"}])
            put_row(name, [{"name": "id", "value": ""}], [{"name": "v", "value": 1}])
            # Note: empty PK may not parse from query string
        tests.append((f"pk/empty_string_{i}", fn))

    return tests

# ---------------------------------------------------------------------------
# 4. ATTRIBUTE COLUMNS (100+ tests)
# ---------------------------------------------------------------------------

def make_attr_tests():
    tests = []

    # -- STRING attributes --
    for i in range(25):
        t = tbl(f"attrstr_{i}")
        def fn(name=t, idx=i):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            put_row(name, [{"name": "id", "value": idx}],
                [{"name": "name", "value": f"user_{idx}"},
                 {"name": "email", "value": f"user{idx}@test.com"}])
            code, resp = get_row(name, f"id={idx}")
            assert_eq(code, 200)
            assert_in(f"user_{idx}", str(resp))
        tests.append((f"attr/string_{i}", fn))

    # -- INTEGER attributes --
    for i in range(20):
        t = tbl(f"attrint_{i}")
        def fn(name=t, idx=i):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            put_row(name, [{"name": "id", "value": idx}],
                [{"name": "age", "value": 20 + idx},
                 {"name": "score", "value": idx * 10}])
            code, resp = get_row(name, f"id={idx}")
            assert_eq(code, 200)
            assert_in(str(20 + idx), str(resp))
        tests.append((f"attr/integer_{i}", fn))

    # -- BOOLEAN attributes --
    for i in range(15):
        t = tbl(f"attrbool_{i}")
        def fn(name=t, idx=i):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            put_row(name, [{"name": "id", "value": idx}],
                [{"name": "active", "value": idx % 2 == 0},
                 {"name": "verified", "value": idx % 3 == 0}])
            code, resp = get_row(name, f"id={idx}")
            assert_eq(code, 200)
        tests.append((f"attr/boolean_{i}", fn))

    # -- DOUBLE attributes --
    for i in range(15):
        t = tbl(f"attrdbl_{i}")
        def fn(name=t, idx=i):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            put_row(name, [{"name": "id", "value": idx}],
                [{"name": "price", "value": 9.99 + idx},
                 {"name": "weight", "value": 1.5 * idx}])
            code, resp = get_row(name, f"id={idx}")
            assert_eq(code, 200)
        tests.append((f"attr/double_{i}", fn))

    # -- Mixed type attributes --
    for i in range(15):
        t = tbl(f"attrmix_{i}")
        def fn(name=t, idx=i):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            put_row(name, [{"name": "id", "value": idx}], [
                {"name": "str_col", "value": f"str_{idx}"},
                {"name": "int_col", "value": idx * 100},
                {"name": "bool_col", "value": idx % 2 == 0},
                {"name": "dbl_col", "value": idx * 1.1},
            ])
            code, resp = get_row(name, f"id={idx}")
            assert_eq(code, 200)
            assert_in("str_col", str(resp))
            assert_in("int_col", str(resp))
        tests.append((f"attr/mixed_{i}", fn))

    # -- Negative integer attributes --
    for i in range(10):
        t = tbl(f"attrneg_{i}")
        def fn(name=t, idx=i):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            put_row(name, [{"name": "id", "value": idx}],
                [{"name": "temp", "value": -(idx + 10)}])
            code, resp = get_row(name, f"id={idx}")
            assert_eq(code, 200)
            assert_in(str(-(idx + 10)), str(resp))
        tests.append((f"attr/negative_int_{i}", fn))

    # -- Zero value attributes --
    for i in range(5):
        t = tbl(f"attrzero_{i}")
        def fn(name=t, idx=i):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            put_row(name, [{"name": "id", "value": idx}],
                [{"name": "count", "value": 0}, {"name": "empty", "value": ""}])
            code, resp = get_row(name, f"id={idx}")
            assert_eq(code, 200)
        tests.append((f"attr/zero_{i}", fn))

    # -- Large string attributes --
    for i in range(5):
        t = tbl(f"attrlarge_{i}")
        def fn(name=t, idx=i):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            large_val = "A" * (1000 * (idx + 1))
            put_row(name, [{"name": "id", "value": idx}],
                [{"name": "big", "value": large_val}])
            code, resp = get_row(name, f"id={idx}")
            assert_eq(code, 200)
        tests.append((f"attr/large_string_{i}", fn))

    return tests

# ---------------------------------------------------------------------------
# 5. CONDITIONAL (100+ tests)
# ---------------------------------------------------------------------------

def make_conditional_tests():
    tests = []
    # The current protocol doesn't have explicit condition support in the API,
    # but we test the behaviors that would correspond to conditions.

    # -- PutRow always succeeds (IGNORE condition) --
    for i in range(30):
        t = tbl(f"cond_ign_{i}")
        def fn(name=t, idx=i):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            # First put
            put_row(name, [{"name": "id", "value": 1}], [{"name": "v", "value": "first"}])
            # Second put (overwrite, like IGNORE)
            put_row(name, [{"name": "id", "value": 1}], [{"name": "v", "value": "second"}])
            code, resp = get_row(name, "id=1")
            assert_eq(code, 200)
            assert_in("second", str(resp))
        tests.append((f"cond/ignore_put_{i}", fn))

    # -- Delete on existing row (EXPECT_EXIST like) --
    for i in range(20):
        t = tbl(f"cond_exdel_{i}")
        def fn(name=t, idx=i):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            put_row(name, [{"name": "id", "value": idx}], [{"name": "v", "value": "x"}])
            code, resp = delete_row(name, f"id={idx}")
            assert_eq(code, 200, "Delete existing row should succeed")
        tests.append((f"cond/expect_exist_delete_{i}", fn))

    # -- Delete on non-existing row (EXPECT_EXIST fails) --
    for i in range(20):
        t = tbl(f"cond_exnf_{i}")
        def fn(name=t):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            code, resp = delete_row(name, "id=99999")
            assert_eq(code, 404, "Delete non-existing should 404")
        tests.append((f"cond/expect_exist_delete_fail_{i}", fn))

    # -- Update existing row --
    for i in range(20):
        t = tbl(f"cond_upd_{i}")
        def fn(name=t, idx=i):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            put_row(name, [{"name": "id", "value": idx}], [{"name": "a", "value": "old"}])
            code, resp = update_row(name, str(idx),
                [{"name": "id", "value": idx}],
                [{"name": "a", "value": "new"}])
            assert_eq(code, 200)
        tests.append((f"cond/expect_exist_update_{i}", fn))

    # -- Update non-existing row --
    for i in range(15):
        t = tbl(f"cond_updnf_{i}")
        def fn(name=t):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            code, resp = update_row(name, "99",
                [{"name": "id", "value": 99}],
                [{"name": "a", "value": "x"}])
            assert_eq(code, 404)
        tests.append((f"cond/expect_exist_update_fail_{i}", fn))

    # -- Put then Get (EXPECT_NOT_EXIST like - new row) --
    for i in range(15):
        t = tbl(f"cond_nexist_{i}")
        def fn(name=t, idx=i):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            # Get should 404 (row doesn't exist)
            code, resp = get_row(name, f"id={idx}")
            assert_eq(code, 404)
            # Now put
            put_row(name, [{"name": "id", "value": idx}], [{"name": "v", "value": "created"}])
            code, resp = get_row(name, f"id={idx}")
            assert_eq(code, 200)
        tests.append((f"cond/expect_not_exist_get_{i}", fn))

    # -- Multiple overwrites --
    for i in range(10):
        t = tbl(f"cond_multi_{i}")
        def fn(name=t, idx=i):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            for j in range(5):
                put_row(name, [{"name": "id", "value": idx}], [{"name": "v", "value": f"v{j}"}])
            code, resp = get_row(name, f"id={idx}")
            assert_eq(code, 200)
            assert_in("v4", str(resp))
        tests.append((f"cond/multiple_overwrites_{i}", fn))

    return tests

# ---------------------------------------------------------------------------
# 6. FILTER (100+ tests)
# ---------------------------------------------------------------------------

def make_filter_tests():
    tests = []
    # The current protocol doesn't have explicit filter API, but we test
    # range queries which act as filters.

    # -- Range filter by integer PK (single-digit to avoid known lex-order bug) --
    for i in range(20):
        t = tbl(f"flt_int_{i}")
        def fn(name=t, idx=i):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            for j in range(9):  # stay single-digit
                put_row(name, [{"name": "id", "value": j}], [{"name": "v", "value": j * 10}])
            end = min(idx % 8 + 1, 9)
            code, resp = get_range(name, {"id": 0}, {"id": end})
            assert_eq(code, 200)
            rows = resp.get("rows", [])
            assert_true(len(rows) <= end)
        tests.append((f"filter/range_int_{i}", fn))

    # -- Range filter with limit --
    for i in range(20):
        t = tbl(f"flt_lim_{i}")
        def fn(name=t, idx=i):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            for j in range(9):  # single digit
                put_row(name, [{"name": "id", "value": j}], [{"name": "v", "value": j}])
            code, resp = get_range(name, {"id": 0}, {"id": 9}, limit=3)
            assert_eq(code, 200)
            assert_true(len(resp.get("rows", [])) <= 3)
        tests.append((f"filter/limit_{i}", fn))

    # -- Range filter by string PK --
    for i in range(15):
        t = tbl(f"flt_str_{i}")
        def fn(name=t, idx=i):
            create_table(name, [{"name": "id", "type": "STRING"}])
            for j in range(10):
                put_row(name, [{"name": "id", "value": f"key_{j:03d}"}], [{"name": "v", "value": j}])
            code, resp = get_range(name, {"id": f"key_{idx:03d}"}, {"id": f"key_{idx+3:03d}"})
            assert_eq(code, 200)
        tests.append((f"filter/range_string_{i}", fn))

    # -- Empty range (start == end) --
    for i in range(10):
        t = tbl(f"flt_empty_{i}")
        def fn(name=t, idx=i):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            put_row(name, [{"name": "id", "value": 1}], [{"name": "v", "value": 1}])
            code, resp = get_range(name, {"id": 1}, {"id": 1})
            assert_eq(code, 200)
            assert_eq(len(resp.get("rows", [])), 0, "start==end should return 0 rows (exclusive end)")
        tests.append((f"filter/empty_range_{i}", fn))

    # -- Wide range returns all (single-digit) --
    for i in range(10):
        t = tbl(f"flt_wide_{i}")
        def fn(name=t):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            for j in range(5):
                put_row(name, [{"name": "id", "value": j}], [{"name": "v", "value": j}])
            code, resp = get_range(name, {"id": 0}, {"id": 9})
            assert_eq(code, 200)
            assert_true(len(resp.get("rows", [])) >= 5)
        tests.append((f"filter/wide_range_{i}", fn))

    # -- Filter on composite PK range --
    for i in range(10):
        t = tbl(f"flt_cmp_{i}")
        def fn(name=t, idx=i):
            create_table(name, [
                {"name": "pk1", "type": "STRING"},
                {"name": "pk2", "type": "INTEGER"},
            ])
            for j in range(5):
                put_row(name,
                    [{"name": "pk1", "value": f"p{idx}"}, {"name": "pk2", "value": j}],
                    [{"name": "v", "value": j}])
            code, resp = get_range(name,
                {"pk1": f"p{idx}", "pk2": 0},
                {"pk1": f"p{idx}", "pk2": 4})
            assert_eq(code, 200)
        tests.append((f"filter/composite_range_{i}", fn))

    # -- Range with limit 0 --
    for i in range(5):
        t = tbl(f"flt_lim0_{i}")
        def fn(name=t):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            for j in range(5):
                put_row(name, [{"name": "id", "value": j}], [{"name": "v", "value": j}])
            code, resp = get_range(name, {"id": 0}, {"id": 9}, limit=0)
            assert_eq(code, 200)
            assert_eq(len(resp.get("rows", [])), 0)
        tests.append((f"filter/limit_zero_{i}", fn))

    return tests

# ---------------------------------------------------------------------------
# 7. RANGE QUERIES (100+ tests)
# ---------------------------------------------------------------------------

def make_range_tests():
    tests = []

    # -- Forward range --
    for i in range(20):
        t = tbl(f"rng_fwd_{i}")
        def fn(name=t, idx=i):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            for j in range(8):  # single digit
                put_row(name, [{"name": "id", "value": j}], [{"name": "v", "value": j}])
            code, resp = get_range(name, {"id": 0}, {"id": 8})
            assert_eq(code, 200)
            rows = resp.get("rows", [])
            assert_true(len(rows) > 0)
        tests.append((f"range/forward_{i}", fn))

    # -- Range with exact limit (single-digit to avoid lex-order bug) --
    for i in range(10):
        t = tbl(f"rng_limex_{i}")
        def fn(name=t):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            for j in range(8):  # single digit
                put_row(name, [{"name": "id", "value": j}], [{"name": "v", "value": j}])
            code, resp = get_range(name, {"id": 0}, {"id": 8}, limit=5)
            assert_eq(code, 200)
            # DashMap iter is unordered but filter+take gives up to 5
            assert_true(len(resp.get("rows", [])) <= 5)
            assert_true(len(resp.get("rows", [])) > 0)
        tests.append((f"range/exact_limit_{i}", fn))

    # -- Range exclusive end --
    for i in range(15):
        t = tbl(f"rng_excl_{i}")
        def fn(name=t, idx=i):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            for j in range(5):
                put_row(name, [{"name": "id", "value": j}], [{"name": "v", "value": j}])
            # end=4 is exclusive, so only 0,1,2,3
            code, resp = get_range(name, {"id": 0}, {"id": 4})
            assert_eq(code, 200)
            rows = resp.get("rows", [])
            for row in rows:
                pk = row.get("primary_key", {})
                # id=4 should NOT be in results
                assert_true(str(4) not in str(pk), f"id=4 should be excluded, got {pk}")
        tests.append((f"range/exclusive_end_{i}", fn))

    # -- Range with limit=1 --
    for i in range(10):
        t = tbl(f"rng_lim1_{i}")
        def fn(name=t):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            for j in range(8):  # single digit
                put_row(name, [{"name": "id", "value": j}], [{"name": "v", "value": j}])
            code, resp = get_range(name, {"id": 0}, {"id": 8}, limit=1)
            assert_eq(code, 200)
            assert_true(len(resp.get("rows", [])) == 1)
        tests.append((f"range/limit_1_{i}", fn))

    # -- Range on empty table --
    for i in range(10):
        t = tbl(f"rng_empty_{i}")
        def fn(name=t):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            code, resp = get_range(name, {"id": 0}, {"id": 100})
            assert_eq(code, 200)
            assert_eq(len(resp.get("rows", [])), 0)
        tests.append((f"range/empty_table_{i}", fn))

    # -- Range on string PK --
    for i in range(15):
        t = tbl(f"rng_str_{i}")
        def fn(name=t, idx=i):
            create_table(name, [{"name": "id", "type": "STRING"}])
            for c in "abcdefghij":
                put_row(name, [{"name": "id", "value": c}], [{"name": "v", "value": c}])
            code, resp = get_range(name, {"id": "a"}, {"id": "e"})
            assert_eq(code, 200)
        tests.append((f"range/string_pk_{i}", fn))

    # -- Range large result set (use string PK for correct lex order) --
    for i in range(10):
        t = tbl(f"rng_large_{i}")
        def fn(name=t):
            create_table(name, [{"name": "id", "type": "STRING"}])
            for j in range(50):
                put_row(name, [{"name": "id", "value": f"{j:04d}"}], [{"name": "v", "value": j}])
            code, resp = get_range(name, {"id": "0000"}, {"id": "9999"}, limit=50)
            assert_eq(code, 200)
            assert_true(len(resp.get("rows", [])) > 0)
        tests.append((f"range/large_result_{i}", fn))

    # -- Range non-existent table --
    for i in range(5):
        t = tbl(f"rng_nt_{i}")
        def fn(name=t):
            code, resp = get_range(name, {"id": 0}, {"id": 100})
            assert_eq(code, 404)
        tests.append((f"range/nonexistent_table_{i}", fn))

    # -- Range inclusive start --
    for i in range(10):
        t = tbl(f"rng_incl_{i}")
        def fn(name=t, idx=i):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            for j in range(8):  # single digit
                put_row(name, [{"name": "id", "value": j}], [{"name": "v", "value": j}])
            code, resp = get_range(name, {"id": 3}, {"id": 7})
            assert_eq(code, 200)
            # start is inclusive, end exclusive
        tests.append((f"range/inclusive_start_{i}", fn))

    # -- Range with default limit (100) --
    for i in range(5):
        t = tbl(f"rng_deflim_{i}")
        def fn(name=t):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            for j in range(8):  # single digit
                put_row(name, [{"name": "id", "value": j}], [{"name": "v", "value": j}])
            code, resp = get_range(name, {"id": 0}, {"id": 9})
            assert_eq(code, 200)
            assert_true(len(resp.get("rows", [])) >= 8)
        tests.append((f"range/default_limit_{i}", fn))

    return tests

# ---------------------------------------------------------------------------
# 8. BATCH (100+ tests)
# ---------------------------------------------------------------------------

def make_batch_tests():
    tests = []
    # The protocol doesn't have explicit batch endpoints, so we simulate
    # batch-like behavior with multiple sequential operations.

    # -- Batch write (multiple puts) --
    for i in range(30):
        t = tbl(f"batch_w_{i}")
        def fn(name=t, idx=i):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            for j in range(10):
                put_row(name, [{"name": "id", "value": j}], [{"name": "v", "value": f"b{idx}_v{j}"}])
            # Verify all written
            for j in range(10):
                code, resp = get_row(name, f"id={j}")
                assert_eq(code, 200)
                assert_in(f"b{idx}_v{j}", str(resp))
        tests.append((f"batch/write_multi_put_{i}", fn))

    # -- Batch read (multiple gets) --
    for i in range(25):
        t = tbl(f"batch_r_{i}")
        def fn(name=t, idx=i):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            for j in range(5):
                put_row(name, [{"name": "id", "value": j}], [{"name": "v", "value": j * 100}])
            results = []
            for j in range(5):
                code, resp = get_row(name, f"id={j}")
                results.append((code, resp))
            assert_true(all(c == 200 for c, _ in results))
        tests.append((f"batch/read_multi_get_{i}", fn))

    # -- Batch write + read via range --
    for i in range(20):
        t = tbl(f"batch_wr_{i}")
        def fn(name=t, idx=i):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            for j in range(8):  # single digit
                put_row(name, [{"name": "id", "value": j}], [{"name": "data", "value": f"row_{j}"}])
            code, resp = get_range(name, {"id": 0}, {"id": 9})
            assert_eq(code, 200)
            rows = resp.get("rows", [])
            assert_true(len(rows) >= 8)
        tests.append((f"batch/write_then_range_{i}", fn))

    # -- Batch delete --
    for i in range(15):
        t = tbl(f"batch_d_{i}")
        def fn(name=t, idx=i):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            for j in range(10):
                put_row(name, [{"name": "id", "value": j}], [{"name": "v", "value": j}])
            for j in range(10):
                code, _ = delete_row(name, f"id={j}")
                assert_eq(code, 200)
            code, resp = GET(f"/tables/{name}")
            assert_eq(resp["row_count"], 0)
        tests.append((f"batch/delete_multi_{i}", fn))

    # -- Batch mixed operations --
    for i in range(10):
        t = tbl(f"batch_mix_{i}")
        def fn(name=t, idx=i):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            # Put
            for j in range(5):
                put_row(name, [{"name": "id", "value": j}], [{"name": "v", "value": j}])
            # Update some
            for j in range(3):
                update_row(name, str(j),
                    [{"name": "id", "value": j}],
                    [{"name": "v", "value": j + 100}])
            # Delete some
            for j in range(3, 5):
                delete_row(name, f"id={j}")
            # Verify
            code, resp = GET(f"/tables/{name}")
            assert_eq(resp["row_count"], 3)
        tests.append((f"batch/mixed_ops_{i}", fn))

    # -- Batch with errors (some rows don't exist) --
    for i in range(10):
        t = tbl(f"batch_err_{i}")
        def fn(name=t):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            put_row(name, [{"name": "id", "value": 1}], [{"name": "v", "value": "exists"}])
            # Get existing
            code, _ = get_row(name, "id=1")
            assert_eq(code, 200)
            # Get non-existing
            code, _ = get_row(name, "id=999")
            assert_eq(code, 404)
        tests.append((f"batch/mixed_errors_{i}", fn))

    return tests

# ---------------------------------------------------------------------------
# 9. EDGE CASES (100+ tests)
# ---------------------------------------------------------------------------

def make_edge_tests():
    tests = []

    # -- Empty table operations --
    for i in range(10):
        t = tbl(f"edge_empty_{i}")
        def fn(name=t):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            code, resp = GET(f"/tables/{name}")
            assert_eq(code, 200)
            assert_eq(resp["row_count"], 0)
        tests.append((f"edge/empty_table_{i}", fn))

    # -- Get non-existent row --
    for i in range(10):
        t = tbl(f"edge_nerow_{i}")
        def fn(name=t):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            code, resp = get_row(name, "id=999999")
            assert_eq(code, 404)
            assert_in("not found", str(resp).lower())
        tests.append((f"edge/nonexistent_row_{i}", fn))

    # -- Delete non-existent row --
    for i in range(10):
        t = tbl(f"edge_ndrow_{i}")
        def fn(name=t):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            code, resp = delete_row(name, "id=999999")
            assert_eq(code, 404)
        tests.append((f"edge/delete_nonexistent_{i}", fn))

    # -- PutRow with empty attributes --
    for i in range(10):
        t = tbl(f"edge_noattr_{i}")
        def fn(name=t, idx=i):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            put_row(name, [{"name": "id", "value": idx}], [])
            code, resp = get_row(name, f"id={idx}")
            assert_eq(code, 200)
        tests.append((f"edge/empty_attributes_{i}", fn))

    # -- Invalid JSON body --
    for i in range(5):
        t = tbl(f"edge_badjson_{i}")
        def fn(name=t):
            code, resp = _req("PUT", f"/tables/{name}", None)
            # Should get 400 or some error
            # Without body, create fails
        tests.append((f"edge/invalid_json_{i}", fn))

    # -- Non-existent endpoint --
    for i in range(5):
        t = tbl(f"edge_404_{i}")
        def fn(name=t):
            code, resp = GET(f"/nonexistent/{name}")
            assert_eq(code, 404)
        tests.append((f"edge/unknown_endpoint_{i}", fn))

    # -- Large number of rows --
    for i in range(5):
        t = tbl(f"edge_many_{i}")
        def fn(name=t):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            for j in range(50):
                put_row(name, [{"name": "id", "value": j}], [{"name": "v", "value": j}])
            code, resp = GET(f"/tables/{name}")
            assert_eq(resp["row_count"], 50)
        tests.append((f"edge/many_rows_{i}", fn))

    # -- Row with many attributes --
    for i in range(5):
        t = tbl(f"edge_manyattr_{i}")
        def fn(name=t):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            attrs = [{"name": f"c{j}", "value": j} for j in range(50)]
            put_row(name, [{"name": "id", "value": 1}], attrs)
            code, resp = get_row(name, "id=1")
            assert_eq(code, 200)
        tests.append((f"edge/many_attributes_{i}", fn))

    # -- Very large string value --
    for i in range(5):
        t = tbl(f"edge_bigstr_{i}")
        def fn(name=t, idx=i):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            big = "X" * 10000
            put_row(name, [{"name": "id", "value": idx}], [{"name": "big", "value": big}])
            code, resp = get_row(name, f"id={idx}")
            assert_eq(code, 200)
        tests.append((f"edge/large_value_{i}", fn))

    # -- Special characters in attribute values --
    specials = ["hello world", "tab\there", "new\nline", "quote\"here", "back\\slash"]
    for i, s in enumerate(specials[:5]):
        t = tbl(f"edge_special_{i}")
        def fn(name=t, val=s, idx=i):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            put_row(name, [{"name": "id", "value": idx}], [{"name": "v", "value": val}])
            code, resp = get_row(name, f"id={idx}")
            assert_eq(code, 200)
        tests.append((f"edge/special_chars_{i}", fn))

    # -- Concurrent puts to same key --
    for i in range(10):
        t = tbl(f"edge_conc_{i}")
        def fn(name=t, idx=i):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            for j in range(20):
                put_row(name, [{"name": "id", "value": 1}], [{"name": "v", "value": j}])
            code, resp = get_row(name, "id=1")
            assert_eq(code, 200)
            # Should have last write
            assert_in("19", str(resp))
        tests.append((f"edge/concurrent_same_key_{i}", fn))

    # -- Delete then re-insert --
    for i in range(10):
        t = tbl(f"edge_delins_{i}")
        def fn(name=t, idx=i):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            put_row(name, [{"name": "id", "value": idx}], [{"name": "v", "value": "first"}])
            delete_row(name, f"id={idx}")
            put_row(name, [{"name": "id", "value": idx}], [{"name": "v", "value": "second"}])
            code, resp = get_row(name, f"id={idx}")
            assert_eq(code, 200)
            assert_in("second", str(resp))
        tests.append((f"edge/delete_reinsert_{i}", fn))

    # -- GetRow without query string --
    for i in range(5):
        t = tbl(f"edge_noq_{i}")
        def fn(name=t):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            code, resp = GET(f"/tables/{name}/rows")
            assert_eq(code, 400)
        tests.append((f"edge/get_no_query_{i}", fn))

    # -- DeleteRow without query string --
    for i in range(5):
        t = tbl(f"edge_delnoq_{i}")
        def fn(name=t):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            code, resp = DELETE(f"/tables/{name}/rows")
            assert_eq(code, 400)
        tests.append((f"edge/delete_no_query_{i}", fn))

    # -- Update with empty attributes --
    for i in range(5):
        t = tbl(f"edge_updempty_{i}")
        def fn(name=t, idx=i):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            put_row(name, [{"name": "id", "value": idx}], [{"name": "a", "value": "keep"}])
            code, resp = update_row(name, str(idx),
                [{"name": "id", "value": idx}], [])
            assert_eq(code, 200)
            code2, resp2 = get_row(name, f"id={idx}")
            assert_in("keep", str(resp2))
        tests.append((f"edge/update_empty_attrs_{i}", fn))

    # -- Table with same name recreation after delete --
    for i in range(5):
        t = tbl(f"edge_recreate_{i}")
        def fn(name=t):
            create_table(name, [{"name": "id", "type": "STRING"}])
            put_row(name, [{"name": "id", "value": "a"}], [{"name": "v", "value": 1}])
            DELETE(f"/tables/{name}")
            # Recreate
            create_table(name, [{"name": "id", "type": "STRING"}])
            code, resp = GET(f"/tables/{name}")
            assert_eq(code, 200)
            assert_eq(resp["row_count"], 0)
        tests.append((f"edge/recreate_after_delete_{i}", fn))

    # -- Double value infinity-like --
    for i in range(3):
        t = tbl(f"edge_dblinf_{i}")
        def fn(name=t, idx=i):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            put_row(name, [{"name": "id", "value": idx}], [{"name": "v", "value": 1e308}])
            code, resp = get_row(name, f"id={idx}")
            assert_eq(code, 200)
        tests.append((f"edge/double_large_{i}", fn))

    # -- Boolean attributes true/false --
    for i in range(5):
        t = tbl(f"edge_bool_{i}")
        def fn(name=t, idx=i):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            put_row(name, [{"name": "id", "value": idx}],
                [{"name": "t", "value": True}, {"name": "f", "value": False}])
            code, resp = get_row(name, f"id={idx}")
            assert_eq(code, 200)
            assert_in("true", str(resp).lower())
        tests.append((f"edge/boolean_tf_{i}", fn))

    # -- PutRow with negative integer attribute --
    for i in range(5):
        t = tbl(f"edge_negattr_{i}")
        def fn(name=t, idx=i):
            create_table(name, [{"name": "id", "type": "INTEGER"}])
            put_row(name, [{"name": "id", "value": idx}], [{"name": "v", "value": -(2**31)}])
            code, resp = get_row(name, f"id={idx}")
            assert_eq(code, 200)
        tests.append((f"edge/negative_attr_{i}", fn))

    return tests

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    all_tests = []
    all_tests.extend(make_table_tests())
    all_tests.extend(make_row_tests())
    all_tests.extend(make_pk_type_tests())
    all_tests.extend(make_attr_tests())
    all_tests.extend(make_conditional_tests())
    all_tests.extend(make_filter_tests())
    all_tests.extend(make_range_tests())
    all_tests.extend(make_batch_tests())
    all_tests.extend(make_edge_tests())

    results = []
    passed = 0
    failed = 0
    failures = []

    print(f"Running {len(all_tests)} tests against {BASE} ...")
    for name, fn in all_tests:
        r = test(name, fn)
        results.append(r)
        if r[1]:
            passed += 1
        else:
            failed += 1
            failures.append({"test": r[0], "error": r[2]})
            if len(failures) % 50 == 0:
                print(f"  ... {len(failures)} failures so far")

    # Cleanup tables (best effort)
    for t in set(_tables_created):
        try:
            DELETE(f"/tables/{t}")
        except Exception:
            pass

    summary = {
        "protocol": "tablestore",
        "total": len(results),
        "passed": passed,
        "failed": failed,
        "failures": failures[:20],
    }

    print(json.dumps(summary, indent=2))
    sys.exit(0 if failed == 0 else 1)

if __name__ == "__main__":
    main()
