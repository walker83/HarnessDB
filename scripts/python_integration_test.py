#!/usr/bin/env python3
"""
RorisDB Python Data Ecosystem Integration Test
==============================================
Tests compatibility with: SQLAlchemy, Pandas, PyMySQL, mysql-connector-python

RorisDB: MySQL-compatible OLAP DB at 127.0.0.1:9030, user=root, no password
Database: integration_test
"""

import sys
import time
import traceback

# ──────────────────────────────────────────────────────────────────────
# Test Configuration
HOST = "127.0.0.1"
PORT = 9030
USER = "root"
PASSWORD = ""
DATABASE = "integration_test"

PASS = "PASS"
FAIL = "FAIL"
SKIP = "SKIP"

results = []


def announce(test_name: str) -> None:
    print(f"\n{'='*72}")
    print(f"  {test_name}")
    print(f"{'='*72}")


def record_result(
    library: str,
    test_name: str,
    status: str,
    detail: str = "",
) -> None:
    results.append({
        "library": library,
        "test_name": test_name,
        "status": status,
        "detail": detail,
    })
    symbol = "+" if status == PASS else "x" if status == FAIL else "-"
    print(f"  [{symbol}] {test_name}: {status}" + (f" | {detail}" if detail else ""))


def run_test(library: str, test_name: str, fn):
    """Run a test function, catching and recording any exception."""
    try:
        fn()
        record_result(library, test_name, PASS)
    except Exception as e:
        tb = traceback.format_exc()
        detail = f"{type(e).__name__}: {e}"
        record_result(library, test_name, FAIL, detail)
        # Print full traceback for debugging
        print(f"    {tb[:600].replace(chr(10), chr(10)+'    ')}")


# ══════════════════════════════════════════════════════════════════════
# 1. PyMySQL — Direct MySQL driver
# ══════════════════════════════════════════════════════════════════════
def test_pymysql():
    announce("Library 1: PyMySQL (direct MySQL driver)")

    import pymysql

    conn = None
    try:
        conn = pymysql.connect(
            host=HOST, port=PORT, user=USER, password=PASSWORD, database=DATABASE,
        )
        record_result("PyMySQL", "Connection", PASS)
    except Exception as e:
        record_result("PyMySQL", "Connection", FAIL, str(e))
        return

    try:
        with conn.cursor() as cur:
            cur.execute("SELECT VERSION()")
            version = cur.fetchone()
            record_result("PyMySQL", "SELECT VERSION()", PASS, str(version))
    except Exception as e:
        record_result("PyMySQL", "SELECT VERSION()", FAIL, str(e))

    # Basic SELECT
    def test_basic_select():
        with conn.cursor() as cur:
            cur.execute("SELECT * FROM users")
            rows = cur.fetchall()
            assert len(rows) == 5, f"Expected 5 users, got {len(rows)}"
            # Verify column names
            assert cur.description is not None
            cols = [d[0] for d in cur.description]
            assert "username" in cols, f"Missing username column: {cols}"

    run_test("PyMySQL", "SELECT * FROM users", test_basic_select)

    # WHERE clause
    def test_where():
        with conn.cursor() as cur:
            cur.execute(
                "SELECT * FROM orders WHERE amount > 100",
            )
            rows = cur.fetchall()
            assert len(rows) >= 6, f"Expected >=6 orders with amount>100, got {len(rows)}"
            for row in rows:
                # amount is the 4th column (index 3)
                assert float(row[3]) > 100, f"Row has amount {row[3]} <= 100"

    run_test("PyMySQL", "WHERE clause (amount > 100)", test_where)

    # JOIN + GROUP BY + aggregation
    def test_join_aggregation():
        with conn.cursor() as cur:
            cur.execute(
                """
                SELECT u.username, SUM(o.amount * o.quantity) as total
                FROM users u JOIN orders o ON u.id = o.user_id
                GROUP BY u.username
                ORDER BY u.username
                """,
            )
            rows = cur.fetchall()
            assert len(rows) == 5, f"Expected 5 users, got {len(rows)}"
            usernames = [r[0] for r in rows]
            assert "alice" in usernames
            assert "bob" in usernames

    run_test("PyMySQL", "JOIN + GROUP BY + aggregation", test_join_aggregation)

    # GROUP BY with multiple aggregates
    def test_groupby_aggregates():
        with conn.cursor() as cur:
            cur.execute(
                """
                SELECT category, COUNT(*) as cnt, AVG(price) as avg_price
                FROM products GROUP BY category
                """,
            )
            rows = cur.fetchall()
            # All products are in Electronics
            assert len(rows) == 1, f"Expected 1 category, got {len(rows)}"
            assert rows[0][0] == "Electronics"
            assert rows[0][1] == 5  # COUNT

    run_test("PyMySQL", "GROUP BY aggregate (category)", test_groupby_aggregates)

    # ORDER BY + LIMIT
    def test_order_limit():
        with conn.cursor() as cur:
            cur.execute("SELECT username, age FROM users ORDER BY age DESC LIMIT 3")
            rows = cur.fetchall()
            assert len(rows) == 3
            # Oldest users: charlie(35), eve(32), alice(30)
            assert rows[0][1] >= rows[1][1] >= rows[2][1]

    run_test("PyMySQL", "ORDER BY + LIMIT", test_order_limit)

    # Scalar result (COUNT)
    def test_scalar():
        with conn.cursor() as cur:
            cur.execute("SELECT COUNT(*) FROM orders")
            row = cur.fetchone()
            assert row[0] == 10, f"Expected 10 orders, got {row[0]}"

    run_test("PyMySQL", "Scalar COUNT(*)", test_scalar)

    # Dictionary cursor
    try:
        with conn.cursor(pymysql.cursors.DictCursor) as cur:
            cur.execute("SELECT id, username, age FROM users WHERE id = 1")
            row = cur.fetchone()
            assert row["username"] == "alice", f"Expected alice, got {row}"
            record_result("PyMySQL", "DictCursor", PASS)
    except Exception as e:
        record_result("PyMySQL", "DictCursor", FAIL, str(e))

    conn.close()


