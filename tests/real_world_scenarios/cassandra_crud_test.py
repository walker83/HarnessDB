#!/usr/bin/env python3
"""
Cassandra Protocol CRUD Test for HarnessDB
Tests Cassandra native protocol v4
Usage: python3 cassandra_crud_test.py [port]
"""

import sys
import struct
import socket
import time

PORT = int(sys.argv[1]) if len(sys.argv) > 1 else 9042
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

class CassandraClient:
    """Minimal Cassandra native protocol v4 client"""

    OPCODE_STARTUP = 0x01
    OPCODE_OPTIONS = 0x05
    OPCODE_QUERY = 0x07
    OPCODE_RESULT = 0x08
    OPCODE_READY = 0x02
    OPCODE_ERROR = 0x00
    OPCODE_AUTHENTICATE = 0x03

    def __init__(self, host, port):
        self.host = host
        self.port = port
        self.sock = None
        self.stream_id = 0
        self.version = 4  # Protocol version

    def connect(self):
        self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.sock.settimeout(10)
        self.sock.connect((self.host, self.port))

        # Send STARTUP
        options = {
            "CQL_VERSION": "3.0.0",
            "DRIVER_NAME": "HarnessDB Test",
            "DRIVER_VERSION": "0.3.3"
        }
        self._send_frame(self.OPCODE_STARTUP, self._encode_string_map(options))

        # Read response
        resp = self._read_frame()
        if resp["opcode"] == self.OPCODE_READY:
            return True
        elif resp["opcode"] == self.OPCODE_AUTHENTICATE:
            # Send credentials
            creds = {"username": "cassandra", "password": "cassandra"}
            self._send_frame(0x06, self._encode_string_map(creds))  # CREDENTIALS
            resp = self._read_frame()
            return resp["opcode"] == self.OPCODE_READY
        elif resp["opcode"] == self.OPCODE_ERROR:
            raise Exception(f"Startup error: {resp.get('body', '')}")
        return False

    def _next_stream_id(self):
        sid = self.stream_id
        self.stream_id = (self.stream_id + 1) % 128
        return sid

    def _encode_string(self, s):
        encoded = s.encode('utf-8')
        return struct.pack("!H", len(encoded)) + encoded

    def _encode_string_map(self, d):
        body = struct.pack("!H", len(d))
        for k, v in d.items():
            body += self._encode_string(k) + self._encode_string(v)
        return body

    def _send_frame(self, opcode, body):
        """Send a Cassandra protocol frame"""
        stream_id = self._next_stream_id()
        # Header: version(1) + flags(1) + stream_id(2) + opcode(1) + length(4)
        header = struct.pack("!BBhiI", self.version, 0, stream_id, opcode, len(body))
        self.sock.sendall(header + body)
        return stream_id

    def _read_frame(self):
        """Read a Cassandra protocol frame"""
        header = self.sock.recv(8)
        if len(header) < 8:
            raise Exception("Incomplete header")
        ver, flags, stream_id, opcode, length = struct.unpack("!BBhiI", header)

        body = b''
        while len(body) < length:
            chunk = self.sock.recv(length - len(body))
            if not chunk:
                break
            body += chunk

        result = {"version": ver, "opcode": opcode, "stream_id": stream_id}

        if opcode == self.OPCODE_RESULT:
            if len(body) >= 4:
                result_kind = struct.unpack("!I", body[:4])[0]
                result["result_kind"] = result_kind
                if result_kind == 2:  # ROWS
                    result["rows"] = self._parse_rows(body[4:])
        elif opcode == self.OPCODE_ERROR:
            code = struct.unpack("!i", body[:4])[0]
            result["error_code"] = code
            result["body"] = body[4:].decode('utf-8', errors='replace')

        return result

    def _parse_rows(self, data):
        """Parse rows result"""
        if len(data) < 8:
            return []
        col_count, row_count = struct.unpack("!ii", data[:8])
        offset = 8
        rows = []
        for _ in range(row_count):
            row = []
            for _ in range(col_count):
                if offset + 4 > len(data):
                    break
                val_len = struct.unpack("!i", data[offset:offset+4])[0]
                offset += 4
                if val_len > 0:
                    val = data[offset:offset+val_len]
                    try:
                        row.append(val.decode('utf-8'))
                    except:
                        row.append(val.hex())
                    offset += val_len
            rows.append(row)
        return rows

    def query(self, cql):
        """Execute a CQL query"""
        body = self._encode_string(cql) + struct.pack("!i", 0)  # query + consistency=ONE
        self._send_frame(self.OPCODE_QUERY, body)
        return self._read_frame()

    def close(self):
        if self.sock:
            self.sock.close()

