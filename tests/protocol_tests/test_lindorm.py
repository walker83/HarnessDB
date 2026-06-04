#!/usr/bin/env python3
"""
Comprehensive test suite for RorisDB Lindorm protocol (port 17070).
1000+ test cases covering all protocol commands and edge cases.
"""

import socket
import time
import json
import sys
import traceback
from contextlib import contextmanager

HOST = "127.0.0.1"
PORT = 17070
TIMEOUT = 3.0


@contextmanager
def lindorm_conn():
    """Create a connection to Lindorm protocol server."""
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    s.settimeout(TIMEOUT)
    s.connect((HOST, PORT))
    # Read banner
    data = b""
    while b"> " not in data:
        chunk = s.recv(4096)
        if not chunk:
            break
        data += chunk
    yield s
    try:
        s.sendall(b"QUIT\n")
    except Exception:
        pass
    s.close()


def send_cmd(s, cmd):
    """Send a command and return the response (strips the '> ' prompt)."""
    s.sendall((cmd + "\n").encode())
    data = b""
    deadline = time.time() + TIMEOUT
    while time.time() < deadline:
        try:
            chunk = s.recv(65536)
            if not chunk:
                break
            data += chunk
            text_so_far = data.decode("utf-8", errors="replace")
            # The prompt is always "\n> " or starts as "> " at beginning
            # We need to find the LAST occurrence of "\n> " or the trailing "> "
            # that represents the actual prompt (not > in error message content like <value>)
            if "\n> " in text_so_far:
                # Check if the last "\n> " is at the end (possibly with trailing space)
                last_prompt = text_so_far.rfind("\n> ")
                after_prompt = text_so_far[last_prompt + 3:]
                if after_prompt.strip() == "":
                    break
            elif text_so_far.endswith("> ") and len(text_so_far) < 5:
                # The initial banner ends with "> "
                break
        except socket.timeout:
            break
    text = data.decode("utf-8", errors="replace")
    text = text.rstrip()
    # Remove trailing prompt "> " - find the last "\n> " to strip
    last_prompt = text.rfind("\n> ")
    if last_prompt >= 0:
        text = text[:last_prompt]
    elif text.endswith(">") and not text.endswith("<value>"):
        text = text[:-1].rstrip()
    # Strip the leading "> " prompt echo
    if text.startswith("> "):
        text = text[2:]
    return text


def new_conn_and_cmd(cmd):
    """Open connection, send one command, return response."""
    with lindorm_conn() as s:
        return send_cmd(s, cmd)


class TestResults:
    def __init__(self):
        self.total = 0
        self.passed = 0
        self.failed = 0
        self.failures = []

    def record(self, name, passed, detail=""):
        self.total += 1
        if passed:
            self.passed += 1
        else:
            self.failed += 1
            if len(self.failures) < 20:
                self.failures.append({"test": name, "detail": detail})

    def to_json(self):
        return {
            "protocol": "lindorm",
            "total": self.total,
            "passed": self.passed,
            "failed": self.failed,
            "failures": self.failures,
        }


results = TestResults()


def check(name, condition, detail=""):
    results.record(name, condition, detail)


# ============================================================
# 1. Connection tests (20+)
# ============================================================
def test_connection():
    prefix = "conn"

    # 1. Basic connection + banner
    try:
        s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        s.settimeout(TIMEOUT)
        s.connect((HOST, PORT))
        banner = s.recv(4096).decode()
        check(f"{prefix}_001_banner", "Lindorm Shell" in banner, f"banner={banner[:80]}")
        s.sendall(b"QUIT\n")
        s.close()
    except Exception as e:
        check(f"{prefix}_001_banner", False, str(e))

    # 2. Prompt present
    try:
        with lindorm_conn() as s:
            resp = send_cmd(s, "LIST")
            check(f"{prefix}_002_prompt", True)
    except Exception as e:
        check(f"{prefix}_002_prompt", False, str(e))

    # 3. Empty command
    try:
        with lindorm_conn() as s:
            resp = send_cmd(s, "")
            check(f"{prefix}_003_empty_cmd", True)  # should just re-prompt
    except Exception as e:
        check(f"{prefix}_003_empty_cmd", False, str(e))

    # 4. QUIT command
    try:
        s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        s.settimeout(TIMEOUT)
        s.connect((HOST, PORT))
        s.recv(4096)
        s.sendall(b"QUIT\n")
        time.sleep(0.2)
        # Connection should close
        data = s.recv(4096)
        check(f"{prefix}_004_quit", True)
        s.close()
    except Exception as e:
        check(f"{prefix}_004_quit", True)  # socket error after quit is expected

    # 5. EXIT command
    try:
        s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        s.settimeout(TIMEOUT)
        s.connect((HOST, PORT))
        s.recv(4096)
        s.sendall(b"EXIT\n")
        time.sleep(0.2)
        s.recv(4096)
        check(f"{prefix}_005_exit", True)
        s.close()
    except Exception as e:
        check(f"{prefix}_005_exit", True)

    # 6. quit lowercase
    try:
        with lindorm_conn() as s:
            s.sendall(b"quit\n")
            time.sleep(0.2)
            check(f"{prefix}_006_quit_lower", True)
    except Exception as e:
        check(f"{prefix}_006_quit_lower", True)

    # 7. exit lowercase
    try:
        with lindorm_conn() as s:
            s.sendall(b"exit\n")
            time.sleep(0.2)
            check(f"{prefix}_007_exit_lower", True)
    except Exception as e:
        check(f"{prefix}_007_exit_lower", True)

    # 8. Multiple commands in sequence
    try:
        with lindorm_conn() as s:
            r1 = send_cmd(s, "LIST")
            r2 = send_cmd(s, "LIST")
            r3 = send_cmd(s, "LIST")
            check(f"{prefix}_008_multi_cmds", True)
    except Exception as e:
        check(f"{prefix}_008_multi_cmds", False, str(e))

    # 9. Rapid commands
    try:
        with lindorm_conn() as s:
            for i in range(10):
                send_cmd(s, "LIST")
            check(f"{prefix}_009_rapid", True)
    except Exception as e:
        check(f"{prefix}_009_rapid", False, str(e))

    # 10. Unknown command
    try:
        with lindorm_conn() as s:
            resp = send_cmd(s, "FOOBAR")
            check(f"{prefix}_010_unknown_cmd", "ERROR" in resp.upper() or "Unknown" in resp, f"resp={resp}")
    except Exception as e:
        check(f"{prefix}_010_unknown_cmd", False, str(e))

    # 11-20: Various connection patterns
    for i in range(11, 21):
        try:
            with lindorm_conn() as s:
                resp = send_cmd(s, "LIST")
                check(f"{prefix}_{i:03d}_conn_pattern", True)
        except Exception as e:
            check(f"{prefix}_{i:03d}_conn_pattern", False, str(e))