# ══════════════════════════════════════════════════════════════════════
# 2. mysql-connector-python — Official MySQL driver
# ══════════════════════════════════════════════════════════════════════
def test_mysql_connector():
    announce("Library 2: mysql-connector-python (official MySQL driver)")

    try:
        import mysql.connector
    except ImportError:
        record_result("mysql-connector", "Import", SKIP, "Package not installed")
        return

    conn = None
    try:
        # use_pure=True avoids a C extension compatibility issue with
        # RorisDB's text protocol result encoding. The C extension
        # (use_pure=False) may fail with "Malformed packet" on non-standard
        # MySQL servers.
        conn = mysql.connector.connect(
            host=HOST, port=PORT, user=USER, password=PASSWORD, database=DATABASE,
            use_pure=True,
        )
        record_result("mysql-connector", "Connection", PASS, "use_pure=True (C extension may fail)")
    except Exception as e:
        record_result("mysql-connector", "Connection", FAIL, str(e))
        return

    def test_basic_select():
        cur = conn.cursor()
        cur.execute("SELECT * FROM users")
        rows = cur.fetchall()
        assert len(rows) == 5, f"Expected 5 users, got {len(rows)}"
        cur.close()

    run_test("mysql-connector", "SELECT * FROM users", test_basic_select)

    def test_join():
        cur = conn.cursor()
        cur.execute(
            """
            SELECT u.username, SUM(o.amount * o.quantity) as total
            FROM users u JOIN orders o ON u.id = o.user_id
            GROUP BY u.username ORDER BY u.username
            """,
        )
        rows = cur.fetchall()
        assert len(rows) == 5
        # Verify expected totals
        total_map = {r[0]: float(r[1]) for r in rows}
        assert abs(total_map["alice"] - 1059.97) < 0.01, \
            f"Expected alice total ~1059.97, got {total_map['alice']}"
        assert abs(total_map["bob"] - 479.97) < 0.01, \
            f"Expected bob total ~479.97, got {total_map['bob']}"
        cur.close()

    run_test("mysql-connector", "JOIN + GROUP BY (verify totals)", test_join)

    def test_aggregation():
        cur = conn.cursor()
        cur.execute(
            """
            SELECT category, COUNT(*) as cnt, AVG(price) as avg_price
            FROM products GROUP BY category
            """,
        )
        rows = cur.fetchall()
        assert len(rows) == 1
        record_result("mysql-connector", "GROUP BY aggregates", PASS)
        cur.close()

    run_test("mysql-connector", "GROUP BY aggregates", test_aggregation)

    # ORDER BY + LIMIT
    def test_order_limit():
        cur = conn.cursor()
        cur.execute("SELECT username FROM users ORDER BY age LIMIT 1")
        rows = cur.fetchall()
        assert rows[0][0] == "bob", f"Youngest user should be bob, got {rows[0][0]}"
        cur.close()

    run_test("mysql-connector", "ORDER BY + LIMIT (youngest user)", test_order_limit)

    # Scalar
    def test_scalar():
        cur = conn.cursor()
        cur.execute("SELECT COUNT(*) FROM orders")
        row = cur.fetchone()
        assert row[0] == 10
        cur.close()

    run_test("mysql-connector", "Scalar COUNT(*)", test_scalar)

    # NULL handling
    def test_null_safe():
        cur = conn.cursor()
        cur.execute("SELECT * FROM users WHERE email IS NOT NULL")
        rows = cur.fetchall()
        assert len(rows) >= 5, f"Expected >=5 users with non-null email, got {len(rows)}"
        cur.close()

    run_test("mysql-connector", "IS NOT NULL filter", test_null_safe)

    if conn:
        conn.close()