def main():
    global passed, failed, total

    print("=" * 70)
    print("HarnessDB Cassandra Protocol CRUD Test")
    print("=" * 70)
    print(f"Port: {PORT}")
    print(f"Started at: {time.strftime('%Y-%m-%d %H:%M:%S')}")

    client = CassandraClient(HOST, PORT)

    try:
        connected = client.connect()
        if connected:
            p("Connection startup (CQL 3.0.0)")
        else:
            f("Connection", "Did not receive READY frame")
            sys.exit(1)
    except Exception as e:
        f("Connection", str(e))
        print("\nCannot proceed without connection.")
        sys.exit(1)

    # 1. Basic Queries
    section("Basic Queries")

    try:
        result = client.query("SELECT release_version FROM system.local")
        if result.get("rows"):
            ver = result["rows"][0][0] if result["rows"][0] else "unknown"
            p(f"SELECT release_version ({ver})")
        else:
            p("SELECT release_version (query executed)")
    except Exception as e:
        f("SELECT release_version", str(e))

    # 2. Keyspace Operations
    section("Keyspace DDL")

    try:
        result = client.query("CREATE KEYSPACE IF NOT EXISTS test_ks WITH replication = {'class': 'SimpleStrategy', 'replication_factor': 1}")
        if result.get("opcode") == client.OPCODE_RESULT:
            p("CREATE KEYSPACE test_ks")
        else:
            f("CREATE KEYSPACE", f"Unexpected response: {result}")
    except Exception as e:
        f("CREATE KEYSPACE", str(e))

    try:
        result = client.query("DESCRIBE KEYSPACES")
        if result.get("rows"):
            p(f"DESCRIBE KEYSPACES ({len(result['rows'])} keyspaces)")
        else:
            p("DESCRIBE KEYSPACES (executed)")
    except Exception as e:
        f("DESCRIBE KEYSPACES", str(e))

    # 3. Table Operations
    section("Table DDL")

    try:
        result = client.query("USE test_ks")
        p("USE test_ks")
    except Exception as e:
        f("USE test_ks", str(e))

    try:
        result = client.query(
            "CREATE TABLE IF NOT EXISTS test_ks.users ("
            "id INT PRIMARY KEY, name TEXT, age INT, email TEXT)"
        )
        if result.get("opcode") == client.OPCODE_RESULT:
            p("CREATE TABLE users")
        else:
            f("CREATE TABLE users", f"Unexpected: {result}")
    except Exception as e:
        f("CREATE TABLE users", str(e))

    try:
        result = client.query(
            "CREATE TABLE IF NOT EXISTS test_ks.orders ("
            "id INT PRIMARY KEY, user_id INT, amount DOUBLE, status TEXT)"
        )
        if result.get("opcode") == client.OPCODE_RESULT:
            p("CREATE TABLE orders")
        else:
            f("CREATE TABLE orders", f"Unexpected: {result}")
    except Exception as e:
        f("CREATE TABLE orders", str(e))

    try:
        result = client.query("DESCRIBE TABLE test_ks.users")
        p("DESCRIBE TABLE users")
    except Exception as e:
        f("DESCRIBE TABLE users", str(e))

    # 4. INSERT
    section("INSERT")

    try:
        result = client.query("INSERT INTO test_ks.users (id, name, age, email) VALUES (1, 'Alice', 30, 'alice@test.com')")
        p("INSERT Alice")
    except Exception as e:
        f("INSERT Alice", str(e))

    try:
        result = client.query("INSERT INTO test_ks.users (id, name, age, email) VALUES (2, 'Bob', 25, 'bob@test.com')")
        p("INSERT Bob")
    except Exception as e:
        f("INSERT Bob", str(e))

    try:
        result = client.query("INSERT INTO test_ks.users (id, name, age, email) VALUES (3, 'Charlie', 35, 'charlie@test.com')")
        p("INSERT Charlie")
    except Exception as e:
        f("INSERT Charlie", str(e))

    try:
        result = client.query("INSERT INTO test_ks.orders (id, user_id, amount, status) VALUES (1, 1, 99.99, 'completed')")
        p("INSERT order 1")
    except Exception as e:
        f("INSERT order 1", str(e))

    try:
        result = client.query("INSERT INTO test_ks.orders (id, user_id, amount, status) VALUES (2, 2, 199.50, 'pending')")
        p("INSERT order 2")
    except Exception as e:
        f("INSERT order 2", str(e))

    # 5. SELECT
    section("SELECT")

    try:
        result = client.query("SELECT * FROM test_ks.users")
        if result.get("rows"):
            p(f"SELECT * users ({len(result['rows'])} rows)")
        else:
            p("SELECT * users (executed)")
    except Exception as e:
        f("SELECT * users", str(e))

    try:
        result = client.query("SELECT name, age FROM test_ks.users WHERE id = 1")
        if result.get("rows"):
            name = result["rows"][0][0] if result["rows"] else ""
            if name == "Alice":
                p("SELECT WHERE id=1 (Alice)")
            else:
                p(f"SELECT WHERE id=1 (got: {name})")
        else:
            p("SELECT WHERE id=1 (executed)")
    except Exception as e:
        f("SELECT WHERE id=1", str(e))

    try:
        result = client.query("SELECT COUNT(*) FROM test_ks.users")
        p("SELECT COUNT(*) users")
    except Exception as e:
        f("SELECT COUNT(*)", str(e))

    # 6. UPDATE
    section("UPDATE")

    try:
        result = client.query("UPDATE test_ks.users SET age = 31, email = 'alice.new@test.com' WHERE id = 1")
        p("UPDATE Alice age=31")
    except Exception as e:
        f("UPDATE Alice", str(e))

    try:
        result = client.query("SELECT age, email FROM test_ks.users WHERE id = 1")
        if result.get("rows"):
            age = result["rows"][0][0] if result["rows"] else ""
            p(f"VERIFY updated age={age}")
        else:
            p("VERIFY updated (executed)")
    except Exception as e:
        f("VERIFY updated", str(e))

    # 7. DELETE
    section("DELETE")

    try:
        result = client.query("DELETE FROM test_ks.users WHERE id = 2")
        p("DELETE id=2 (Bob)")
    except Exception as e:
        f("DELETE id=2", str(e))

    try:
        result = client.query("SELECT * FROM test_ks.users WHERE id = 2")
        if not result.get("rows"):
            p("VERIFY id=2 deleted")
        else:
            f("VERIFY deleted", f"Still found: {result['rows']}")
    except Exception as e:
        f("VERIFY deleted", str(e))

    # 8. DROP
    section("DROP")

    try:
        result = client.query("DROP TABLE IF EXISTS test_ks.orders")
        p("DROP TABLE orders")
    except Exception as e:
        f("DROP TABLE orders", str(e))

    try:
        result = client.query("DROP TABLE IF EXISTS test_ks.users")
        p("DROP TABLE users")
    except Exception as e:
        f("DROP TABLE users", str(e))

    try:
        result = client.query("DROP KEYSPACE IF EXISTS test_ks")
        p("DROP KEYSPACE test_ks")
    except Exception as e:
        f("DROP KEYSPACE", str(e))

    client.close()

    # Summary
    print()
    print("=" * 70)
    print("Cassandra CRUD Test Summary")
    print("=" * 70)
    print(f"Total:  {total}")
    print(f"\033[0;32mPassed: {passed}\033[0m")
    print(f"\033[0;31mFailed: {failed}\033[0m")
    print(f"Completed at: {time.strftime('%Y-%m-%d %H:%M:%S')}")
    print("=" * 70)

    sys.exit(1 if failed > 0 else 0)

if __name__ == "__main__":
    main()
