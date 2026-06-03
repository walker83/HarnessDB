#!/usr/bin/env python3
"""
PostgreSQL Protocol CRUD Test for HarnessDB
Tests PostgreSQL wire protocol (Hologres compatibility)
Usage: python3 pg_crud_test.py [port]
"""

import sys
import socket
import struct
import time

PORT = int(sys.argv[1]) if len(sys.argv) > 1 else 15432
HOST = "127.0.0.1"

passed = 0
failed = 0
total = 0

def p(name):
    global passed, total
    passed += 1
    total += 1
    print(f"  \033[0;32m✓\033[0m {name}")

def f(name, msg):
    global failed, total
    failed += 1
    total += 1
    print(f"  \033[0;31m✗\033[0m {name}: \033[0;31m{msg}\033[0m")

def section(name):
    print(f"\n\033[0;34m[{name}]\033[0m")

class PgClient:
    """Minimal PostgreSQL wire protocol client (Simple Query mode)"""

    def __init__(self, host, port):
        self.host = host
        self.port = port
        self.sock = None
        self.pid = 0
        self.secret = 0

    def connect(self, user="harness", password="harness-secret", database="harness"):
        """Startup connection using PostgreSQL v3 protocol"""
        self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.sock.settimeout(10)
        self.sock.connect((self.host, self.port))

        # Build startup message
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

    def _read_startup_response(self):
        """Read authentication and ready response"""
        while True:
            msg_type = self.sock.recv(1)
            if not msg_type:
                break
            msg_type = ord(msg_type)
            length = struct.unpack("!i", self.sock.recv(4))[0]
            data = self.sock.recv(length - 4)

            if msg_type == ord('R'):  # Authentication
                auth_type = struct.unpack("!i", data[:4])[0]
                if auth_type == 0:  # Trust
                    continue
                elif auth_type == 3:  # Cleartext password
                    # Send password
                    pwd_msg = b'p' + struct.pack("!i", len(data) + 1) + b"harness-secret\n"
                    self.sock.sendall(pwd_msg)
                elif auth_type == 5:  # MD5
                    salt = data[4:8]
                    # Simple MD5 auth (just send placeholder)
                    import hashlib
                    pwd_hash = hashlib.md5(f"harness-secretharness".encode()).hexdigest()
                    full_hash = hashlib.md5(f"{pwd_hash}{salt.hex()}".encode()).hexdigest()
                    pwd_msg = b'p' + struct.pack("!i", len(f"md5{full_hash}\n") + 4) + f"md5{full_hash}\n".encode()
                    self.sock.sendall(pwd_msg)
                elif auth_type == 10:  # SASL SCRAM
                    pass  # Skip for now

            elif msg_type == ord('K'):  # BackendKeyData
                self.pid, self.secret = struct.unpack("!ii", data[:8])

            elif msg_type == ord('S'):  # ParameterStatus
                pass

            elif msg_type == ord('Z'):  # ReadyForQuery
                break

    def query(self, sql):
        """Execute a simple query"""
        msg = b'Q' + struct.pack("!i", len(sql.encode()) + 5) + sql.encode() + b'\x00'
        self.sock.sendall(msg)

        rows = []
        columns = []
        error = None

        while True:
            msg_type = self.sock.recv(1)
            if not msg_type:
                break
            msg_type = ord(msg_type)
            length = struct.unpack("!i", self.sock.recv(4))[0]
            data = self.sock.recv(length - 4)

            if msg_type == ord('T'):  # RowDescription
                num_cols = struct.unpack("!h", data[:2])[0]
                offset = 2
                for _ in range(num_cols):
                    null_idx = data.index(b'\x00', offset)
                    col_name = data[offset:null_idx].decode()
                    columns.append(col_name)
                    offset = null_idx + 19  # Skip rest of column info

            elif msg_type == ord('D'):  # DataRow
                num_cols = struct.unpack("!h", data[:2])[0]
                offset = 2
                row = {}
                col_idx = 0
                for _ in range(num_cols):
                    col_len = struct.unpack("!i", data[offset:offset+4])[0]
                    offset += 4
                    if col_len == -1:
                        row[columns[col_idx] if col_idx < len(columns) else str(col_idx)] = None
                    else:
                        val = data[offset:offset+col_len].decode()
                        row[columns[col_idx] if col_idx < len(columns) else str(col_idx)] = val
                    offset += col_len
                    col_idx += 1
                rows.append(row)

            elif msg_type == ord('C'):  # CommandComplete
                tag = data.decode().rstrip('\x00')
                break

            elif msg_type == ord('E'):  # ErrorResponse
                error = data.decode('utf-8', errors='replace').rstrip('\x00')

            elif msg_type == ord('Z'):  # ReadyForQuery
                break

            elif msg_type == ord('1'):  # ParseComplete
                pass

            elif msg_type == ord('2'):  # BindComplete
                pass

            elif msg_type == ord('s'):  # PortalSuspended
                pass

            elif msg_type == ord('n'):  # NoData
                pass

            elif msg_type == ord('I'):  # EmptyQueryResponse
                break

        return {"rows": rows, "columns": columns, "error": error}

    def close(self):
        if self.sock:
            self.sock.sendall(b'X' + struct.pack("!i", 4))  # Terminate
            self.sock.close()