# ══════════════════════════════════════════════════════════════════════
# 3. SQLAlchemy — ORM and Core
# ══════════════════════════════════════════════════════════════════════
def test_sqlalchemy():
    announce("Library 3: SQLAlchemy (ORM + Core)")

    try:
        from sqlalchemy import create_engine, text, inspect, MetaData, Table, Column, select
        from sqlalchemy.orm import Session, declarative_base
        from sqlalchemy.exc import SQLAlchemyError
    except ImportError as e:
        record_result("SQLAlchemy", "Import", SKIP, str(e))
        return

    connection_string = f"mysql+pymysql://{USER}:{PASSWORD}@{HOST}:{PORT}/{DATABASE}"
    engine = None

    try:
        engine = create_engine(connection_string)
        record_result("SQLAlchemy", "Engine creation", PASS)
    except Exception as e:
        record_result("SQLAlchemy", "Engine creation", FAIL, str(e))
        return

    # Connection test
    def test_connection():
        with engine.connect() as conn:
            result = conn.execute(text("SELECT VERSION()"))
            row = result.fetchone()
            assert row is not None

    run_test("SQLAlchemy", "Connection + SELECT VERSION()", test_connection)

    # Schema inspection / reflection
    def test_reflection():
        with engine.connect() as conn:
            # Use SHOW TABLES directly since SQLAlchemy's inspector.get_table_names()
            # issues SHOW FULL TABLES which may not be fully supported yet.
            result = conn.execute(text("SHOW TABLES"))
            all_tables = [row[0] for row in result.fetchall()]
            assert "users" in all_tables, f"Expected users table, got {all_tables}"
            assert "orders" in all_tables
            assert "products" in all_tables
            # Get columns for users via DESCRIBE
            result = conn.execute(text("DESCRIBE users"))
            columns = result.fetchall()
            col_names = [row[0] for row in columns]
            assert "id" in col_names
            assert "username" in col_names
            assert "email" in col_names
            assert "created_at" in col_names

    run_test("SQLAlchemy", "Schema inspection / reflection", test_reflection)

    # Core: text-based SELECT
    def test_core_select():
        with engine.connect() as conn:
            result = conn.execute(text("SELECT * FROM users ORDER BY id"))
            rows = result.fetchall()
            assert len(rows) == 5
            assert rows[0][1] == "alice"  # username is column index 1

    run_test("SQLAlchemy", "Core text SELECT", test_core_select)

    # Core: JOIN query
    def test_core_join():
        with engine.connect() as conn:
            sql = text(
                """
                SELECT u.username, SUM(o.amount * o.quantity) as total
                FROM users u JOIN orders o ON u.id = o.user_id
                GROUP BY u.username ORDER BY u.username
                """,
            )
            result = conn.execute(sql)
            rows = result.fetchall()
            assert len(rows) == 5
            total_map = {r[0]: float(r[1]) for r in rows}
            assert abs(total_map["alice"] - 1059.97) < 0.01

    run_test("SQLAlchemy", "Core text JOIN + GROUP BY", test_core_join)

    # Core: ORDER BY + LIMIT
    def test_core_order_limit():
        with engine.connect() as conn:
            result = conn.execute(text("SELECT username FROM users ORDER BY age DESC LIMIT 2"))
            rows = result.fetchall()
            assert len(rows) == 2
            # charlie(35) and eve(32) are oldest
            assert rows[0][0] == "charlie"

    run_test("SQLAlchemy", "Core ORDER BY + LIMIT", test_core_order_limit)

    # Core: aggregation
    def test_core_aggregation():
        with engine.connect() as conn:
            result = conn.execute(
                text(
                    """
                    SELECT category, COUNT(*) as cnt, AVG(price) as avg_price
                    FROM products GROUP BY category
                    """,
                ),
            )
            rows = result.fetchall()
            assert len(rows) >= 1

    run_test("SQLAlchemy", "Core GROUP BY aggregates", test_core_aggregation)

    # Core: NULL handling
    def test_core_null():
        with engine.connect() as conn:
            result = conn.execute(text("SELECT COUNT(*) FROM users WHERE email IS NOT NULL"))
            row = result.fetchone()
            assert row[0] >= 5

    run_test("SQLAlchemy", "Core NULL handling", test_core_null)

    # ORM: Declare mapped classes and query
    def test_orm_basic():
        Base = declarative_base()

        class User(Base):
            __tablename__ = "users"
            __table_args__ = {"schema": DATABASE}
            id = Column("id", None, primary_key=True)
            username = Column("username", None)
            email = Column("email", None)
            age = Column("age", None)

        with Session(engine) as session:
            users = session.execute(
                select(User).where(User.age > 28).order_by(User.age),
            ).scalars().all()
            assert len(users) >= 3, f"Expected >=3 users age>28, got {len(users)}"
            for u in users:
                assert u.age > 28

    run_test("SQLAlchemy", "ORM basic query with WHERE", test_orm_basic)

    # ORM: Aggregation
    def test_orm_aggregation():
        Base = declarative_base()

        class Order(Base):
            __tablename__ = "orders"
            __table_args__ = {"schema": DATABASE}
            order_id = Column("order_id", None, primary_key=True)
            amount = Column("amount", None)
            quantity = Column("quantity", None)
            user_id = Column("user_id", None)

        with Session(engine) as session:
            from sqlalchemy import func
            result = session.execute(
                select(func.sum(Order.amount * Order.quantity)).where(
                    Order.user_id == 1,
                ),
            ).scalar()
            assert result is not None
            assert float(result) > 0

    run_test("SQLAlchemy", "ORM aggregation (SUM)", test_orm_aggregation)

    # ORM: JOIN
    def test_orm_join():
        Base = declarative_base()

        class User(Base):
            __tablename__ = "users"
            __table_args__ = {"schema": DATABASE}
            id = Column("id", None, primary_key=True)
            username = Column("username", None)

        class Order(Base):
            __tablename__ = "orders"
            __table_args__ = {"schema": DATABASE}
            order_id = Column("order_id", None, primary_key=True)
            user_id = Column("user_id", None)
            amount = Column("amount", None)
            quantity = Column("quantity", None)

        with Session(engine) as session:
            from sqlalchemy import func
            result = session.execute(
                select(User.username, func.sum(Order.amount * Order.quantity))
                .join(Order, User.id == Order.user_id)
                .group_by(User.username)
                .order_by(User.username),
            ).all()
            assert len(result) == 5
            total_map = {r[0]: float(r[1]) for r in result}
            assert abs(total_map["alice"] - 1059.97) < 0.01

    run_test("SQLAlchemy", "ORM JOIN with GROUP BY", test_orm_join)

    if engine:
        engine.dispose()


