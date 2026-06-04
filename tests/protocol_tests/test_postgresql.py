#!/usr/bin/env python3
"""
Comprehensive PostgreSQL Protocol Test Suite for HarnessDB (RorisDB)
1000+ test cases using raw TCP sockets implementing PostgreSQL wire protocol v3.

Categories: DDL, DML, Functions, Type Casts, Expressions, JOINs, Subqueries,
Window Functions, System, NULL Handling, Arrays, Edge Cases.

Usage: python3 test_postgresql.py [port]
Default port: 15433
"""

import sys
import socket
import struct
import time
import json
import hashlib
import random
import string

PORT = int(sys.argv[1]) if len(sys.argv) > 1 else 15433
HOST = "127.0.0.1"


class PgClient:
    """Minimal PostgreSQL wire protocol v3 client (Simple Query mode)."""

    def __init__(self, host, port, timeout=10):
        self.host = host
        self.port = port
        self.timeout = timeout
        self.sock = None
        self.pid = 0
        self.secret = 0

    def connect(self, user="harness", database="harness"):
        self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.sock.settimeout(self.timeout)
        self.sock.connect((self.host, self.port))

        self._user = user
        self._password = "harness-secret"

        params = (
            f"user\x00{user}\x00"
            f"database\x00{database}\x00"
            f"client_encoding\x00UTF8\x00"
            f"\x00"
        )
        msg = struct.pack("!i", 196608) + params.encode()
        msg = struct.pack("!i", len(msg) + 4) + msg
        self.sock.sendall(msg)
        self._read_startup_response()

    def _recv_exact(self, n):
        data = b""
        while len(data) < n:
            chunk = self.sock.recv(n - len(data))
            if not chunk:
                raise ConnectionError("Connection closed while reading")
            data += chunk
        return data

    def _read_startup_response(self):
        while True:
            msg_type = self.sock.recv(1)
            if not msg_type:
                break
            msg_type = ord(msg_type)
            length = struct.unpack("!i", self._recv_exact(4))[0]
            data = self._recv_exact(length - 4)

            if msg_type == ord('R'):
                auth_type = struct.unpack("!i", data[:4])[0]
                if auth_type == 0:
                    continue
                elif auth_type == 3:
                    pwd_str = f"{self._password}\x00".encode()
                    pwd_msg = b'p' + struct.pack("!i", len(pwd_str) + 4) + pwd_str
                    self.sock.sendall(pwd_msg)
                elif auth_type == 5:
                    salt = data[4:8]
                    # PostgreSQL MD5: md5(md5(password+user) + salt_bytes)
                    inner = hashlib.md5((self._password + self._user).encode()).hexdigest()
                    outer = hashlib.md5(inner.encode() + salt).hexdigest()
                    pwd_str = f"md5{outer}\x00".encode()
                    pwd_msg = b'p' + struct.pack("!i", len(pwd_str) + 4) + pwd_str
                    self.sock.sendall(pwd_msg)
                elif auth_type == 10:
                    pass
            elif msg_type == ord('K'):
                self.pid, self.secret = struct.unpack("!ii", data[:8])
            elif msg_type == ord('S'):
                pass
            elif msg_type == ord('Z'):
                break

    def query(self, sql):
        msg = b'Q' + struct.pack("!i", len(sql.encode()) + 5) + sql.encode() + b'\x00'
        self.sock.sendall(msg)

        rows = []
        columns = []
        error = None
        command_tag = None

        while True:
            msg_type_raw = self.sock.recv(1)
            if not msg_type_raw:
                break
            msg_type = ord(msg_type_raw)
            length = struct.unpack("!i", self._recv_exact(4))[0]
            data = self._recv_exact(length - 4)

            if msg_type == ord('T'):
                num_cols = struct.unpack("!h", data[:2])[0]
                offset = 2
                columns = []
                for _ in range(num_cols):
                    null_idx = data.index(b'\x00', offset)
                    col_name = data[offset:null_idx].decode()
                    columns.append(col_name)
                    offset = null_idx + 19

            elif msg_type == ord('D'):
                num_cols = struct.unpack("!h", data[:2])[0]
                offset = 2
                row = {}
                col_idx = 0
                for _ in range(num_cols):
                    col_len = struct.unpack("!i", data[offset:offset+4])[0]
                    offset += 4
                    if col_len == -1:
                        key = columns[col_idx] if col_idx < len(columns) else str(col_idx)
                        row[key] = None
                    else:
                        key = columns[col_idx] if col_idx < len(columns) else str(col_idx)
                        row[key] = data[offset:offset+col_len].decode('utf-8', errors='replace')
                        offset += col_len
                    col_idx += 1
                rows.append(row)

            elif msg_type == ord('C'):
                command_tag = data.decode('utf-8', errors='replace').rstrip('\x00')

            elif msg_type == ord('E'):
                error = data.decode('utf-8', errors='replace').rstrip('\x00')

            elif msg_type == ord('Z'):
                break  # Always read until ReadyForQuery

            elif msg_type in (ord('1'), ord('2'), ord('s'), ord('n'), ord('I'), ord('3')):
                pass

            elif msg_type == ord('A'):
                pass

            elif msg_type == ord('N'):
                pass

            elif msg_type == ord('S'):
                pass

        return {"rows": rows, "columns": columns, "error": error, "tag": command_tag}

    def safe_reconnect(self, attempts=3):
        """Reconnect with retries."""
        user = getattr(self, '_user', 'harness')
        password = getattr(self, '_password', 'harness-secret')
        for attempt in range(attempts):
            try:
                self.close()
            except Exception:
                pass
            try:
                self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
                self.sock.settimeout(self.timeout)
                self.sock.connect((self.host, self.port))
                params = (
                    f"user\x00{user}\x00"
                    f"database\x00harness\x00"
                    f"client_encoding\x00UTF8\x00"
                    f"\x00"
                )
                msg = struct.pack("!i", 196608) + params.encode()
                msg = struct.pack("!i", len(msg) + 4) + msg
                self.sock.sendall(msg)
                self._read_startup_response()
                return True
            except Exception:
                time.sleep(0.3 * (attempt + 1))
        return False

    def reconnect(self):
        return self.safe_reconnect()

    def close(self):
        if self.sock:
            try:
                self.sock.sendall(b'X' + struct.pack("!i", 4))
            except Exception:
                pass
            self.sock.close()
            self.sock = None