def main():
    global passed, failed, total

    print("=" * 70)
    print("HarnessDB PostgreSQL Protocol CRUD Test")
    print("=" * 70)
    print(f"Port: {PORT}")
    print(f"Started at: {time.strftime('%Y-%m-%d %H:%M:%S')}")

    client = PgClient(HOST, PORT)

    try:
        client.connect(user="harness", password="harness-secret", database="harness")
        p(f"Connection startup (pid={client.pid})")
    except Exception as e:
        f("Connection", str(e))
        print("\nCannot proceed without connection.")
        sys.exit(1)

    # 1. Basic Queries
    section("Basic Queries")

    try:
        result = client.query("SELECT 1 AS test")
        if result["error"]:
            f("SELECT 1", result["error"])
        elif len(result["rows"]) >= 1:
            p("SELECT 1 AS test")
        else:
            f("SELECT 1", f"No rows returned: {result}")
    except Exception as e:
        f("SELECT 1", str(e))

    try:
        result = client.query("SELECT version()")
        if result["rows"] and len(result["rows"]) > 0:
            ver = result["rows"][0].get("version", "unknown")
            p(f"SELECT version() ({ver})")
        else:
            f("SELECT version()", f"No result: {result}")
    except Exception as e:
        f("SELECT version()", str(e))

    # 2. DDL Operations
    section("DDL (CREATE/DROP)")

    try:
        result = client.query("CREATE DATABASE IF NOT EXISTS pg_test")
        if result["error"]:
            f("CREATE DATABASE pg_test", result["error"])
        else:
            p("CREATE DATABASE pg_test")
    except Exception as e:
        f("CREATE DATABASE pg_test", str(e))

    try:
        result = client.query("CREATE TABLE IF NOT EXISTS pg_test_users (id INT, name VARCHAR(100), age INT, email VARCHAR(200))")
        if result["error"]:
            f("CREATE TABLE pg_test_users", result["error"])
        else:
            p("CREATE TABLE pg_test_users")
    except Exception as e:
        f("CREATE TABLE pg_test_users", str(e))

    try:
        result = client.query("CREATE TABLE IF NOT EXISTS pg_test_orders (id INT, user_id INT, amount FLOAT, status VARCHAR(50))")
        if result["error"]:
            f("CREATE TABLE pg_test_orders", result["error"])
        else:
            p("CREATE TABLE pg_test_orders")
    except Exception as e:
        f("CREATE TABLE pg_test_orders", str(e))

    try:
        result = client.query("SHOW TABLES")
        if result["rows"]:
            p(f"SHOW TABLES ({len(result['rows'])} tables)")
        else:
            f("SHOW TABLES", "No tables found")
    except Exception as e:
        f("SHOW TABLES", str(e))

    # 3. INSERT (CREATE)
    section("INSERT (CREATE)")

    statements = [
        "INSERT INTO pg_test_users VALUES (1, 'Alice', 30, 'alice@test.com')",
        "INSERT INTO pg_test_users VALUES (2, 'Bob', 25, 'bob@test.com')",
        "INSERT INTO pg_test_users VALUES (3, 'Charlie', 35, 'charlie@test.com')",
        "INSERT INTO pg_test_users VALUES (4, 'Diana', 28, 'diana@test.com')",
        "INSERT INTO pg_test_orders VALUES (1, 1, 99.99, 'completed')",
        "INSERT INTO pg_test_orders VALUES (2, 2, 199.50, 'pending')",
        "INSERT INTO pg_test_orders VALUES (3, 1, 50.00, 'completed')",
    ]

    for sql in statements:
        try:
            result = client.query(sql)
            if result["error"]:
                f(sql[:50], result["error"])
            else:
                p(sql[:50])
        except Exception as e:
            f(sql[:50], str(e))

    # 4. SELECT (READ)
    section("SELECT (READ)")

    try:
        result = client.query("SELECT * FROM pg_test_users")
        if result["rows"] and len(result["rows"]) >= 4:
            p(f"SELECT * users ({len(result['rows'])} rows)")
        else:
            f("SELECT * users", f"Expected >= 4 rows, got {len(result.get('rows', []))}")
    except Exception as e:
        f("SELECT * users", str(e))

    try:
        result = client.query("SELECT name, age FROM pg_test_users WHERE age > 28")
        if result["rows"] and len(result["rows"]) >= 2:
            p(f"SELECT WHERE age > 28 ({len(result['rows'])} rows)")
        else:
            f("SELECT WHERE age > 28", f"Expected >= 2 rows, got {len(result.get('rows', []))}")
    except Exception as e:
        f("SELECT WHERE", str(e))

    try:
        result = client.query("SELECT COUNT(*) FROM pg_test_users")
        if result["rows"]:
            p(f"SELECT COUNT(*) users")
        else:
            f("SELECT COUNT(*)", str(result))
    except Exception as e:
        f("SELECT COUNT(*)", str(e))

    try:
        result = client.query("SELECT * FROM pg_test_users ORDER BY age DESC LIMIT 2")
        if result["rows"] and len(result["rows"]) == 2:
            p("SELECT ORDER BY DESC LIMIT 2")
        else:
            f("SELECT ORDER BY LIMIT", f"Expected 2 rows, got {len(result.get('rows', []))}")
    except Exception as e:
        f("SELECT ORDER BY LIMIT", str(e))

    # 5. UPDATE
    section("UPDATE")

    try:
        result = client.query("UPDATE pg_test_users SET age = 31 WHERE name = 'Alice'")
        if result["error"]:
            f("UPDATE Alice age", result["error"])
        else:
            p("UPDATE Alice age = 31")
    except Exception as e:
        f("UPDATE Alice", str(e))

    try:
        result = client.query("SELECT age FROM pg_test_users WHERE name = 'Alice'")
        if result["rows"] and result["rows"][0].get("age") == "31":
            p("VERIFY updated age = 31")
        else:
            f("VERIFY updated age", f"Got: {result}")
    except Exception as e:
        f("VERIFY updated", str(e))

    # 6. DELETE
    section("DELETE")

    try:
        result = client.query("DELETE FROM pg_test_users WHERE name = 'Bob'")
        if result["error"]:
            f("DELETE Bob", result["error"])
        else:
            p("DELETE Bob")
    except Exception as e:
        f("DELETE Bob", str(e))

    try:
        result = client.query("SELECT COUNT(*) FROM pg_test_users")
        if result["rows"]:
            p("SELECT COUNT(*) after delete")
        else:
            f("SELECT COUNT(*) after delete", str(result))
    except Exception as e:
        f("SELECT COUNT after delete", str(e))

    # 7. DROP
    section("DROP")

    try:
        result = client.query("DROP TABLE IF EXISTS pg_test_orders")
        if result["error"]:
            f("DROP TABLE pg_test_orders", result["error"])
        else:
            p("DROP TABLE pg_test_orders")
    except Exception as e:
        f("DROP TABLE orders", str(e))

    try:
        result = client.query("DROP TABLE IF EXISTS pg_test_users")
        if result["error"]:
            f("DROP TABLE pg_test_users", result["error"])
        else:
            p("DROP TABLE pg_test_users")
    except Exception as e:
        f("DROP TABLE users", str(e))

    try:
        result = client.query("DROP DATABASE IF EXISTS pg_test")
        if result["error"]:
            f("DROP DATABASE pg_test", result["error"])
        else:
            p("DROP DATABASE pg_test")
    except Exception as e:
        f("DROP DATABASE", str(e))

    client.close()

    # Summary
    print()
    print("=" * 70)
    print("PostgreSQL CRUD Test Summary")
    print("=" * 70)
    print(f"Total:  {total}")
    print(f"\033[0;32mPassed: {passed}\033[0m")
    print(f"\033[0;31mFailed: {failed}\033[0m")
    print(f"Completed at: {time.strftime('%Y-%m-%d %H:%M:%S')}")
    print("=" * 70)

    sys.exit(1 if failed > 0 else 0)

if __name__ == "__main__":
    main()
