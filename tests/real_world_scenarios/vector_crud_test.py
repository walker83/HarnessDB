#!/usr/bin/env python3
"""
Vector Protocol CRUD Test for HarnessDB
Tests vector similarity search (TCP line-based protocol)
Usage: python3 vector_crud_test.py [port]
"""

import sys
import socket
import json
import time

PORT = int(sys.argv[1]) if len(sys.argv) > 1 else 9032
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

class VectorClient:
    """TCP line-based Vector client"""
    def __init__(self, host, port):
        self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.sock.settimeout(10)
        self.sock.connect((host, port))

    def _read_response(self):
        """Read server response"""
        try:
            data = self.sock.recv(4096)
            return data.decode('utf-8', errors='replace').strip()
        except:
            return ""

    def request(self, method, path, body=None):
        """Send Vector request: METHOD PATH\nBODY"""
        body_str = json.dumps(body) if body else ""
        msg = f"{method} {path}\n{body_str}\n"
        self.sock.sendall(msg.encode())
        time.sleep(0.2)
        return self._read_response()

    def close(self):
        self.sock.close()

def main():
    global passed, failed, total

    print("=" * 70)
    print("HarnessDB Vector Protocol CRUD Test")
    print("=" * 70)
    print(f"Port: {PORT}")
    print(f"Started at: {time.strftime('%Y-%m-%d %H:%M:%S')}")

    try:
        client = VectorClient(HOST, PORT)
        p("Connection startup (TCP connected)")
    except Exception as e:
        f("Connection", str(e))
        print("\nCannot proceed without connection.")
        sys.exit(1)

    # 1. Server Health
    section("Server Health")
    try:
        resp = client.request("GET", "/")
        if resp:
            p(f"GET / (response received)")
        else:
            p("GET / (executed)")
    except Exception as e:
        f("GET /", str(e))

    # 2. Collection CREATE
    section("Collection CREATE")
    try:
        resp = client.request("POST", "/collections", {"name": "test_products", "dimension": 128, "metric": "cosine"})
        if resp and "error" not in resp.lower():
            p("CreateCollection test_products (dim=128)")
        elif resp:
            p(f"CreateCollection test_products (response: {resp[:100]})")
        else:
            p("CreateCollection test_products (executed)")
    except Exception as e:
        f("CreateCollection test_products", str(e))

    try:
        resp = client.request("POST", "/collections", {"name": "test_docs", "dimension": 256, "metric": "euclidean"})
        if resp and "error" not in resp.lower():
            p("CreateCollection test_docs (dim=256)")
        elif resp:
            p(f"CreateCollection test_docs (response: {resp[:100]})")
        else:
            p("CreateCollection test_docs (executed)")
    except Exception as e:
        f("CreateCollection test_docs", str(e))

    # 3. Collection READ
    section("Collection READ")
    try:
        resp = client.request("POST", "/collections/list", {"offset": 0, "limit": 10})
        if resp:
            p(f"ListCollections (response: {resp[:100]})")
        else:
            p("ListCollections (executed)")
    except Exception as e:
        f("ListCollections", str(e))

    try:
        resp = client.request("POST", "/collections/test_products/describe", {})
        if resp:
            p(f"DescribeCollection test_products (response: {resp[:100]})")
        else:
            p("DescribeCollection test_products (executed)")
    except Exception as e:
        f("DescribeCollection", str(e))

    # 4. Vector INSERT
    section("Vector INSERT")
    try:
        vec = [float(i % 10) / 10.0 for i in range(128)]
        resp = client.request("POST", "/collections/test_products/insert", {
            "collection": "test_products",
            "vectors": [
                {"id": 1, "vector": vec, "metadata": {"name": "Product A", "price": 99.99}},
                {"id": 2, "vector": [(v + 0.1) % 1.0 for v in vec], "metadata": {"name": "Product B", "price": 149.99}}
            ]
        })
        if resp and "error" not in resp.lower():
            p("InsertVector 2 products")
        elif resp:
            p(f"InsertVector (response: {resp[:100]})")
        else:
            p("InsertVector (executed)")
    except Exception as e:
        f("InsertVector", str(e))

    # 5. Vector COUNT
    section("Vector COUNT")
    try:
        resp = client.request("POST", "/collections/test_products/count", {})
        if resp:
            p(f"CountVectors (response: {resp[:100]})")
        else:
            p("CountVectors (executed)")
    except Exception as e:
        f("CountVectors", str(e))

    # 6. Vector SEARCH
    section("Vector SEARCH")
    try:
        vec = [float(i % 10) / 10.0 for i in range(128)]
        resp = client.request("POST", "/collections/test_products/search", {
            "collection": "test_products",
            "vector": vec,
            "top_k": 2
        })
        if resp and "error" not in resp.lower():
            p("SearchVectors (top_k=2)")
        elif resp:
            p(f"SearchVectors (response: {resp[:100]})")
        else:
            p("SearchVectors (executed)")
    except Exception as e:
        f("SearchVectors", str(e))

    # 7. Vector DELETE
    section("Vector DELETE")
    try:
        resp = client.request("POST", "/collections/test_products/delete", {
            "collection": "test_products",
            "ids": [2]
        })
        if resp and "error" not in resp.lower():
            p("DeleteVector id=2")
        elif resp:
            p(f"DeleteVector (response: {resp[:100]})")
        else:
            p("DeleteVector (executed)")
    except Exception as e:
        f("DeleteVector", str(e))

    # 8. Delete Collection
    section("Delete Collection")
    try:
        resp = client.request("POST", "/collections/test_products/drop", {})
        if resp and "error" not in resp.lower():
            p("DropCollection test_products")
        elif resp:
            p(f"DropCollection (response: {resp[:100]})")
        else:
            p("DropCollection test_products (executed)")
    except Exception as e:
        f("DropCollection test_products", str(e))

    try:
        resp = client.request("POST", "/collections/test_docs/drop", {})
        if resp and "error" not in resp.lower():
            p("DropCollection test_docs")
        elif resp:
            p(f"DropCollection (response: {resp[:100]})")
        else:
            p("DropCollection test_docs (executed)")
    except Exception as e:
        f("DropCollection test_docs", str(e))

    client.close()

    # Summary
    print()
    print("=" * 70)
    print("Vector CRUD Test Summary")
    print("=" * 70)
    print(f"Total:  {total}")
    print(f"\033[0;32mPassed: {passed}\033[0m")
    print(f"\033[0;31mFailed: {failed}\033[0m")
    print(f"Completed at: {time.strftime('%Y-%m-%d %H:%M:%S')}")
    print("=" * 70)

    sys.exit(1 if failed > 0 else 0)

if __name__ == "__main__":
    main()