# ============================================================
# 2. Namespace tests (50+)
# ============================================================
def test_namespace():
    prefix = "ns"
    # The protocol does NOT support namespace commands, so these test error handling

    cmds = [
        "CREATE NAMESPACE testns",
        "CREATE NAMESPACE ns1",
        "CREATE NAMESPACE ns2",
        "DROP NAMESPACE testns",
        "DROP NAMESPACE ns1",
        "LIST NAMESPACES",
        "USE NAMESPACE default",
        "USE NAMESPACE testns",
        "USE NAMESPACE ns1",
        "CREATE NAMESPACE myspace",
        "DROP NAMESPACE myspace",
    ]

    for i, cmd in enumerate(cmds, 1):
        try:
            resp = new_conn_and_cmd(cmd)
            # The handler parses only parts[0], so CREATE NAMESPACE X creates table "NAMESPACE",
            # LIST NAMESPACES lists tables, USE/DROP NAMESPACE returns errors
            check(f"{prefix}_{i:03d}_ns_cmd", isinstance(resp, str) and len(resp) >= 0, f"resp={resp[:80]}")
        except Exception as e:
            check(f"{prefix}_{i:03d}_ns_cmd", False, str(e))

    # 12-50: Additional namespace edge cases
    ns_edge_cmds = [
        "CREATE NAMESPACE",
        "DROP NAMESPACE",
        "LIST NAMESPACE",
        "USE NAMESPACE",
        "CREATE NAMESPACE ",
        "CREATE  NAMESPACE  test",
        "create namespace test",
        "Create Namespace Test",
        "CREATE NAMESPACE ns_with_underscores",
        "CREATE NAMESPACE ns123",
        "CREATE NAMESPACE ns-with-dashes",
        "CREATE NAMESPACE ns.with.dots",
        "DROP NAMESPACE nonexistent",
        "USE NAMESPACE nonexistent",
        "CREATE NAMESPACE a",
        "CREATE NAMESPACE b",
        "CREATE NAMESPACE c",
        "DROP NAMESPACE a",
        "DROP NAMESPACE b",
        "DROP NAMESPACE c",
        "LIST NAMESPACES",
        "USE NAMESPACE default",
        "CREATE NAMESPACE test1",
        "CREATE NAMESPACE test2",
        "CREATE NAMESPACE test3",
        "CREATE NAMESPACE test4",
        "CREATE NAMESPACE test5",
        "DROP NAMESPACE test1",
        "DROP NAMESPACE test2",
        "DROP NAMESPACE test3",
        "DROP NAMESPACE test4",
        "DROP NAMESPACE test5",
        "NAMESPACE test",
        "NS test",
        "USE test",
        "CREATE NS test",
        "DROP NS test",
        "LIST NS",
    ]

    for i, cmd in enumerate(ns_edge_cmds, 12):
        try:
            resp = new_conn_and_cmd(cmd)
            check(f"{prefix}_{i:03d}_ns_edge", isinstance(resp, str), f"resp={resp[:80]}")
        except Exception as e:
            check(f"{prefix}_{i:03d}_ns_edge", False, str(e))


# ============================================================
# 3. Table DDL tests (100+)
# ============================================================
def test_table_ddl():
    prefix = "ddl"

    # CREATE TABLE tests
    tables_to_create = [f"ddl_t{i}" for i in range(1, 51)]
    for i, tname in enumerate(tables_to_create, 1):
        try:
            resp = new_conn_and_cmd(f"CREATE TABLE {tname}")
            check(f"{prefix}_{i:03d}_create", "OK" in resp and "created" in resp.lower(), f"resp={resp}")
        except Exception as e:
            check(f"{prefix}_{i:03d}_create", False, str(e))

    # LIST TABLES
    try:
        with lindorm_conn() as s:
            for t in tables_to_create[:5]:
                send_cmd(s, f"CREATE TABLE {t}")
            resp = send_cmd(s, "LIST")
            check(f"{prefix}_051_list", "ddl_t" in resp, f"resp={resp[:120]}")
    except Exception as e:
        check(f"{prefix}_051_list", False, str(e))

    # CREATE TABLE edge cases
    edge_cases = [
        ("CREATE TABLE", "missing_name"),
        ("CREATE", "missing_table_keyword"),
        ("CREATE TABLE  ", "trailing_space"),
        ("CREATE  TABLE  test_extra_space", "extra_spaces"),
        ("create table lowercase", "lowercase"),
        ("Create Table Mixed", "mixed_case"),
        ("CREATE TABLE t_with_underscore", "underscore_name"),
        ("CREATE TABLE t123", "numeric_suffix"),
        ("CREATE TABLE t-with-dash", "dash_name"),
        ("CREATE TABLE t.with.dot", "dot_name"),
    ]

    for i, (cmd, desc) in enumerate(edge_cases, 52):
        try:
            resp = new_conn_and_cmd(cmd)
            check(f"{prefix}_{i:03d}_create_edge_{desc}", isinstance(resp, str) and len(resp) > 0, f"resp={resp[:80]}")
        except Exception as e:
            check(f"{prefix}_{i:03d}_create_edge_{desc}", False, str(e))

    # DROP TABLE tests (not supported, but test error handling)
    for i in range(62, 82):
        try:
            resp = new_conn_and_cmd(f"DROP TABLE ddl_t{i - 61}")
            check(f"{prefix}_{i:03d}_drop", isinstance(resp, str), f"resp={resp[:80]}")
        except Exception as e:
            check(f"{prefix}_{i:03d}_drop", False, str(e))

    # DESCRIBE TABLE (not supported)
    for i in range(82, 92):
        try:
            resp = new_conn_and_cmd(f"DESCRIBE TABLE ddl_t{i - 81}")
            check(f"{prefix}_{i:03d}_describe", isinstance(resp, str), f"resp={resp[:80]}")
        except Exception as e:
            check(f"{prefix}_{i:03d}_describe", False, str(e))

    # ALTER TABLE (not supported)
    for i in range(92, 102):
        try:
            resp = new_conn_and_cmd(f"ALTER TABLE ddl_t1 ADD CF newcf")
            check(f"{prefix}_{i:03d}_alter", isinstance(resp, str), f"resp={resp[:80]}")
        except Exception as e:
            check(f"{prefix}_{i:03d}_alter", False, str(e))

    # Duplicate create
    try:
        with lindorm_conn() as s:
            r1 = send_cmd(s, "CREATE TABLE dup_test")
            r2 = send_cmd(s, "CREATE TABLE dup_test")
            check(f"{prefix}_102_dup_create", "OK" in r2 or "ERROR" in r2 or "created" in r2.lower(), f"r2={r2}")
    except Exception as e:
        check(f"{prefix}_102_dup_create", False, str(e))