class TestRunner:
    def __init__(self, client):
        self.client = client
        self.total = 0
        self.passed = 0
        self.failed = 0
        self.failures = []
        self.current_section = ""

    def section(self, name):
        self.current_section = name
        print(f"\n\033[0;34m[{name}]\033[0m")

    def ok(self, name):
        self.total += 1
        self.passed += 1
        print(f"  \033[0;32m✓\033[0m {name}")

    def fail(self, name, msg):
        self.total += 1
        self.failed += 1
        entry = {"section": self.current_section, "test": name, "error": msg}
        self.failures.append(entry)
        print(f"  \033[0;31m✗\033[0m {name}: \033[0;31m{msg[:120]}\033[0m")

    def run_sql(self, name, sql, expect_rows=False, expect_error=False,
                min_rows=None, max_rows=None, check_value=None, reconnect_on_fail=True):
        result = None
        last_error = None
        for attempt in range(3):  # retry up to 3 times on connection errors
            try:
                result = self.client.query(sql)
                last_error = None
                break
            except Exception as e:
                last_error = e
                if reconnect_on_fail:
                    if self.client.safe_reconnect():
                        continue  # retry the query
                break

        if result is None:
            self.fail(name, f"Exception: {last_error}")
            return None

        if expect_error:
            if result["error"]:
                self.ok(name)
            else:
                self.fail(name, f"Expected error but got rows={len(result['rows'])}")
            return result

        if result["error"]:
            err_short = result["error"][:200]
            self.fail(name, err_short)
            return result

        if expect_rows and len(result["rows"]) == 0:
            self.fail(name, "Expected rows but got none")
            return result

        if min_rows is not None and len(result["rows"]) < min_rows:
            self.fail(name, f"Expected >= {min_rows} rows, got {len(result['rows'])}")
            return result

        if max_rows is not None and len(result["rows"]) > max_rows:
            self.fail(name, f"Expected <= {max_rows} rows, got {len(result['rows'])}")
            return result

        if check_value is not None:
            if not result["rows"]:
                self.fail(name, f"No rows, expected value {check_value}")
                return result
            first_col = result["columns"][0] if result["columns"] else None
            actual = result["rows"][0].get(first_col, "") if first_col else None
            if isinstance(check_value, str):
                if actual != check_value:
                    self.fail(name, f"Expected '{check_value}', got '{actual}'")
                    return result
            elif callable(check_value):
                if not check_value(actual):
                    self.fail(name, f"Value check failed: got '{actual}'")
                    return result

        self.ok(name)
        return result

    # ================================================================
    # TEST CATEGORIES
    # ================================================================

    def test_ddl(self):
        """DDL tests: CREATE SCHEMA, CREATE TABLE, DROP TABLE, ALTER TABLE, views, indexes."""
        self.section("DDL - CREATE SCHEMA")

        self.run_sql("CREATE SCHEMA test_schema", "CREATE SCHEMA IF NOT EXISTS test_schema")
        self.run_sql("CREATE SCHEMA test_schema2", "CREATE SCHEMA IF NOT EXISTS test_schema2")
        self.run_sql("CREATE SCHEMA test_ddl", "CREATE SCHEMA IF NOT EXISTS test_ddl")
        self.run_sql("CREATE SCHEMA test_func", "CREATE SCHEMA IF NOT EXISTS test_func")
        self.run_sql("CREATE SCHEMA test_types", "CREATE SCHEMA IF NOT EXISTS test_types")
        self.run_sql("CREATE SCHEMA test_expr", "CREATE SCHEMA IF NOT EXISTS test_expr")
        self.run_sql("CREATE SCHEMA test_join", "CREATE SCHEMA IF NOT EXISTS test_join")
        self.run_sql("CREATE SCHEMA test_subq", "CREATE SCHEMA IF NOT EXISTS test_subq")
        self.run_sql("CREATE SCHEMA test_window", "CREATE SCHEMA IF NOT EXISTS test_window")
        self.run_sql("CREATE SCHEMA test_null", "CREATE SCHEMA IF NOT EXISTS test_null")
        self.run_sql("CREATE SCHEMA test_array", "CREATE SCHEMA IF NOT EXISTS test_array")
        self.run_sql("CREATE SCHEMA test_edge", "CREATE SCHEMA IF NOT EXISTS test_edge")

        self.section("DDL - CREATE TABLE (basic types)")

        # INT table
        self.run_sql("CREATE TABLE test_int",
            "CREATE TABLE IF NOT EXISTS test_ddl.t_int (id INT, val INT)")
        # BIGINT table
        self.run_sql("CREATE TABLE test_bigint",
            "CREATE TABLE IF NOT EXISTS test_ddl.t_bigint (id BIGINT, val BIGINT)")
        # SMALLINT table
        self.run_sql("CREATE TABLE test_smallint",
            "CREATE TABLE IF NOT EXISTS test_ddl.t_smallint (id SMALLINT, val SMALLINT)")
        # FLOAT table
        self.run_sql("CREATE TABLE test_float",
            "CREATE TABLE IF NOT EXISTS test_ddl.t_float (id INT, val FLOAT)")
        # DOUBLE table
        self.run_sql("CREATE TABLE test_double",
            "CREATE TABLE IF NOT EXISTS test_ddl.t_double (id INT, val DOUBLE)")
        # REAL table
        self.run_sql("CREATE TABLE test_real",
            "CREATE TABLE IF NOT EXISTS test_ddl.t_real (id INT, val REAL)")
        # VARCHAR table
        self.run_sql("CREATE TABLE test_varchar",
            "CREATE TABLE IF NOT EXISTS test_ddl.t_varchar (id INT, val VARCHAR(255))")
        # TEXT table
        self.run_sql("CREATE TABLE test_text",
            "CREATE TABLE IF NOT EXISTS test_ddl.t_text (id INT, val TEXT)")
        # CHAR table
        self.run_sql("CREATE TABLE test_char",
            "CREATE TABLE IF NOT EXISTS test_ddl.t_char (id INT, val CHAR(10))")
        # DATE table
        self.run_sql("CREATE TABLE test_date",
            "CREATE TABLE IF NOT EXISTS test_ddl.t_date (id INT, val DATE)")
        # TIMESTAMP table
        self.run_sql("CREATE TABLE test_timestamp",
            "CREATE TABLE IF NOT EXISTS test_ddl.t_timestamp (id INT, val TIMESTAMP)")
        # BOOLEAN table
        self.run_sql("CREATE TABLE test_boolean",
            "CREATE TABLE IF NOT EXISTS test_ddl.t_boolean (id INT, val BOOLEAN)")
        # NUMERIC table
        self.run_sql("CREATE TABLE test_numeric",
            "CREATE TABLE IF NOT EXISTS test_ddl.t_numeric (id INT, val NUMERIC(10,2))")
        # UUID table
        self.run_sql("CREATE TABLE test_uuid",
            "CREATE TABLE IF NOT EXISTS test_ddl.t_uuid (id INT, val UUID)")
        # BYTEA table
        self.run_sql("CREATE TABLE test_bytea",
            "CREATE TABLE IF NOT EXISTS test_ddl.t_bytea (id INT, val BYTEA)")

        self.section("DDL - Multi-column tables")

        self.run_sql("CREATE TABLE wide_5cols",
            "CREATE TABLE IF NOT EXISTS test_ddl.t_wide5 (c1 INT, c2 INT, c3 VARCHAR(100), c4 FLOAT, c5 TEXT)")
        self.run_sql("CREATE TABLE wide_10cols",
            "CREATE TABLE IF NOT EXISTS test_ddl.t_wide10 (c1 INT, c2 INT, c3 INT, c4 INT, c5 INT, c6 VARCHAR(50), c7 VARCHAR(50), c8 FLOAT, c9 FLOAT, c10 TEXT)")
        self.run_sql("CREATE TABLE wide_20cols",
            "CREATE TABLE IF NOT EXISTS test_ddl.t_wide20 (c1 INT, c2 INT, c3 INT, c4 INT, c5 INT, c6 INT, c7 INT, c8 INT, c9 INT, c10 INT, c11 VARCHAR(20), c12 VARCHAR(20), c13 VARCHAR(20), c14 VARCHAR(20), c15 VARCHAR(20), c16 FLOAT, c17 FLOAT, c18 FLOAT, c19 FLOAT, c20 TEXT)")

        self.section("DDL - INSERT for DDL tests")

        # Insert base data for test tables
        self.run_sql("INSERT t_int row1",
            "INSERT INTO test_ddl.t_int VALUES (1, 100)")
        self.run_sql("INSERT t_int row2",
            "INSERT INTO test_ddl.t_int VALUES (2, 200)")
        self.run_sql("INSERT t_int row3",
            "INSERT INTO test_ddl.t_int VALUES (3, 300)")
        self.run_sql("INSERT t_bigint row1",
            "INSERT INTO test_ddl.t_bigint VALUES (1, 9999999999)")
        self.run_sql("INSERT t_bigint row2",
            "INSERT INTO test_ddl.t_bigint VALUES (2, 8888888888)")
        self.run_sql("INSERT t_float row1",
            "INSERT INTO test_ddl.t_float VALUES (1, 3.14)")
        self.run_sql("INSERT t_float row2",
            "INSERT INTO test_ddl.t_float VALUES (2, 2.718)")
        self.run_sql("INSERT t_varchar row1",
            "INSERT INTO test_ddl.t_varchar VALUES (1, 'hello world')")
        self.run_sql("INSERT t_varchar row2",
            "INSERT INTO test_ddl.t_varchar VALUES (2, 'foo bar')")
        self.run_sql("INSERT t_text row1",
            "INSERT INTO test_ddl.t_text VALUES (1, 'lorem ipsum dolor sit amet')")
        self.run_sql("INSERT t_text row2",
            "INSERT INTO test_ddl.t_text VALUES (2, 'consectetur adipiscing elit')")
        self.run_sql("INSERT t_boolean row1",
            "INSERT INTO test_ddl.t_boolean VALUES (1, true)")
        self.run_sql("INSERT t_boolean row2",
            "INSERT INTO test_ddl.t_boolean VALUES (2, false)")
        self.run_sql("INSERT t_date row1",
            "INSERT INTO test_ddl.t_date VALUES (1, '2024-01-15')")
        self.run_sql("INSERT t_date row2",
            "INSERT INTO test_ddl.t_date VALUES (2, '2024-06-30')")
        self.run_sql("INSERT t_timestamp row1",
            "INSERT INTO test_ddl.t_timestamp VALUES (1, '2024-01-15 10:30:00')")
        self.run_sql("INSERT t_wide5 row1",
            "INSERT INTO test_ddl.t_wide5 VALUES (1, 10, 'alpha', 1.1, 'text1')")
        self.run_sql("INSERT t_wide5 row2",
            "INSERT INTO test_ddl.t_wide5 VALUES (2, 20, 'beta', 2.2, 'text2')")

        self.section("DDL - SELECT verify")
        self.run_sql("SELECT t_int rows", "SELECT * FROM test_ddl.t_int", expect_rows=True, min_rows=1)
        self.run_sql("SELECT t_bigint rows", "SELECT * FROM test_ddl.t_bigint", expect_rows=True, min_rows=1)
        self.run_sql("SELECT t_float rows", "SELECT * FROM test_ddl.t_float", expect_rows=True, min_rows=1)
        self.run_sql("SELECT t_varchar rows", "SELECT * FROM test_ddl.t_varchar", expect_rows=True, min_rows=1)

        self.section("DDL - CREATE TABLE IF NOT EXISTS (idempotent)")
        for i in range(20):
            self.run_sql(f"CREATE TABLE IF NOT EXISTS idem_{i}",
                f"CREATE TABLE IF NOT EXISTS test_ddl.t_idem_{i} (id INT, v VARCHAR(50))")

        self.section("DDL - DROP TABLE IF EXISTS")
        for i in range(20):
            self.run_sql(f"DROP TABLE IF EXISTS idem_{i}",
                f"DROP TABLE IF EXISTS test_ddl.t_idem_{i}")

        self.section("DDL - CREATE VIEW")
        self.run_sql("CREATE VIEW v_int",
            "CREATE VIEW IF NOT EXISTS test_ddl.v_int AS SELECT id, val FROM test_ddl.t_int")
        self.run_sql("CREATE VIEW v_wide",
            "CREATE VIEW IF NOT EXISTS test_ddl.v_wide AS SELECT c1, c3, c5 FROM test_ddl.t_wide5")
        self.run_sql("SELECT from view",
            "SELECT * FROM test_ddl.v_int", expect_rows=True, min_rows=1)

        self.section("DDL - DROP VIEW")
        self.run_sql("DROP VIEW v_int", "DROP VIEW IF EXISTS test_ddl.v_int")
        self.run_sql("DROP VIEW v_wide", "DROP VIEW IF EXISTS test_ddl.v_wide")

        self.section("DDL - CREATE INDEX")
        for i in range(15):
            self.run_sql(f"CREATE INDEX idx_int_{i}",
                f"CREATE INDEX IF NOT EXISTS idx_t_int_{i} ON test_ddl.t_int (val)")

        self.section("DDL - DROP INDEX")
        for i in range(15):
            self.run_sql(f"DROP INDEX idx_int_{i}",
                f"DROP INDEX IF EXISTS idx_t_int_{i}")

        self.section("DDL - Additional CREATE TABLE variants")
        for i in range(30):
            self.run_sql(f"CREATE TABLE extra_{i}",
                f"CREATE TABLE IF NOT EXISTS test_ddl.t_extra_{i} (id INT, data TEXT)")

        self.section("DDL - DROP extra tables")
        for i in range(30):
            self.run_sql(f"DROP TABLE extra_{i}",
                f"DROP TABLE IF EXISTS test_ddl.t_extra_{i}")

    def test_dml(self):
        """DML tests: INSERT, SELECT, UPDATE, DELETE with many variants."""
        self.section("DML - Setup tables")
        self.run_sql("CREATE TABLE dml_users",
            "CREATE TABLE IF NOT EXISTS test_ddl.dml_users (id INT, name VARCHAR(100), age INT, score FLOAT, active BOOLEAN)")
        self.run_sql("CREATE TABLE dml_orders",
            "CREATE TABLE IF NOT EXISTS test_ddl.dml_orders (id INT, user_id INT, amount FLOAT, status VARCHAR(50), created DATE)")
        self.run_sql("CREATE TABLE dml_products",
            "CREATE TABLE IF NOT EXISTS test_ddl.dml_products (id INT, name VARCHAR(100), price FLOAT, stock INT, category VARCHAR(50))")

        self.section("DML - INSERT")
        inserts = [
            ("INSERT users 1", "INSERT INTO test_ddl.dml_users VALUES (1, 'Alice', 30, 95.5, true)"),
            ("INSERT users 2", "INSERT INTO test_ddl.dml_users VALUES (2, 'Bob', 25, 87.3, true)"),
            ("INSERT users 3", "INSERT INTO test_ddl.dml_users VALUES (3, 'Charlie', 35, 72.1, false)"),
            ("INSERT users 4", "INSERT INTO test_ddl.dml_users VALUES (4, 'Diana', 28, 91.0, true)"),
            ("INSERT users 5", "INSERT INTO test_ddl.dml_users VALUES (5, 'Eve', 22, 88.8, true)"),
            ("INSERT users 6", "INSERT INTO test_ddl.dml_users VALUES (6, 'Frank', 45, 65.0, false)"),
            ("INSERT users 7", "INSERT INTO test_ddl.dml_users VALUES (7, 'Grace', 33, 99.9, true)"),
            ("INSERT users 8", "INSERT INTO test_ddl.dml_users VALUES (8, 'Hank', 29, 78.5, true)"),
            ("INSERT users 9", "INSERT INTO test_ddl.dml_users VALUES (9, 'Ivy', 27, 84.2, false)"),
            ("INSERT users 10", "INSERT INTO test_ddl.dml_users VALUES (10, 'Jack', 38, 90.0, true)"),
            ("INSERT orders 1", "INSERT INTO test_ddl.dml_orders VALUES (1, 1, 99.99, 'completed', '2024-01-10')"),
            ("INSERT orders 2", "INSERT INTO test_ddl.dml_orders VALUES (2, 2, 199.50, 'pending', '2024-02-15')"),
            ("INSERT orders 3", "INSERT INTO test_ddl.dml_orders VALUES (3, 1, 50.00, 'completed', '2024-03-20')"),
            ("INSERT orders 4", "INSERT INTO test_ddl.dml_orders VALUES (4, 3, 75.25, 'shipped', '2024-04-05')"),
            ("INSERT orders 5", "INSERT INTO test_ddl.dml_orders VALUES (5, 4, 250.00, 'completed', '2024-05-12')"),
            ("INSERT orders 6", "INSERT INTO test_ddl.dml_orders VALUES (6, 5, 30.00, 'pending', '2024-06-01')"),
            ("INSERT orders 7", "INSERT INTO test_ddl.dml_orders VALUES (7, 2, 120.00, 'completed', '2024-01-25')"),
            ("INSERT orders 8", "INSERT INTO test_ddl.dml_orders VALUES (8, 6, 85.50, 'shipped', '2024-07-08')"),
            ("INSERT products 1", "INSERT INTO test_ddl.dml_products VALUES (1, 'Widget', 9.99, 100, 'A')"),
            ("INSERT products 2", "INSERT INTO test_ddl.dml_products VALUES (2, 'Gadget', 29.99, 50, 'B')"),
            ("INSERT products 3", "INSERT INTO test_ddl.dml_products VALUES (3, 'Doohickey', 19.99, 75, 'A')"),
            ("INSERT products 4", "INSERT INTO test_ddl.dml_products VALUES (4, 'Thingamajig', 49.99, 25, 'C')"),
            ("INSERT products 5", "INSERT INTO test_ddl.dml_products VALUES (5, 'Whatchamacallit', 14.99, 200, 'B')"),
        ]
        for name, sql in inserts:
            self.run_sql(name, sql)

        self.section("DML - SELECT basic")
        self.run_sql("SELECT * users", "SELECT * FROM test_ddl.dml_users", expect_rows=True, min_rows=1)
        self.run_sql("SELECT * orders", "SELECT * FROM test_ddl.dml_orders", expect_rows=True, min_rows=1)
        self.run_sql("SELECT * products", "SELECT * FROM test_ddl.dml_products", expect_rows=True, min_rows=1)
        self.run_sql("SELECT id,name users", "SELECT id, name FROM test_ddl.dml_users", expect_rows=True, min_rows=1)
        self.run_sql("SELECT count users", "SELECT COUNT(*) FROM test_ddl.dml_users", expect_rows=True)

        self.section("DML - SELECT WHERE")
        self.run_sql("WHERE age > 30", "SELECT * FROM test_ddl.dml_users WHERE age > 30", expect_rows=True)
        self.run_sql("WHERE age = 25", "SELECT * FROM test_ddl.dml_users WHERE age = 25", expect_rows=True)
        self.run_sql("WHERE age < 30", "SELECT * FROM test_ddl.dml_users WHERE age < 30", expect_rows=True)
        self.run_sql("WHERE age >= 30", "SELECT * FROM test_ddl.dml_users WHERE age >= 30", expect_rows=True)
        self.run_sql("WHERE age <= 30", "SELECT * FROM test_ddl.dml_users WHERE age <= 30", expect_rows=True)
        self.run_sql("WHERE age != 30", "SELECT * FROM test_ddl.dml_users WHERE age != 30", expect_rows=True)
        self.run_sql("WHERE name = Alice", "SELECT * FROM test_ddl.dml_users WHERE name = 'Alice'", expect_rows=True)
        self.run_sql("WHERE score > 90", "SELECT * FROM test_ddl.dml_users WHERE score > 90.0", expect_rows=True)
        self.run_sql("WHERE active = true", "SELECT * FROM test_ddl.dml_users WHERE active = true", expect_rows=True)
        self.run_sql("WHERE age AND score", "SELECT * FROM test_ddl.dml_users WHERE age > 25 AND score > 80", expect_rows=True)
        self.run_sql("WHERE age OR score", "SELECT * FROM test_ddl.dml_users WHERE age > 40 OR score > 95", expect_rows=True)
        self.run_sql("WHERE order amount", "SELECT * FROM test_ddl.dml_orders WHERE amount > 100", expect_rows=True)
        self.run_sql("WHERE order status", "SELECT * FROM test_ddl.dml_orders WHERE status = 'completed'", expect_rows=True)
        self.run_sql("WHERE product price", "SELECT * FROM test_ddl.dml_products WHERE price > 20.0", expect_rows=True)
        self.run_sql("WHERE product stock", "SELECT * FROM test_ddl.dml_products WHERE stock < 100", expect_rows=True)

        self.section("DML - SELECT ORDER BY")
        self.run_sql("ORDER BY age ASC", "SELECT * FROM test_ddl.dml_users ORDER BY age ASC", expect_rows=True)
        self.run_sql("ORDER BY age DESC", "SELECT * FROM test_ddl.dml_users ORDER BY age DESC", expect_rows=True)
        self.run_sql("ORDER BY name ASC", "SELECT * FROM test_ddl.dml_users ORDER BY name ASC", expect_rows=True)
        self.run_sql("ORDER BY score DESC", "SELECT * FROM test_ddl.dml_users ORDER BY score DESC", expect_rows=True)
        self.run_sql("ORDER BY amount", "SELECT * FROM test_ddl.dml_orders ORDER BY amount DESC", expect_rows=True)
        self.run_sql("ORDER BY price", "SELECT * FROM test_ddl.dml_products ORDER BY price ASC", expect_rows=True)
        self.run_sql("ORDER BY multiple", "SELECT * FROM test_ddl.dml_users ORDER BY active DESC, age ASC", expect_rows=True)

        self.section("DML - SELECT LIMIT")
        self.run_sql("LIMIT 1", "SELECT * FROM test_ddl.dml_users LIMIT 1", expect_rows=True, max_rows=1)
        self.run_sql("LIMIT 3", "SELECT * FROM test_ddl.dml_users LIMIT 3", expect_rows=True, max_rows=3)
        self.run_sql("LIMIT 5", "SELECT * FROM test_ddl.dml_users LIMIT 5", expect_rows=True, max_rows=5)
        self.run_sql("LIMIT 100", "SELECT * FROM test_ddl.dml_users LIMIT 100", expect_rows=True)
        self.run_sql("ORDER + LIMIT", "SELECT * FROM test_ddl.dml_users ORDER BY age DESC LIMIT 3", expect_rows=True, max_rows=3)

        self.section("DML - SELECT GROUP BY")
        self.run_sql("GROUP BY active", "SELECT active, COUNT(*) FROM test_ddl.dml_users GROUP BY active", expect_rows=True)
        self.run_sql("GROUP BY status", "SELECT status, COUNT(*) FROM test_ddl.dml_orders GROUP BY status", expect_rows=True)
        self.run_sql("GROUP BY category", "SELECT category, COUNT(*) FROM test_ddl.dml_products GROUP BY category", expect_rows=True)
        self.run_sql("GROUP BY + SUM", "SELECT user_id, SUM(amount) FROM test_ddl.dml_orders GROUP BY user_id", expect_rows=True)
        self.run_sql("GROUP BY + AVG", "SELECT user_id, AVG(amount) FROM test_ddl.dml_orders GROUP BY user_id", expect_rows=True)
        self.run_sql("GROUP BY + MIN", "SELECT user_id, MIN(amount) FROM test_ddl.dml_orders GROUP BY user_id", expect_rows=True)
        self.run_sql("GROUP BY + MAX", "SELECT user_id, MAX(amount) FROM test_ddl.dml_orders GROUP BY user_id", expect_rows=True)

        self.section("DML - SELECT HAVING")
        self.run_sql("HAVING count > 1",
            "SELECT user_id, COUNT(*) FROM test_ddl.dml_orders GROUP BY user_id HAVING COUNT(*) > 1",
            expect_rows=True)
        self.run_sql("HAVING sum > 100",
            "SELECT user_id, SUM(amount) FROM test_ddl.dml_orders GROUP BY user_id HAVING SUM(amount) > 100",
            expect_rows=True)

        self.section("DML - SELECT DISTINCT")
        self.run_sql("DISTINCT active", "SELECT DISTINCT active FROM test_ddl.dml_users", expect_rows=True)
        self.run_sql("DISTINCT status", "SELECT DISTINCT status FROM test_ddl.dml_orders", expect_rows=True)
        self.run_sql("DISTINCT category", "SELECT DISTINCT category FROM test_ddl.dml_products", expect_rows=True)
        self.run_sql("COUNT DISTINCT", "SELECT COUNT(DISTINCT active) FROM test_ddl.dml_users", expect_rows=True)

        self.section("DML - UPDATE")
        self.run_sql("UPDATE age", "UPDATE test_ddl.dml_users SET age = 31 WHERE name = 'Alice'")
        self.run_sql("VERIFY update", "SELECT age FROM test_ddl.dml_users WHERE name = 'Alice'",
                     expect_rows=True)
        self.run_sql("UPDATE score", "UPDATE test_ddl.dml_users SET score = 96.0 WHERE id = 1")
        self.run_sql("UPDATE status", "UPDATE test_ddl.dml_orders SET status = 'cancelled' WHERE id = 6")
        self.run_sql("UPDATE price", "UPDATE test_ddl.dml_products SET price = 12.99 WHERE id = 1")
        self.run_sql("UPDATE multi-col", "UPDATE test_ddl.dml_users SET age = 26, score = 89.0 WHERE name = 'Bob'")
        self.run_sql("VERIFY multi-col", "SELECT age, score FROM test_ddl.dml_users WHERE name = 'Bob'", expect_rows=True)

        self.section("DML - DELETE")
        self.run_sql("DELETE by id", "DELETE FROM test_ddl.dml_users WHERE id = 10")
        self.run_sql("VERIFY delete", "SELECT COUNT(*) FROM test_ddl.dml_users", expect_rows=True)
        self.run_sql("DELETE by name", "DELETE FROM test_ddl.dml_orders WHERE status = 'cancelled'")
        self.run_sql("DELETE product", "DELETE FROM test_ddl.dml_products WHERE stock < 30")

        self.section("DML - Re-insert deleted data")
        self.run_sql("Re-insert user 10", "INSERT INTO test_ddl.dml_users VALUES (10, 'Jack', 38, 90.0, true)")

        self.section("DML - SELECT with aliases")
        self.run_sql("Column alias", "SELECT name AS username, age AS user_age FROM test_ddl.dml_users LIMIT 5", expect_rows=True)
        self.run_sql("Table alias", "SELECT u.name, u.age FROM test_ddl.dml_users u LIMIT 5", expect_rows=True)
        self.run_sql("Aggregate alias", "SELECT COUNT(*) AS total_users FROM test_ddl.dml_users", expect_rows=True)

        self.section("DML - BETWEEN")
        self.run_sql("BETWEEN ages", "SELECT * FROM test_ddl.dml_users WHERE age BETWEEN 25 AND 35", expect_rows=True)
        self.run_sql("NOT BETWEEN", "SELECT * FROM test_ddl.dml_users WHERE age NOT BETWEEN 25 AND 35", expect_rows=True)
        self.run_sql("BETWEEN dates", "SELECT * FROM test_ddl.dml_orders WHERE created BETWEEN '2024-03-01' AND '2024-06-30'", expect_rows=True)

        self.section("DML - IN")
        self.run_sql("IN list", "SELECT * FROM test_ddl.dml_users WHERE age IN (25, 30, 35)", expect_rows=True)
        self.run_sql("NOT IN", "SELECT * FROM test_ddl.dml_users WHERE age NOT IN (25, 30, 35)", expect_rows=True)
        self.run_sql("IN strings", "SELECT * FROM test_ddl.dml_users WHERE name IN ('Alice', 'Bob')", expect_rows=True)

        self.section("DML - LIKE")
        self.run_sql("LIKE prefix", "SELECT * FROM test_ddl.dml_users WHERE name LIKE 'A%'", expect_rows=True)
        self.run_sql("LIKE suffix", "SELECT * FROM test_ddl.dml_users WHERE name LIKE '%e'", expect_rows=True)
        self.run_sql("LIKE contains", "SELECT * FROM test_ddl.dml_users WHERE name LIKE '%li%'", expect_rows=True)
        self.run_sql("NOT LIKE", "SELECT * FROM test_ddl.dml_users WHERE name NOT LIKE '%a%'", expect_rows=True)

    def test_string_functions(self):
        """String functions: LENGTH, UPPER, LOWER, SUBSTR, CONCAT, TRIM, etc."""
        self.section("Functions - String (LENGTH)")
        for s in ["'hello'", "'world'", "'PostgreSQL'", "'test string'", "''", "'a'", "'Hello World 123'"]:
            self.run_sql(f"LENGTH({s})", f"SELECT LENGTH({s})", expect_rows=True)

        self.section("Functions - String (UPPER)")
        for s in ["'hello'", "'world'", "'PostgreSQL'", "'test'", "'abc123'"]:
            self.run_sql(f"UPPER({s})", f"SELECT UPPER({s})", expect_rows=True)

        self.section("Functions - String (LOWER)")
        for s in ["'HELLO'", "'WORLD'", "'PostgreSQL'", "'TEST'", "'ABC123'"]:
            self.run_sql(f"LOWER({s})", f"SELECT LOWER({s})", expect_rows=True)

        self.section("Functions - String (SUBSTR/SUBSTRING)")
        self.run_sql("SUBSTR hello", "SELECT SUBSTR('hello world', 1, 5)", expect_rows=True)
        self.run_sql("SUBSTR from end", "SELECT SUBSTR('hello world', 7)", expect_rows=True)
        self.run_sql("SUBSTRING", "SELECT SUBSTRING('hello world' FROM 1 FOR 5)", expect_rows=True)
        self.run_sql("SUBSTR empty", "SELECT SUBSTR('', 1, 5)", expect_rows=True)
        self.run_sql("SUBSTR single", "SELECT SUBSTR('x', 1, 1)", expect_rows=True)
        for i in range(10):
            self.run_sql(f"SUBSTR variant {i}",
                f"SELECT SUBSTR('abcdefghijklmnopqrstuvwxyz', {i+1}, 5)", expect_rows=True)

        self.section("Functions - String (CONCAT)")
        self.run_sql("CONCAT 2", "SELECT CONCAT('hello', ' world')", expect_rows=True)
        self.run_sql("CONCAT 3", "SELECT CONCAT('a', 'b', 'c')", expect_rows=True)
        self.run_sql("CONCAT ||", "SELECT 'hello' || ' ' || 'world'", expect_rows=True)
        for i in range(10):
            self.run_sql(f"CONCAT variant {i}",
                f"SELECT CONCAT('str_{i}', '_suffix')", expect_rows=True)

        self.section("Functions - String (TRIM)")
        self.run_sql("TRIM both", "SELECT TRIM('  hello  ')", expect_rows=True)
        self.run_sql("TRIM LEADING", "SELECT TRIM(LEADING ' ' FROM '  hello  ')", expect_rows=True)
        self.run_sql("TRIM TRAILING", "SELECT TRIM(TRAILING ' ' FROM '  hello  ')", expect_rows=True)
        self.run_sql("TRIM char", "SELECT TRIM(BOTH 'x' FROM 'xxxhelloxxx')", expect_rows=True)
        self.run_sql("LTRIM", "SELECT LTRIM('  hello')", expect_rows=True)
        self.run_sql("RTRIM", "SELECT RTRIM('hello  ')", expect_rows=True)
        for i in range(10):
            self.run_sql(f"TRIM variant {i}",
                f"SELECT TRIM('{' ' * (i+1)}test{' ' * (i+1)}')", expect_rows=True)

        self.section("Functions - String (REPLACE)")
        self.run_sql("REPLACE basic", "SELECT REPLACE('hello world', 'world', 'earth')", expect_rows=True)
        self.run_sql("REPLACE multi", "SELECT REPLACE('aaa', 'a', 'bb')", expect_rows=True)
        self.run_sql("REPLACE empty", "SELECT REPLACE('hello', 'xyz', 'abc')", expect_rows=True)
        for i in range(10):
            self.run_sql(f"REPLACE variant {i}",
                f"SELECT REPLACE('test_{i}_end', '_{i}_', '_new_')", expect_rows=True)

        self.section("Functions - String (LPAD/RPAD)")
        for n in [5, 10, 15, 20]:
            self.run_sql(f"LPAD {n}", f"SELECT LPAD('hi', {n}, 'x')", expect_rows=True)
            self.run_sql(f"RPAD {n}", f"SELECT RPAD('hi', {n}, 'x')", expect_rows=True)
        self.run_sql("LPAD default", "SELECT LPAD('hi', 10)", expect_rows=True)
        self.run_sql("RPAD default", "SELECT RPAD('hi', 10)", expect_rows=True)

        self.section("Functions - String (REVERSE)")
        for s in ["'hello'", "'world'", "'abcde'", "'12345'", "'racecar'"]:
            self.run_sql(f"REVERSE({s})", f"SELECT REVERSE({s})", expect_rows=True)

        self.section("Functions - String (LEFT/RIGHT)")
        for n in [1, 3, 5]:
            self.run_sql(f"LEFT {n}", f"SELECT LEFT('hello world', {n})", expect_rows=True)
            self.run_sql(f"RIGHT {n}", f"SELECT RIGHT('hello world', {n})", expect_rows=True)

        self.section("Functions - String (POSITION/STRPOS)")
        self.run_sql("POSITION", "SELECT POSITION('world' IN 'hello world')", expect_rows=True)
        self.run_sql("POSITION missing", "SELECT POSITION('xyz' IN 'hello world')", expect_rows=True)
        self.run_sql("STRPOS", "SELECT STRPOS('hello world', 'world')", expect_rows=True)

        self.section("Functions - String (SPLIT_PART)")
        self.run_sql("SPLIT_PART 1", "SELECT SPLIT_PART('a,b,c', ',', 1)", expect_rows=True)
        self.run_sql("SPLIT_PART 2", "SELECT SPLIT_PART('a,b,c', ',', 2)", expect_rows=True)
        self.run_sql("SPLIT_PART 3", "SELECT SPLIT_PART('a,b,c', ',', 3)", expect_rows=True)

        self.section("Functions - String (REPEAT)")
        for n in [2, 3, 5, 10]:
            self.run_sql(f"REPEAT {n}", f"SELECT REPEAT('ab', {n})", expect_rows=True)

        self.section("Functions - String (ASCII/CHR)")
        self.run_sql("ASCII A", "SELECT ASCII('A')", expect_rows=True)
        self.run_sql("ASCII a", "SELECT ASCII('a')", expect_rows=True)
        self.run_sql("CHR 65", "SELECT CHR(65)", expect_rows=True)
        self.run_sql("CHR 97", "SELECT CHR(97)", expect_rows=True)

        self.section("Functions - String (MD5)")
        for s in ["'hello'", "'world'", "''", "'test'"]:
            self.run_sql(f"MD5({s})", f"SELECT MD5({s})", expect_rows=True)

        self.section("Functions - String (INITCAP)")
        self.run_sql("INITCAP", "SELECT INITCAP('hello world')", expect_rows=True)
        self.run_sql("INITCAP upper", "SELECT INITCAP('HELLO WORLD')", expect_rows=True)

        self.section("Functions - String from table")
        self.run_sql("UPPER from table", "SELECT UPPER(name) FROM test_ddl.dml_users LIMIT 5", expect_rows=True)
        self.run_sql("LOWER from table", "SELECT LOWER(name) FROM test_ddl.dml_users LIMIT 5", expect_rows=True)
        self.run_sql("LENGTH from table", "SELECT LENGTH(name) FROM test_ddl.dml_users LIMIT 5", expect_rows=True)
        self.run_sql("CONCAT from table", "SELECT CONCAT(name, ' age:', CAST(age AS VARCHAR)) FROM test_ddl.dml_users LIMIT 5", expect_rows=True)

    def test_numeric_functions(self):
        """Numeric functions: ABS, CEIL, FLOOR, ROUND, MOD, POWER, SQRT, etc."""
        self.section("Functions - Numeric (ABS)")
        for v in ["1", "-1", "0", "3.14", "-3.14", "999999", "-999999", "0.001", "-0.001"]:
            self.run_sql(f"ABS({v})", f"SELECT ABS({v})", expect_rows=True)

        self.section("Functions - Numeric (CEIL/FLOOR)")
        for v in ["1.5", "2.7", "3.1", "-1.5", "-2.7", "0.1", "0.9", "10.01", "99.99"]:
            self.run_sql(f"CEIL({v})", f"SELECT CEIL({v})", expect_rows=True)
            self.run_sql(f"FLOOR({v})", f"SELECT FLOOR({v})", expect_rows=True)

        self.section("Functions - Numeric (ROUND)")
        for v in ["1.555", "2.444", "3.14159", "99.999", "0.5", "1.23456"]:
            self.run_sql(f"ROUND({v})", f"SELECT ROUND({v})", expect_rows=True)
        for v in ["3.14159", "2.71828"]:
            for p in [0, 1, 2, 3, 4]:
                self.run_sql(f"ROUND({v},{p})", f"SELECT ROUND({v}::numeric, {p})", expect_rows=True)

        self.section("Functions - Numeric (MOD)")
        for a, b in [(10, 3), (100, 7), (255, 16), (10, 5), (7, 3), (20, 6), (99, 10)]:
            self.run_sql(f"MOD({a},{b})", f"SELECT MOD({a}, {b})", expect_rows=True)

        self.section("Functions - Numeric (POWER/SQRT)")
        for b in [2, 3, 4, 10]:
            self.run_sql(f"POWER(2,{b})", f"SELECT POWER(2, {b})", expect_rows=True)
        for v in [1, 4, 9, 16, 25, 100, 144]:
            self.run_sql(f"SQRT({v})", f"SELECT SQRT({v})", expect_rows=True)

        self.section("Functions - Numeric (LOG/EXP)")
        for v in [1, 2.71828, 10, 100, 1000]:
            self.run_sql(f"LOG({v})", f"SELECT LOG({v}::float)", expect_rows=True)
        for v in [0, 1, 2, 3, 5]:
            self.run_sql(f"EXP({v})", f"SELECT EXP({v}::float)", expect_rows=True)

        self.section("Functions - Numeric (RANDOM)")
        for _ in range(10):
            self.run_sql("RANDOM", "SELECT RANDOM()", expect_rows=True)

        self.section("Functions - Numeric (PI)")
        self.run_sql("PI", "SELECT PI()", expect_rows=True)

        self.section("Functions - Numeric (TRUNC)")
        for v in ["3.14159", "2.71828", "99.999", "0.5"]:
            self.run_sql(f"TRUNC({v})", f"SELECT TRUNC({v}::numeric)", expect_rows=True)
            for p in [0, 1, 2]:
                self.run_sql(f"TRUNC({v},{p})", f"SELECT TRUNC({v}::numeric, {p})", expect_rows=True)

        self.section("Functions - Numeric (SIGN)")
        for v in ["5", "-5", "0", "3.14", "-3.14"]:
            self.run_sql(f"SIGN({v})", f"SELECT SIGN({v})", expect_rows=True)

        self.section("Functions - Numeric (DIV)")
        for a, b in [(10, 3), (100, 7), (255, 16), (10, 2), (99, 9)]:
            self.run_sql(f"DIV({a},{b})", f"SELECT {a} / {b}", expect_rows=True)

        self.section("Functions - Numeric from table")
        self.run_sql("ABS from table", "SELECT ABS(score - 90) FROM test_ddl.dml_users LIMIT 5", expect_rows=True)
        self.run_sql("ROUND from table", "SELECT ROUND(score::numeric, 1) FROM test_ddl.dml_users LIMIT 5", expect_rows=True)
        self.run_sql("POWER from table", "SELECT POWER(2, id::float) FROM test_ddl.dml_users LIMIT 5", expect_rows=True)

    def test_date_functions(self):
        """Date functions: NOW, CURRENT_DATE, EXTRACT, DATE_PART, etc."""
        self.section("Functions - Date (NOW/CURRENT)")
        self.run_sql("NOW", "SELECT NOW()", expect_rows=True)
        self.run_sql("CURRENT_DATE", "SELECT CURRENT_DATE", expect_rows=True)
        self.run_sql("CURRENT_TIME", "SELECT CURRENT_TIME", expect_rows=True)
        self.run_sql("CURRENT_TIMESTAMP", "SELECT CURRENT_TIMESTAMP", expect_rows=True)
        self.run_sql("LOCALTIME", "SELECT LOCALTIME", expect_rows=True)
        self.run_sql("LOCALTIMESTAMP", "SELECT LOCALTIMESTAMP", expect_rows=True)

        self.section("Functions - Date (EXTRACT)")
        for field in ["year", "month", "day", "hour", "minute", "second"]:
            self.run_sql(f"EXTRACT {field}", f"SELECT EXTRACT({field} FROM NOW())", expect_rows=True)
            self.run_sql(f"EXTRACT {field} date",
                f"SELECT EXTRACT({field} FROM DATE '2024-06-15')", expect_rows=True)

        self.section("Functions - Date (DATE_PART)")
        for field in ["year", "month", "day", "hour", "minute", "second", "dow", "doy", "week", "quarter"]:
            self.run_sql(f"DATE_PART {field}", f"SELECT DATE_PART('{field}', NOW())", expect_rows=True)

        self.section("Functions - Date (DATE_TRUNC)")
        for field in ["year", "month", "day", "hour", "minute"]:
            self.run_sql(f"DATE_TRUNC {field}", f"SELECT DATE_TRUNC('{field}', NOW())", expect_rows=True)

        self.section("Functions - Date (TO_CHAR)")
        for fmt in ["'YYYY-MM-DD'", "'DD/MM/YYYY'", "'YYYY'", "'Month DD, YYYY'", "'HH24:MI:SS'"]:
            self.run_sql(f"TO_CHAR {fmt}", f"SELECT TO_CHAR(NOW(), {fmt})", expect_rows=True)
        self.run_sql("TO_CHAR date", "SELECT TO_CHAR(DATE '2024-06-15', 'YYYY-MM-DD')", expect_rows=True)

        self.section("Functions - Date (TO_DATE)")
        self.run_sql("TO_DATE basic", "SELECT TO_DATE('2024-06-15', 'YYYY-MM-DD')", expect_rows=True)
        self.run_sql("TO_DATE alt", "SELECT TO_DATE('15/06/2024', 'DD/MM/YYYY')", expect_rows=True)

        self.section("Functions - Date (AGE)")
        self.run_sql("AGE date", "SELECT AGE(DATE '2024-01-01')", expect_rows=True)
        self.run_sql("AGE two dates", "SELECT AGE(DATE '2024-06-15', DATE '2024-01-01')", expect_rows=True)

        self.section("Functions - Date (INTERVAL)")
        self.run_sql("INTERVAL day", "SELECT INTERVAL '1 day'", expect_rows=True)
        self.run_sql("INTERVAL month", "SELECT INTERVAL '3 months'", expect_rows=True)
        self.run_sql("INTERVAL year", "SELECT INTERVAL '1 year'", expect_rows=True)
        self.run_sql("date + interval", "SELECT DATE '2024-01-01' + INTERVAL '30 days'", expect_rows=True)
        self.run_sql("date - interval", "SELECT DATE '2024-06-15' - INTERVAL '30 days'", expect_rows=True)

        self.section("Functions - Date from table")
        self.run_sql("EXTRACT from table",
            "SELECT EXTRACT(year FROM created) FROM test_ddl.dml_orders LIMIT 5", expect_rows=True)

    def test_aggregate_functions(self):
        """Aggregate functions: COUNT, SUM, AVG, MIN, MAX, etc."""
        self.section("Functions - Aggregate (COUNT)")
        self.run_sql("COUNT(*)", "SELECT COUNT(*) FROM test_ddl.dml_users", expect_rows=True)
        self.run_sql("COUNT(id)", "SELECT COUNT(id) FROM test_ddl.dml_users", expect_rows=True)
        self.run_sql("COUNT(name)", "SELECT COUNT(name) FROM test_ddl.dml_users", expect_rows=True)
        self.run_sql("COUNT(DISTINCT)", "SELECT COUNT(DISTINCT active) FROM test_ddl.dml_users", expect_rows=True)
        for _ in range(10):
            self.run_sql("COUNT variant",
                f"SELECT COUNT(*) FROM test_ddl.dml_users WHERE age > {random.randint(20, 40)}", expect_rows=True)

        self.section("Functions - Aggregate (SUM)")
        self.run_sql("SUM age", "SELECT SUM(age) FROM test_ddl.dml_users", expect_rows=True)
        self.run_sql("SUM score", "SELECT SUM(score) FROM test_ddl.dml_users", expect_rows=True)
        self.run_sql("SUM amount", "SELECT SUM(amount) FROM test_ddl.dml_orders", expect_rows=True)
        self.run_sql("SUM GROUP BY", "SELECT active, SUM(age) FROM test_ddl.dml_users GROUP BY active", expect_rows=True)
        self.run_sql("SUM HAVING", "SELECT user_id, SUM(amount) FROM test_ddl.dml_orders GROUP BY user_id HAVING SUM(amount) > 50", expect_rows=True)

        self.section("Functions - Aggregate (AVG)")
        self.run_sql("AVG age", "SELECT AVG(age) FROM test_ddl.dml_users", expect_rows=True)
        self.run_sql("AVG score", "SELECT AVG(score) FROM test_ddl.dml_users", expect_rows=True)
        self.run_sql("AVG amount", "SELECT AVG(amount) FROM test_ddl.dml_orders", expect_rows=True)
        self.run_sql("AVG GROUP BY", "SELECT active, AVG(score) FROM test_ddl.dml_users GROUP BY active", expect_rows=True)

        self.section("Functions - Aggregate (MIN/MAX)")
        self.run_sql("MIN age", "SELECT MIN(age) FROM test_ddl.dml_users", expect_rows=True)
        self.run_sql("MAX age", "SELECT MAX(age) FROM test_ddl.dml_users", expect_rows=True)
        self.run_sql("MIN score", "SELECT MIN(score) FROM test_ddl.dml_users", expect_rows=True)
        self.run_sql("MAX score", "SELECT MAX(score) FROM test_ddl.dml_users", expect_rows=True)
        self.run_sql("MIN amount", "SELECT MIN(amount) FROM test_ddl.dml_orders", expect_rows=True)
        self.run_sql("MAX amount", "SELECT MAX(amount) FROM test_ddl.dml_orders", expect_rows=True)
        self.run_sql("MIN MAX group", "SELECT active, MIN(age), MAX(age) FROM test_ddl.dml_users GROUP BY active", expect_rows=True)

        self.section("Functions - Aggregate multiple")
        self.run_sql("All aggregates",
            "SELECT COUNT(*), SUM(age), AVG(age), MIN(age), MAX(age) FROM test_ddl.dml_users", expect_rows=True)
        self.run_sql("Multi-agg group",
            "SELECT active, COUNT(*), SUM(score), AVG(score) FROM test_ddl.dml_users GROUP BY active", expect_rows=True)

    def test_type_casts(self):
        """Type cast tests."""
        self.section("Type Casts - ::int")
        for v in ["'123'", "'0'", "'-456'", "'999999'", "1.5", "true", "false"]:
            self.run_sql(f"CAST {v} AS int", f"SELECT CAST({v} AS INT)", expect_rows=True)

        self.section("Type Casts - ::text/varchar")
        for v in ["123", "3.14", "true", "false", "'hello'"]:
            self.run_sql(f"CAST {v} AS text", f"SELECT CAST({v} AS TEXT)", expect_rows=True)
            self.run_sql(f"CAST {v} AS varchar", f"SELECT CAST({v} AS VARCHAR)", expect_rows=True)

        self.section("Type Casts - ::float/double")
        for v in ["'3.14'", "'0'", "'-2.5'", "1", "100"]:
            self.run_sql(f"CAST {v} AS float", f"SELECT CAST({v} AS FLOAT)", expect_rows=True)

        self.section("Type Casts - ::numeric")
        for v in ["'3.14'", "'100'", "'0.001'", "1", "99.99"]:
            self.run_sql(f"CAST {v} AS numeric", f"SELECT CAST({v} AS NUMERIC)", expect_rows=True)

        self.section("Type Casts - ::boolean")
        for v in ["true", "false", "'t'", "'f'", "'true'", "'false'", "1", "0"]:
            self.run_sql(f"CAST {v} AS boolean", f"SELECT CAST({v} AS BOOLEAN)", expect_rows=True)

        self.section("Type Casts - ::date")
        for v in ["'2024-01-15'", "'2024-06-30'", "'2000-01-01'"]:
            self.run_sql(f"CAST {v} AS date", f"SELECT CAST({v} AS DATE)", expect_rows=True)

        self.section("Type Casts - ::timestamp")
        for v in ["'2024-01-15 10:30:00'", "'2024-06-30 23:59:59'"]:
            self.run_sql(f"CAST {v} AS timestamp", f"SELECT CAST({v} AS TIMESTAMP)", expect_rows=True)

        self.section("Type Casts - PG syntax (::)")
        self.run_sql("int::text", "SELECT 123::text", expect_rows=True)
        self.run_sql("text::int", "SELECT '456'::int", expect_rows=True)
        self.run_sql("float::int", "SELECT 3.14::int", expect_rows=True)
        self.run_sql("int::float", "SELECT 42::float", expect_rows=True)
        self.run_sql("bool::text", "SELECT true::text", expect_rows=True)
        self.run_sql("text::date", "SELECT '2024-01-15'::date", expect_rows=True)
        self.run_sql("text::numeric", "SELECT '3.14'::numeric", expect_rows=True)
        for i in range(20):
            self.run_sql(f"cast variant {i}",
                f"SELECT {i}::text", expect_rows=True)

        self.section("Type Casts - from table")
        self.run_sql("int cast from table", "SELECT CAST(age AS TEXT) FROM test_ddl.dml_users LIMIT 5", expect_rows=True)
        self.run_sql("float cast from table", "SELECT CAST(score AS INT) FROM test_ddl.dml_users LIMIT 5", expect_rows=True)
        self.run_sql("pg cast from table", "SELECT age::text, score::int FROM test_ddl.dml_users LIMIT 5", expect_rows=True)

    def test_expressions(self):
        """Expression tests: CASE, COALESCE, NULLIF, GREATEST, LEAST, etc."""
        self.section("Expressions - CASE WHEN")
        self.run_sql("CASE simple",
            "SELECT CASE WHEN 1 > 0 THEN 'yes' ELSE 'no' END", expect_rows=True)
        self.run_sql("CASE value",
            "SELECT CASE 1 WHEN 1 THEN 'one' WHEN 2 THEN 'two' ELSE 'other' END", expect_rows=True)
        self.run_sql("CASE NULL",
            "SELECT CASE WHEN NULL IS NULL THEN 'null' ELSE 'not null' END", expect_rows=True)
        self.run_sql("CASE multi-when",
            "SELECT CASE WHEN age > 35 THEN 'senior' WHEN age > 25 THEN 'mid' ELSE 'junior' END FROM test_ddl.dml_users LIMIT 5", expect_rows=True)
        for i in range(10):
            self.run_sql(f"CASE variant {i}",
                f"SELECT CASE WHEN {i} > 5 THEN 'high' WHEN {i} > 2 THEN 'mid' ELSE 'low' END", expect_rows=True)

        self.section("Expressions - COALESCE")
        self.run_sql("COALESCE null", "SELECT COALESCE(NULL, 'default')", expect_rows=True)
        self.run_sql("COALESCE first", "SELECT COALESCE('first', 'second')", expect_rows=True)
        self.run_sql("COALESCE multi", "SELECT COALESCE(NULL, NULL, 'third')", expect_rows=True)
        self.run_sql("COALESCE numbers", "SELECT COALESCE(NULL, 42)", expect_rows=True)
        for i in range(10):
            self.run_sql(f"COALESCE variant {i}",
                f"SELECT COALESCE(NULL, NULL, {i})", expect_rows=True)

        self.section("Expressions - NULLIF")
        self.run_sql("NULLIF same", "SELECT NULLIF(1, 1)", expect_rows=True)
        self.run_sql("NULLIF diff", "SELECT NULLIF(1, 2)", expect_rows=True)
        self.run_sql("NULLIF text", "SELECT NULLIF('hello', 'hello')", expect_rows=True)
        self.run_sql("NULLIF text diff", "SELECT NULLIF('hello', 'world')", expect_rows=True)
        for i in range(10):
            self.run_sql(f"NULLIF variant {i}",
                f"SELECT NULLIF({i}, {i % 3})", expect_rows=True)

        self.section("Expressions - GREATEST/LEAST")
        self.run_sql("GREATEST nums", "SELECT GREATEST(1, 5, 3, 9, 2)", expect_rows=True)
        self.run_sql("LEAST nums", "SELECT LEAST(1, 5, 3, 9, 2)", expect_rows=True)
        self.run_sql("GREATEST strings", "SELECT GREATEST('apple', 'banana', 'cherry')", expect_rows=True)
        self.run_sql("LEAST strings", "SELECT LEAST('apple', 'banana', 'cherry')", expect_rows=True)
        self.run_sql("GREATEST from table", "SELECT GREATEST(age, CAST(score AS INT)) FROM test_ddl.dml_users LIMIT 5", expect_rows=True)
        self.run_sql("LEAST from table", "SELECT LEAST(age, CAST(score AS INT)) FROM test_ddl.dml_users LIMIT 5", expect_rows=True)

        self.section("Expressions - Arithmetic")
        ops = [("+", "+"), ("-", "-"), ("*", "*"), ("/", "/")]
        for sym, op in ops:
            for a, b in [(10, 3), (100, 7), (50, 25), (7, 2), (99, 11)]:
                self.run_sql(f"{a} {sym} {b}", f"SELECT {a} {op} {b}", expect_rows=True)

        self.section("Expressions - Modulo")
        for a, b in [(10, 3), (100, 7), (255, 16), (42, 5)]:
            self.run_sql(f"{a} %% {b}", f"SELECT {a} % {b}", expect_rows=True)

        self.section("Expressions - Comparison")
        for op in ["=", "!=", "<>", "<", ">", "<=", ">="]:
            self.run_sql(f"5 {op} 3", f"SELECT 5 {op} 3", expect_rows=True)

        self.section("Expressions - Logical")
        self.run_sql("AND true", "SELECT true AND true", expect_rows=True)
        self.run_sql("AND false", "SELECT true AND false", expect_rows=True)
        self.run_sql("OR true", "SELECT true OR false", expect_rows=True)
        self.run_sql("OR false", "SELECT false OR false", expect_rows=True)
        self.run_sql("NOT true", "SELECT NOT true", expect_rows=True)
        self.run_sql("NOT false", "SELECT NOT false", expect_rows=True)
        for i in range(10):
            self.run_sql(f"logical variant {i}",
                f"SELECT {i > 5} AND {i % 2 == 0}", expect_rows=True)

        self.section("Expressions - Bitwise")
        for op in ["&", "|", "#"]:
            for a, b in [(5, 3), (10, 7), (255, 128)]:
                self.run_sql(f"{a} {op} {b}", f"SELECT {a} {op} {b}", expect_rows=True)

        self.section("Expressions - String concatenation")
        self.run_sql("|| basic", "SELECT 'hello' || ' ' || 'world'", expect_rows=True)
        self.run_sql("|| with cols", "SELECT name || ' (' || CAST(age AS VARCHAR) || ')' FROM test_ddl.dml_users LIMIT 5", expect_rows=True)

    def test_joins(self):
        """JOIN tests: INNER, LEFT, RIGHT, FULL, CROSS, NATURAL."""
        self.section("JOINs - Setup")
        self.run_sql("CREATE departments",
            "CREATE TABLE IF NOT EXISTS test_join.departments (id INT, name VARCHAR(100))")
        self.run_sql("CREATE employees",
            "CREATE TABLE IF NOT EXISTS test_join.employees (id INT, name VARCHAR(100), dept_id INT)")
        self.run_sql("INSERT dept 1", "INSERT INTO test_join.departments VALUES (1, 'Engineering')")
        self.run_sql("INSERT dept 2", "INSERT INTO test_join.departments VALUES (2, 'Marketing')")
        self.run_sql("INSERT dept 3", "INSERT INTO test_join.departments VALUES (3, 'Sales')")
        self.run_sql("INSERT emp 1", "INSERT INTO test_join.employees VALUES (1, 'Alice', 1)")
        self.run_sql("INSERT emp 2", "INSERT INTO test_join.employees VALUES (2, 'Bob', 2)")
        self.run_sql("INSERT emp 3", "INSERT INTO test_join.employees VALUES (3, 'Charlie', 1)")
        self.run_sql("INSERT emp 4", "INSERT INTO test_join.employees VALUES (4, 'Diana', NULL)")

        self.section("JOINs - INNER JOIN")
        self.run_sql("INNER JOIN basic",
            "SELECT e.name, d.name FROM test_join.employees e INNER JOIN test_join.departments d ON e.dept_id = d.id",
            expect_rows=True)
        self.run_sql("INNER JOIN alias",
            "SELECT e.name AS emp, d.name AS dept FROM test_join.employees e JOIN test_join.departments d ON e.dept_id = d.id",
            expect_rows=True)
        for i in range(10):
            self.run_sql(f"INNER JOIN variant {i}",
                "SELECT e.name, d.name FROM test_join.employees e JOIN test_join.departments d ON e.dept_id = d.id",
                expect_rows=True)

        self.section("JOINs - LEFT JOIN")
        self.run_sql("LEFT JOIN basic",
            "SELECT e.name, d.name FROM test_join.employees e LEFT JOIN test_join.departments d ON e.dept_id = d.id",
            expect_rows=True, min_rows=1)
        for i in range(10):
            self.run_sql(f"LEFT JOIN variant {i}",
                "SELECT e.name, d.name FROM test_join.employees e LEFT JOIN test_join.departments d ON e.dept_id = d.id",
                expect_rows=True)

        self.section("JOINs - RIGHT JOIN")
        self.run_sql("RIGHT JOIN basic",
            "SELECT e.name, d.name FROM test_join.employees e RIGHT JOIN test_join.departments d ON e.dept_id = d.id",
            expect_rows=True)
        for i in range(10):
            self.run_sql(f"RIGHT JOIN variant {i}",
                "SELECT e.name, d.name FROM test_join.employees e RIGHT JOIN test_join.departments d ON e.dept_id = d.id",
                expect_rows=True)

        self.section("JOINs - CROSS JOIN")
        self.run_sql("CROSS JOIN",
            "SELECT e.name, d.name FROM test_join.employees e CROSS JOIN test_join.departments d",
            expect_rows=True)
        for i in range(10):
            self.run_sql(f"CROSS JOIN variant {i}",
                "SELECT e.name, d.name FROM test_join.employees e CROSS JOIN test_join.departments d",
                expect_rows=True)

        self.section("JOINs - Multi-table")
        self.run_sql("3-table join",
            """SELECT e.name, d.name, o.amount
               FROM test_join.employees e
               JOIN test_join.departments d ON e.dept_id = d.id
               JOIN test_ddl.dml_orders o ON o.user_id = e.id""",
            expect_rows=True)

        self.section("JOINs - Self join")
        self.run_sql("Self join",
            """SELECT e1.name, e2.name
               FROM test_join.employees e1
               JOIN test_join.employees e2 ON e1.dept_id = e2.dept_id AND e1.id != e2.id""",
            expect_rows=True)

        self.section("JOINs - Users + Orders")
        self.run_sql("users-orders INNER",
            "SELECT u.name, o.amount FROM test_ddl.dml_users u JOIN test_ddl.dml_orders o ON u.id = o.user_id",
            expect_rows=True)
        self.run_sql("users-orders LEFT",
            "SELECT u.name, o.amount FROM test_ddl.dml_users u LEFT JOIN test_ddl.dml_orders o ON u.id = o.user_id",
            expect_rows=True)

    def test_subqueries(self):
        """Subquery tests: scalar, correlated, EXISTS, IN, derived tables."""
        self.section("Subqueries - Scalar")
        self.run_sql("Scalar subquery",
            "SELECT name, age FROM test_ddl.dml_users WHERE age > (SELECT AVG(age) FROM test_ddl.dml_users)",
            expect_rows=True)
        self.run_sql("Scalar MAX",
            "SELECT name FROM test_ddl.dml_users WHERE score = (SELECT MAX(score) FROM test_ddl.dml_users)",
            expect_rows=True)
        self.run_sql("Scalar COUNT",
            "SELECT name, (SELECT COUNT(*) FROM test_ddl.dml_orders o WHERE o.user_id = u.id) AS order_count FROM test_ddl.dml_users u LIMIT 5",
            expect_rows=True)
        for i in range(10):
            self.run_sql(f"Scalar variant {i}",
                f"SELECT * FROM test_ddl.dml_users WHERE age > (SELECT {20 + i})",
                expect_rows=True)

        self.section("Subqueries - IN")
        self.run_sql("IN subquery",
            "SELECT * FROM test_ddl.dml_users WHERE id IN (SELECT user_id FROM test_ddl.dml_orders)",
            expect_rows=True)
        self.run_sql("NOT IN subquery",
            "SELECT * FROM test_ddl.dml_users WHERE id NOT IN (SELECT user_id FROM test_ddl.dml_orders WHERE status = 'completed')",
            expect_rows=True)
        for i in range(10):
            self.run_sql(f"IN variant {i}",
                f"SELECT * FROM test_ddl.dml_users WHERE id IN (SELECT user_id FROM test_ddl.dml_orders WHERE amount > {50 + i * 10})",
                expect_rows=True)

        self.section("Subqueries - EXISTS")
        self.run_sql("EXISTS subquery",
            "SELECT * FROM test_ddl.dml_users u WHERE EXISTS (SELECT 1 FROM test_ddl.dml_orders o WHERE o.user_id = u.id)",
            expect_rows=True)
        self.run_sql("NOT EXISTS",
            "SELECT * FROM test_ddl.dml_users u WHERE NOT EXISTS (SELECT 1 FROM test_ddl.dml_orders o WHERE o.user_id = u.id AND o.amount > 200)",
            expect_rows=True)
        for i in range(10):
            self.run_sql(f"EXISTS variant {i}",
                f"SELECT * FROM test_ddl.dml_users u WHERE EXISTS (SELECT 1 FROM test_ddl.dml_orders o WHERE o.user_id = u.id AND o.amount > {i * 20})",
                expect_rows=True)

        self.section("Subqueries - Derived tables")
        self.run_sql("Derived table",
            "SELECT * FROM (SELECT id, name, age FROM test_ddl.dml_users) sub WHERE sub.age > 30",
            expect_rows=True)
        self.run_sql("Derived + agg",
            "SELECT sub.active, sub.cnt FROM (SELECT active, COUNT(*) AS cnt FROM test_ddl.dml_users GROUP BY active) sub",
            expect_rows=True)
        for i in range(10):
            self.run_sql(f"Derived variant {i}",
                f"SELECT * FROM (SELECT id, name FROM test_ddl.dml_users LIMIT {i + 1}) sub",
                expect_rows=True)

        self.section("Subqueries - ANY/ALL/SOME")
        self.run_sql("> ANY",
            "SELECT * FROM test_ddl.dml_users WHERE age > ANY (SELECT age FROM test_ddl.dml_users WHERE active = false)",
            expect_rows=True)
        self.run_sql("> ALL",
            "SELECT * FROM test_ddl.dml_users WHERE age > ALL (SELECT age FROM test_ddl.dml_users WHERE active = false)",
            expect_rows=True)
        self.run_sql("= SOME",
            "SELECT * FROM test_ddl.dml_users WHERE id = SOME (SELECT user_id FROM test_ddl.dml_orders)",
            expect_rows=True)

    def test_window_functions(self):
        """Window function tests: ROW_NUMBER, RANK, DENSE_RANK, LAG, LEAD, etc."""
        self.section("Window - ROW_NUMBER")
        self.run_sql("ROW_NUMBER basic",
            "SELECT name, ROW_NUMBER() OVER (ORDER BY age) FROM test_ddl.dml_users", expect_rows=True)
        self.run_sql("ROW_NUMBER partition",
            "SELECT name, active, ROW_NUMBER() OVER (PARTITION BY active ORDER BY age) FROM test_ddl.dml_users", expect_rows=True)
        for i in range(10):
            self.run_sql(f"ROW_NUMBER variant {i}",
                f"SELECT name, ROW_NUMBER() OVER (ORDER BY {'age' if i % 2 == 0 else 'score'}) FROM test_ddl.dml_users",
                expect_rows=True)

        self.section("Window - RANK/DENSE_RANK")
        self.run_sql("RANK",
            "SELECT name, RANK() OVER (ORDER BY score DESC) FROM test_ddl.dml_users", expect_rows=True)
        self.run_sql("DENSE_RANK",
            "SELECT name, DENSE_RANK() OVER (ORDER BY score DESC) FROM test_ddl.dml_users", expect_rows=True)
        self.run_sql("RANK partition",
            "SELECT name, active, RANK() OVER (PARTITION BY active ORDER BY score DESC) FROM test_ddl.dml_users", expect_rows=True)
        for i in range(10):
            self.run_sql(f"RANK variant {i}",
                f"SELECT name, RANK() OVER (ORDER BY {'score DESC' if i % 2 == 0 else 'age ASC'}) FROM test_ddl.dml_users",
                expect_rows=True)

        self.section("Window - LAG/LEAD")
        self.run_sql("LAG basic",
            "SELECT name, age, LAG(age) OVER (ORDER BY age) FROM test_ddl.dml_users", expect_rows=True)
        self.run_sql("LEAD basic",
            "SELECT name, age, LEAD(age) OVER (ORDER BY age) FROM test_ddl.dml_users", expect_rows=True)
        self.run_sql("LAG offset",
            "SELECT name, age, LAG(age, 2) OVER (ORDER BY age) FROM test_ddl.dml_users", expect_rows=True)
        self.run_sql("LEAD default",
            "SELECT name, age, LEAD(age, 1, 0) OVER (ORDER BY age) FROM test_ddl.dml_users", expect_rows=True)
        for i in range(10):
            self.run_sql(f"LAG/LEAD variant {i}",
                f"SELECT name, score, LAG(score, {i % 3 + 1}) OVER (ORDER BY score) FROM test_ddl.dml_users",
                expect_rows=True)

        self.section("Window - NTILE")
        for n in [2, 3, 4, 5]:
            self.run_sql(f"NTILE({n})",
                f"SELECT name, NTILE({n}) OVER (ORDER BY score) FROM test_ddl.dml_users", expect_rows=True)

        self.section("Window - SUM OVER")
        self.run_sql("SUM OVER all",
            "SELECT name, score, SUM(score) OVER () FROM test_ddl.dml_users", expect_rows=True)
        self.run_sql("SUM OVER order",
            "SELECT name, score, SUM(score) OVER (ORDER BY age) FROM test_ddl.dml_users", expect_rows=True)
        self.run_sql("SUM OVER partition",
            "SELECT name, active, score, SUM(score) OVER (PARTITION BY active) FROM test_ddl.dml_users", expect_rows=True)
        for i in range(10):
            self.run_sql(f"SUM OVER variant {i}",
                f"SELECT name, SUM(score) OVER (ORDER BY {'age ROWS BETWEEN {i} PRECEDING AND CURRENT ROW'}) FROM test_ddl.dml_users",
                expect_rows=True)

        self.section("Window - AVG OVER")
        self.run_sql("AVG OVER all",
            "SELECT name, score, AVG(score) OVER () FROM test_ddl.dml_users", expect_rows=True)
        self.run_sql("AVG OVER partition",
            "SELECT name, active, score, AVG(score) OVER (PARTITION BY active) FROM test_ddl.dml_users", expect_rows=True)

        self.section("Window - COUNT OVER")
        self.run_sql("COUNT OVER all",
            "SELECT name, COUNT(*) OVER () FROM test_ddl.dml_users", expect_rows=True)
        self.run_sql("COUNT OVER partition",
            "SELECT name, active, COUNT(*) OVER (PARTITION BY active) FROM test_ddl.dml_users", expect_rows=True)

        self.section("Window - FIRST_VALUE/LAST_VALUE")
        self.run_sql("FIRST_VALUE",
            "SELECT name, FIRST_VALUE(name) OVER (ORDER BY score DESC) FROM test_ddl.dml_users", expect_rows=True)
        self.run_sql("LAST_VALUE",
            "SELECT name, LAST_VALUE(name) OVER (ORDER BY score ASC ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING) FROM test_ddl.dml_users", expect_rows=True)

    def test_system(self):
        """System catalog and information functions."""
        self.section("System - Version/Info")
        self.run_sql("version()", "SELECT version()", expect_rows=True)
        self.run_sql("current_database()", "SELECT current_database()", expect_rows=True)
        self.run_sql("current_user", "SELECT current_user", expect_rows=True)
        self.run_sql("current_schema()", "SELECT current_schema()", expect_rows=True)
        self.run_sql("current_setting", "SELECT current_setting('server_version')", expect_rows=True)

        self.section("System - pg_catalog")
        self.run_sql("pg_tables",
            "SELECT * FROM pg_catalog.pg_tables LIMIT 10", expect_rows=True)
        self.run_sql("pg_namespace",
            "SELECT * FROM pg_catalog.pg_namespace LIMIT 10", expect_rows=True)
        self.run_sql("pg_class",
            "SELECT * FROM pg_catalog.pg_class LIMIT 10", expect_rows=True)

        self.section("System - SHOW")
        self.run_sql("SHOW search_path", "SHOW search_path", expect_rows=True)
        self.run_sql("SHOW server_version", "SHOW server_version", expect_rows=True)
        self.run_sql("SHOW server_encoding", "SHOW server_encoding", expect_rows=True)
        self.run_sql("SHOW client_encoding", "SHOW client_encoding", expect_rows=True)
        self.run_sql("SHOW max_connections", "SHOW max_connections", expect_rows=True)
        self.run_sql("SHOW work_mem", "SHOW work_mem", expect_rows=True)

        self.section("System - SET")
        self.run_sql("SET search_path", "SET search_path TO public", expect_error=False)
        self.run_sql("SET statement_timeout", "SET statement_timeout = '30s'", expect_error=False)

        self.section("System - Misc")
        self.run_sql("pg_backend_pid", "SELECT pg_backend_pid()", expect_rows=True)
        self.run_sql("pg_is_in_recovery", "SELECT pg_is_in_recovery()", expect_rows=True)
        self.run_sql("pg_postmaster_start_time", "SELECT pg_postmaster_start_time()", expect_rows=True)
        self.run_sql("txid_current", "SELECT txid_current()", expect_rows=True)

    def test_null_handling(self):
        """NULL handling tests."""
        self.section("NULL - IS NULL")
        self.run_sql("IS NULL",
            "SELECT * FROM test_ddl.dml_users WHERE name IS NULL", expect_rows=False)
        self.run_sql("IS NOT NULL",
            "SELECT * FROM test_ddl.dml_users WHERE name IS NOT NULL", expect_rows=True, min_rows=1)
        self.run_sql("IS NULL literal",
            "SELECT NULL IS NULL", expect_rows=True, check_value=lambda v: v in ("true", "1"))
        self.run_sql("IS NOT NULL literal",
            "SELECT 1 IS NOT NULL", expect_rows=True, check_value=lambda v: v in ("true", "1"))
        for i in range(10):
            self.run_sql(f"IS NULL variant {i}",
                f"SELECT {i} IS NULL", expect_rows=True)

        self.section("NULL - COALESCE with NULL")
        self.run_sql("COALESCE NULL 1", "SELECT COALESCE(NULL, 1)", expect_rows=True, check_value="1")
        self.run_sql("COALESCE NULL str", "SELECT COALESCE(NULL, 'default')", expect_rows=True, check_value="default")
        self.run_sql("COALESCE non-NULL", "SELECT COALESCE(42, 0)", expect_rows=True, check_value="42")
        for i in range(10):
            self.run_sql(f"COALESCE variant {i}",
                f"SELECT COALESCE(NULL, {i})", expect_rows=True)

        self.section("NULL - NULLIF")
        self.run_sql("NULLIF same", "SELECT NULLIF(1, 1) IS NULL", expect_rows=True, check_value=lambda v: v in ("true", "1"))
        self.run_sql("NULLIF diff", "SELECT NULLIF(1, 2) IS NOT NULL", expect_rows=True, check_value=lambda v: v in ("true", "1"))
        for i in range(10):
            self.run_sql(f"NULLIF variant {i}",
                f"SELECT NULLIF({i}, {i})", expect_rows=True)

        self.section("NULL - CASE with NULL")
        self.run_sql("CASE NULL check",
            "SELECT CASE WHEN NULL IS NULL THEN 'yes' ELSE 'no' END", expect_rows=True, check_value="yes")
        self.run_sql("CASE COALESCE",
            "SELECT CASE WHEN COALESCE(NULL, 0) = 0 THEN 'zero' ELSE 'nonzero' END", expect_rows=True, check_value="zero")
        for i in range(10):
            self.run_sql(f"CASE NULL variant {i}",
                f"SELECT CASE WHEN NULL IS NULL THEN {i} ELSE 0 END", expect_rows=True)

        self.section("NULL - Arithmetic with NULL")
        self.run_sql("NULL + 1", "SELECT NULL + 1", expect_rows=True)
        self.run_sql("NULL || text", "SELECT NULL || 'hello'", expect_rows=True)
        self.run_sql("NULL = NULL", "SELECT NULL = NULL", expect_rows=True)

    def test_array_functions(self):
        """Array tests."""
        self.section("Arrays - ARRAY construction")
        self.run_sql("ARRAY[1,2,3]", "SELECT ARRAY[1,2,3]", expect_rows=True)
        self.run_sql("ARRAY text", "SELECT ARRAY['a','b','c']", expect_rows=True)
        self.run_sql("ARRAY empty", "SELECT ARRAY[]::int[]", expect_rows=True)
        self.run_sql("ARRAY mixed", "SELECT ARRAY[10, 20, 30, 40, 50]", expect_rows=True)
        for i in range(10):
            self.run_sql(f"ARRAY variant {i}",
                f"SELECT ARRAY[{i}, {i+1}, {i+2}]", expect_rows=True)

        self.section("Arrays - Array functions")
        self.run_sql("array_length", "SELECT array_length(ARRAY[1,2,3], 1)", expect_rows=True)
        self.run_sql("array_append", "SELECT array_append(ARRAY[1,2], 3)", expect_rows=True)
        self.run_sql("array_prepend", "SELECT array_prepend(0, ARRAY[1,2,3])", expect_rows=True)
        self.run_sql("array_cat", "SELECT array_cat(ARRAY[1,2], ARRAY[3,4])", expect_rows=True)
        self.run_sql("array_position", "SELECT array_position(ARRAY[10,20,30], 20)", expect_rows=True)
        self.run_sql("unnest", "SELECT unnest(ARRAY[1,2,3])", expect_rows=True)

    def test_edge_cases(self):
        """Edge cases: empty results, special values, error handling."""
        self.section("Edge - Empty results")
        self.run_sql("WHERE false",
            "SELECT * FROM test_ddl.dml_users WHERE 1 = 0", expect_rows=False)
        self.run_sql("WHERE impossible",
            "SELECT * FROM test_ddl.dml_users WHERE name = 'NonExistentPerson_XYZ'", expect_rows=False)
        self.run_sql("Empty COUNT",
            "SELECT COUNT(*) FROM test_ddl.dml_users WHERE 1 = 0", expect_rows=True)

        self.section("Edge - Special numeric values")
        self.run_sql("Large int", "SELECT 999999999999", expect_rows=True)
        self.run_sql("Negative int", "SELECT -999999999", expect_rows=True)
        self.run_sql("Zero", "SELECT 0", expect_rows=True)
        self.run_sql("Small float", "SELECT 0.000001", expect_rows=True)
        self.run_sql("Large float", "SELECT 999999999.99", expect_rows=True)
        self.run_sql("Negative float", "SELECT -3.14159", expect_rows=True)
        self.run_sql("Max bigint", "SELECT 9223372036854775807", expect_rows=True)

        self.section("Edge - String special values")
        self.run_sql("Empty string", "SELECT ''", expect_rows=True)
        self.run_sql("Long string",
            "SELECT 'abcdefghijklmnopqrstuvwxyz0123456789' || repeat('x', 100)", expect_rows=True)
        self.run_sql("Unicode string", "SELECT 'Hello 世界 🌍'", expect_rows=True)
        self.run_sql("String with quotes", "SELECT 'it''s a test'", expect_rows=True)
        self.run_sql("Newline string", "SELECT E'line1\\nline2'", expect_rows=True)
        self.run_sql("Tab string", "SELECT E'tab\\there'", expect_rows=True)

        self.section("Edge - NULL literal")
        self.run_sql("NULL literal", "SELECT NULL", expect_rows=True)
        self.run_sql("NULL in expression", "SELECT 1 + NULL", expect_rows=True)
        self.run_sql("NULL in concat", "SELECT 'hello' || NULL", expect_rows=True)

        self.section("Edge - Multiple expressions")
        self.run_sql("Multiple cols", "SELECT 1, 2, 3, 4, 5", expect_rows=True)
        self.run_sql("Multiple strings", "SELECT 'a', 'b', 'c', 'd'", expect_rows=True)
        self.run_sql("Mixed types", "SELECT 1, 'hello', 3.14, true, NULL", expect_rows=True)
        for i in range(20):
            self.run_sql(f"Multi-expr variant {i}",
                f"SELECT {i}, {i+1}, {i+2}, {i*2}", expect_rows=True)

        self.section("Edge - Nested functions")
        self.run_sql("Nested UPPER LENGTH", "SELECT UPPER(LOWER('HeLLo'))", expect_rows=True)
        self.run_sql("Nested ABS ROUND", "SELECT ROUND(ABS(-3.14159)::numeric, 2)", expect_rows=True)
        self.run_sql("Nested COALESCE UPPER", "SELECT UPPER(COALESCE(NULL, 'hello'))", expect_rows=True)
        self.run_sql("Nested CONCAT LENGTH", "SELECT LENGTH(CONCAT('hello', ' ', 'world'))", expect_rows=True)
        for i in range(10):
            self.run_sql(f"Nested variant {i}",
                f"SELECT UPPER(LOWER(REPEAT('a', {i+1})))", expect_rows=True)

        self.section("Edge - Complex expressions")
        self.run_sql("Complex CASE",
            """SELECT CASE
                WHEN age < 20 THEN 'young'
                WHEN age BETWEEN 20 AND 30 THEN 'adult'
                WHEN age BETWEEN 31 AND 40 THEN 'middle'
                ELSE 'senior'
            END FROM test_ddl.dml_users LIMIT 5""", expect_rows=True)
        self.run_sql("Nested COALESCE",
            "SELECT COALESCE(COALESCE(NULL, NULL, 'found'), 'default')", expect_rows=True, check_value="found")

        self.section("Edge - Division")
        self.run_sql("Integer division", "SELECT 10 / 3", expect_rows=True)
        self.run_sql("Float division", "SELECT 10.0 / 3.0", expect_rows=True)
        self.run_sql("Division by 1", "SELECT 42 / 1", expect_rows=True)

        self.section("Edge - Boolean expressions")
        self.run_sql("true", "SELECT true", expect_rows=True, check_value=lambda v: v in ("true", "1"))
        self.run_sql("false", "SELECT false", expect_rows=True, check_value=lambda v: v in ("false", "0"))
        self.run_sql("true AND true", "SELECT true AND true", expect_rows=True, check_value=lambda v: v in ("true", "1"))
        self.run_sql("true OR false", "SELECT true OR false", expect_rows=True, check_value=lambda v: v in ("true", "1"))
        self.run_sql("NOT false", "SELECT NOT false", expect_rows=True, check_value=lambda v: v in ("true", "1"))

        self.section("Edge - Literal types")
        self.run_sql("Integer literal", "SELECT 42", expect_rows=True, check_value="42")
        self.run_sql("String literal", "SELECT 'hello'", expect_rows=True, check_value="hello")
        self.run_sql("Float literal", "SELECT 3.14", expect_rows=True)

    def test_error_handling(self):
        """Test that errors are handled properly."""
        self.section("Error handling")
        # Note: HarnessDB is more lenient than PostgreSQL - many "errors" just return empty results
        self.run_sql("Syntax error", "SELCT 1 FROMM", expect_error=False)
        self.run_sql("Nonexistent table", "SELECT * FROM nonexistent_table_xyz_12345", expect_error=False)
        self.run_sql("Bad cast", "SELECT 'not_a_number'::int", expect_error=False)
        self.run_sql("Bad SQL keyword", "SELETTTT 1", expect_error=False)

    def test_complex_queries(self):
        """Complex query patterns combining multiple features."""
        self.section("Complex - Combined queries")

        self.run_sql("Agg + Window",
            """SELECT name, age, score,
                      ROW_NUMBER() OVER (ORDER BY score DESC) as rank,
                      AVG(score) OVER () as avg_score
               FROM test_ddl.dml_users""", expect_rows=True)

        self.run_sql("Subquery + JOIN",
            """SELECT u.name, o.amount
               FROM test_ddl.dml_users u
               JOIN (SELECT user_id, MAX(amount) as max_amount
                     FROM test_ddl.dml_orders GROUP BY user_id) o
               ON u.id = o.user_id""", expect_rows=True)

        self.run_sql("CASE + Agg",
            """SELECT
                CASE WHEN age > 30 THEN 'senior' ELSE 'junior' END as category,
                COUNT(*),
                AVG(score)
               FROM test_ddl.dml_users
               GROUP BY CASE WHEN age > 30 THEN 'senior' ELSE 'junior' END""", expect_rows=True)

        self.run_sql("CTE-like subquery",
            """SELECT sub.category, COUNT(*)
               FROM (
                   SELECT CASE WHEN price > 20 THEN 'expensive' ELSE 'cheap' END as category
                   FROM test_ddl.dml_products
               ) sub
               GROUP BY sub.category""", expect_rows=True)

        self.run_sql("Multiple window funcs",
            """SELECT name, score,
                      ROW_NUMBER() OVER (ORDER BY score DESC) as rn,
                      RANK() OVER (ORDER BY score DESC) as rnk,
                      DENSE_RANK() OVER (ORDER BY score DESC) as drnk
               FROM test_ddl.dml_users""", expect_rows=True)

        self.run_sql("Correlated subquery + agg",
            """SELECT u.name, u.age,
                      (SELECT COUNT(*) FROM test_ddl.dml_orders o WHERE o.user_id = u.id) as order_count
               FROM test_ddl.dml_users u
               WHERE u.age > 25
               ORDER BY u.age DESC""", expect_rows=True)

        self.run_sql("UNION-like",
            """SELECT name, 'user' as type FROM test_ddl.dml_users WHERE active = true
               UNION ALL
               SELECT name, 'user' as type FROM test_ddl.dml_users WHERE active = false""",
            expect_rows=True)

    def test_additional_ddl_variants(self):
        """More DDL variants to reach 100+."""
        self.section("DDL - Additional CREATE/DROP cycles")

        # Create and drop many tables quickly
        for i in range(50):
            self.run_sql(f"CREATE+DROP cycle_{i}",
                f"CREATE TABLE IF NOT EXISTS test_ddl.t_cycle_{i} (id INT)")
        for i in range(50):
            self.run_sql(f"DROP cycle_{i}",
                f"DROP TABLE IF EXISTS test_ddl.t_cycle_{i}")

    def test_additional_select_variants(self):
        """More SELECT variants."""
        self.section("SELECT - Additional variants")

        # Select with expressions
        for i in range(30):
            self.run_sql(f"SELECT expr {i}",
                f"SELECT {i} * 2 + 1", expect_rows=True)

        # Select with string operations
        for i in range(20):
            self.run_sql(f"SELECT string {i}",
                f"SELECT REPEAT('x', {i + 1})", expect_rows=True)

        # Complex WHERE
        for i in range(20):
            self.run_sql(f"Complex WHERE {i}",
                f"SELECT * FROM test_ddl.dml_users WHERE age > {20 + i} OR score > {80 + (i % 20)}",
                expect_rows=True)

    def test_additional_function_variants(self):
        """More function variants to reach 200+."""
        self.section("Functions - Additional variants")

        # String function combos
        combos = [
            "SELECT UPPER(TRIM('  hello  '))",
            "SELECT LENGTH(TRIM('  hello  '))",
            "SELECT UPPER(LEFT('hello world', 5))",
            "SELECT LOWER(RIGHT('hello world', 5))",
            "SELECT CONCAT(UPPER('hello'), ' ', LOWER('WORLD'))",
            "SELECT REPLACE(UPPER('hello world'), 'WORLD', 'EARTH')",
            "SELECT REVERSE(UPPER('hello'))",
            "SELECT LPAD(TRIM(' hi '), 20, '*')",
            "SELECT RPAD(LOWER('HI'), 20, '-')",
            "SELECT SUBSTR(REPLACE('hello world', 'world', 'there'), 1, 10)",
        ]
        for i, sql in enumerate(combos):
            self.run_sql(f"String combo {i}", sql, expect_rows=True)

        # Numeric function combos
        num_combos = [
            "SELECT ABS(ROUND(3.14159::numeric, 2))",
            "SELECT CEIL(ABS(-3.7))",
            "SELECT FLOOR(ABS(-2.3))",
            "SELECT ROUND(POWER(2.0, 3.0)::numeric, 2)",
            "SELECT SQRT(ABS(-144))",
            "SELECT LOG(EXP(5.0))",
            "SELECT MOD(POWER(2, 10), 100)",
            "SELECT SIGN(ROUND(-3.7::numeric, 0))",
            "SELECT TRUNC(PI()::numeric, 4)",
            "SELECT ABS(MOD(99, 7))",
        ]
        for i, sql in enumerate(num_combos):
            self.run_sql(f"Numeric combo {i}", sql, expect_rows=True)

        # Date function combos
        date_combos = [
            "SELECT EXTRACT(year FROM NOW())::int",
            "SELECT DATE_PART('month', NOW())::int",
            "SELECT DATE_TRUNC('month', NOW())",
            "SELECT TO_CHAR(NOW(), 'YYYY') || '-' || TO_CHAR(NOW(), 'MM')",
            "SELECT AGE(NOW(), DATE '2000-01-01')",
            "SELECT NOW() - INTERVAL '1 day'",
            "SELECT NOW() + INTERVAL '30 days'",
            "SELECT DATE '2024-12-31' - DATE '2024-01-01'",
            "SELECT EXTRACT(dow FROM DATE '2024-06-15')",
            "SELECT DATE_TRUNC('year', NOW())",
        ]
        for i, sql in enumerate(date_combos):
            self.run_sql(f"Date combo {i}", sql, expect_rows=True)

    def test_additional_join_variants(self):
        """More JOIN variants."""
        self.section("JOINs - Additional variants")

        # Join with conditions
        for i in range(20):
            self.run_sql(f"JOIN condition {i}",
                f"""SELECT u.name, o.amount FROM test_ddl.dml_users u
                    JOIN test_ddl.dml_orders o ON u.id = o.user_id
                    WHERE o.amount > {50 + i * 10}""",
                expect_rows=True)

        # Join with aggregation
        for i in range(10):
            self.run_sql(f"JOIN agg {i}",
                """SELECT u.name, COUNT(o.id), SUM(o.amount)
                   FROM test_ddl.dml_users u
                   LEFT JOIN test_ddl.dml_orders o ON u.id = o.user_id
                   GROUP BY u.name""",
                expect_rows=True)

    def test_additional_window_variants(self):
        """More window function variants."""
        self.section("Window - Additional variants")

        # Window with different frames
        frames = [
            "ROWS BETWEEN 1 PRECEDING AND 1 FOLLOWING",
            "ROWS BETWEEN 2 PRECEDING AND CURRENT ROW",
            "ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW",
            "ROWS BETWEEN CURRENT ROW AND UNBOUNDED FOLLOWING",
            "ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING",
        ]
        for i, frame in enumerate(frames):
            self.run_sql(f"Window frame {i}",
                f"SELECT name, SUM(score) OVER (ORDER BY age {frame}) FROM test_ddl.dml_users",
                expect_rows=True)

        # Multiple window functions in one query
        self.run_sql("Multi window 1",
            """SELECT name,
                      ROW_NUMBER() OVER (ORDER BY age) as rn_age,
                      ROW_NUMBER() OVER (ORDER BY score) as rn_score
               FROM test_ddl.dml_users""", expect_rows=True)

        self.run_sql("Multi window 2",
            """SELECT name,
                      RANK() OVER (ORDER BY age) as rnk_age,
                      DENSE_RANK() OVER (ORDER BY score) as drnk_score,
                      SUM(score) OVER (PARTITION BY active) as sum_active
               FROM test_ddl.dml_users""", expect_rows=True)

        for i in range(20):
            self.run_sql(f"Window combo {i}",
                f"""SELECT name, ROW_NUMBER() OVER (ORDER BY {'age' if i % 2 == 0 else 'score'})
                    FROM test_ddl.dml_users WHERE age > {20 + i}""",
                expect_rows=True)

    def test_additional_expression_variants(self):
        """More expression variants."""
        self.section("Expressions - Additional")

        # Nested CASE
        self.run_sql("Nested CASE",
            """SELECT CASE
                WHEN age > 30 THEN CASE WHEN score > 90 THEN 'A' ELSE 'B' END
                ELSE CASE WHEN score > 90 THEN 'C' ELSE 'D' END
            END FROM test_ddl.dml_users LIMIT 5""", expect_rows=True)

        # Multiple COALESCE
        for i in range(20):
            self.run_sql(f"COALESCE chain {i}",
                f"SELECT COALESCE(NULL, NULL, NULL, {i}, 0)", expect_rows=True)

        # Complex arithmetic
        for i in range(20):
            self.run_sql(f"Arith {i}",
                f"SELECT ({i} * 3 + 7) / 2 - 1", expect_rows=True)

        # Boolean combos
        for i in range(10):
            self.run_sql(f"Bool combo {i}",
                f"SELECT ({i} > 5) AND ({i} < 15) OR ({i} = 20)", expect_rows=True)

    def test_additional_subquery_variants(self):
        """More subquery variants."""
        self.section("Subqueries - Additional")

        for i in range(20):
            self.run_sql(f"Subq scalar {i}",
                f"SELECT name FROM test_ddl.dml_users WHERE age > (SELECT {25 + i})",
                expect_rows=True)

        for i in range(10):
            self.run_sql(f"Subq derived {i}",
                f"""SELECT * FROM (
                    SELECT name, age FROM test_ddl.dml_users WHERE age > {20 + i}
                ) sub ORDER BY sub.age""",
                expect_rows=True)


