#!/usr/bin/env python3
"""
MongoDB CRUD Test for HarnessDB
Tests MongoDB wire protocol (OP_MSG) with all CRUD operations.
Usage: python3 mongodb_crud_test.py [port]
"""

import sys
import time
from pymongo import MongoClient
from pymongo.errors import PyMongoError

PORT = int(sys.argv[1]) if len(sys.argv) > 1 else 27017
HOST = "127.0.0.1"
URI = f"mongodb://{HOST}:{PORT}/?serverSelectionTimeoutMS=5000"

passed = 0
failed = 0
total = 0

def p(name):
    global passed, total
    passed += 1
    total += 1
    print(f"  ✓ {name}")

def f(name, msg):
    global failed, total
    failed += 1
    total += 1
    print(f"  ✗ {name}: {msg}")

def section(name):
    print(f"\n\033[0;34m[{name}]\033[0m")

def main():
    global passed, failed, total

    print("=" * 70)
    print("HarnessDB MongoDB Protocol CRUD Test")
    print("=" * 70)
    print(f"Port: {PORT}")
    print(f"Started at: {time.strftime('%Y-%m-%d %H:%M:%S')}")

    try:
        client = MongoClient(URI)
    except Exception as e:
        print(f"\n✗ Cannot connect to MongoDB on port {PORT}: {e}")
        print(f"  Make sure harness-server is running with MongoDB protocol enabled.")
        sys.exit(1)

    # 1. Connection & Server Info
    section("Connection & Server Info")
    try:
        result = client.admin.command("ping")
        assert result.get("ok") == 1
        p("ping")
    except Exception as e:
        f("ping", str(e))

    try:
        result = client.admin.command("ismaster")
        assert result.get("ismaster") == True
        assert result.get("ok") == 1
        p("ismaster")
    except Exception as e:
        f("ismaster", str(e))

    try:
        result = client.admin.command("hello")
        assert result.get("ok") == 1
        p("hello")
    except Exception as e:
        f("hello", str(e))

    try:
        result = client.admin.command("buildInfo")
        assert result.get("ok") == 1
        assert "7.0.0" in result.get("version", "")
        p("buildInfo")
    except Exception as e:
        f("buildInfo", str(e))

    try:
        result = client.admin.command("serverStatus")
        assert result.get("ok") == 1
        assert "harness" in result.get("host", "")
        p("serverStatus")
    except Exception as e:
        f("serverStatus", str(e))

    # 2. Database Operations
    section("Database Operations")
    try:
        result = client.admin.command("listDatabases")
        assert "databases" in result or result.get("ok") == 1
        p("listDatabases")
    except Exception as e:
        f("listDatabases", str(e))

    test_db = client["harness_test"]

    # 3. Collection Operations
    section("Collection CRUD")
    try:
        test_db.create_collection("users")
        p("createCollection users")
    except Exception as e:
        f("createCollection users", str(e))

    try:
        test_db.create_collection("orders")
        p("createCollection orders")
    except Exception as e:
        f("createCollection orders", str(e))

    try:
        collections = test_db.list_collection_names()
        assert "users" in collections
        p("list_collection_names")
    except Exception as e:
        f("list_collection_names", str(e))

    # 4. Document Insert (CREATE)
    section("Document CREATE (Insert)")
    try:
        result = test_db.users.insert_one({
            "name": "Alice",
            "age": 30,
            "email": "alice@test.com",
            "role": "admin",
            "active": True,
            "tags": ["python", "dev"],
            "address": {"city": "Beijing", "zip": "100000"}
        })
        assert result.inserted_id is not None
        alice_id = result.inserted_id
        p("insert_one Alice")
    except Exception as e:
        f("insert_one Alice", str(e))

    try:
        result = test_db.users.insert_many([
            {"name": "Bob", "age": 25, "email": "bob@test.com", "role": "user"},
            {"name": "Charlie", "age": 35, "email": "charlie@test.com", "role": "user"},
            {"name": "Diana", "age": 28, "email": "diana@test.com", "role": "admin"}
        ])
        assert len(result.inserted_ids) == 3
        p("insert_many 3 users")
    except Exception as e:
        f("insert_many 3 users", str(e))

    # 5. Document READ
    section("Document READ (Query)")
    try:
        doc = test_db.users.find_one({"name": "Alice"})
        assert doc is not None
        assert doc["name"] == "Alice"
        assert doc["age"] == 30
        p("find_one Alice")
    except Exception as e:
        f("find_one Alice", str(e))

    try:
        docs = list(test_db.users.find({"role": "admin"}))
        assert len(docs) >= 2  # Alice + Diana
        p("find role=admin")
    except Exception as e:
        f("find role=admin", str(e))

    try:
        docs = list(test_db.users.find({"age": {"$gte": 30}}))
        assert len(docs) >= 2  # Alice(30) + Charlie(35)
        p("find age >= 30")
    except Exception as e:
        f("find age >= 30", str(e))

    try:
        docs = list(test_db.users.find({"name": {"$in": ["Alice", "Bob"]}}))
        assert len(docs) == 2
        p("find name $in [Alice, Bob]")
    except Exception as e:
        f("find name $in", str(e))

    try:
        docs = list(test_db.users.find({"email": {"$regex": ".*@test.com"}}))
        assert len(docs) >= 4
        p("find email $regex")
    except Exception as e:
        f("find email $regex", str(e))

    try:
        docs = list(test_db.users.find({"address.city": "Beijing"}))
        assert len(docs) >= 1
        p("find nested address.city")
    except Exception as e:
        f("find nested field", str(e))

    try:
        count = test_db.users.count_documents({})
        assert count >= 4
        p(f"count_documents (total={count})")
    except Exception as e:
        f("count_documents", str(e))

    # 6. Document UPDATE
    section("Document UPDATE")
    try:
        result = test_db.users.update_one(
            {"name": "Alice"},
            {"$set": {"age": 31, "role": "superadmin"}}
        )
        assert result.modified_count == 1
        doc = test_db.users.find_one({"name": "Alice"})
        assert doc["age"] == 31
        assert doc["role"] == "superadmin"
        p("update_one Alice (set age+role)")
    except Exception as e:
        f("update_one Alice", str(e))

    try:
        result = test_db.users.update_many(
            {"role": "user"},
            {"$set": {"status": "active"}}
        )
        assert result.modified_count >= 2
        p(f"update_many users (modified={result.modified_count})")
    except Exception as e:
        f("update_many users", str(e))

    # 7. Document DELETE
    section("Document DELETE")
    try:
        result = test_db.users.delete_one({"name": "Bob"})
        assert result.deleted_count == 1
        doc = test_db.users.find_one({"name": "Bob"})
        assert doc is None
        p("delete_one Bob")
    except Exception as e:
        f("delete_one Bob", str(e))

    try:
        result = test_db.users.delete_many({"role": "user"})
        assert result.deleted_count >= 1
        p(f"delete_many role=user (deleted={result.deleted_count})")
    except Exception as e:
        f("delete_many role=user", str(e))

    try:
        count = test_db.users.count_documents({})
        assert count >= 1  # Alice should still exist
        p(f"count after deletes (total={count})")
    except Exception as e:
        f("count after deletes", str(e))

    # 8. Drop collections and database
    section("Drop Operations")
    try:
        test_db.drop_collection("orders")
        collections = test_db.list_collection_names()
        assert "orders" not in collections
        p("dropCollection orders")
    except Exception as e:
        f("dropCollection orders", str(e))

    try:
        test_db.drop_collection("users")
        collections = test_db.list_collection_names()
        assert "users" not in collections
        p("dropCollection users")
    except Exception as e:
        f("dropCollection users", str(e))

    # 9. Orders CRUD
    section("Orders CRUD (Full Lifecycle)")
    try:
        test_db.orders.insert_many([
            {"user_id": 1, "amount": 99.99, "status": "pending", "created_at": "2024-01-01"},
            {"user_id": 2, "amount": 199.50, "status": "completed", "created_at": "2024-01-15"},
            {"user_id": 1, "amount": 50.00, "status": "completed", "created_at": "2024-02-01"},
        ])
        p("insert_many orders")
    except Exception as e:
        f("insert_many orders", str(e))

    try:
        docs = list(test_db.orders.find({"status": "completed"}))
        assert len(docs) == 2
        p("find completed orders")
    except Exception as e:
        f("find completed orders", str(e))

    try:
        result = test_db.orders.update_one(
            {"user_id": 1, "status": "pending"},
            {"$set": {"status": "shipped"}}
        )
        assert result.modified_count == 1
        p("update order status")
    except Exception as e:
        f("update order status", str(e))

    try:
        result = test_db.orders.delete_one({"user_id": 1, "status": "shipped"})
        assert result.deleted_count == 1
        p("delete shipped order")
    except Exception as e:
        f("delete shipped order", str(e))

    # Cleanup
    client.drop_database("harness_test")
    client.close()

    # Summary
    print()
    print("=" * 70)
    print("MongoDB CRUD Test Summary")
    print("=" * 70)
    print(f"Total:  {total}")
    print(f"\033[0;32mPassed: {passed}\033[0m")
    print(f"\033[0;31mFailed: {failed}\033[0m")
    print(f"Completed at: {time.strftime('%Y-%m-%d %H:%M:%S')}")
    print("=" * 70)

    sys.exit(1 if failed > 0 else 0)

if __name__ == "__main__":
    main()