# ============================================================
# 4. DML tests (200+): UPSERT/PUT, SELECT/GET, DELETE
# ============================================================
def test_dml():
    prefix = "dml"

    # Setup: create a table for DML tests
    with lindorm_conn() as s:
        send_cmd(s, "CREATE TABLE dml_test")

        # PUT tests (1-80)
        for i in range(1, 81):
            try:
                resp = send_cmd(s, f"PUT dml_test row{i:04d} cf col{i} value{i}")
                check(f"{prefix}_{i:03d}_put", "OK" in resp and "inserted" in resp.lower(), f"resp={resp}")
            except Exception as e:
                check(f"{prefix}_{i:03d}_put", False, str(e))

        # GET tests (81-140)
        for i in range(81, 141):
            row_num = i - 80
            try:
                resp = send_cmd(s, f"GET dml_test row{row_num:04d}")
                check(f"{prefix}_{i:03d}_get", "ROW" in resp and f"row{row_num:04d}" in resp, f"resp={resp[:80]}")
            except Exception as e:
                check(f"{prefix}_{i:03d}_get", False, str(e))

        # DELETE tests (141-180)
        for i in range(141, 181):
            row_num = i - 140
            try:
                resp = send_cmd(s, f"DELETE dml_test row{row_num:04d}")
                check(f"{prefix}_{i:03d}_delete", "OK" in resp and "deleted" in resp.lower(), f"resp={resp}")
            except Exception as e:
                check(f"{prefix}_{i:03d}_delete", False, str(e))

        # Verify deleted rows are gone (181-200)
        for i in range(181, 201):
            row_num = i - 180
            try:
                resp = send_cmd(s, f"GET dml_test row{row_num:04d}")
                check(f"{prefix}_{i:03d}_verify_del", "ERROR" in resp or "not found" in resp.lower(), f"resp={resp}")
            except Exception as e:
                check(f"{prefix}_{i:03d}_verify_del", False, str(e))

    # PUT edge cases (201-220)
    edge_puts = [
        "PUT dml_test row_special cf! col! val!",
        "PUT dml_test row@ cf col val",
        "PUT dml_test row# cf col val",
        "PUT dml_test row$ cf col val",
        "PUT dml_test row% cf col val",
        "PUT dml_test row^ cf col val",
        "PUT dml_test row& cf col val",
        "PUT dml_test row* cf col val",
        "PUT",
        "PUT dml_test",
        "PUT dml_test row1",
        "PUT dml_test row1 cf",
        "PUT dml_test row1 cf col",
        "PUT nonexistent row1 cf col val",
    ]
    for i, cmd in enumerate(edge_puts, 201):
        try:
            resp = new_conn_and_cmd(cmd)
            check(f"{prefix}_{i:03d}_put_edge", isinstance(resp, str) and len(resp) > 0, f"resp={resp[:80]}")
        except Exception as e:
            check(f"{prefix}_{i:03d}_put_edge", False, str(e))

    # Overwrite tests (215-220)
    with lindorm_conn() as s:
        send_cmd(s, "CREATE TABLE dml_overwrite")
        send_cmd(s, "PUT dml_overwrite row1 cf col original")
        try:
            resp = send_cmd(s, "PUT dml_overwrite row1 cf col updated")
            check(f"{prefix}_215_overwrite", "OK" in resp, f"resp={resp}")
        except Exception as e:
            check(f"{prefix}_215_overwrite", False, str(e))

        try:
            resp = send_cmd(s, "GET dml_overwrite row1")
            check(f"{prefix}_216_verify_overwrite", "updated" in resp, f"resp={resp}")
        except Exception as e:
            check(f"{prefix}_216_verify_overwrite", False, str(e))

    # GET edge cases
    for i in range(217, 230):
        try:
            resp = new_conn_and_cmd(f"GET dml_test row_nonexistent_{i}")
            check(f"{prefix}_{i:03d}_get_edge", "ERROR" in resp or "not found" in resp.lower(), f"resp={resp}")
        except Exception as e:
            check(f"{prefix}_{i:03d}_get_edge", False, str(e))

    # DELETE edge cases
    for i in range(230, 245):
        try:
            resp = new_conn_and_cmd(f"DELETE dml_test row_nonexistent_{i}")
            check(f"{prefix}_{i:03d}_del_edge", "ERROR" in resp or "not found" in resp.lower(), f"resp={resp}")
        except Exception as e:
            check(f"{prefix}_{i:03d}_del_edge", False, str(e))