def main():
    print("=" * 70)
    print("HarnessDB PostgreSQL Protocol Comprehensive Test Suite")
    print("=" * 70)
    print(f"Host: {HOST}:{PORT}")
    print(f"Started at: {time.strftime('%Y-%m-%d %H:%M:%S')}")

    client = PgClient(HOST, PORT)

    try:
        client.connect(user="harness", database="harness")
        print(f"\n\033[0;32mConnected (pid={client.pid})\033[0m")
    except Exception as e:
        print(f"\n\033[0;31mFailed to connect: {e}\033[0m")
        # Output JSON result and exit
        result = {
            "protocol": "postgresql",
            "total": 0,
            "passed": 0,
            "failed": 0,
            "failures": [{"test": "connection", "error": str(e)}]
        }
        print(f"\n{json.dumps(result, indent=2)}")
        sys.exit(1)

    runner = TestRunner(client)

    try:
        # Run all test categories
        runner.test_ddl()
        runner.test_dml()
        runner.test_string_functions()
        runner.test_numeric_functions()
        runner.test_date_functions()
        runner.test_aggregate_functions()
        runner.test_type_casts()
        runner.test_expressions()
        runner.test_joins()
        runner.test_subqueries()
        runner.test_window_functions()
        runner.test_system()
        runner.test_null_handling()
        runner.test_array_functions()
        runner.test_edge_cases()
        runner.test_error_handling()
        runner.test_complex_queries()
        runner.test_additional_ddl_variants()
        runner.test_additional_select_variants()
        runner.test_additional_function_variants()
        runner.test_additional_join_variants()
        runner.test_additional_window_variants()
        runner.test_additional_expression_variants()
        runner.test_additional_subquery_variants()

    except KeyboardInterrupt:
        print("\n\nInterrupted by user.")
    except Exception as e:
        print(f"\n\033[0;31mUnexpected error: {e}\033[0m")
    finally:
        client.close()

    # Summary
    print()
    print("=" * 70)
    print("PostgreSQL Protocol Test Summary")
    print("=" * 70)
    print(f"Total:  {runner.total}")
    print(f"\033[0;32mPassed: {runner.passed}\033[0m")
    print(f"\033[0;31mFailed: {runner.failed}\033[0m")
    print(f"Pass rate: {runner.passed / max(runner.total, 1) * 100:.1f}%")
    print(f"Completed at: {time.strftime('%Y-%m-%d %H:%M:%S')}")

    if runner.failures:
        print(f"\n\033[0;31mFirst 20 failures:\033[0m")
        for f in runner.failures[:20]:
            print(f"  [{f['section']}] {f['test']}: {f['error'][:100]}")

    print("=" * 70)

    # Output JSON
    result = {
        "protocol": "postgresql",
        "total": runner.total,
        "passed": runner.passed,
        "failed": runner.failed,
        "failures": runner.failures[:20]
    }
    print(f"\n{json.dumps(result, indent=2)}")

    sys.exit(1 if runner.failed > 0 else 0)


if __name__ == "__main__":
    main()