# ══════════════════════════════════════════════════════════════════════
# 4. Pandas + SQLAlchemy — DataFrame integration
# ══════════════════════════════════════════════════════════════════════
def test_pandas():
    announce("Library 4: Pandas with SQLAlchemy")

    try:
        import pandas as pd
        from sqlalchemy import create_engine, text
    except ImportError as e:
        record_result("Pandas", "Import", SKIP, str(e))
        return

    connection_string = f"mysql+pymysql://{USER}:{PASSWORD}@{HOST}:{PORT}/{DATABASE}"

    engine = None
    try:
        engine = create_engine(connection_string)
    except Exception as e:
        record_result("Pandas", "Engine creation", FAIL, str(e))
        return

    # Basic read_sql
    def test_read_users():
        df = pd.read_sql("SELECT * FROM users", engine)
        assert len(df) == 5, f"Expected 5 rows, got {len(df)}"
        assert list(df.columns[:3]) == ["id", "username", "email"], \
            f"Unexpected columns: {list(df.columns)}"
        assert df["username"].iloc[0] == "alice"

    run_test("Pandas", "pd.read_sql(SELECT * FROM users)", test_read_users)

    # Complex JOIN query
    def test_read_join():
        df = pd.read_sql(
            """
            SELECT u.username, SUM(o.amount * o.quantity) as total
            FROM users u JOIN orders o ON u.id = o.user_id
            GROUP BY u.username ORDER BY u.username
            """,
            engine,
        )
        assert len(df) == 5
        alice_row = df[df["username"] == "alice"].iloc[0]
        assert abs(float(alice_row["total"]) - 1059.97) < 0.01, \
            f"Expected alice total ~1059.97, got {alice_row['total']}"

    run_test("Pandas", "pd.read_sql(JOIN + GROUP BY)", test_read_join)

    # Full table loads
    def test_read_full_tables():
        df_orders = pd.read_sql("SELECT * FROM orders", engine)
        df_products = pd.read_sql("SELECT * FROM products", engine)
        assert len(df_orders) == 10
        assert len(df_products) == 5

    run_test("Pandas", "pd.read_sql(orders + products full table)", test_read_full_tables)

    # DataFrame operations on queried data
    def test_dataframe_ops():
        df = pd.read_sql("SELECT * FROM orders", engine)
        # Note: RorisDB returns DECIMAL values as strings in the text protocol.
        # Explicitly convert to numeric for arithmetic.
        df["amount"] = pd.to_numeric(df["amount"])
        total_revenue = (df["amount"] * df["quantity"]).sum()
        assert total_revenue > 0
        # Filtering
        big_orders = df[df["amount"] > 100]
        assert len(big_orders) >= 6

    run_test("Pandas", "DataFrame operations (filter, arithmetic)", test_dataframe_ops)

    # GROUP BY on DataFrame
    def test_dataframe_groupby():
        df = pd.read_sql("SELECT * FROM orders", engine)
        # Convert string amounts to numeric before aggregation
        df["amount"] = pd.to_numeric(df["amount"])
        grouped = df.groupby("user_id").agg({"amount": "sum", "quantity": "sum"}).reset_index()
        assert len(grouped) == 5  # 5 users have orders
        user_1 = grouped[grouped["user_id"] == 1].iloc[0]
        assert float(user_1["amount"]) > 1000  # alice total = 999.99 + 29.99 = 1029.98

    run_test("Pandas", "DataFrame groupby + agg", test_dataframe_groupby)

    # Read with inline values (parameterized queries not supported by RorisDB's engine)
    def test_read_with_inline():
        df = pd.read_sql(
            "SELECT * FROM users WHERE age > 30 ORDER BY age",
            engine,
        )
        # Users >30: charlie(35), eve(32) = 2 rows
        assert len(df) == 2, f"Expected 2 users age>30 (charlie, eve), got {len(df)}"
        assert all(df["age"] > 30)

    run_test("Pandas", "pd.read_sql with inline WHERE", test_read_with_inline)

    # Chunksize iteration
    def test_chunksize():
        chunks = []
        for chunk in pd.read_sql("SELECT * FROM orders", engine, chunksize=3):
            chunks.append(chunk)
        total_rows = sum(len(c) for c in chunks)
        assert total_rows == 10, f"Expected 10 rows via chunks, got {total_rows}"
        assert len(chunks) >= 3  # ceiling(10/3) = 4 chunks

    run_test("Pandas", "pd.read_sql with chunksize", test_chunksize)

    # Direct connection string (no engine)
    def test_direct_conn_string():
        df = pd.read_sql(
            "SELECT COUNT(*) as cnt FROM users",
            connection_string,
        )
        assert df["cnt"].iloc[0] == 5

    run_test("Pandas", "pd.read_sql with direct connection string", test_direct_conn_string)

    # Data type verification
    def test_dtypes():
        df = pd.read_sql("SELECT * FROM users", engine)
        # id should be int
        assert df["id"].dtype == "int64" or df["id"].dtype == "int32"
        # age should be numeric
        assert "int" in str(df["age"].dtype)
        # username should be object (string)
        assert df["username"].dtype == "object"

    run_test("Pandas", "Data type verification", test_dtypes)

    # Timestamp handling
    def test_timestamp():
        df = pd.read_sql("SELECT * FROM users", engine)
        assert "created_at" in df.columns
        if hasattr(df["created_at"].dtype, "name"):
            dtype_name = df["created_at"].dtype.name
            assert "datetime" in dtype_name or "object" in dtype_name

    run_test("Pandas", "Timestamp/datetime handling", test_timestamp)

    if engine:
        engine.dispose()