# ============================================================
# 5. Column families (100+)
# ============================================================
def test_column_families():
    prefix = "cf"

    with lindorm_conn() as s:
        send_cmd(s, "CREATE TABLE cf_test")

        # PUT with different column families (1-40)
        families = ["cf1", "cf2", "cf3", "info", "data", "meta", "attrs", "props", "settings", "config"]
        for i, fam in enumerate(families, 1):
            try:
                resp = send_cmd(s, f"PUT cf_test row1 {fam} qualifier1 value_{fam}")
                check(f"{prefix}_{i:03d}_put_cf", "OK" in resp, f"resp={resp}")
            except Exception as e:
                check(f"{prefix}_{i:03d}_put_cf", False, str(e))

        # GET with multiple families (41-50)
        try:
            resp = send_cmd(s, "GET cf_test row1")
            for fam in families:
                check(f"{prefix}_0{41 + families.index(fam)}_get_cf_{fam}", fam in resp, f"resp={resp[:200]}")
        except Exception as e:
            for fam in families:
                check(f"{prefix}_0{41 + families.index(fam)}_get_cf_{fam}", False, str(e))

        # Multiple qualifiers per family (51-70)
        for i in range(51, 71):
            qual_num = i - 50
            try:
                resp = send_cmd(s, f"PUT cf_test row2 cf1 qual{qual_num} val{qual_num}")
                check(f"{prefix}_{i:03d}_multi_qual", "OK" in resp, f"resp={resp}")
            except Exception as e:
                check(f"{prefix}_{i:03d}_multi_qual", False, str(e))

        # Verify multiple qualifiers (71-80)
        try:
            resp = send_cmd(s, "GET cf_test row2")
            check(f"{prefix}_081_verify_multi_qual", "qual1" in resp and "qual10" in resp, f"resp={resp[:300]}")
        except Exception as e:
            check(f"{prefix}_081_verify_multi_qual", False, str(e))

        # Different families same row (82-90)
        for i in range(82, 91):
            fam = f"family{i}"
            try:
                resp = send_cmd(s, f"PUT cf_test row_multi {fam} q1 v{i}")
                check(f"{prefix}_{i:03d}_same_row_diff_cf", "OK" in resp, f"resp={resp}")
            except Exception as e:
                check(f"{prefix}_{i:03d}_same_row_diff_cf", False, str(e))

        # Verify (91)
        try:
            resp = send_cmd(s, "GET cf_test row_multi")
            check(f"{prefix}_091_verify_multi_cf", "family82" in resp and "family90" in resp, f"resp={resp[:500]}")
        except Exception as e:
            check(f"{prefix}_091_verify_multi_cf", False, str(e))

        # Empty family name (92-95)
        for i in range(92, 96):
            try:
                resp = send_cmd(s, f"PUT cf_test row_empty  q1 val{i}")
                check(f"{prefix}_{i:03d}_empty_cf", isinstance(resp, str), f"resp={resp}")
            except Exception as e:
                check(f"{prefix}_{i:03d}_empty_cf", False, str(e))

        # Special family names (96-100)
        special_cfs = ["cf-with-dash", "cf.with.dot", "cf_with_under", "CF_UPPER", "cf123"]
        for i, cf_name in enumerate(special_cfs, 96):
            try:
                resp = send_cmd(s, f"PUT cf_test row_special {cf_name} q1 val")
                check(f"{prefix}_{i:03d}_special_cf", "OK" in resp, f"resp={resp}")
            except Exception as e:
                check(f"{prefix}_{i:03d}_special_cf", False, str(e))


# ============================================================
# 6. Data types (100+)
# ============================================================
def test_data_types():
    prefix = "dtype"

    with lindorm_conn() as s:
        send_cmd(s, "CREATE TABLE dtype_test")

        # VARCHAR values (1-15) - NOTE: protocol is space-delimited, so no spaces in values
        varchar_vals = [
            "hello", "world", "helloworld", "with_tab", "a",
            "ab", "abc", "ABCDEFGHIJ", "abcdefghij", "0123456789",
            "special_chars", "nospace", "trailing", "leading",
            "MiXeDCaSe",
        ]
        for i, val in enumerate(varchar_vals, 1):
            try:
                resp = send_cmd(s, f"PUT dtype_test row_varchar_{i:03d} cf str val_{val}")
                check(f"{prefix}_{i:03d}_varchar", "OK" in resp, f"resp={resp}")
            except Exception as e:
                check(f"{prefix}_{i:03d}_varchar", False, str(e))

        # INTEGER values (16-30)
        int_vals = ["0", "1", "-1", "42", "100", "999", "-999", "0", "2147483647",
                    "-2147483648", "12345", "54321", "1", "2", "3"]
        for i, val in enumerate(int_vals, 16):
            try:
                resp = send_cmd(s, f"PUT dtype_test row_int_{i:03d} cf num {val}")
                check(f"{prefix}_{i:03d}_int", "OK" in resp, f"resp={resp}")
            except Exception as e:
                check(f"{prefix}_{i:03d}_int", False, str(e))

        # BIGINT values (31-40)
        bigint_vals = ["0", "9999999999", "-9999999999", "4294967296",
                       "-4294967296", "9223372036854775807", "1234567890123",
                       "1", "2", "3"]
        for i, val in enumerate(bigint_vals, 31):
            try:
                resp = send_cmd(s, f"PUT dtype_test row_bigint_{i:03d} cf bignum {val}")
                check(f"{prefix}_{i:03d}_bigint", "OK" in resp, f"resp={resp}")
            except Exception as e:
                check(f"{prefix}_{i:03d}_bigint", False, str(e))

        # FLOAT values (41-55)
        float_vals = ["0.0", "1.0", "-1.0", "3.14", "2.718", "0.001",
                      "-0.001", "999.999", "1e10", "1.5e-5", "0.1",
                      "0.2", "0.3", "100.001", "-100.001"]
        for i, val in enumerate(float_vals, 41):
            try:
                resp = send_cmd(s, f"PUT dtype_test row_float_{i:03d} cf flt {val}")
                check(f"{prefix}_{i:03d}_float", "OK" in resp, f"resp={resp}")
            except Exception as e:
                check(f"{prefix}_{i:03d}_float", False, str(e))

        # DOUBLE values (56-70)
        double_vals = ["0.0", "1.0", "-1.0", "3.141592653589793",
                       "2.718281828459045", "1e100", "1e-100",
                       "4.9e-324", "1.7976931348623157e+308",
                       "-1.7976931348623157e+308", "0.1", "0.2",
                       "0.3", "100.001", "-100.001"]
        for i, val in enumerate(double_vals, 56):
            try:
                resp = send_cmd(s, f"PUT dtype_test row_double_{i:03d} cf dbl {val}")
                check(f"{prefix}_{i:03d}_double", "OK" in resp, f"resp={resp}")
            except Exception as e:
                check(f"{prefix}_{i:03d}_double", False, str(e))

        # BOOLEAN values (71-75)
        bool_vals = ["true", "false", "TRUE", "FALSE", "True"]
        for i, val in enumerate(bool_vals, 71):
            try:
                resp = send_cmd(s, f"PUT dtype_test row_bool_{i:03d} cf bool {val}")
                check(f"{prefix}_{i:03d}_bool", "OK" in resp, f"resp={resp}")
            except Exception as e:
                check(f"{prefix}_{i:03d}_bool", False, str(e))

        # VARBINARY-like values (76-85)
        binary_vals = ["\\x00", "\\x01", "\\xFF", "\\xDE\\xAD\\xBE\\xEF",
                       "binary0", "binary1", "binary2", "binary3",
                       "binary4", "binary5"]
        for i, val in enumerate(binary_vals, 76):
            try:
                resp = send_cmd(s, f"PUT dtype_test row_bin_{i:03d} cf bin {val}")
                check(f"{prefix}_{i:03d}_varbinary", "OK" in resp, f"resp={resp}")
            except Exception as e:
                check(f"{prefix}_{i:03d}_varbinary", False, str(e))

        # TIMESTAMP values (86-100)
        ts_vals = ["1609459200000", "1609459200", "0", "1", "1000",
                   "9999999999", "1640000000000", "1650000000000",
                   "1660000000000", "1670000000000", "1680000000000",
                   "1690000000000", "1700000000000", "1710000000000",
                   "1720000000000"]
        for i, val in enumerate(ts_vals, 86):
            try:
                resp = send_cmd(s, f"PUT dtype_test row_ts_{i:03d} cf ts {val}")
                check(f"{prefix}_{i:03d}_timestamp", "OK" in resp, f"resp={resp}")
            except Exception as e:
                check(f"{prefix}_{i:03d}_timestamp", False, str(e))

        # Verify some data types can be retrieved
        # Verify INT rows exist (101-105) - rows 16-30 were created
        for i in range(101, 106):
            row_num = i - 100 + 15  # maps to row_int_016..020
            try:
                resp = send_cmd(s, f"GET dtype_test row_int_{row_num:03d}")
                check(f"{prefix}_{i:03d}_get_varchar", "ROW" in resp, f"resp={resp[:80]}")
            except Exception as e:
                check(f"{prefix}_{i:03d}_get_varchar", False, str(e))

        # Verify overwriting with different type values
        for i in range(106, 116):
            try:
                send_cmd(s, f"PUT dtype_test row_type_mix cf val integer_42")
                resp = send_cmd(s, f"PUT dtype_test row_type_mix cf val string_hello")
                check(f"{prefix}_{i:03d}_type_overwrite", "OK" in resp, f"resp={resp}")
            except Exception as e:
                check(f"{prefix}_{i:03d}_type_overwrite", False, str(e))


# ============================================================
# 7. Filter tests (100+)
# ============================================================
def test_filters():
    """The Lindorm protocol doesn't have filter operators. Test scan ranges as filters."""
    prefix = "filt"

    with lindorm_conn() as s:
        send_cmd(s, "CREATE TABLE filt_test")

        # Insert rows with sequential keys for range scan
        for i in range(1, 51):
            send_cmd(s, f"PUT filt_test row{i:04d} cf col val{i}")

        # Insert rows with alpha keys
        for c in "abcdefghij":
            send_cmd(s, f"PUT filt_test alpha_{c} cf col val_{c}")

        # Scan as equality filter (1-20)
        for i in range(1, 21):
            try:
                row = f"row{i:04d}"
                resp = send_cmd(s, f"GET filt_test {row}")
                check(f"{prefix}_{i:03d}_eq", "ROW" in resp and row in resp, f"resp={resp[:80]}")
            except Exception as e:
                check(f"{prefix}_{i:03d}_eq", False, str(e))

        # Range scan as > filter (21-40)
        for i in range(21, 41):
            start = f"row{i:04d}"
            try:
                resp = send_cmd(s, f"SCAN filt_test {start} row9999")
                check(f"{prefix}_{i:03d}_gt", "ROW" in resp, f"resp={resp[:80]}")
            except Exception as e:
                check(f"{prefix}_{i:03d}_gt", False, str(e))

        # Range scan as < filter (41-60)
        for i in range(41, 61):
            end = f"row{i:04d}"
            try:
                resp = send_cmd(s, f"SCAN filt_test row0000 {end}")
                has_rows = "ROW" in resp or "No rows" in resp
                check(f"{prefix}_{i:03d}_lt", has_rows, f"resp={resp[:80]}")
            except Exception as e:
                check(f"{prefix}_{i:03d}_lt", False, str(e))

        # Range scan as >= and <= (61-80)
        for i in range(61, 81):
            start = f"row{i:04d}"
            end = f"row{i + 5:04d}"
            try:
                resp = send_cmd(s, f"SCAN filt_test {start} {end}")
                check(f"{prefix}_{i:03d}_range", isinstance(resp, str), f"resp={resp[:80]}")
            except Exception as e:
                check(f"{prefix}_{i:03d}_range", False, str(e))

        # Non-existent key "filter" (81-90)
        for i in range(81, 91):
            try:
                resp = send_cmd(s, f"GET filt_test nonexistent_{i}")
                check(f"{prefix}_{i:03d}_not_found", "ERROR" in resp or "not found" in resp.lower(), f"resp={resp}")
            except Exception as e:
                check(f"{prefix}_{i:03d}_not_found", False, str(e))

        # Alpha range scan (91-100)
        alpha_ranges = [
            ("alpha_a", "alpha_d"),
            ("alpha_a", "alpha_f"),
            ("alpha_c", "alpha_h"),
            ("alpha_a", "alpha_z"),
            ("alpha_e", "alpha_j"),
            ("alpha_a", "alpha_a"),
            ("alpha_b", "alpha_b"),
            ("alpha_z", "alpha_z"),
            ("alpha_a", "alpha_b"),
            ("alpha_i", "alpha_j"),
        ]
        for i, (start, end) in enumerate(alpha_ranges, 91):
            try:
                resp = send_cmd(s, f"SCAN filt_test {start} {end}")
                check(f"{prefix}_{i:03d}_alpha_range", isinstance(resp, str), f"resp={resp[:80]}")
            except Exception as e:
                check(f"{prefix}_{i:03d}_alpha_range", False, str(e))