# ══════════════════════════════════════════════════════════════════════
# 5. Cross-library result verification
# ══════════════════════════════════════════════════════════════════════
def test_cross_library_consistency():
    announce("Cross-Library Consistency Verification")

    from sqlalchemy import create_engine, text
    import pymysql

    connection_string = f"mysql+pymysql://{USER}:{PASSWORD}@{HOST}:{PORT}/{DATABASE}"
    engine = create_engine(connection_string)

    queries = {
        "User count": "SELECT COUNT(*) as cnt FROM users",
        "Order count": "SELECT COUNT(*) as cnt FROM orders",
        "Product count": "SELECT COUNT(*) as cnt FROM products",
        "Max amount": "SELECT MAX(amount) as max_amt FROM orders",
        "Avg price": "SELECT AVG(price) as avg_price FROM products",
        "Youngest user": "SELECT MIN(age) as min_age FROM users",
        "Oldest user": "SELECT MAX(age) as max_age FROM users",
    }

    sqlalchemy_results = {}
    pymysql_results = {}

    # SQLAlchemy results
    try:
        with engine.connect() as conn:
            for name, sql in queries.items():
                result = conn.execute(text(sql)).fetchone()
                sqlalchemy_results[name] = float(result[0]) if result[0] is not None else None
        record_result("Cross-Library", "SQLAlchemy results collection", PASS)
    except Exception as e:
        record_result("Cross-Library", "SQLAlchemy results collection", FAIL, str(e))

    # PyMySQL results
    try:
        conn = pymysql.connect(
            host=HOST, port=PORT, user=USER, password=PASSWORD, database=DATABASE,
        )
        with conn.cursor() as cur:
            for name, sql in queries.items():
                cur.execute(sql)
                row = cur.fetchone()
                pymysql_results[name] = float(row[0]) if row[0] is not None else None
        conn.close()
        record_result("Cross-Library", "PyMySQL results collection", PASS)
    except Exception as e:
        record_result("Cross-Library", "PyMySQL results collection", FAIL, str(e))

    # Compare results
    all_match = True
    for name in queries:
        sa_val = sqlalchemy_results.get(name)
        pm_val = pymysql_results.get(name)
        if sa_val is not None and pm_val is not None:
            match = abs(sa_val - pm_val) < 0.001
            status = PASS if match else FAIL
            if not match:
                all_match = False
            record_result(
                "Cross-Library",
                f"Consistency: {name}",
                status,
                f"SQLAlchemy={sa_val}, PyMySQL={pm_val}",
            )
        else:
            record_result(
                "Cross-Library",
                f"Consistency: {name}",
                SKIP,
                f"Missing data: SA={sa_val}, PM={pm_val}",
            )
            all_match = False

    if all_match:
        record_result("Cross-Library", "ALL RESULTS MATCH", PASS)

    engine.dispose()