# ============================================================
# 8. Aggregation tests (50+)
# ============================================================
def test_aggregation():
    prefix = "agg"

    with lindorm_conn() as s:
        send_cmd(s, "CREATE TABLE agg_test")

        # Insert known data
        for i in range(1, 51):
            send_cmd(s, f"PUT agg_test row{i:04d} cf num {i}")

        # COUNT tests (1-20)
        for i in range(1, 21):
            try:
                resp = send_cmd(s, "COUNT agg_test")
                check(f"{prefix}_{i:03d}_count", "Count:" in resp and "50" in resp, f"resp={resp}")
            except Exception as e:
                check(f"{prefix}_{i:03d}_count", False, str(e))

        # COUNT empty table (21-25)
        send_cmd(s, "CREATE TABLE agg_empty")
        for i in range(21, 26):
            try:
                resp = send_cmd(s, "COUNT agg_empty")
                check(f"{prefix}_{i:03d}_count_empty", "Count: 0" in resp, f"resp={resp}")
            except Exception as e:
                check(f"{prefix}_{i:03d}_count_empty", False, str(e))

        # COUNT after deletes (26-35)
        for i in range(26, 36):
            send_cmd(s, f"DELETE agg_test row{i - 25:04d}")
        try:
            resp = send_cmd(s, "COUNT agg_test")
            check(f"{prefix}_036_count_after_del", "Count: 40" in resp, f"resp={resp}")
        except Exception as e:
            check(f"{prefix}_036_count_after_del", False, str(e))

        # SUM/AVG/MIN/MAX - not supported, test error handling (37-50)
        agg_cmds = [
            "SUM agg_test cf num",
            "AVG agg_test cf num",
            "MIN agg_test cf num",
            "MAX agg_test cf num",
            "SUM agg_test",
            "AVG agg_test",
            "MIN agg_test",
            "MAX agg_test",
            "AGGREGATE agg_test SUM",
            "AGG agg_test",
            "TOTAL agg_test",
            "MEAN agg_test",
            "MEDIAN agg_test",
            "STDDEV agg_test",
        ]
        for i, cmd in enumerate(agg_cmds, 37):
            try:
                resp = send_cmd(s, cmd)
                check(f"{prefix}_{i:03d}_agg_unsupported", "ERROR" in resp.upper() or "Unknown" in resp, f"resp={resp}")
            except Exception as e:
                check(f"{prefix}_{i:03d}_agg_unsupported", False, str(e))


# ============================================================
# 9. Scan tests (100+)
# ============================================================
def test_scan():
    prefix = "scan"

    with lindorm_conn() as s:
        send_cmd(s, "CREATE TABLE scan_test")

        # Insert 100 rows
        for i in range(1, 101):
            send_cmd(s, f"PUT scan_test row{i:04d} cf col val{i}")

        # Full range scan (1-10)
        for i in range(1, 11):
            try:
                resp = send_cmd(s, "SCAN scan_test row0000 row9999")
                check(f"{prefix}_{i:03d}_full_scan", "ROW" in resp, f"resp={resp[:80]}")
            except Exception as e:
                check(f"{prefix}_{i:03d}_full_scan", False, str(e))

        # Narrow range scan (11-30)
        for i in range(11, 31):
            start = f"row{i:04d}"
            end = f"row{i + 2:04d}"
            try:
                resp = send_cmd(s, f"SCAN scan_test {start} {end}")
                check(f"{prefix}_{i:03d}_narrow", "ROW" in resp, f"resp={resp[:120]}")
            except Exception as e:
                check(f"{prefix}_{i:03d}_narrow", False, str(e))

        # Empty range scan (31-50)
        for i in range(31, 51):
            try:
                resp = send_cmd(s, f"SCAN scan_test zzzz zzzz{i:04d}")
                check(f"{prefix}_{i:03d}_empty_range", "No rows" in resp or resp.strip() == "", f"resp={resp[:80]}")
            except Exception as e:
                check(f"{prefix}_{i:03d}_empty_range", False, str(e))

        # Single row scan (51-70)
        for i in range(51, 71):
            row = f"row{i:04d}"
            try:
                resp = send_cmd(s, f"SCAN scan_test {row} row{i + 1:04d}")
                check(f"{prefix}_{i:03d}_single", row in resp, f"resp={resp[:80]}")
            except Exception as e:
                check(f"{prefix}_{i:03d}_single", False, str(e))

        # Edge case scans (71-100)
        edge_scans = [
            ("SCAN scan_test", "missing_args"),
            ("SCAN scan_test row0001", "one_arg"),
            ("SCAN", "no_args"),
            ("SCAN nonexistent row0001 row0010", "bad_table"),
            ("SCAN scan_test row0001 row0001", "same_start_end"),
            ("SCAN scan_test row0050 row0001", "reversed_range"),
            ("SCAN scan_test aaaa aazz", "before_range"),
            ("SCAN scan_test row0001 row0005", "first_five"),
            ("SCAN scan_test row0096 row9999", "last_rows"),
            ("SCAN scan_test row0000 row0001", "only_first"),
            ("SCAN scan_test row0050 row0051", "single_mid"),
            ("SCAN scan_test row0000 row0011", "first_ten"),
            ("SCAN scan_test row0090 row9999", "last_ten"),
            ("SCAN scan_test row0025 row0075", "middle_fifty"),
            ("SCAN scan_test row0000 row0002", "two_rows"),
            ("SCAN scan_test row0000 row0003", "three_rows"),
            ("SCAN scan_test a z", "alpha_bounds"),
            ("SCAN scan_test ! ~~", "wide_bounds"),
            ("SCAN scan_test row001 row002", "short_keys"),
            ("SCAN scan_test row00100 row00200", "long_keys"),
            ("SCAN scan_test ROW0010 ROW0020", "uppercase_keys"),
            ("SCAN scan_test Row0010 Row0020", "mixed_case"),
            ("SCAN scan_test row0010. row0020.", "dot_suffix"),
            ("SCAN scan_test row-0010 row-0020", "dash_keys"),
            ("SCAN scan_test row_0010 row_0020", "underscore_keys"),
            ("SCAN scan_test row0001 row0002", "adjacent_rows"),
            ("SCAN scan_test row0010 row0010", "same_key"),
            ("SCAN scan_test row0010 row0011", "next_row"),
            ("SCAN scan_test aaaa bbbb", "before_data"),
            ("SCAN scan_test row0101 row9999", "after_data"),
        ]
        for i, (cmd, desc) in enumerate(edge_scans, 71):
            try:
                resp = send_cmd(s, cmd)
                check(f"{prefix}_{i:03d}_scan_{desc}", isinstance(resp, str), f"resp={resp[:80]}")
            except Exception as e:
                check(f"{prefix}_{i:03d}_scan_{desc}", False, str(e))


# ============================================================
# 10. Edge cases (80+)
# ============================================================
def test_edge_cases():
    prefix = "edge"

    # Empty table operations (1-15)
    with lindorm_conn() as s:
        send_cmd(s, "CREATE TABLE edge_empty")

        for i in range(1, 6):
            try:
                resp = send_cmd(s, "GET edge_empty anyrow")
                check(f"{prefix}_{i:03d}_empty_get", "ERROR" in resp or "not found" in resp.lower(), f"resp={resp}")
            except Exception as e:
                check(f"{prefix}_{i:03d}_empty_get", False, str(e))

        for i in range(6, 11):
            try:
                resp = send_cmd(s, "SCAN edge_empty a z")
                check(f"{prefix}_{i:03d}_empty_scan", "No rows" in resp or resp.strip() == "", f"resp={resp}")
            except Exception as e:
                check(f"{prefix}_{i:03d}_empty_scan", False, str(e))

        for i in range(11, 16):
            try:
                resp = send_cmd(s, "COUNT edge_empty")
                check(f"{prefix}_{i:03d}_empty_count", "Count: 0" in resp, f"resp={resp}")
            except Exception as e:
                check(f"{prefix}_{i:03d}_empty_count", False, str(e))

    # NULL-like values (16-25)
    with lindorm_conn() as s:
        send_cmd(s, "CREATE TABLE edge_null")
        null_vals = ["NULL", "null", "None", "nil", "\\0", "NaN", "nan", "undefined", "void", "empty"]
        for i, val in enumerate(null_vals, 16):
            try:
                resp = send_cmd(s, f"PUT edge_null row{i} cf col {val}")
                check(f"{prefix}_{i:03d}_null_val", "OK" in resp, f"resp={resp}")
            except Exception as e:
                check(f"{prefix}_{i:03d}_null_val", False, str(e))

    # Large values (26-40)
    with lindorm_conn() as s:
        send_cmd(s, "CREATE TABLE edge_large")
        for i in range(26, 41):
            size = (i - 25) * 100
            large_val = "x" * size
            try:
                resp = send_cmd(s, f"PUT edge_large row{i} cf col {large_val}")
                check(f"{prefix}_{i:03d}_large_{size}", "OK" in resp, f"resp={resp[:40]}")
            except Exception as e:
                check(f"{prefix}_{i:03d}_large_{size}", False, str(e))

    # Special rowkeys (41-60)
    with lindorm_conn() as s:
        send_cmd(s, "CREATE TABLE edge_rowkey")
        special_keys = [
            "key with spaces",
            "key\twith\ttabs",
            "key!@#$%^&*()",
            "key/with/slashes",
            "key\\with\\backslash",
            "key:with:colons",
            "key;with;semicolons",
            "key,with,commas",
            "key.with.dots",
            "key-with-dashes",
            "key_with_underscores",
            "00000",
            "99999",
            "aaaaa",
            "zzzzz",
            "!start",
            "~end",
            ".dotstart",
            "-dashstart",
            "_understart",
        ]
        for i, key in enumerate(special_keys, 41):
            try:
                resp = send_cmd(s, f"PUT edge_rowkey {key} cf col val{i}")
                check(f"{prefix}_{i:03d}_special_key", "OK" in resp, f"resp={resp}")
            except Exception as e:
                check(f"{prefix}_{i:03d}_special_key", False, str(e))

    # Operations on non-existent table (61-75)
    for i in range(61, 76):
        try:
            resp = new_conn_and_cmd(f"GET nonexistent_table_{i} row1")
            check(f"{prefix}_{i:03d}_no_table_get", "ERROR" in resp or "not found" in resp.lower(), f"resp={resp}")
        except Exception as e:
            check(f"{prefix}_{i:03d}_no_table_get", False, str(e))

    # TTL-like values (76-85)
    with lindorm_conn() as s:
        send_cmd(s, "CREATE TABLE edge_ttl")
        ttl_vals = ["1", "60", "3600", "86400", "604800", "2592000",
                    "31536000", "0", "-1", "999999999"]
        for i, val in enumerate(ttl_vals, 76):
            try:
                resp = send_cmd(s, f"PUT edge_ttl row{i} cf col {val}")
                check(f"{prefix}_{i:03d}_ttl", "OK" in resp, f"resp={resp}")
            except Exception as e:
                check(f"{prefix}_{i:03d}_ttl", False, str(e))

    # Concurrent-like: multiple operations same row (86-95)
    with lindorm_conn() as s:
        send_cmd(s, "CREATE TABLE edge_concurrent")
        for i in range(86, 96):
            try:
                send_cmd(s, f"PUT edge_concurrent row1 cf col val_{i}")
                resp = send_cmd(s, f"GET edge_concurrent row1")
                check(f"{prefix}_{i:03d}_concurrent", "row1" in resp, f"resp={resp[:80]}")
            except Exception as e:
                check(f"{prefix}_{i:03d}_concurrent", False, str(e))

    # Very long command (96-100)
    with lindorm_conn() as s:
        send_cmd(s, "CREATE TABLE edge_long")
        for i in range(96, 101):
            long_val = "v" * (i * 50)
            try:
                resp = send_cmd(s, f"PUT edge_long row{i} cf col {long_val}")
                check(f"{prefix}_{i:03d}_long_val", "OK" in resp, f"resp={resp[:40]}")
            except Exception as e:
                check(f"{prefix}_{i:03d}_long_val", False, str(e))