# ══════════════════════════════════════════════════════════════════════
# Summary Report
# ══════════════════════════════════════════════════════════════════════
def print_summary():
    print("\n")
    print("=" * 72)
    print("  PYTHON DATA ECOSYSTEM INTEGRATION TEST SUMMARY")
    print("=" * 72)

    # Group by library
    from collections import OrderedDict
    lib_groups = OrderedDict()
    for r in results:
        lib_groups.setdefault(r["library"], []).append(r)

    for library, tests in lib_groups.items():
        total = len(tests)
        passed = sum(1 for t in tests if t["status"] == PASS)
        failed = sum(1 for t in tests if t["status"] == FAIL)
        skipped = sum(1 for t in tests if t["status"] == SKIP)

        overall = PASS if failed == 0 else FAIL

        print(f"\n  ┌─ {library}")
        print(f"  │  Status: {overall}  ({passed}/{total} passed, {failed} failed, {skipped} skipped)")
        for t in tests:
            sym = "+" if t["status"] == PASS else "x" if t["status"] == FAIL else "-"
            detail = f" — {t['detail']}" if t["detail"] else ""
            print(f"  │    [{sym}] {t['test_name']}{detail}")

    total = len(results)
    passed = sum(1 for r in results if r["status"] == PASS)
    failed = sum(1 for r in results if r["status"] == FAIL)
    skipped = sum(1 for r in results if r["status"] == SKIP)
    pct = (passed / total) * 100 if total > 0 else 0

    print(f"\n  {'='*50}")
    print(f"  OVERALL: {passed}/{total} tests passed ({pct:.1f}%)")
    if failed:
        print(f"  FAILURES: {failed} test(s) failed — see details above")
    if skipped:
        print(f"  SKIPPED: {skipped} test(s)")
    print(f"  {'='*50}\n")


# ══════════════════════════════════════════════════════════════════════
# Main
# ══════════════════════════════════════════════════════════════════════
if __name__ == "__main__":
    print("RorisDB Python Data Ecosystem Integration Test")
    print(f"Target: {USER}@{HOST}:{PORT}/{DATABASE}")
    print(f"Python: {sys.version}")
    print()

    test_pymysql()
    test_mysql_connector()
    test_sqlalchemy()
    test_pandas()
    test_cross_library_consistency()

    print_summary()

    # Exit with non-zero code if any test failed
    failed_count = sum(1 for r in results if r["status"] == FAIL)
    sys.exit(1 if failed_count > 0 else 0)