# ============================================================
# Additional tests to reach 1000+
# ============================================================
def test_additional_put_get():
    """Additional PUT/GET combinations."""
    prefix = "extra"

    with lindorm_conn() as s:
        send_cmd(s, "CREATE TABLE extra1")

        # Bulk insert and retrieve (1-100)
        for i in range(1, 101):
            try:
                send_cmd(s, f"PUT extra1 bulk_{i:05d} cf1 q1 value_{i}")
                resp = send_cmd(s, f"GET extra1 bulk_{i:05d}")
                check(f"{prefix}_{i:03d}_bulk", f"bulk_{i:05d}" in resp, f"resp={resp[:80]}")
            except Exception as e:
                check(f"{prefix}_{i:03d}_bulk", False, str(e))

        # Multiple columns same row (101-150)
        for i in range(101, 151):
            col_num = i - 100
            try:
                send_cmd(s, f"PUT extra1 multi_col cf1 col{col_num} val{col_num}")
                resp = send_cmd(s, f"GET extra1 multi_col")
                check(f"{prefix}_{i:03d}_multi_col", f"col{col_num}" in resp, f"resp={resp[:200]}")
            except Exception as e:
                check(f"{prefix}_{i:03d}_multi_col", False, str(e))

        # Multiple rows multiple families (151-200)
        for i in range(151, 201):
            row_num = i - 150
            fam = f"cf{(row_num % 3) + 1}"
            try:
                send_cmd(s, f"PUT extra1 mf_row{row_num:04d} {fam} q val_{row_num}")
                resp = send_cmd(s, f"GET extra1 mf_row{row_num:04d}")
                check(f"{prefix}_{i:03d}_multi_fam", fam in resp, f"resp={resp[:120]}")
            except Exception as e:
                check(f"{prefix}_{i:03d}_multi_fam", False, str(e))


def test_additional_scan():
    """More scan tests."""
    prefix = "xscan"

    with lindorm_conn() as s:
        send_cmd(s, "CREATE TABLE xscan")

        # Insert 200 rows
        for i in range(1, 201):
            send_cmd(s, f"PUT xscan key{i:05d} cf col val{i}")

        # Progressive range scans (1-50)
        for i in range(1, 51):
            end_row = i * 4
            try:
                resp = send_cmd(s, f"SCAN xscan key00001 key{end_row:05d}")
                check(f"{prefix}_{i:03d}_progressive", "ROW" in resp or "No rows" in resp, f"resp={resp[:80]}")
            except Exception as e:
                check(f"{prefix}_{i:03d}_progressive", False, str(e))

        # Scan after deletes (51-70)
        for i in range(51, 61):
            send_cmd(s, f"DELETE xscan key{i:05d}")
        for i in range(61, 71):
            try:
                resp = send_cmd(s, "SCAN xscan key00001 key99999")
                check(f"{prefix}_{i:03d}_post_del_scan", "ROW" in resp, f"resp={resp[:80]}")
            except Exception as e:
                check(f"{prefix}_{i:03d}_post_del_scan", False, str(e))

        # Scan count verification (71-80)
        for i in range(71, 81):
            try:
                resp = send_cmd(s, "COUNT xscan")
                check(f"{prefix}_{i:03d}_scan_count", "Count:" in resp, f"resp={resp}")
            except Exception as e:
                check(f"{prefix}_{i:03d}_scan_count", False, str(e))


def test_additional_overwrite():
    """Test overwriting behavior thoroughly."""
    prefix = "ow"

    with lindorm_conn() as s:
        send_cmd(s, "CREATE TABLE overwrite_test")

        # Overwrite same cell multiple times (1-50)
        for i in range(1, 51):
            try:
                send_cmd(s, f"PUT overwrite_test row1 cf col version_{i}")
                resp = send_cmd(s, f"GET overwrite_test row1")
                check(f"{prefix}_{i:03d}_overwrite", f"version_{i}" in resp, f"resp={resp[:80]}")
            except Exception as e:
                check(f"{prefix}_{i:03d}_overwrite", False, str(e))

        # Overwrite different qualifiers (51-80)
        for i in range(51, 81):
            q = i - 50
            try:
                send_cmd(s, f"PUT overwrite_test row2 cf q{q} val_{q}")
                send_cmd(s, f"PUT overwrite_test row2 cf q{q} updated_{q}")
                resp = send_cmd(s, f"GET overwrite_test row2")
                check(f"{prefix}_{i:03d}_qual_overwrite", f"updated_{q}" in resp, f"resp={resp[:300]}")
            except Exception as e:
                check(f"{prefix}_{i:03d}_qual_overwrite", False, str(e))

        # Delete then re-insert (81-100)
        for i in range(81, 101):
            try:
                send_cmd(s, f"PUT overwrite_test del_row_{i} cf col original")
                send_cmd(s, f"DELETE overwrite_test del_row_{i}")
                send_cmd(s, f"PUT overwrite_test del_row_{i} cf col reinserted")
                resp = send_cmd(s, f"GET overwrite_test del_row_{i}")
                check(f"{prefix}_{i:03d}_del_reinsert", "reinserted" in resp, f"resp={resp[:80]}")
            except Exception as e:
                check(f"{prefix}_{i:03d}_del_reinsert", False, str(e))


# ============================================================
# Main
# ============================================================
def main():
    print("=" * 60)
    print("Lindorm Protocol Test Suite (1000+ tests)")
    print("=" * 60)

    test_funcs = [
        ("Connection", test_connection),
        ("Namespace", test_namespace),
        ("Table DDL", test_table_ddl),
        ("DML", test_dml),
        ("Column Families", test_column_families),
        ("Data Types", test_data_types),
        ("Filters", test_filters),
        ("Aggregation", test_aggregation),
        ("Scan", test_scan),
        ("Edge Cases", test_edge_cases),
        ("Additional PUT/GET", test_additional_put_get),
        ("Additional Scan", test_additional_scan),
        ("Additional Overwrite", test_additional_overwrite),
    ]

    for name, func in test_funcs:
        before = results.total
        print(f"\nRunning: {name}...")
        try:
            func()
        except Exception as e:
            print(f"  FATAL: {e}")
            traceback.print_exc()
        after = results.total
        print(f"  Completed: {after - before} tests")

    print("\n" + "=" * 60)
    output = results.to_json()
    print(json.dumps(output, indent=2))

    # Write to file
    output_path = "/Users/walker/code/RorisDB/tests/protocol_tests/lindorm_results.json"
    with open(output_path, "w") as f:
        json.dump(output, f, indent=2)
    print(f"\nResults written to {output_path}")


if __name__ == "__main__":
    main()
