#!/usr/bin/env python3
"""
Comprehensive test suite for RorisDB ADB MySQL protocol.
1000+ test cases covering DDL, DML, functions, SHOW, variables, INFORMATION_SCHEMA,
transactions, JOINs, and edge cases.
"""

import subprocess
import json
import sys
import time
import uuid

HOST = "127.0.0.1"
PORT = 18124
USER = "root"
FAILURES_LIMIT = 20

def run_sql(sql, db=None):
    """Run a SQL statement via mysql CLI and return (success, output)."""
    cmd = ["mysql", "-h", HOST, "-P", str(PORT), f"-u{USER}", "--protocol=tcp", "-N"]
    if db:
        cmd += ["-D", db]
    cmd += ["-e", sql]
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=30)
        if result.returncode == 0:
            return True, result.stdout.strip()
        else:
            return False, result.stderr.strip()
    except subprocess.TimeoutExpired:
        return False, "TIMEOUT"
    except Exception as e:
        return False, str(e)

def uid():
    return uuid.uuid4().hex[:8]

def test(name, sql, db=None, expect_success=True, expect_contains=None, expect_not_contains=None):
    ok, output = run_sql(sql, db)
    if expect_success and not ok:
        return False, output
    if not expect_success and ok:
        return False, f"Expected failure but got success: {output[:200]}"
    if expect_contains and expect_contains.lower() not in output.lower():
        return False, f"Expected '{expect_contains}' in output, got: {output[:200]}"
    if expect_not_contains and expect_not_contains.lower() in output.lower():
        return False, f"Did not expect '{expect_not_contains}' in output, got: {output[:200]}"
    return True, ""

def run_tests():
    results = []
    test_db = f"test_{uid()}"

    def add(name, **kwargs):
        results.append((name, kwargs))

    # =========================================================================
    # 1. DDL TESTS (100+)
    # =========================================================================

    # -- Create databases --
    for i in range(10):
        add(f"CREATE_DATABASE_{i}", sql=f"CREATE DATABASE IF NOT EXISTS ddl_db_{i}")

    # Drop databases
    for i in range(10):
        add(f"DROP_DATABASE_{i}", sql=f"DROP DATABASE IF EXISTS ddl_db_{i}")

    # Create the main test database
    add("CREATE_TEST_DB", sql=f"CREATE DATABASE IF NOT EXISTS {test_db}")

    # -- Create tables with various column types --
    col_types = [
        ("INT", "42"),
        ("BIGINT", "9999999999"),
        ("SMALLINT", "100"),
        ("TINYINT", "7"),
        ("FLOAT", "3.14"),
        ("DOUBLE", "2.718281828"),
        ("DECIMAL(10,2)", "12345.67"),
        ("VARCHAR(255)", "'hello'"),
        ("VARCHAR(100)", "'world'"),
        ("CHAR(10)", "'abc'"),
        ("TEXT", "'some text'"),
        ("DATE", "'2024-01-15'"),
        ("DATETIME", "'2024-01-15 10:30:00'"),
        ("TIMESTAMP", "'2024-01-15 10:30:00'"),
        ("BOOLEAN", "1"),
        ("BLOB", "'binarydata'"),
    ]

    for i, (ctype, val) in enumerate(col_types):
        tbl = f"t_type_{i}"
        add(f"CREATE_TABLE_TYPE_{i}",
            sql=f"CREATE TABLE IF NOT EXISTS {tbl} (id INT, val {ctype})",
            db=test_db)
        add(f"INSERT_TYPE_{i}",
            sql=f"INSERT INTO {tbl} VALUES (1, {val})",
            db=test_db)
        add(f"SELECT_TYPE_{i}",
            sql=f"SELECT * FROM {tbl}",
            db=test_db)
        add(f"DROP_TABLE_TYPE_{i}",
            sql=f"DROP TABLE IF EXISTS {tbl}",
            db=test_db)

    # -- Create tables with constraints --
    add("CREATE_TABLE_PK", sql="CREATE TABLE IF NOT EXISTS t_pk (id INT PRIMARY KEY, name VARCHAR(50))", db=test_db)
    add("INSERT_PK", sql="INSERT INTO t_pk VALUES (1, 'alice')", db=test_db)
    add("SELECT_PK", sql="SELECT * FROM t_pk WHERE id = 1", db=test_db, expect_contains="alice")
    add("DROP_TABLE_PK", sql="DROP TABLE IF EXISTS t_pk", db=test_db)

    add("CREATE_TABLE_NOTNULL", sql="CREATE TABLE IF NOT EXISTS t_nn (id INT NOT NULL, val VARCHAR(50) NOT NULL)", db=test_db)
    add("INSERT_NOTNULL", sql="INSERT INTO t_nn VALUES (1, 'x')", db=test_db)
    add("DROP_TABLE_NOTNULL", sql="DROP TABLE IF EXISTS t_nn", db=test_db)

    add("CREATE_TABLE_DEFAULT", sql="CREATE TABLE IF NOT EXISTS t_def (id INT DEFAULT 0, name VARCHAR(50) DEFAULT 'unknown')", db=test_db)
    add("INSERT_DEFAULT", sql="INSERT INTO t_def (id) VALUES (1)", db=test_db)
    add("SELECT_DEFAULT", sql="SELECT name FROM t_def WHERE id = 1", db=test_db, expect_contains="unknown")
    add("DROP_TABLE_DEFAULT", sql="DROP TABLE IF EXISTS t_def", db=test_db)

    # -- Multi-column tables --
    add("CREATE_TABLE_MULTI", sql="""CREATE TABLE IF NOT EXISTS t_multi (
        id INT, name VARCHAR(100), age INT, salary DECIMAL(10,2),
        hire_date DATE, active BOOLEAN, bio TEXT
    )""", db=test_db)
    add("INSERT_MULTI", sql="INSERT INTO t_multi VALUES (1, 'Bob', 30, 50000.50, '2023-06-01', 1, 'A good worker')", db=test_db)
    add("INSERT_MULTI_2", sql="INSERT INTO t_multi VALUES (2, 'Carol', 25, 60000.00, '2022-03-15', 1, 'Excellent')", db=test_db)
    add("INSERT_MULTI_3", sql="INSERT INTO t_multi VALUES (3, 'Dave', 45, 75000.00, '2020-01-10', 0, 'Senior staff')", db=test_db)
    add("SELECT_MULTI", sql="SELECT * FROM t_multi", db=test_db)
    add("DROP_TABLE_MULTI", sql="DROP TABLE IF EXISTS t_multi", db=test_db)

    # -- ALTER TABLE tests --
    add("CREATE_TABLE_ALTER", sql="CREATE TABLE IF NOT EXISTS t_alter (id INT, name VARCHAR(50))", db=test_db)
    add("ALTER_ADD_COL", sql="ALTER TABLE t_alter ADD COLUMN age INT", db=test_db)
    add("ALTER_ADD_COL2", sql="ALTER TABLE t_alter ADD COLUMN email VARCHAR(100) DEFAULT 'none'", db=test_db)
    add("ALTER_DROP_COL", sql="ALTER TABLE t_alter DROP COLUMN email", db=test_db)
    add("DROP_TABLE_ALTER", sql="DROP TABLE IF EXISTS t_alter", db=test_db)

    # -- TRUNCATE --
    add("CREATE_TABLE_TRUNC", sql="CREATE TABLE IF NOT EXISTS t_trunc (id INT)", db=test_db)
    add("INSERT_TRUNC_1", sql="INSERT INTO t_trunc VALUES (1)", db=test_db)
    add("INSERT_TRUNC_2", sql="INSERT INTO t_trunc VALUES (2)", db=test_db)
    add("TRUNCATE_TABLE", sql="TRUNCATE TABLE t_trunc", db=test_db)
    add("SELECT_TRUNC_EMPTY", sql="SELECT COUNT(*) FROM t_trunc", db=test_db, expect_contains="0")
    add("DROP_TABLE_TRUNC", sql="DROP TABLE IF EXISTS t_trunc", db=test_db)

    # -- CREATE/DROP VIEW --
    add("CREATE_TABLE_FOR_VIEW", sql="CREATE TABLE IF NOT EXISTS t_v_src (id INT, val INT)", db=test_db)
    add("INSERT_VIEW_SRC", sql="INSERT INTO t_v_src VALUES (1, 10), (2, 20), (3, 30)", db=test_db)
    add("CREATE_VIEW", sql="CREATE OR REPLACE VIEW v_test AS SELECT id, val FROM t_v_src WHERE val > 10", db=test_db)
    add("SELECT_VIEW", sql="SELECT * FROM v_test", db=test_db)
    add("DROP_VIEW", sql="DROP VIEW IF EXISTS v_test", db=test_db)
    add("DROP_TABLE_VIEW_SRC", sql="DROP TABLE IF EXISTS t_v_src", db=test_db)

    # -- CREATE/DROP INDEX --
    add("CREATE_TABLE_IDX", sql="CREATE TABLE IF NOT EXISTS t_idx (id INT, name VARCHAR(50), age INT)", db=test_db)
    add("CREATE_INDEX", sql="CREATE INDEX idx_name ON t_idx (name)", db=test_db)
    add("CREATE_INDEX_AGE", sql="CREATE INDEX idx_age ON t_idx (age)", db=test_db)
    add("DROP_INDEX", sql="DROP INDEX idx_name ON t_idx", db=test_db)
    add("DROP_TABLE_IDX", sql="DROP TABLE IF EXISTS t_idx", db=test_db)

    # -- CREATE TABLE IF NOT EXISTS (idempotent) --
    add("CREATE_IF_NOT_EXISTS_1", sql="CREATE TABLE IF NOT EXISTS t_ine (id INT)", db=test_db)
    add("CREATE_IF_NOT_EXISTS_2", sql="CREATE TABLE IF NOT EXISTS t_ine (id INT)", db=test_db)
    add("DROP_TABLE_INE", sql="DROP TABLE IF EXISTS t_ine", db=test_db)

    # -- DROP TABLE IF EXISTS (idempotent) --
    add("DROP_IF_EXISTS_1", sql="DROP TABLE IF EXISTS t_nonexistent_xyz", db=test_db)
    add("DROP_IF_EXISTS_2", sql="DROP TABLE IF EXISTS t_nonexistent_xyz", db=test_db)

    # -- CREATE TABLE with multiple primary keys (compound) --
    add("CREATE_TABLE_COMPOUND_PK", sql="CREATE TABLE IF NOT EXISTS t_cpk (id1 INT, id2 INT, val VARCHAR(50), PRIMARY KEY (id1, id2))", db=test_db)
    add("INSERT_COMPOUND_PK", sql="INSERT INTO t_cpk VALUES (1, 1, 'a')", db=test_db)
    add("DROP_TABLE_COMPOUND_PK", sql="DROP TABLE IF EXISTS t_cpk", db=test_db)

    # -- CREATE TABLE with various numeric precisions --
    add("CREATE_TABLE_DEC1", sql="CREATE TABLE IF NOT EXISTS t_dec1 (v DECIMAL(5,0))", db=test_db)
    add("CREATE_TABLE_DEC2", sql="CREATE TABLE IF NOT EXISTS t_dec2 (v DECIMAL(20,10))", db=test_db)
    add("CREATE_TABLE_DEC3", sql="CREATE TABLE IF NOT EXISTS t_dec3 (v FLOAT)", db=test_db)
    add("CREATE_TABLE_DEC4", sql="CREATE TABLE IF NOT EXISTS t_dec4 (v DOUBLE)", db=test_db)
    for t in ["t_dec1","t_dec2","t_dec3","t_dec4"]:
        add(f"DROP_{t}", sql=f"DROP TABLE IF EXISTS {t}", db=test_db)

    # -- CREATE TABLE with UNSIGNED --
    add("CREATE_TABLE_UNS", sql="CREATE TABLE IF NOT EXISTS t_uns (v INT UNSIGNED)", db=test_db)
    add("INSERT_UNS", sql="INSERT INTO t_uns VALUES (4294967295)", db=test_db)
    add("DROP_TABLE_UNS", sql="DROP TABLE IF EXISTS t_uns", db=test_db)

    # -- CREATE TABLE with AUTO_INCREMENT --
    add("CREATE_TABLE_AI", sql="CREATE TABLE IF NOT EXISTS t_ai (id INT AUTO_INCREMENT PRIMARY KEY, name VARCHAR(50))", db=test_db)
    add("INSERT_AI_1", sql="INSERT INTO t_ai (name) VALUES ('a')", db=test_db)
    add("INSERT_AI_2", sql="INSERT INTO t_ai (name) VALUES ('b')", db=test_db)
    add("SELECT_AI", sql="SELECT id FROM t_ai", db=test_db)
    add("DROP_TABLE_AI", sql="DROP TABLE IF EXISTS t_ai", db=test_db)

    # =========================================================================
    # 2. DML TESTS (100+)
    # =========================================================================

    # Setup a DML test table
    add("CREATE_DML_TABLE", sql="CREATE TABLE IF NOT EXISTS t_dml (id INT, name VARCHAR(100), age INT, salary DECIMAL(10,2), city VARCHAR(50))", db=test_db)
    add("INSERT_DML_1", sql="INSERT INTO t_dml VALUES (1, 'Alice', 30, 50000, 'NYC')", db=test_db)
    add("INSERT_DML_2", sql="INSERT INTO t_dml VALUES (2, 'Bob', 25, 60000, 'LA')", db=test_db)
    add("INSERT_DML_3", sql="INSERT INTO t_dml VALUES (3, 'Carol', 35, 70000, 'NYC')", db=test_db)
    add("INSERT_DML_4", sql="INSERT INTO t_dml VALUES (4, 'Dave', 28, 55000, 'Chicago')", db=test_db)
    add("INSERT_DML_5", sql="INSERT INTO t_dml VALUES (5, 'Eve', 32, 65000, 'LA')", db=test_db)
    add("INSERT_DML_6", sql="INSERT INTO t_dml VALUES (6, 'Frank', 40, 80000, 'NYC')", db=test_db)
    add("INSERT_DML_7", sql="INSERT INTO t_dml VALUES (7, 'Grace', 27, 52000, 'Chicago')", db=test_db)
    add("INSERT_DML_8", sql="INSERT INTO t_dml VALUES (8, 'Hank', 45, 90000, 'LA')", db=test_db)
    add("INSERT_DML_9", sql="INSERT INTO t_dml VALUES (9, 'Ivy', 22, 45000, 'NYC')", db=test_db)
    add("INSERT_DML_10", sql="INSERT INTO t_dml VALUES (10, 'Jack', 38, 72000, 'Chicago')", db=test_db)

    # SELECT with WHERE
    add("SELECT_WHERE_EQ", sql="SELECT name FROM t_dml WHERE id = 1", db=test_db, expect_contains="Alice")
    add("SELECT_WHERE_GT", sql="SELECT name FROM t_dml WHERE age > 35", db=test_db)
    add("SELECT_WHERE_LT", sql="SELECT name FROM t_dml WHERE age < 25", db=test_db, expect_contains="Ivy")
    add("SELECT_WHERE_GTE", sql="SELECT COUNT(*) FROM t_dml WHERE age >= 30", db=test_db)
    add("SELECT_WHERE_LTE", sql="SELECT COUNT(*) FROM t_dml WHERE age <= 30", db=test_db)
    add("SELECT_WHERE_NEQ", sql="SELECT COUNT(*) FROM t_dml WHERE city != 'NYC'", db=test_db)
    add("SELECT_WHERE_AND", sql="SELECT name FROM t_dml WHERE age > 25 AND city = 'NYC'", db=test_db)
    add("SELECT_WHERE_OR", sql="SELECT COUNT(*) FROM t_dml WHERE city = 'NYC' OR city = 'LA'", db=test_db)
    add("SELECT_WHERE_IN", sql="SELECT COUNT(*) FROM t_dml WHERE city IN ('NYC', 'LA')", db=test_db)
    add("SELECT_WHERE_NOT_IN", sql="SELECT COUNT(*) FROM t_dml WHERE city NOT IN ('NYC', 'LA')", db=test_db)
    add("SELECT_WHERE_BETWEEN", sql="SELECT COUNT(*) FROM t_dml WHERE age BETWEEN 25 AND 35", db=test_db)
    add("SELECT_WHERE_NOT_BETWEEN", sql="SELECT COUNT(*) FROM t_dml WHERE age NOT BETWEEN 25 AND 35", db=test_db)
    add("SELECT_WHERE_LIKE", sql="SELECT name FROM t_dml WHERE name LIKE 'A%'", db=test_db, expect_contains="Alice")
    add("SELECT_WHERE_NOT_LIKE", sql="SELECT COUNT(*) FROM t_dml WHERE name NOT LIKE 'A%'", db=test_db)
    add("SELECT_WHERE_LIKE_UNDERSCORE", sql="SELECT name FROM t_dml WHERE name LIKE 'Bo_'", db=test_db, expect_contains="Bob")
    add("SELECT_WHERE_IS_NULL", sql="SELECT COUNT(*) FROM t_dml WHERE name IS NULL", db=test_db, expect_contains="0")
    add("SELECT_WHERE_IS_NOT_NULL", sql="SELECT COUNT(*) FROM t_dml WHERE name IS NOT NULL", db=test_db, expect_contains="10")

    # ORDER BY
    add("SELECT_ORDER_ASC", sql="SELECT name FROM t_dml ORDER BY age ASC LIMIT 1", db=test_db, expect_contains="Ivy")
    add("SELECT_ORDER_DESC", sql="SELECT name FROM t_dml ORDER BY age DESC LIMIT 1", db=test_db, expect_contains="Hank")
    add("SELECT_ORDER_MULTI", sql="SELECT name FROM t_dml ORDER BY city ASC, age DESC LIMIT 3", db=test_db)
    add("SELECT_ORDER_BY_NUM", sql="SELECT name FROM t_dml ORDER BY 2 ASC LIMIT 1", db=test_db)

    # LIMIT
    add("SELECT_LIMIT_1", sql="SELECT COUNT(*) FROM (SELECT * FROM t_dml LIMIT 1) t", db=test_db, expect_contains="1")
    add("SELECT_LIMIT_5", sql="SELECT COUNT(*) FROM (SELECT * FROM t_dml LIMIT 5) t", db=test_db, expect_contains="5")
    add("SELECT_LIMIT_OFFSET", sql="SELECT name FROM t_dml ORDER BY id LIMIT 2 OFFSET 3", db=test_db)
    add("SELECT_LIMIT_LARGE", sql="SELECT COUNT(*) FROM (SELECT * FROM t_dml LIMIT 1000) t", db=test_db, expect_contains="10")

    # GROUP BY
    add("SELECT_GROUP_COUNT", sql="SELECT city, COUNT(*) FROM t_dml GROUP BY city", db=test_db)
    add("SELECT_GROUP_SUM", sql="SELECT city, SUM(salary) FROM t_dml GROUP BY city", db=test_db)
    add("SELECT_GROUP_AVG", sql="SELECT city, AVG(age) FROM t_dml GROUP BY city", db=test_db)
    add("SELECT_GROUP_MIN", sql="SELECT city, MIN(age) FROM t_dml GROUP BY city", db=test_db)
    add("SELECT_GROUP_MAX", sql="SELECT city, MAX(salary) FROM t_dml GROUP BY city", db=test_db)
    add("SELECT_GROUP_MULTI", sql="SELECT city, COUNT(*), AVG(salary) FROM t_dml GROUP BY city", db=test_db)

    # HAVING
    add("SELECT_HAVING", sql="SELECT city, COUNT(*) AS cnt FROM t_dml GROUP BY city HAVING cnt > 2", db=test_db)
    add("SELECT_HAVING_SUM", sql="SELECT city, SUM(salary) AS total FROM t_dml GROUP BY city HAVING total > 100000", db=test_db)
    add("SELECT_HAVING_AVG", sql="SELECT city, AVG(age) AS a FROM t_dml GROUP BY city HAVING a > 30", db=test_db)

    # DISTINCT
    add("SELECT_DISTINCT_CITY", sql="SELECT DISTINCT city FROM t_dml", db=test_db)
    add("SELECT_DISTINCT_COUNT", sql="SELECT COUNT(DISTINCT city) FROM t_dml", db=test_db, expect_contains="3")

    # Aggregates
    add("SELECT_COUNT", sql="SELECT COUNT(*) FROM t_dml", db=test_db, expect_contains="10")
    add("SELECT_SUM", sql="SELECT SUM(salary) FROM t_dml", db=test_db)
    add("SELECT_AVG", sql="SELECT AVG(salary) FROM t_dml", db=test_db)
    add("SELECT_MIN", sql="SELECT MIN(salary) FROM t_dml", db=test_db)
    add("SELECT_MAX", sql="SELECT MAX(salary) FROM t_dml", db=test_db)
    add("SELECT_COUNT_DISTINCT", sql="SELECT COUNT(DISTINCT city) FROM t_dml", db=test_db)

    # Subqueries
    add("SUBQUERY_WHERE", sql="SELECT name FROM t_dml WHERE salary > (SELECT AVG(salary) FROM t_dml)", db=test_db)
    add("SUBQUERY_IN", sql="SELECT name FROM t_dml WHERE city IN (SELECT city FROM t_dml WHERE age > 35)", db=test_db)

    # UPDATE
    add("UPDATE_BASIC", sql="UPDATE t_dml SET salary = 51000 WHERE id = 1", db=test_db)
    add("UPDATE_VERIFY", sql="SELECT salary FROM t_dml WHERE id = 1", db=test_db, expect_contains="51000")
    add("UPDATE_MULTI_COL", sql="UPDATE t_dml SET age = 31, city = 'SF' WHERE id = 1", db=test_db)
    add("UPDATE_VERIFY2", sql="SELECT age, city FROM t_dml WHERE id = 1", db=test_db)
    add("UPDATE_ALL", sql="UPDATE t_dml SET salary = salary + 1000 WHERE city = 'LA'", db=test_db)
    add("UPDATE_EXPRESSION", sql="UPDATE t_dml SET age = age + 1 WHERE id = 2", db=test_db)
    add("UPDATE_STRING", sql="UPDATE t_dml SET name = 'Robert' WHERE name = 'Bob'", db=test_db)

    # DELETE
    add("DELETE_BASIC", sql="DELETE FROM t_dml WHERE id = 10", db=test_db)
    add("DELETE_VERIFY", sql="SELECT COUNT(*) FROM t_dml", db=test_db, expect_contains="9")
    add("DELETE_MULTI", sql="DELETE FROM t_dml WHERE city = 'SF'", db=test_db)
    add("DELETE_RANGE", sql="DELETE FROM t_dml WHERE age < 25", db=test_db)

    # INSERT multi-row
    add("INSERT_MULTI_ROW", sql="INSERT INTO t_dml VALUES (11, 'Kate', 29, 58000, 'Boston'), (12, 'Leo', 33, 67000, 'Denver')", db=test_db)
    add("INSERT_MULTI_VERIFY", sql="SELECT COUNT(*) FROM t_dml", db=test_db)

    # SELECT with expressions
    add("SELECT_EXPR_ADD", sql="SELECT salary + 1000 FROM t_dml WHERE id = 1", db=test_db)
    add("SELECT_EXPR_MUL", sql="SELECT salary * 2 FROM t_dml WHERE id = 2", db=test_db)
    add("SELECT_ALIAS", sql="SELECT name AS n, salary AS s FROM t_dml WHERE id = 2", db=test_db)
    add("SELECT_STAR_COUNT", sql="SELECT COUNT(*) AS total FROM t_dml", db=test_db)

    # CASE WHEN
    add("CASE_WHEN", sql="SELECT CASE WHEN age > 30 THEN 'senior' ELSE 'junior' END FROM t_dml WHERE id = 2", db=test_db)
    add("CASE_WHEN_NULL", sql="SELECT CASE WHEN name IS NULL THEN 'unknown' ELSE name END FROM t_dml LIMIT 1", db=test_db)

    # UNION
    add("UNION_ALL", sql="SELECT name FROM t_dml WHERE city = 'LA' UNION ALL SELECT name FROM t_dml WHERE city = 'Chicago'", db=test_db)
    add("UNION_DISTINCT", sql="SELECT city FROM t_dml WHERE id <= 5 UNION SELECT city FROM t_dml WHERE id > 5", db=test_db)

    # COALESCE / IFNULL
    add("COALESCE", sql="SELECT COALESCE(name, 'unknown') FROM t_dml LIMIT 1", db=test_db)

    # CAST
    add("CAST_INT_TO_STR", sql="SELECT CAST(id AS VARCHAR) FROM t_dml WHERE id = 2", db=test_db, expect_contains="2")
    add("CAST_STR_TO_INT", sql="SELECT CAST('123' AS INT)", db=test_db, expect_contains="123")

    # EXISTS
    add("EXISTS_SUBQUERY", sql="SELECT name FROM t_dml WHERE EXISTS (SELECT 1 FROM t_dml WHERE city = 'LA')", db=test_db)

    # DROP DML table
    add("DROP_DML_TABLE", sql="DROP TABLE IF EXISTS t_dml", db=test_db)

    # =========================================================================
    # 3. STRING FUNCTIONS (80+)
    # =========================================================================

    add("STR_CONCAT", sql="SELECT CONCAT('hello', ' ', 'world')")
    add("STR_CONCAT_WS", sql="SELECT CONCAT_WS('-', 'a', 'b', 'c')", db=test_db)
    add("STR_LENGTH", sql="SELECT LENGTH('hello')", db=test_db, expect_contains="5")
    add("STR_CHAR_LENGTH", sql="SELECT CHAR_LENGTH('hello')", db=test_db, expect_contains="5")
    add("STR_UPPER", sql="SELECT UPPER('hello')", db=test_db, expect_contains="HELLO")
    add("STR_LOWER", sql="SELECT LOWER('HELLO')", db=test_db, expect_contains="hello")
    add("STR_UCASE", sql="SELECT UCASE('hello')", db=test_db, expect_contains="HELLO")
    add("STR_LCASE", sql="SELECT LCASE('HELLO')", db=test_db, expect_contains="hello")
    add("STR_SUBSTRING_1", sql="SELECT SUBSTRING('hello world', 1, 5)", db=test_db, expect_contains="hello")
    add("STR_SUBSTRING_2", sql="SELECT SUBSTRING('hello', 2, 3)", db=test_db, expect_contains="ell")
    add("STR_SUBSTR", sql="SELECT SUBSTR('hello', 1, 3)", db=test_db, expect_contains="hel")
    add("STR_LEFT", sql="SELECT LEFT('hello', 3)", db=test_db, expect_contains="hel")
    add("STR_RIGHT", sql="SELECT RIGHT('hello', 3)", db=test_db, expect_contains="llo")
    add("STR_LTRIM", sql="SELECT LTRIM('  hello  ')", db=test_db, expect_contains="hello")
    add("STR_RTRIM", sql="SELECT RTRIM('  hello  ')", db=test_db, expect_contains="hello")
    add("STR_TRIM", sql="SELECT TRIM('  hello  ')", db=test_db, expect_contains="hello")
    add("STR_TRIM_CHAR", sql="SELECT TRIM('x' FROM 'xxxhelloxxx')", db=test_db, expect_contains="hello")
    add("STR_REPLACE", sql="SELECT REPLACE('hello world', 'world', 'there')", db=test_db, expect_contains="hello there")
    add("STR_REVERSE", sql="SELECT REVERSE('hello')", db=test_db, expect_contains="olleh")
    add("STR_REPEAT", sql="SELECT REPEAT('ab', 3)", db=test_db, expect_contains="ababab")
    add("STR_SPACE", sql="SELECT CONCAT('a', SPACE(3), 'b')", db=test_db, expect_contains="a   b")
    add("STR_LPAD", sql="SELECT LPAD('hi', 5, 'x')", db=test_db, expect_contains="xxxhi")
    add("STR_RPAD", sql="SELECT RPAD('hi', 5, 'x')", db=test_db, expect_contains="hixxx")
    add("STR_LOCATE_1", sql="SELECT LOCATE('world', 'hello world')", db=test_db, expect_contains="7")
    add("STR_LOCATE_2", sql="SELECT LOCATE('xyz', 'hello world')", db=test_db, expect_contains="0")
    add("STR_INSTR", sql="SELECT INSTR('hello world', 'world')", db=test_db, expect_contains="7")
    add("STR_POSITION", sql="SELECT POSITION('world' IN 'hello world')", db=test_db, expect_contains="7")
    add("STR_ASCII", sql="SELECT ASCII('A')", db=test_db, expect_contains="65")
    add("STR_CHAR", sql="SELECT CHAR(65)", db=test_db, expect_contains="A")
    add("STR_FIELD", sql="SELECT FIELD('b', 'a', 'b', 'c')", db=test_db, expect_contains="2")
    add("STR_FIND_IN_SET", sql="SELECT FIND_IN_SET('b', 'a,b,c')", db=test_db, expect_contains="2")
    add("STR_FORMAT", sql="SELECT FORMAT(1234567.891, 2)", db=test_db)
    add("STR_HEX", sql="SELECT HEX('A')", db=test_db, expect_contains="41")
    add("STR_UNHEX", sql="SELECT UNHEX('41')", db=test_db, expect_contains="A")
    add("STR_OCT", sql="SELECT OCT(8)", db=test_db, expect_contains="10")
    add("STR_BIN", sql="SELECT BIN(10)", db=test_db, expect_contains="1010")
    add("STR_ORD", sql="SELECT ORD('A')", db=test_db, expect_contains="65")
    add("STR_BIT_LENGTH", sql="SELECT BIT_LENGTH('a')", db=test_db, expect_contains="8")
    add("STR_QUOTE", sql="SELECT QUOTE('hello world')", db=test_db)
    add("STR_INSERT_FUNC", sql="SELECT INSERT('hello', 1, 0, 'X')", db=test_db)
    add("STR_ELT", sql="SELECT ELT(2, 'a', 'b', 'c')", db=test_db, expect_contains="b")
    add("STR_MAKE_SET", sql="SELECT MAKE_SET(3, 'a', 'b')", db=test_db)
    add("STR_EXPORT_SET", sql="SELECT EXPORT_SET(5, 'Y', 'N', ',', 5)", db=test_db)
    add("STR_CMP", sql="SELECT CMP('a', 'b')", db=test_db)
    add("STR_STRCMP", sql="SELECT STRCMP('a', 'b')", db=test_db)
    add("STR_MID", sql="SELECT MID('hello', 2, 3)", db=test_db, expect_contains="ell")
    add("STR_RLIKE", sql="SELECT 'hello' RLIKE '^h'", db=test_db, expect_contains="1")
    add("STR_REGEXP", sql="SELECT 'hello' REGEXP 'llo$'", db=test_db, expect_contains="1")
    add("STR_NOT_REGEXP", sql="SELECT 'hello' NOT REGEXP '^x'", db=test_db, expect_contains="1")

    # String functions with table data
    add("CREATE_STR_TABLE", sql="CREATE TABLE IF NOT EXISTS t_str (id INT, name VARCHAR(100), bio TEXT)", db=test_db)
    add("INSERT_STR_1", sql="INSERT INTO t_str VALUES (1, 'Alice', 'Engineer from NYC')", db=test_db)
    add("INSERT_STR_2", sql="INSERT INTO t_str VALUES (2, 'Bob Smith', 'Designer from LA')", db=test_db)
    add("STR_UPPER_TABLE", sql="SELECT UPPER(name) FROM t_str WHERE id = 1", db=test_db, expect_contains="ALICE")
    add("STR_LOWER_TABLE", sql="SELECT LOWER(name) FROM t_str WHERE id = 1", db=test_db, expect_contains="alice")
    add("STR_LENGTH_TABLE", sql="SELECT LENGTH(name) FROM t_str WHERE id = 1", db=test_db, expect_contains="5")
    add("STR_CONCAT_TABLE", sql="SELECT CONCAT(name, ' - ', bio) FROM t_str WHERE id = 1", db=test_db)
    add("STR_SUBSTRING_TABLE", sql="SELECT SUBSTRING(name, 1, 3) FROM t_str WHERE id = 1", db=test_db, expect_contains="Ali")
    add("STR_REPLACE_TABLE", sql="SELECT REPLACE(bio, 'NYC', 'Boston') FROM t_str WHERE id = 1", db=test_db)
    add("STR_TRIM_TABLE", sql="SELECT TRIM(name) FROM t_str WHERE id = 1", db=test_db, expect_contains="Alice")
    add("STR_LIKE_TABLE", sql="SELECT name FROM t_str WHERE name LIKE 'Al%'", db=test_db, expect_contains="Alice")
    add("STR_REVERSE_TABLE", sql="SELECT REVERSE(name) FROM t_str WHERE id = 1", db=test_db, expect_contains="ecilA")
    add("STR_LOCATE_TABLE", sql="SELECT LOCATE('from', bio) FROM t_str WHERE id = 1", db=test_db)
    add("STR_REPEAT_TABLE", sql="SELECT REPEAT('x', 3) FROM t_str WHERE id = 1", db=test_db, expect_contains="xxx")
    add("DROP_STR_TABLE", sql="DROP TABLE IF EXISTS t_str", db=test_db)

    # Additional string edge cases
    add("STR_EMPTY", sql="SELECT LENGTH('')", db=test_db, expect_contains="0")
    add("STR_CONCAT_NULL", sql="SELECT CONCAT('a', NULL, 'b')", db=test_db)
    add("STR_NULL_LENGTH", sql="SELECT LENGTH(NULL)", db=test_db)
    add("STR_UPPER_NULL", sql="SELECT UPPER(NULL)", db=test_db)
    add("STR_LOWER_NULL", sql="SELECT LOWER(NULL)", db=test_db)
    add("STR_SUBSTRING_EMPTY", sql="SELECT SUBSTRING('', 1, 1)", db=test_db)
    add("STR_REPLACE_NO_MATCH", sql="SELECT REPLACE('hello', 'xyz', 'abc')", db=test_db, expect_contains="hello")
    add("STR_REPEAT_ZERO", sql="SELECT REPEAT('x', 0)", db=test_db)
    add("STR_REVERSE_EMPTY", sql="SELECT REVERSE('')", db=test_db)
    add("STR_ASCII_ZERO", sql="SELECT ASCII('')", db=test_db)

    # =========================================================================
    # 4. NUMERIC FUNCTIONS (60+)
    # =========================================================================

    add("NUM_ABS_POS", sql="SELECT ABS(5)", db=test_db, expect_contains="5")
    add("NUM_ABS_NEG", sql="SELECT ABS(-5)", db=test_db, expect_contains="5")
    add("NUM_CEIL", sql="SELECT CEIL(4.1)", db=test_db, expect_contains="5")
    add("NUM_CEILING", sql="SELECT CEILING(4.1)", db=test_db, expect_contains="5")
    add("NUM_FLOOR", sql="SELECT FLOOR(4.9)", db=test_db, expect_contains="4")
    add("NUM_ROUND_0", sql="SELECT ROUND(4.567, 0)", db=test_db, expect_contains="5")
    add("NUM_ROUND_2", sql="SELECT ROUND(4.567, 2)", db=test_db, expect_contains="4.57")
    add("NUM_ROUND_NEG", sql="SELECT ROUND(4567, -2)", db=test_db)
    add("NUM_TRUNCATE", sql="SELECT TRUNCATE(4.567, 2)", db=test_db, expect_contains="4.56")
    add("NUM_MOD", sql="SELECT MOD(10, 3)", db=test_db, expect_contains="1")
    add("NUM_MOD_NEG", sql="SELECT MOD(-10, 3)", db=test_db)
    add("NUM_POWER", sql="SELECT POWER(2, 10)", db=test_db, expect_contains="1024")
    add("NUM_POW", sql="SELECT POW(2, 10)", db=test_db, expect_contains="1024")
    add("NUM_SQRT", sql="SELECT SQRT(144)", db=test_db, expect_contains="12")
    add("NUM_SQRT2", sql="SELECT SQRT(2)")
    add("NUM_CBRT", sql="SELECT CBRT(27)", db=test_db, expect_contains="3")
    add("NUM_EXP", sql="SELECT EXP(1)")
    add("NUM_LN", sql="SELECT LN(2.718281828)")
    add("NUM_LOG", sql="SELECT LOG(2, 8)", db=test_db, expect_contains="3")
    add("NUM_LOG2", sql="SELECT LOG2(8)", db=test_db, expect_contains="3")
    add("NUM_LOG10", sql="SELECT LOG10(1000)", db=test_db, expect_contains="3")
    add("NUM_LOG10_2", sql="SELECT LOG10(100)", db=test_db, expect_contains="2")
    add("NUM_SIGN_POS", sql="SELECT SIGN(42)", db=test_db, expect_contains="1")
    add("NUM_SIGN_NEG", sql="SELECT SIGN(-42)", db=test_db, expect_contains="-1")
    add("NUM_SIGN_ZERO", sql="SELECT SIGN(0)", db=test_db, expect_contains="0")
    add("NUM_PI", sql="SELECT PI()")
    add("NUM_SIN", sql="SELECT SIN(0)", db=test_db, expect_contains="0")
    add("NUM_COS", sql="SELECT COS(0)", db=test_db, expect_contains="1")
    add("NUM_TAN", sql="SELECT TAN(0)", db=test_db, expect_contains="0")
    add("NUM_ASIN", sql="SELECT ASIN(0)", db=test_db, expect_contains="0")
    add("NUM_ACOS", sql="SELECT ACOS(1)", db=test_db, expect_contains="0")
    add("NUM_ATAN", sql="SELECT ATAN(0)", db=test_db, expect_contains="0")
    add("NUM_ATAN2", sql="SELECT ATAN2(0, 1)", db=test_db, expect_contains="0")
    add("NUM_COT", sql="SELECT COT(1)")
    add("NUM_DEGREES", sql="SELECT DEGREES(3.14159265)")
    add("NUM_RADIANS", sql="SELECT RADIANS(180)")
    add("NUM_CRC32", sql="SELECT CRC32('hello')")
    add("NUM_CONV", sql="SELECT CONV('a', 16, 10)", db=test_db, expect_contains="10")
    add("NUM_CONV_BIN", sql="SELECT CONV(10, 10, 2)", db=test_db, expect_contains="1010")
    add("NUM_CONV_OCT", sql="SELECT CONV(8, 10, 8)", db=test_db, expect_contains="10")
    add("NUM_RAND", sql="SELECT RAND() >= 0", db=test_db, expect_contains="1")
    add("NUM_RAND_SEED", sql="SELECT RAND(42) >= 0", db=test_db, expect_contains="1")
    add("NUM_UUID", sql="SELECT LENGTH(UUID()) > 0", db=test_db, expect_contains="1")
    add("NUM_UUID_SHORT", sql="SELECT UUID_SHORT() > 0", db=test_db, expect_contains="1")
    add("NUM_BIN_FUNC", sql="SELECT BIN(42)", db=test_db, expect_contains="101010")
    add("NUM_OCT_FUNC", sql="SELECT OCT(8)", db=test_db, expect_contains="10")
    add("NUM_HEX_FUNC", sql="SELECT HEX(255)", db=test_db, expect_contains="FF")
    add("NUM_BIT_COUNT", sql="SELECT BIT_COUNT(7)", db=test_db, expect_contains="3")

    # Numeric with table
    add("CREATE_NUM_TABLE", sql="CREATE TABLE IF NOT EXISTS t_num (id INT, val DOUBLE)", db=test_db)
    add("INSERT_NUM_1", sql="INSERT INTO t_num VALUES (1, 3.14159)", db=test_db)
    add("INSERT_NUM_2", sql="INSERT INTO t_num VALUES (2, -2.5)", db=test_db)
    add("INSERT_NUM_3", sql="INSERT INTO t_num VALUES (3, 100)", db=test_db)
    add("NUM_ABS_TABLE", sql="SELECT ABS(val) FROM t_num WHERE id = 2", db=test_db, expect_contains="2.5")
    add("NUM_CEIL_TABLE", sql="SELECT CEIL(val) FROM t_num WHERE id = 2", db=test_db, expect_contains="-2")
    add("NUM_FLOOR_TABLE", sql="SELECT FLOOR(val) FROM t_num WHERE id = 1", db=test_db, expect_contains="3")
    add("NUM_ROUND_TABLE", sql="SELECT ROUND(val, 2) FROM t_num WHERE id = 1", db=test_db, expect_contains="3.14")
    add("NUM_SQRT_TABLE", sql="SELECT SQRT(val) FROM t_num WHERE id = 3", db=test_db, expect_contains="10")
    add("NUM_MOD_TABLE", sql="SELECT MOD(val, 3) FROM t_num WHERE id = 3", db=test_db, expect_contains="1")
    add("NUM_SUM_TABLE", sql="SELECT SUM(val) FROM t_num", db=test_db)
    add("NUM_AVG_TABLE", sql="SELECT AVG(val) FROM t_num", db=test_db)
    add("DROP_NUM_TABLE", sql="DROP TABLE IF EXISTS t_num", db=test_db)

    # Numeric edge cases
    add("NUM_DIV_ZERO", sql="SELECT 1/0", db=test_db)
    add("NUM_MOD_ZERO", sql="SELECT MOD(1, 0)", db=test_db)
    add("NUM_SQRT_NEG", sql="SELECT SQRT(-1)", db=test_db)
    add("NUM_LOG_ZERO", sql="SELECT LN(0)", db=test_db)
    add("NUM_LOG_NEG", sql="SELECT LN(-1)", db=test_db)
    add("NUM_POWER_LARGE", sql="SELECT POWER(2, 20)", db=test_db, expect_contains="1048576")
    add("NUM_ZERO_DIV", sql="SELECT 0/1", db=test_db, expect_contains="0")
    add("NUM_NEG_MOD", sql="SELECT -7 MOD 3", db=test_db)

    # =========================================================================
    # 5. DATE FUNCTIONS (60+)
    # =========================================================================

    add("DATE_NOW", sql="SELECT LENGTH(NOW()) > 0", db=test_db, expect_contains="1")
    add("DATE_CURDATE", sql="SELECT LENGTH(CURDATE()) > 0", db=test_db, expect_contains="1")
    add("DATE_CURDATE_2", sql="SELECT LENGTH(CURRENT_DATE()) > 0", db=test_db, expect_contains="1")
    add("DATE_CURTIME", sql="SELECT LENGTH(CURTIME()) > 0", db=test_db, expect_contains="1")
    add("DATE_CURTIME_2", sql="SELECT LENGTH(CURRENT_TIME()) > 0", db=test_db, expect_contains="1")
    add("DATE_CURRENT_TIMESTAMP", sql="SELECT LENGTH(CURRENT_TIMESTAMP()) > 0", db=test_db, expect_contains="1")
    add("DATE_LOCALTIME", sql="SELECT LENGTH(LOCALTIME()) > 0", db=test_db, expect_contains="1")
    add("DATE_LOCALTIMESTAMP", sql="SELECT LENGTH(LOCALTIMESTAMP()) > 0", db=test_db, expect_contains="1")
    add("DATE_SYSDATE", sql="SELECT LENGTH(SYSDATE()) > 0", db=test_db, expect_contains="1")
    add("DATE_UTC_DATE", sql="SELECT LENGTH(UTC_DATE()) > 0", db=test_db, expect_contains="1")
    add("DATE_UTC_TIME", sql="SELECT LENGTH(UTC_TIME()) > 0", db=test_db, expect_contains="1")
    add("DATE_UTC_TIMESTAMP", sql="SELECT LENGTH(UTC_TIMESTAMP()) > 0", db=test_db, expect_contains="1")
    add("DATE_UNIX_TIMESTAMP", sql="SELECT UNIX_TIMESTAMP() > 0", db=test_db, expect_contains="1")
    add("DATE_UNIX_TS_DATE", sql="SELECT UNIX_TIMESTAMP('2024-01-01')", db=test_db)
    add("DATE_FROM_UNIX", sql="SELECT LENGTH(FROM_UNIXTIME(1700000000)) > 0", db=test_db, expect_contains="1")
    add("DATE_YEAR", sql="SELECT YEAR('2024-06-15')", db=test_db, expect_contains="2024")
    add("DATE_MONTH", sql="SELECT MONTH('2024-06-15')", db=test_db, expect_contains="6")
    add("DATE_DAY", sql="SELECT DAY('2024-06-15')", db=test_db, expect_contains="15")
    add("DATE_HOUR", sql="SELECT HOUR('10:30:00')", db=test_db, expect_contains="10")
    add("DATE_MINUTE", sql="SELECT MINUTE('10:30:00')", db=test_db, expect_contains="30")
    add("DATE_SECOND", sql="SELECT SECOND('10:30:45')", db=test_db, expect_contains="45")
    add("DATE_DAYOFWEEK", sql="SELECT DAYOFWEEK('2024-06-15')", db=test_db)
    add("DATE_DAYOFMONTH", sql="SELECT DAYOFMONTH('2024-06-15')", db=test_db, expect_contains="15")
    add("DATE_DAYOFYEAR", sql="SELECT DAYOFYEAR('2024-06-15')", db=test_db)
    add("DATE_WEEK", sql="SELECT WEEK('2024-06-15')", db=test_db)
    add("DATE_WEEKOFYEAR", sql="SELECT WEEKOFYEAR('2024-06-15')", db=test_db)
    add("DATE_QUARTER", sql="SELECT QUARTER('2024-06-15')", db=test_db, expect_contains="2")
    add("DATE_MONTHNAME", sql="SELECT MONTHNAME('2024-06-15')", db=test_db)
    add("DATE_DAYNAME", sql="SELECT DAYNAME('2024-06-15')", db=test_db)
    add("DATE_EXTRACT_YEAR", sql="SELECT EXTRACT(YEAR FROM '2024-06-15')", db=test_db, expect_contains="2024")
    add("DATE_EXTRACT_MONTH", sql="SELECT EXTRACT(MONTH FROM '2024-06-15')", db=test_db, expect_contains="6")
    add("DATE_EXTRACT_DAY", sql="SELECT EXTRACT(DAY FROM '2024-06-15')", db=test_db, expect_contains="15")
    add("DATE_DATE_FORMAT", sql="SELECT DATE_FORMAT('2024-06-15', '%Y/%m/%d')", db=test_db, expect_contains="2024/06/15")
    add("DATE_DATE_FORMAT_2", sql="SELECT DATE_FORMAT('2024-06-15', '%M %D, %Y')", db=test_db)
    add("DATE_TIME_FORMAT", sql="SELECT TIME_FORMAT('10:30:00', '%H:%i')", db=test_db, expect_contains="10:30")
    add("DATE_STR_TO_DATE", sql="SELECT LENGTH(STR_TO_DATE('2024-06-15', '%Y-%m-%d')) > 0", db=test_db, expect_contains="1")
    add("DATE_MAKEDATE", sql="SELECT MAKEDATE(2024, 100)", db=test_db)
    add("DATE_MAKETIME", sql="SELECT MAKETIME(10, 30, 0)", db=test_db, expect_contains="10:30")
    add("DATE_DATEDIFF", sql="SELECT DATEDIFF('2024-06-15', '2024-01-01')", db=test_db, expect_contains="166")
    add("DATE_TIMEDIFF", sql="SELECT TIMEDIFF('12:00:00', '10:30:00')", db=test_db, expect_contains="01:30")
    add("DATE_ADD_DAYS", sql="SELECT DATE_ADD('2024-06-15', INTERVAL 10 DAY)", db=test_db, expect_contains="2024-06-25")
    add("DATE_ADD_MONTHS", sql="SELECT DATE_ADD('2024-06-15', INTERVAL 1 MONTH)", db=test_db, expect_contains="2024-07-15")
    add("DATE_ADD_YEARS", sql="SELECT DATE_ADD('2024-06-15', INTERVAL 1 YEAR)", db=test_db, expect_contains="2025-06-15")
    add("DATE_ADD_HOURS", sql="SELECT DATE_ADD('2024-06-15 10:00:00', INTERVAL 5 HOUR)", db=test_db, expect_contains="15:00")
    add("DATE_SUB_DAYS", sql="SELECT DATE_SUB('2024-06-15', INTERVAL 10 DAY)", db=test_db, expect_contains="2024-06-05")
    add("DATE_SUB_MONTHS", sql="SELECT DATE_SUB('2024-06-15', INTERVAL 1 MONTH)", db=test_db, expect_contains="2024-05-15")
    add("DATE_ADDDATE", sql="SELECT ADDDATE('2024-06-15', INTERVAL 5 DAY)", db=test_db, expect_contains="2024-06-20")
    add("DATE_SUBDATE", sql="SELECT SUBDATE('2024-06-15', INTERVAL 5 DAY)", db=test_db, expect_contains="2024-06-10")
    add("DATE_ADDTIME", sql="SELECT LENGTH(ADDTIME('10:00:00', '01:30:00')) > 0", db=test_db, expect_contains="1")
    add("DATE_SUBTIME", sql="SELECT LENGTH(SUBTIME('10:00:00', '01:30:00')) > 0", db=test_db, expect_contains="1")
    add("DATE_LAST_DAY", sql="SELECT LAST_DAY('2024-02-15')", db=test_db, expect_contains="2024-02-29")
    add("DATE_LAST_DAY_2", sql="SELECT LAST_DAY('2023-02-15')", db=test_db, expect_contains="2023-02-28")
    add("DATE_TO_DAYS", sql="SELECT TO_DAYS('2024-06-15')", db=test_db)
    add("DATE_FROM_DAYS", sql="SELECT FROM_DAYS(738000)", db=test_db)
    add("DATE_PERIOD_ADD", sql="SELECT PERIOD_ADD(202406, 3)", db=test_db)
    add("DATE_PERIOD_DIFF", sql="SELECT PERIOD_DIFF(202406, 202401)", db=test_db, expect_contains="5")
    add("DATE_SEC_TO_TIME", sql="SELECT SEC_TO_TIME(3661)", db=test_db)
    add("DATE_TIME_TO_SEC", sql="SELECT TIME_TO_SEC('01:01:01')", db=test_db, expect_contains="3661")
    add("DATE_MICROSECOND", sql="SELECT MICROSECOND('10:30:00.123456')", db=test_db, expect_contains="123456")
    add("DATE_WEEKDAY", sql="SELECT WEEKDAY('2024-06-15')", db=test_db)

    # Date with table
    add("CREATE_DATE_TABLE", sql="CREATE TABLE IF NOT EXISTS t_date (id INT, dt DATE, ts DATETIME)", db=test_db)
    add("INSERT_DATE_1", sql="INSERT INTO t_date VALUES (1, '2024-01-15', '2024-01-15 10:30:00')", db=test_db)
    add("INSERT_DATE_2", sql="INSERT INTO t_date VALUES (2, '2024-06-20', '2024-06-20 14:45:00')", db=test_db)
    add("DATE_YEAR_TABLE", sql="SELECT YEAR(dt) FROM t_date WHERE id = 1", db=test_db, expect_contains="2024")
    add("DATE_MONTH_TABLE", sql="SELECT MONTH(dt) FROM t_date WHERE id = 1", db=test_db, expect_contains="1")
    add("DATE_DAY_TABLE", sql="SELECT DAY(dt) FROM t_date WHERE id = 2", db=test_db, expect_contains="20")
    add("DATE_DATEDIFF_TABLE", sql="SELECT DATEDIFF(dt, '2024-01-01') FROM t_date WHERE id = 1", db=test_db, expect_contains="14")
    add("DATE_FORMAT_TABLE", sql="SELECT DATE_FORMAT(dt, '%Y/%m') FROM t_date WHERE id = 1", db=test_db, expect_contains="2024/01")
    add("DROP_DATE_TABLE", sql="DROP TABLE IF EXISTS t_date", db=test_db)

    # =========================================================================
    # 6. SHOW TESTS (50+)
    # =========================================================================

    # Setup tables for SHOW tests
    add("CREATE_SHOW_TABLE", sql="CREATE TABLE IF NOT EXISTS t_show (id INT PRIMARY KEY, name VARCHAR(50), age INT, email VARCHAR(100))", db=test_db)
    add("INSERT_SHOW_1", sql="INSERT INTO t_show VALUES (1, 'Alice', 30, 'alice@test.com')", db=test_db)
    add("CREATE_SHOW_INDEX", sql="CREATE INDEX idx_show_name ON t_show (name)", db=test_db)

    # Setup table for INFORMATION_SCHEMA tests
    add("CREATE_INFO_TABLE", sql="CREATE TABLE IF NOT EXISTS t_info (id INT PRIMARY KEY, name VARCHAR(50) NOT NULL DEFAULT 'unknown', age INT, score DECIMAL(5,2), bio TEXT)", db=test_db)
    add("INSERT_INFO_1", sql="INSERT INTO t_info VALUES (1, 'Test', 25, 95.5, 'A test record')", db=test_db)
    add("CREATE_INFO_INDEX", sql="CREATE INDEX idx_info_name ON t_info (name)", db=test_db)

    add("SHOW_DATABASES", sql="SHOW DATABASES")
    add("SHOW_TABLES", sql="SHOW TABLES", db=test_db)
    add("SHOW_CREATE_DB", sql="SHOW CREATE DATABASE {test_db}", db=test_db)
    add("SHOW_TABLES_LIKE", sql="SHOW TABLES LIKE '%nonexist%'", db=test_db)
    add("SHOW_COLUMNS", sql="SHOW COLUMNS FROM t_show", db=test_db)
    add("SHOW_FULL_COLUMNS", sql="SHOW FULL COLUMNS FROM t_show", db=test_db)
    add("SHOW_INDEX", sql="SHOW INDEX FROM t_show", db=test_db)
    add("SHOW_INDEXES", sql="SHOW INDEXES FROM t_show", db=test_db)
    add("SHOW_KEYS", sql="SHOW KEYS FROM t_show", db=test_db)
    add("SHOW_VARIABLES", sql="SHOW VARIABLES")
    add("SHOW_GLOBAL_VARIABLES", sql="SHOW GLOBAL VARIABLES")
    add("SHOW_SESSION_VARIABLES", sql="SHOW SESSION VARIABLES")
    add("SHOW_VARIABLES_LIKE", sql="SHOW VARIABLES LIKE 'version%'")
    add("SHOW_STATUS", sql="SHOW STATUS")
    add("SHOW_GLOBAL_STATUS", sql="SHOW GLOBAL STATUS")
    add("SHOW_SESSION_STATUS", sql="SHOW SESSION STATUS")
    add("SHOW_COLLATION", sql="SHOW COLLATION")
    add("SHOW_COLLATION_LIKE", sql="SHOW COLLATION LIKE 'utf8%'")
    add("SHOW_CHARACTER_SET", sql="SHOW CHARACTER SET")
    add("SHOW_CHARSET", sql="SHOW CHARSET")
    add("SHOW_ENGINES", sql="SHOW ENGINES")
    add("SHOW_ENGINE_INNODB", sql="SHOW ENGINE INNODB STATUS")
    add("SHOW_GRANTS", sql="SHOW GRANTS")
    add("SHOW_GRANTS_USER", sql="SHOW GRANTS FOR 'root'@'localhost'")
    add("SHOW_PRIVILEGES", sql="SHOW PRIVILEGES")
    add("SHOW_PROCESSLIST", sql="SHOW PROCESSLIST")
    add("SHOW_FULL_PROCESSLIST", sql="SHOW FULL PROCESSLIST")
    add("SHOW_WARNINGS", sql="SHOW WARNINGS")
    add("SHOW_ERRORS", sql="SHOW ERRORS")
    add("SHOW_COUNT_WARNINGS", sql="SHOW COUNT(*) WARNINGS")
    add("SHOW_COUNT_ERRORS", sql="SHOW COUNT(*) ERRINGS")
    add("SHOW_TABLE_STATUS", sql="SHOW TABLE STATUS", db=test_db)
    add("SHOW_TABLE_STATUS_LIKE", sql="SHOW TABLE STATUS LIKE '%'", db=test_db)
    add("SHOW_CREATE_TABLE", sql="SHOW CREATE TABLE t_show", db=test_db)
    add("SHOW_DATABASES_LIKE", sql="SHOW DATABASES LIKE 'test%'")
    add("SHOW_DATABASES_LIKE2", sql="SHOW DATABASES LIKE '%nonexist%'")
    add("SHOW_VARIABLES_VERSION", sql="SHOW VARIABLES LIKE 'version'")
    add("SHOW_VARIABLES_MAX", sql="SHOW VARIABLES LIKE 'max_allowed_packet'")
    add("SHOW_VARIABLES_AUTO", sql="SHOW VARIABLES LIKE 'autocommit'")
    add("SHOW_CHARSET_UTF8", sql="SHOW CHARACTER SET LIKE 'utf8%'")
    add("SHOW_CHARSET_LATIN", sql="SHOW CHARACTER SET LIKE 'latin1%'")
    add("SHOW_COLLATION_UTF8", sql="SHOW COLLATION LIKE 'utf8%'")
    add("SHOW_COLLATION_BIN", sql="SHOW COLLATION LIKE '%_bin'")
    add("SHOW_MASTER_STATUS", sql="SHOW MASTER STATUS")
    add("SHOW_SLAVE_STATUS", sql="SHOW SLAVE STATUS")
    add("SHOW_BINLOG_EVENTS", sql="SHOW BINLOG EVENTS LIMIT 1")
    add("SHOW_BINARY_LOGS", sql="SHOW BINARY LOGS")
    add("SHOW_MASTER_LOGS", sql="SHOW MASTER LOGS")
    add("SHOW_RELAYLOG_EVENTS", sql="SHOW RELAYLOG EVENTS LIMIT 1")
    add("SHOW_TRIGGERS", sql="SHOW TRIGGERS", db=test_db)
    add("SHOW_PROCEDURE_STATUS", sql="SHOW PROCEDURE STATUS")
    add("SHOW_FUNCTION_STATUS", sql="SHOW FUNCTION STATUS")
    add("SHOW_OPEN_TABLES", sql="SHOW OPEN TABLES", db=test_db)
    add("SHOW_FIELDS", sql="SHOW FIELDS FROM t_show", db=test_db)

    # =========================================================================
    # 7. @@variables (40+)
    # =========================================================================

    add("VAR_VERSION", sql="SELECT @@version")
    add("VAR_VERSION_COMMENT", sql="SELECT @@version_comment")
    add("VAR_AUTOCommit", sql="SELECT @@autocommit")
    add("VAR_GLOBAL_AUTOCommit", sql="SELECT @@global.autocommit")
    add("VAR_SESSION_AUTOCommit", sql="SELECT @@session.autocommit")
    add("VAR_MAX_ALLOWED_PACKET", sql="SELECT @@max_allowed_packet")
    add("VAR_MAX_CONNECTIONS", sql="SELECT @@max_connections")
    add("VAR_CHARACTER_SET", sql="SELECT @@character_set_server")
    add("VAR_COLLATION", sql="SELECT @@collation_server")
    add("VAR_DATADIR", sql="SELECT @@datadir")
    add("VAR_HOSTNAME", sql="SELECT @@hostname")
    add("VAR_PORT", sql="SELECT @@port")
    add("VAR_SOCKET", sql="SELECT @@socket")
    add("VAR_SQL_MODE", sql="SELECT @@sql_mode")
    add("VAR_TIME_ZONE", sql="SELECT @@time_zone")
    add("VAR_SYSTEM_TIME_ZONE", sql="SELECT @@system_time_zone")
    add("VAR_TX_ISOLATION", sql="SELECT @@transaction_isolation")
    add("VAR_WAIT_TIMEOUT", sql="SELECT @@wait_timeout")
    add("VAR_INTERACTIVE_TIMEOUT", sql="SELECT @@interactive_timeout")
    add("VAR_NET_READ_TIMEOUT", sql="SELECT @@net_read_timeout")
    add("VAR_NET_WRITE_TIMEOUT", sql="SELECT @@net_write_timeout")
    add("VAR_LONG_QUERY_TIME", sql="SELECT @@long_query_time")
    add("VAR_SLOW_QUERY_LOG", sql="SELECT @@slow_query_log")
    add("VAR_LOG_ERROR", sql="SELECT @@log_error")
    add("VAR_GENERAL_LOG", sql="SELECT @@general_log")
    add("VAR_LOWER_CASE_TABLE", sql="SELECT @@lower_case_table_names")
    add("VAR_TABLE_OPEN_CACHE", sql="SELECT @@table_open_cache")
    add("VAR_THREAD_CACHE", sql="SELECT @@thread_cache_size")
    add("VAR_QUERY_CACHE_SIZE", sql="SELECT @@query_cache_size")
    add("VAR_SORT_BUFFER", sql="SELECT @@sort_buffer_size")
    add("VAR_JOIN_BUFFER", sql="SELECT @@join_buffer_size")
    add("VAR_TMP_TABLE_SIZE", sql="SELECT @@tmp_table_size")
    add("VAR_MAX_HEAP_TABLE", sql="SELECT @@max_heap_table_size")
    add("VAR_READ_BUFFER", sql="SELECT @@read_buffer_size")
    add("VAR_READ_RND_BUFFER", sql="SELECT @@read_rnd_buffer_size")
    add("VAR_BINLOG_FORMAT", sql="SELECT @@binlog_format")
    add("VAR_INNODB_BUFFER", sql="SELECT @@innodb_buffer_pool_size")
    add("VAR_INNODB_LOG_FILE", sql="SELECT @@innodb_log_file_size")
    add("VAR_INNODB_FLUSH", sql="SELECT @@innodb_flush_log_at_trx_commit")
    add("VAR_SERVER_ID", sql="SELECT @@server_id")
    add("VAR_BASEDIR", sql="SELECT @@basedir")
    add("VAR_PID_FILE", sql="SELECT @@pid_file")
    add("VAR_PERFORMANCE_SCHEMA", sql="SELECT @@performance_schema")

    # =========================================================================
    # 8. INFORMATION_SCHEMA (40+)
    # =========================================================================

    add("INFO_SCHEMATA", sql="SELECT SCHEMA_NAME FROM INFORMATION_SCHEMA.SCHEMATA")
    add("INFO_SCHEMATA_LIKE", sql="SELECT SCHEMA_NAME FROM INFORMATION_SCHEMA.SCHEMATA WHERE SCHEMA_NAME LIKE 'test%'")
    add("INFO_TABLES", sql="SELECT TABLE_NAME FROM INFORMATION_SCHEMA.TABLES WHERE TABLE_SCHEMA = '{test_db}'")
    add("INFO_TABLES_ALL", sql="SELECT TABLE_SCHEMA, TABLE_NAME FROM INFORMATION_SCHEMA.TABLES LIMIT 10")
    add("INFO_TABLES_TYPE", sql="SELECT TABLE_NAME, TABLE_TYPE FROM INFORMATION_SCHEMA.TABLES WHERE TABLE_SCHEMA = '{test_db}'")
    add("INFO_TABLES_ENGINE", sql="SELECT TABLE_NAME, ENGINE FROM INFORMATION_SCHEMA.TABLES WHERE TABLE_SCHEMA = '{test_db}'")
    add("INFO_COLUMNS", sql="SELECT COLUMN_NAME FROM INFORMATION_SCHEMA.COLUMNS WHERE TABLE_SCHEMA = '{test_db}' AND TABLE_NAME = 't_info'")
    add("INFO_COLUMNS_ALL", sql="SELECT TABLE_NAME, COLUMN_NAME FROM INFORMATION_SCHEMA.COLUMNS WHERE TABLE_SCHEMA = '{test_db}' LIMIT 10")
    add("INFO_COLUMNS_TYPE", sql="SELECT COLUMN_NAME, DATA_TYPE FROM INFORMATION_SCHEMA.COLUMNS WHERE TABLE_SCHEMA = '{test_db}' AND TABLE_NAME = 't_info'")
    add("INFO_COLUMNS_NULL", sql="SELECT COLUMN_NAME, IS_NULLABLE FROM INFORMATION_SCHEMA.COLUMNS WHERE TABLE_SCHEMA = '{test_db}' AND TABLE_NAME = 't_info'")
    add("INFO_COLUMNS_KEY", sql="SELECT COLUMN_NAME, COLUMN_KEY FROM INFORMATION_SCHEMA.COLUMNS WHERE TABLE_SCHEMA = '{test_db}' AND TABLE_NAME = 't_info'")
    add("INFO_COLUMNS_DEFAULT", sql="SELECT COLUMN_NAME, COLUMN_DEFAULT FROM INFORMATION_SCHEMA.COLUMNS WHERE TABLE_SCHEMA = '{test_db}' AND TABLE_NAME = 't_info'")
    add("INFO_COLUMNS_ORDINAL", sql="SELECT COLUMN_NAME, ORDINAL_POSITION FROM INFORMATION_SCHEMA.COLUMNS WHERE TABLE_SCHEMA = '{test_db}' AND TABLE_NAME = 't_info'")
    add("INFO_STATS", sql="SELECT TABLE_NAME FROM INFORMATION_SCHEMA.STATISTICS WHERE TABLE_SCHEMA = '{test_db}'")
    add("INFO_KEY_COLUMN", sql="SELECT TABLE_NAME, COLUMN_NAME FROM INFORMATION_SCHEMA.KEY_COLUMN_USAGE WHERE TABLE_SCHEMA = '{test_db}'")
    add("INFO_ENGINES", sql="SELECT ENGINE FROM INFORMATION_SCHEMA.ENGINES")
    add("INFO_CHARSETS", sql="SELECT CHARACTER_SET_NAME FROM INFORMATION_SCHEMA.CHARACTER_SETS")
    add("INFO_COLLATIONS", sql="SELECT COLLATION_NAME FROM INFORMATION_SCHEMA.COLLATIONS LIMIT 10")
    add("INFO_ROUTINES", sql="SELECT ROUTINE_NAME FROM INFORMATION_SCHEMA.ROUTINES")
    add("INFO_VIEWS", sql="SELECT TABLE_NAME FROM INFORMATION_SCHEMA.VIEWS WHERE TABLE_SCHEMA = '{test_db}'")
    add("INFO_TRIGGERS", sql="SELECT TRIGGER_NAME FROM INFORMATION_SCHEMA.TRIGGERS WHERE TRIGGER_SCHEMA = '{test_db}'")
    add("INFO_USER_PRIV", sql="SELECT GRANTEE FROM INFORMATION_SCHEMA.USER_PRIVILEGES")
    add("INFO_SCHEMA_PRIV", sql="SELECT GRANTEE FROM INFORMATION_SCHEMA.SCHEMA_PRIVILEGES")
    add("INFO_TABLE_PRIV", sql="SELECT GRANTEE FROM INFORMATION_SCHEMA.TABLE_PRIVILEGES")
    add("INFO_COLUMN_PRIV", sql="SELECT GRANTEE FROM INFORMATION_SCHEMA.COLUMN_PRIVILEGES")
    add("INFO_TABLE_CONSTRAINTS", sql="SELECT TABLE_NAME, CONSTRAINT_TYPE FROM INFORMATION_SCHEMA.TABLE_CONSTRAINTS WHERE TABLE_SCHEMA = '{test_db}'")
    add("INFO_REFERENTIAL", sql="SELECT CONSTRAINT_NAME FROM INFORMATION_SCHEMA.REFERENTIAL_CONSTRAINTS")
    add("INFO_PARTITIONS", sql="SELECT TABLE_NAME FROM INFORMATION_SCHEMA.PARTITIONS WHERE TABLE_SCHEMA = '{test_db}'")
    add("INFO_PLUGINS", sql="SELECT PLUGIN_NAME FROM INFORMATION_SCHEMA.PLUGINS LIMIT 5")
    add("INFO_PROCESSLIST", sql="SELECT ID FROM INFORMATION_SCHEMA.PROCESSLIST")
    add("INFO_GLOBAL_STATUS", sql="SELECT VARIABLE_NAME FROM INFORMATION_SCHEMA.GLOBAL_STATUS LIMIT 5")
    add("INFO_SESSION_STATUS", sql="SELECT VARIABLE_NAME FROM INFORMATION_SCHEMA.SESSION_STATUS LIMIT 5")
    add("INFO_GLOBAL_VARS", sql="SELECT VARIABLE_NAME FROM INFORMATION_SCHEMA.GLOBAL_VARIABLES LIMIT 5")
    add("INFO_SESSION_VARS", sql="SELECT VARIABLE_NAME FROM INFORMATION_SCHEMA.SESSION_VARIABLES LIMIT 5")
    add("INFO_TABLESPACES", sql="SELECT TABLESPACE_NAME FROM INFORMATION_SCHEMA.TABLESPACES")
    add("INFO_FILES", sql="SELECT FILE_NAME FROM INFORMATION_SCHEMA.FILES")
    add("INFO_OPTIMIZER_TRACE", sql="SELECT QUERY FROM INFORMATION_SCHEMA.OPTIMIZER_TRACE")
    add("INFO_PARAMETERS", sql="SELECT PARAMETER_NAME FROM INFORMATION_SCHEMA.PARAMETERS LIMIT 5")
    add("INFO_SCHEMATA_DEFAULT", sql="SELECT DEFAULT_CHARACTER_SET_NAME FROM INFORMATION_SCHEMA.SCHEMATA WHERE SCHEMA_NAME = '{test_db}'")
    add("INFO_SCHEMATA_COLLATION", sql="SELECT DEFAULT_COLLATION_NAME FROM INFORMATION_SCHEMA.SCHEMATA WHERE SCHEMA_NAME = '{test_db}'")
    add("INFO_COLUMNS_EXTRA", sql="SELECT COLUMN_NAME, EXTRA FROM INFORMATION_SCHEMA.COLUMNS WHERE TABLE_SCHEMA = '{test_db}' AND TABLE_NAME = 't_info'")

    # =========================================================================
    # 9. TRANSACTION TESTS (30+)
    # =========================================================================

    add("CREATE_TX_TABLE", sql="CREATE TABLE IF NOT EXISTS t_tx (id INT, val INT)", db=test_db)

    # Basic transactions
    add("TX_BEGIN_COMMIT", sql="BEGIN", db=test_db)
    add("TX_INSERT_1", sql="INSERT INTO t_tx VALUES (1, 100)", db=test_db)
    add("TX_COMMIT_1", sql="COMMIT", db=test_db)
    add("TX_VERIFY_1", sql="SELECT val FROM t_tx WHERE id = 1", db=test_db, expect_contains="100")

    add("TX_BEGIN_ROLLBACK", sql="BEGIN", db=test_db)
    add("TX_INSERT_2", sql="INSERT INTO t_tx VALUES (2, 200)", db=test_db)
    add("TX_ROLLBACK_1", sql="ROLLBACK", db=test_db)

    add("TX_BEGIN_COMMIT2", sql="BEGIN", db=test_db)
    add("TX_INSERT_3", sql="INSERT INTO t_tx VALUES (3, 300)", db=test_db)
    add("TX_COMMIT_2", sql="COMMIT", db=test_db)
    add("TX_VERIFY_3", sql="SELECT val FROM t_tx WHERE id = 3", db=test_db, expect_contains="300")

    # Start transaction syntax
    add("TX_START", sql="START TRANSACTION", db=test_db)
    add("TX_INSERT_4", sql="INSERT INTO t_tx VALUES (4, 400)", db=test_db)
    add("TX_COMMIT_3", sql="COMMIT", db=test_db)

    # Autocommit
    add("TX_SET_AUTOCOMMIT_0", sql="SET autocommit = 0", db=test_db)
    add("TX_INSERT_5", sql="INSERT INTO t_tx VALUES (5, 500)", db=test_db)
    add("TX_COMMIT_4", sql="COMMIT", db=test_db)
    add("TX_SET_AUTOCOMMIT_1", sql="SET autocommit = 1", db=test_db)

    # Multiple ops in transaction
    add("TX_MULTI_BEGIN", sql="BEGIN", db=test_db)
    add("TX_MULTI_INSERT", sql="INSERT INTO t_tx VALUES (6, 600)", db=test_db)
    add("TX_MULTI_UPDATE", sql="UPDATE t_tx SET val = 666 WHERE id = 6", db=test_db)
    add("TX_MULTI_DELETE", sql="DELETE FROM t_tx WHERE id = 5", db=test_db)
    add("TX_MULTI_COMMIT", sql="COMMIT", db=test_db)
    add("TX_MULTI_VERIFY", sql="SELECT val FROM t_tx WHERE id = 6", db=test_db, expect_contains="666")

    # Nested BEGIN
    add("TX_BEGIN_AGAIN", sql="BEGIN", db=test_db)
    add("TX_INSERT_7", sql="INSERT INTO t_tx VALUES (7, 700)", db=test_db)
    add("TX_COMMIT_5", sql="COMMIT", db=test_db)

    # SELECT after rollback
    add("TX_ROLLBACK_SELECT", sql="SELECT COUNT(*) FROM t_tx", db=test_db)

    # Savepoint tests
    add("TX_SAVEPOINT", sql="SAVEPOINT sp1", db=test_db)
    add("TX_INSERT_8", sql="INSERT INTO t_tx VALUES (8, 800)", db=test_db)
    add("TX_ROLLBACK_TO", sql="ROLLBACK TO sp1", db=test_db)

    # Transaction isolation
    add("TX_SET_ISOLATION_RC", sql="SET TRANSACTION ISOLATION LEVEL READ COMMITTED", db=test_db)
    add("TX_SET_ISOLATION_RR", sql="SET TRANSACTION ISOLATION LEVEL REPEATABLE READ", db=test_db)
    add("TX_SET_ISOLATION_RU", sql="SET TRANSACTION ISOLATION LEVEL READ UNCOMMITTED", db=test_db)
    add("TX_SET_ISOLATION_SER", sql="SET TRANSACTION ISOLATION LEVEL SERIALIZABLE", db=test_db)

    add("DROP_TX_TABLE", sql="DROP TABLE IF EXISTS t_tx", db=test_db)

    # =========================================================================
    # 10. JOIN TESTS (30+)
    # =========================================================================

    # Setup join tables
    add("CREATE_JOIN_EMP", sql="CREATE TABLE IF NOT EXISTS t_emp (id INT, name VARCHAR(50), dept_id INT)", db=test_db)
    add("CREATE_JOIN_DEPT", sql="CREATE TABLE IF NOT EXISTS t_dept (id INT, dept_name VARCHAR(50))", db=test_db)
    add("CREATE_JOIN_PROJ", sql="CREATE TABLE IF NOT EXISTS t_proj (id INT, proj_name VARCHAR(50), emp_id INT)", db=test_db)

    add("INSERT_EMP_1", sql="INSERT INTO t_emp VALUES (1, 'Alice', 1)", db=test_db)
    add("INSERT_EMP_2", sql="INSERT INTO t_emp VALUES (2, 'Bob', 2)", db=test_db)
    add("INSERT_EMP_3", sql="INSERT INTO t_emp VALUES (3, 'Carol', 1)", db=test_db)
    add("INSERT_EMP_4", sql="INSERT INTO t_emp VALUES (4, 'Dave', 3)", db=test_db)
    add("INSERT_EMP_5", sql="INSERT INTO t_emp VALUES (5, 'Eve', NULL)", db=test_db)

    add("INSERT_DEPT_1", sql="INSERT INTO t_dept VALUES (1, 'Engineering')", db=test_db)
    add("INSERT_DEPT_2", sql="INSERT INTO t_dept VALUES (2, 'Marketing')", db=test_db)
    add("INSERT_DEPT_3", sql="INSERT INTO t_dept VALUES (3, 'Sales')", db=test_db)
    add("INSERT_DEPT_4", sql="INSERT INTO t_dept VALUES (4, 'HR')", db=test_db)

    add("INSERT_PROJ_1", sql="INSERT INTO t_proj VALUES (1, 'Alpha', 1)", db=test_db)
    add("INSERT_PROJ_2", sql="INSERT INTO t_proj VALUES (2, 'Beta', 2)", db=test_db)
    add("INSERT_PROJ_3", sql="INSERT INTO t_proj VALUES (3, 'Gamma', 1)", db=test_db)
    add("INSERT_PROJ_4", sql="INSERT INTO t_proj VALUES (4, 'Delta', 3)", db=test_db)

    # INNER JOIN
    add("JOIN_INNER", sql="SELECT e.name, d.dept_name FROM t_emp e INNER JOIN t_dept d ON e.dept_id = d.id", db=test_db)
    add("JOIN_INNER_2", sql="SELECT e.name, d.dept_name FROM t_emp e JOIN t_dept d ON e.dept_id = d.id WHERE d.dept_name = 'Engineering'", db=test_db)
    add("JOIN_INNER_COUNT", sql="SELECT COUNT(*) FROM t_emp e INNER JOIN t_dept d ON e.dept_id = d.id", db=test_db, expect_contains="4")
    add("JOIN_INNER_WHERE", sql="SELECT e.name FROM t_emp e INNER JOIN t_dept d ON e.dept_id = d.id WHERE e.id > 2", db=test_db)

    # LEFT JOIN
    add("JOIN_LEFT", sql="SELECT e.name, d.dept_name FROM t_emp e LEFT JOIN t_dept d ON e.dept_id = d.id", db=test_db)
    add("JOIN_LEFT_COUNT", sql="SELECT COUNT(*) FROM t_emp e LEFT JOIN t_dept d ON e.dept_id = d.id", db=test_db, expect_contains="5")
    add("JOIN_LEFT_WHERE", sql="SELECT e.name FROM t_emp e LEFT JOIN t_dept d ON e.dept_id = d.id WHERE d.id IS NULL", db=test_db, expect_contains="Eve")
    add("JOIN_LEFT_NULL_CHECK", sql="SELECT e.name, d.dept_name FROM t_emp e LEFT JOIN t_dept d ON e.dept_id = d.id WHERE d.dept_name IS NULL", db=test_db)

    # RIGHT JOIN
    add("JOIN_RIGHT", sql="SELECT e.name, d.dept_name FROM t_emp e RIGHT JOIN t_dept d ON e.dept_id = d.id", db=test_db)
    add("JOIN_RIGHT_COUNT", sql="SELECT COUNT(*) FROM t_emp e RIGHT JOIN t_dept d ON e.dept_id = d.id", db=test_db)
    add("JOIN_RIGHT_WHERE", sql="SELECT d.dept_name FROM t_emp e RIGHT JOIN t_dept d ON e.dept_id = d.id WHERE e.id IS NULL", db=test_db, expect_contains="HR")

    # CROSS JOIN
    add("JOIN_CROSS", sql="SELECT e.name, d.dept_name FROM t_emp e CROSS JOIN t_dept d", db=test_db)
    add("JOIN_CROSS_COUNT", sql="SELECT COUNT(*) FROM t_emp e CROSS JOIN t_dept d", db=test_db, expect_contains="20")
    add("JOIN_CROSS_WHERE", sql="SELECT e.name, d.dept_name FROM t_emp e CROSS JOIN t_dept d WHERE e.dept_id = d.id", db=test_db)

    # Self JOIN
    add("JOIN_SELF", sql="SELECT a.name AS e1, b.name AS e2 FROM t_emp a JOIN t_emp b ON a.dept_id = b.dept_id AND a.id < b.id", db=test_db)
    add("JOIN_SELF_COUNT", sql="SELECT COUNT(*) FROM t_emp a JOIN t_emp b ON a.dept_id = b.dept_id AND a.id < b.id", db=test_db)

    # Multi-table JOIN
    add("JOIN_MULTI", sql="SELECT e.name, d.dept_name, p.proj_name FROM t_emp e INNER JOIN t_dept d ON e.dept_id = d.id INNER JOIN t_proj p ON p.emp_id = e.id", db=test_db)
    add("JOIN_MULTI_LEFT", sql="SELECT e.name, d.dept_name, p.proj_name FROM t_emp e LEFT JOIN t_dept d ON e.dept_id = d.id LEFT JOIN t_proj p ON p.emp_id = e.id", db=test_db)

    # JOIN with aggregate
    add("JOIN_AGG_COUNT", sql="SELECT d.dept_name, COUNT(e.id) FROM t_dept d LEFT JOIN t_emp e ON e.dept_id = d.id GROUP BY d.dept_name", db=test_db)
    add("JOIN_AGG_SUM", sql="SELECT d.dept_name, SUM(e.id) FROM t_dept d INNER JOIN t_emp e ON e.dept_id = d.id GROUP BY d.dept_name", db=test_db)

    # JOIN with ORDER BY and LIMIT
    add("JOIN_ORDER", sql="SELECT e.name, d.dept_name FROM t_emp e INNER JOIN t_dept d ON e.dept_id = d.id ORDER BY e.name LIMIT 3", db=test_db)
    add("JOIN_ORDER_DESC", sql="SELECT e.name, d.dept_name FROM t_emp e INNER JOIN t_dept d ON e.dept_id = d.id ORDER BY d.dept_name DESC", db=test_db)

    # JOIN with WHERE conditions
    add("JOIN_WHERE_MULTI", sql="SELECT e.name, d.dept_name FROM t_emp e INNER JOIN t_dept d ON e.dept_id = d.id WHERE e.age > 0 OR d.dept_name = 'Engineering'", db=test_db)

    # NATURAL JOIN
    add("JOIN_NATURAL", sql="SELECT * FROM t_emp NATURAL JOIN t_dept", db=test_db)

    # JOIN with USING
    add("JOIN_USING", sql="SELECT e.name, d.dept_name FROM t_emp e JOIN t_dept d USING(id)", db=test_db)

    # Cleanup join tables
    add("DROP_JOIN_PROJ", sql="DROP TABLE IF EXISTS t_proj", db=test_db)
    add("DROP_JOIN_EMP", sql="DROP TABLE IF EXISTS t_emp", db=test_db)
    add("DROP_JOIN_DEPT", sql="DROP TABLE IF EXISTS t_dept", db=test_db)

    # =========================================================================
    # 11. EDGE CASES (50+)
    # =========================================================================

    # NULL handling
    add("EDGE_NULL_SELECT", sql="SELECT NULL")
    add("EDGE_NULL_COMPARE", sql="SELECT NULL = NULL", db=test_db)
    add("EDGE_NULL_IS", sql="SELECT NULL IS NULL", db=test_db, expect_contains="1")
    add("EDGE_NULL_ISNOT", sql="SELECT NULL IS NOT NULL", db=test_db, expect_contains="0")
    add("EDGE_NULLIF", sql="SELECT NULLIF(1, 1)", db=test_db)
    add("EDGE_NULLIF_2", sql="SELECT NULLIF(1, 2)", db=test_db, expect_contains="1")
    add("EDGE_IFNULL", sql="SELECT IFNULL(NULL, 'default')", db=test_db, expect_contains="default")
    add("EDGE_IFNULL_2", sql="SELECT IFNULL('value', 'default')", db=test_db, expect_contains="value")
    add("EDGE_IF_FUNC", sql="SELECT IF(1 > 0, 'yes', 'no')", db=test_db, expect_contains="yes")
    add("EDGE_IF_FUNC_2", sql="SELECT IF(1 < 0, 'yes', 'no')", db=test_db, expect_contains="no")

    # Unicode
    add("EDGE_UNICODE_CHINESE", sql="SELECT '你好世界'", db=test_db, expect_contains="你好世界")
    add("EDGE_UNICODE_JAPANESE", sql="SELECT 'こんにちは'", db=test_db, expect_contains="こんにちは")
    add("EDGE_UNICODE_KOREAN", sql="SELECT '안녕하세요'", db=test_db, expect_contains="안녕하세요")
    add("EDGE_UNICODE_EMOJI", sql="SELECT '🎉🚀💡'", db=test_db, expect_contains="🎉")
    add("EDGE_UNICODE_MIXED", sql="SELECT 'Hello 世界 🌍'", db=test_db, expect_contains="Hello")
    add("EDGE_UNICODE_ARABIC", sql="SELECT 'مرحبا'", db=test_db, expect_contains="مرحبا")
    add("EDGE_UNICODE_RUSSIAN", sql="SELECT 'Привет мир'", db=test_db, expect_contains="Привет")
    add("EDGE_UNICODE_LENGTH", sql="SELECT LENGTH('hello')", db=test_db, expect_contains="5")

    # Special characters in strings
    add("EDGE_SPECIAL_QUOTE", sql="SELECT 'it''s a test'", db=test_db, expect_contains="it's a test")
    add("EDGE_SPECIAL_BACKSLASH", sql="SELECT 'hello\\\\world'", db=test_db)
    add("EDGE_SPECIAL_NEWLINE", sql="SELECT 'line1\\nline2'", db=test_db)
    add("EDGE_SPECIAL_TAB", sql="SELECT 'col1\\tcol2'", db=test_db)
    add("EDGE_SPECIAL_PERCENT", sql="SELECT '100%'", db=test_db, expect_contains="100%")
    add("EDGE_SPECIAL_UNDERSCORE", sql="SELECT 'a_b_c'", db=test_db, expect_contains="a_b_c")
    add("EDGE_SPECIAL_AT", sql="SELECT '@user'", db=test_db, expect_contains="@user")
    add("EDGE_SPECIAL_HASH", sql="SELECT '#tag'", db=test_db, expect_contains="#tag")
    add("EDGE_SPECIAL_DOLLAR", sql="SELECT '$100'", db=test_db, expect_contains="$100")

    # Large values
    add("EDGE_BIGINT_MAX", sql="SELECT 9223372036854775807", db=test_db, expect_contains="9223372036854775807")
    add("EDGE_BIGINT_MIN", sql="SELECT -9223372036854775808", db=test_db, expect_contains="-9223372036854775808")
    add("EDGE_DECIMAL_PREC", sql="SELECT 123456789.123456789", db=test_db)
    add("EDGE_FLOAT_PREC", sql="SELECT CAST(3.14159265 AS DOUBLE)", db=test_db)

    # Empty results
    add("EDGE_EMPTY_SELECT", sql="SELECT 1 WHERE 1 = 0", db=test_db)
    add("EDGE_EMPTY_COUNT", sql="SELECT COUNT(*) FROM (SELECT 1 WHERE 1=0) t", db=test_db, expect_contains="0")

    # Boolean / truth values
    add("EDGE_TRUE", sql="SELECT TRUE", db=test_db, expect_contains="1")
    add("EDGE_FALSE", sql="SELECT FALSE", db=test_db, expect_contains="0")
    add("EDGE_BOOL_AND", sql="SELECT 1 AND 1", db=test_db, expect_contains="1")
    add("EDGE_BOOL_OR", sql="SELECT 0 OR 1", db=test_db, expect_contains="1")
    add("EDGE_BOOL_NOT", sql="SELECT NOT 0", db=test_db, expect_contains="1")
    add("EDGE_BOOL_XOR", sql="SELECT 1 XOR 0", db=test_db, expect_contains="1")

    # Multiple expressions
    add("EDGE_MULTI_SELECT", sql="SELECT 1, 2, 3", db=test_db)
    add("EDGE_MULTI_ALIAS", sql="SELECT 1 AS a, 2 AS b, 3 AS c", db=test_db)

    # Comments in SQL
    add("EDGE_COMMENT_LINE", sql="SELECT 1 -- this is a comment", db=test_db, expect_contains="1")
    add("EDGE_COMMENT_BLOCK", sql="SELECT /* comment */ 1", db=test_db, expect_contains="1")

    # Arithmetic edge cases
    add("EDGE_ARITH_OVERFLOW", sql="SELECT 999999999 * 999999999", db=test_db)
    add("EDGE_ARITH_NEG", sql="SELECT -1 * -1", db=test_db, expect_contains="1")
    add("EDGE_ARITH_ZERO", sql="SELECT 0 * 999999", db=test_db, expect_contains="0")
    add("EDGE_ARITH_MOD_1", sql="SELECT 100 MOD 1", db=test_db, expect_contains="0")

    # String comparison
    add("EDGE_STR_CMP_EQ", sql="SELECT 'abc' = 'abc'", db=test_db, expect_contains="1")
    add("EDGE_STR_CMP_NEQ", sql="SELECT 'abc' != 'def'", db=test_db, expect_contains="1")
    add("EDGE_STR_CMP_LT", sql="SELECT 'a' < 'b'", db=test_db, expect_contains="1")
    add("EDGE_STR_CMP_GT", sql="SELECT 'z' > 'a'", db=test_db, expect_contains="1")

    # LIMIT 0
    add("EDGE_LIMIT_0", sql="SELECT * FROM (SELECT 1 UNION SELECT 2) t LIMIT 0", db=test_db)

    # OFFSET beyond results
    add("EDGE_OFFSET_LARGE", sql="SELECT * FROM (SELECT 1 UNION SELECT 2) t LIMIT 10 OFFSET 100", db=test_db)

    # Duplicate values
    add("EDGE_DUP_INSERT", sql="CREATE TABLE IF NOT EXISTS t_dup (id INT)", db=test_db)
    add("EDGE_DUP_INS1", sql="INSERT INTO t_dup VALUES (1)", db=test_db)
    add("EDGE_DUP_INS2", sql="INSERT INTO t_dup VALUES (1)", db=test_db)
    add("EDGE_DUP_COUNT", sql="SELECT COUNT(*) FROM t_dup", db=test_db, expect_contains="2")
    add("EDGE_DUP_DROP", sql="DROP TABLE IF EXISTS t_dup", db=test_db)

    # Select without FROM
    add("EDGE_SELECT_LITERAL", sql="SELECT 42", db=test_db, expect_contains="42")
    add("EDGE_SELECT_STRING", sql="SELECT 'hello'", db=test_db, expect_contains="hello")
    add("EDGE_SELECT_EXPR", sql="SELECT 1 + 2 * 3", db=test_db, expect_contains="7")
    add("EDGE_SELECT_FUNC", sql="SELECT UPPER('test')", db=test_db, expect_contains="TEST")

    # Division and modulo edge cases
    add("EDGE_DIV_INT", sql="SELECT 10 / 3", db=test_db)
    add("EDGE_DIV_INT_DIV", sql="SELECT 10 DIV 3", db=test_db, expect_contains="3")
    add("EDGE_MOD_1", sql="SELECT 7 MOD 3", db=test_db, expect_contains="1")

    # Bitwise operations
    add("EDGE_BIT_AND", sql="SELECT 5 & 3", db=test_db, expect_contains="1")
    add("EDGE_BIT_OR", sql="SELECT 5 | 3", db=test_db, expect_contains="7")
    add("EDGE_BIT_XOR", sql="SELECT 5 ^ 3", db=test_db, expect_contains="6")
    add("EDGE_BIT_NOT", sql="SELECT ~0", db=test_db)
    add("EDGE_BIT_SHIFT_L", sql="SELECT 1 << 4", db=test_db, expect_contains="16")
    add("EDGE_BIT_SHIFT_R", sql="SELECT 16 >> 2", db=test_db, expect_contains="4")

    # Complex expressions
    add("EDGE_COMPLEX_1", sql="SELECT (1 + 2) * (3 + 4)", db=test_db, expect_contains="21")
    add("EDGE_COMPLEX_2", sql="SELECT CASE WHEN 1 > 0 THEN CASE WHEN 2 > 1 THEN 'nested' END END", db=test_db, expect_contains="nested")

    # Miscellaneous functions
    add("EDGE_DATABASE_FUNC", sql="SELECT DATABASE()", db=test_db)
    add("EDGE_USER_FUNC", sql="SELECT USER()")
    add("EDGE_CURRENT_USER", sql="SELECT CURRENT_USER()")
    add("EDGE_CONNECTION_ID", sql="SELECT CONNECTION_ID()")
    add("EDGE_FOUND_ROWS", sql="SELECT FOUND_ROWS()")
    add("EDGE_ROW_COUNT", sql="SELECT ROW_COUNT()")
    add("EDGE_LAST_INSERT_ID", sql="SELECT LAST_INSERT_ID()")
    add("EDGE_BENCHMARK", sql="SELECT BENCHMARK(1, SHA2('test', 256))", db=test_db)
    add("EDGE_SLEEP", sql="SELECT SLEEP(0.01)", db=test_db)
    add("EDGE_IS_FREE_LOCK", sql="SELECT IS_FREE_LOCK('test_lock')", db=test_db)
    add("EDGE_GET_LOCK", sql="SELECT GET_LOCK('test_lock_abc', 1)", db=test_db)
    add("EDGE_RELEASE_LOCK", sql="SELECT RELEASE_LOCK('test_lock_abc')", db=test_db)
    add("EDGE_VERSION_FUNC", sql="SELECT VERSION()")
    add("EDGE_CHARSET_FUNC", sql="SELECT CHARSET('hello')", db=test_db)
    add("EDGE_COLLATION_FUNC", sql="SELECT COLLATION('hello')", db=test_db)
    add("EDGE_COERCIBILITY", sql="SELECT COERCIBILITY('hello')", db=test_db)
    add("EDGE_ENCODING", sql="SELECT 'test'")

    # GROUP_CONCAT
    add("EDGE_GROUP_CONCAT", sql="CREATE TABLE IF NOT EXISTS t_gc (id INT, grp INT, val VARCHAR(50))", db=test_db)
    add("EDGE_GC_INS1", sql="INSERT INTO t_gc VALUES (1, 1, 'a')", db=test_db)
    add("EDGE_GC_INS2", sql="INSERT INTO t_gc VALUES (2, 1, 'b')", db=test_db)
    add("EDGE_GC_INS3", sql="INSERT INTO t_gc VALUES (3, 2, 'c')", db=test_db)
    add("EDGE_GC_SELECT", sql="SELECT grp, GROUP_CONCAT(val ORDER BY val SEPARATOR ',') FROM t_gc GROUP BY grp", db=test_db)
    add("EDGE_GC_DROP", sql="DROP TABLE IF EXISTS t_gc", db=test_db)

    # Window-like aggregate variations
    add("EDGE_COUNT_ALL", sql="SELECT COUNT(*) FROM (SELECT 1 AS x UNION ALL SELECT 2 UNION ALL SELECT 3) t", db=test_db, expect_contains="3")
    add("EDGE_SUM_RANGE", sql="SELECT SUM(x) FROM (SELECT 1 AS x UNION ALL SELECT 2 UNION ALL SELECT 3) t", db=test_db, expect_contains="6")

    # =========================================================================
    # ADDITIONAL DDL TESTS
    # =========================================================================

    # Tables with various column types - additional
    add("CREATE_TABLE_JSON_LIKE", sql="CREATE TABLE IF NOT EXISTS t_jsonlike (id INT, data TEXT)", db=test_db)
    add("INSERT_JSONLIKE", sql="INSERT INTO t_jsonlike VALUES (1, '{\"key\": \"value\"}')", db=test_db)
    add("SELECT_JSONLIKE", sql="SELECT data FROM t_jsonlike WHERE id = 1", db=test_db, expect_contains="key")
    add("DROP_TABLE_JSONLIKE", sql="DROP TABLE IF EXISTS t_jsonlike", db=test_db)

    # Tables with ENUM-like pattern
    add("CREATE_TABLE_ENUM", sql="CREATE TABLE IF NOT EXISTS t_enum (id INT, status VARCHAR(20))", db=test_db)
    add("INSERT_ENUM", sql="INSERT INTO t_enum VALUES (1, 'active')", db=test_db)
    add("INSERT_ENUM2", sql="INSERT INTO t_enum VALUES (2, 'inactive')", db=test_db)
    add("DROP_TABLE_ENUM", sql="DROP TABLE IF EXISTS t_enum", db=test_db)

    # Multiple ALTER operations
    add("CREATE_TABLE_ALTER2", sql="CREATE TABLE IF NOT EXISTS t_alter2 (id INT)", db=test_db)
    add("ALTER2_ADD_A", sql="ALTER TABLE t_alter2 ADD COLUMN a INT", db=test_db)
    add("ALTER2_ADD_B", sql="ALTER TABLE t_alter2 ADD COLUMN b VARCHAR(50)", db=test_db)
    add("ALTER2_ADD_C", sql="ALTER TABLE t_alter2 ADD COLUMN c DOUBLE", db=test_db)
    add("ALTER2_DROP_A", sql="ALTER TABLE t_alter2 DROP COLUMN a", db=test_db)
    add("ALTER2_DROP_B", sql="ALTER TABLE t_alter2 DROP COLUMN b", db=test_db)
    add("DROP_TABLE_ALTER2", sql="DROP TABLE IF EXISTS t_alter2", db=test_db)

    # CREATE TABLE with various constraints
    add("CREATE_TABLE_UK", sql="CREATE TABLE IF NOT EXISTS t_uk (id INT, email VARCHAR(100), UNIQUE(email))", db=test_db)
    add("INSERT_UK_1", sql="INSERT INTO t_uk VALUES (1, 'a@b.com')", db=test_db)
    add("DROP_TABLE_UK", sql="DROP TABLE IF EXISTS t_uk", db=test_db)

    # CREATE TABLE with CHECK constraint (may or may not be supported)
    add("CREATE_TABLE_CHECK", sql="CREATE TABLE IF NOT EXISTS t_check (id INT, age INT)", db=test_db)
    add("INSERT_CHECK", sql="INSERT INTO t_check VALUES (1, 25)", db=test_db)
    add("DROP_TABLE_CHECK", sql="DROP TABLE IF EXISTS t_check", db=test_db)

    # CREATE TABLE with COMMENT
    add("CREATE_TABLE_COMMENT", sql="CREATE TABLE IF NOT EXISTS t_comment (id INT COMMENT 'primary key', name VARCHAR(50) COMMENT 'user name')", db=test_db)
    add("DROP_TABLE_COMMENT", sql="DROP TABLE IF EXISTS t_comment", db=test_db)

    # CREATE TABLE with ENGINE specification
    add("CREATE_TABLE_ENGINE", sql="CREATE TABLE IF NOT EXISTS t_engine (id INT) ENGINE=InnoDB", db=test_db)
    add("DROP_TABLE_ENGINE", sql="DROP TABLE IF EXISTS t_engine", db=test_db)

    # CREATE TABLE with CHARSET
    add("CREATE_TABLE_CHARSET", sql="CREATE TABLE IF NOT EXISTS t_charset (id INT, name VARCHAR(50)) DEFAULT CHARSET=utf8mb4", db=test_db)
    add("DROP_TABLE_CHARSET", sql="DROP TABLE IF EXISTS t_charset", db=test_db)

    # CREATE TABLE with COLLATE
    add("CREATE_TABLE_COLLATE", sql="CREATE TABLE IF NOT EXISTS t_collate (id INT, name VARCHAR(50)) DEFAULT COLLATE=utf8mb4_general_ci", db=test_db)
    add("DROP_TABLE_COLLATE", sql="DROP TABLE IF EXISTS t_collate", db=test_db)

    # TRUNCATE with FK-like setup
    add("CREATE_TABLE_TR2", sql="CREATE TABLE IF NOT EXISTS t_tr2 (id INT, val INT)", db=test_db)
    for j in range(5):
        add(f"INSERT_TR2_{j}", sql=f"INSERT INTO t_tr2 VALUES ({j}, {j*10})", db=test_db)
    add("TRUNCATE_TR2", sql="TRUNCATE TABLE t_tr2", db=test_db)
    add("SELECT_TR2_EMPTY", sql="SELECT COUNT(*) FROM t_tr2", db=test_db, expect_contains="0")
    add("DROP_TABLE_TR2", sql="DROP TABLE IF EXISTS t_tr2", db=test_db)

    # Multiple views
    add("CREATE_TABLE_VW", sql="CREATE TABLE IF NOT EXISTS t_vw (id INT, category VARCHAR(50), amount DECIMAL(10,2))", db=test_db)
    add("INSERT_VW_1", sql="INSERT INTO t_vw VALUES (1, 'A', 100)", db=test_db)
    add("INSERT_VW_2", sql="INSERT INTO t_vw VALUES (2, 'B', 200)", db=test_db)
    add("INSERT_VW_3", sql="INSERT INTO t_vw VALUES (3, 'A', 150)", db=test_db)
    add("CREATE_VIEW_VW1", sql="CREATE OR REPLACE VIEW v_a AS SELECT * FROM t_vw WHERE category = 'A'", db=test_db)
    add("CREATE_VIEW_VW2", sql="CREATE OR REPLACE VIEW v_b AS SELECT * FROM t_vw WHERE category = 'B'", db=test_db)
    add("SELECT_VIEW_VW1", sql="SELECT * FROM v_a", db=test_db)
    add("SELECT_VIEW_VW2", sql="SELECT * FROM v_b", db=test_db)
    add("DROP_VIEW_VW1", sql="DROP VIEW IF EXISTS v_a", db=test_db)
    add("DROP_VIEW_VW2", sql="DROP VIEW IF EXISTS v_b", db=test_db)
    add("DROP_TABLE_VW", sql="DROP TABLE IF EXISTS t_vw", db=test_db)

    # =========================================================================
    # ADDITIONAL DML TESTS
    # =========================================================================

    # Subquery variations
    add("CREATE_SUB_TABLE", sql="CREATE TABLE IF NOT EXISTS t_sub (id INT, parent_id INT, val INT)", db=test_db)
    add("INSERT_SUB_1", sql="INSERT INTO t_sub VALUES (1, NULL, 10)", db=test_db)
    add("INSERT_SUB_2", sql="INSERT INTO t_sub VALUES (2, 1, 20)", db=test_db)
    add("INSERT_SUB_3", sql="INSERT INTO t_sub VALUES (3, 1, 30)", db=test_db)
    add("INSERT_SUB_4", sql="INSERT INTO t_sub VALUES (4, 2, 40)", db=test_db)
    add("INSERT_SUB_5", sql="INSERT INTO t_sub VALUES (5, 3, 50)", db=test_db)
    add("SUBQUERY_SELF", sql="SELECT a.val, b.val FROM t_sub a, t_sub b WHERE a.id = b.parent_id", db=test_db)
    add("SUBQUERY_CORRELATED", sql="SELECT val FROM t_sub a WHERE val > (SELECT AVG(val) FROM t_sub b WHERE b.parent_id = a.id)", db=test_db)
    add("DROP_SUB_TABLE", sql="DROP TABLE IF EXISTS t_sub", db=test_db)

    # INSERT ... SELECT
    add("CREATE_SEL_SRC", sql="CREATE TABLE IF NOT EXISTS t_sel_src (id INT, val INT)", db=test_db)
    add("CREATE_SEL_DST", sql="CREATE TABLE IF NOT EXISTS t_sel_dst (id INT, val INT)", db=test_db)
    add("INSERT_SEL_SRC", sql="INSERT INTO t_sel_src VALUES (1, 10), (2, 20), (3, 30)", db=test_db)
    add("INSERT_SELECT", sql="INSERT INTO t_sel_dst SELECT * FROM t_sel_src", db=test_db)
    add("SELECT_DST_VERIFY", sql="SELECT COUNT(*) FROM t_sel_dst", db=test_db, expect_contains="3")
    add("DROP_SEL_SRC", sql="DROP TABLE IF EXISTS t_sel_src", db=test_db)
    add("DROP_SEL_DST", sql="DROP TABLE IF EXISTS t_sel_dst", db=test_db)

    # REPLACE INTO
    add("CREATE_REP_TABLE", sql="CREATE TABLE IF NOT EXISTS t_rep (id INT PRIMARY KEY, val INT)", db=test_db)
    add("INSERT_REP_1", sql="INSERT INTO t_rep VALUES (1, 100)", db=test_db)
    add("REPLACE_REP_1", sql="REPLACE INTO t_rep VALUES (1, 200)", db=test_db)
    add("SELECT_REP_VERIFY", sql="SELECT val FROM t_rep WHERE id = 1", db=test_db, expect_contains="200")
    add("DROP_REP_TABLE", sql="DROP TABLE IF EXISTS t_rep", db=test_db)

    # ON DUPLICATE KEY UPDATE
    add("CREATE_ODKU", sql="CREATE TABLE IF NOT EXISTS t_odku (id INT PRIMARY KEY, val INT)", db=test_db)
    add("INSERT_ODKU_1", sql="INSERT INTO t_odku VALUES (1, 100)", db=test_db)
    add("ODKU_UPDATE", sql="INSERT INTO t_odku VALUES (1, 200) ON DUPLICATE KEY UPDATE val = 200", db=test_db)
    add("SELECT_ODKU_VERIFY", sql="SELECT val FROM t_odku WHERE id = 1", db=test_db, expect_contains="200")
    add("DROP_ODKU", sql="DROP TABLE IF EXISTS t_odku", db=test_db)

    # Complex WHERE with nested conditions
    add("CREATE_CX_TABLE", sql="CREATE TABLE IF NOT EXISTS t_cx (id INT, a INT, b INT, c VARCHAR(50))", db=test_db)
    for j in range(20):
        add(f"INSERT_CX_{j}", sql=f"INSERT INTO t_cx VALUES ({j}, {j%5}, {j%3}, 'item_{j}')", db=test_db)
    add("CX_WHERE_COMPLEX", sql="SELECT * FROM t_cx WHERE (a = 1 OR a = 2) AND (b = 0 OR b = 1) AND c LIKE 'item_%'", db=test_db)
    add("CX_WHERE_BETWEEN_AND", sql="SELECT * FROM t_cx WHERE a BETWEEN 1 AND 3 AND b BETWEEN 0 AND 1", db=test_db)
    add("CX_WHERE_NOT", sql="SELECT COUNT(*) FROM t_cx WHERE NOT (a = 0)", db=test_db)
    add("CX_ORDER_LIMIT", sql="SELECT * FROM t_cx ORDER BY a DESC, b ASC LIMIT 5", db=test_db)
    add("CX_GROUP_HAVING_ORDER", sql="SELECT a, COUNT(*) AS cnt FROM t_cx GROUP BY a HAVING cnt >= 3 ORDER BY cnt DESC", db=test_db)
    add("DROP_CX_TABLE", sql="DROP TABLE IF EXISTS t_cx", db=test_db)

    # Multiple aggregates in one query
    add("CREATE_AGG_TABLE", sql="CREATE TABLE IF NOT EXISTS t_agg (grp INT, val1 INT, val2 DOUBLE)", db=test_db)
    for j in range(15):
        add(f"INSERT_AGG_{j}", sql=f"INSERT INTO t_agg VALUES ({j%3}, {j*10}, {j*1.5})", db=test_db)
    add("AGG_MULTI", sql="SELECT grp, COUNT(*), SUM(val1), AVG(val1), MIN(val2), MAX(val2) FROM t_agg GROUP BY grp", db=test_db)
    add("AGG_COUNT_DISTINCT", sql="SELECT COUNT(DISTINCT grp) FROM t_agg", db=test_db, expect_contains="3")
    add("AGG_SUM_DISTINCT", sql="SELECT SUM(DISTINCT grp) FROM t_agg", db=test_db, expect_contains="3")
    add("DROP_AGG_TABLE", sql="DROP TABLE IF EXISTS t_agg", db=test_db)

    # DELETE with complex conditions
    add("CREATE_DEL_TABLE", sql="CREATE TABLE IF NOT EXISTS t_del (id INT, val INT, grp VARCHAR(10))", db=test_db)
    for j in range(10):
        add(f"INSERT_DEL_{j}", sql=f"INSERT INTO t_del VALUES ({j}, {j*5}, CASE WHEN {j} < 5 THEN 'A' ELSE 'B' END)", db=test_db)
    add("DEL_WHERE_IN", sql="DELETE FROM t_del WHERE id IN (1, 3, 5)", db=test_db)
    add("DEL_WHERE_LIKE", sql="DELETE FROM t_del WHERE grp LIKE 'B' AND val > 30", db=test_db)
    add("DEL_VERIFY", sql="SELECT COUNT(*) FROM t_del", db=test_db)
    add("DROP_DEL_TABLE", sql="DROP TABLE IF EXISTS t_del", db=test_db)

    # UPDATE with complex SET
    add("CREATE_UPD_TABLE", sql="CREATE TABLE IF NOT EXISTS t_upd (id INT, val INT, flag INT)", db=test_db)
    for j in range(5):
        add(f"INSERT_UPD_{j}", sql=f"INSERT INTO t_upd VALUES ({j}, {j*10}, 0)", db=test_db)
    add("UPD_MULTI_COL", sql="UPDATE t_upd SET val = val + 1, flag = 1 WHERE id > 0", db=test_db)
    add("UPD_VERIFY", sql="SELECT flag FROM t_upd WHERE id = 1", db=test_db, expect_contains="1")
    add("UPD_SET_NULL", sql="UPDATE t_upd SET flag = NULL WHERE id = 0", db=test_db)
    add("UPD_VERIFY_NULL", sql="SELECT flag FROM t_upd WHERE id = 0", db=test_db)
    add("DROP_UPD_TABLE", sql="DROP TABLE IF EXISTS t_upd", db=test_db)

    # ORDER BY with multiple columns
    add("CREATE_ORD_TABLE", sql="CREATE TABLE IF NOT EXISTS t_ord (a INT, b INT, c VARCHAR(10))", db=test_db)
    for j in range(10):
        add(f"INSERT_ORD_{j}", sql=f"INSERT INTO t_ord VALUES ({j%3}, {j%2}, 'r{j}')", db=test_db)
    add("ORD_MULTI_ASC", sql="SELECT * FROM t_ord ORDER BY a ASC, b ASC", db=test_db)
    add("ORD_MULTI_MIX", sql="SELECT * FROM t_ord ORDER BY a ASC, b DESC", db=test_db)
    add("ORD_BY_ALIAS", sql="SELECT a AS x, b AS y FROM t_ord ORDER BY x, y", db=test_db)
    add("DROP_ORD_TABLE", sql="DROP TABLE IF EXISTS t_ord", db=test_db)

    # LIMIT with various offsets
    add("CREATE_LIM_TABLE", sql="CREATE TABLE IF NOT EXISTS t_lim (id INT)", db=test_db)
    for j in range(20):
        add(f"INSERT_LIM_{j}", sql=f"INSERT INTO t_lim VALUES ({j})", db=test_db)
    add("LIM_0_5", sql="SELECT COUNT(*) FROM (SELECT * FROM t_lim LIMIT 5 OFFSET 0) t", db=test_db, expect_contains="5")
    add("LIM_5_5", sql="SELECT COUNT(*) FROM (SELECT * FROM t_lim LIMIT 5 OFFSET 5) t", db=test_db, expect_contains="5")
    add("LIM_15_10", sql="SELECT COUNT(*) FROM (SELECT * FROM t_lim LIMIT 10 OFFSET 15) t", db=test_db, expect_contains="5")
    add("LIM_20_0", sql="SELECT COUNT(*) FROM (SELECT * FROM t_lim LIMIT 0 OFFSET 20) t", db=test_db, expect_contains="0")
    add("DROP_LIM_TABLE", sql="DROP TABLE IF EXISTS t_lim", db=test_db)

    # =========================================================================
    # ADDITIONAL STRING FUNCTION TESTS
    # =========================================================================

    for s in ["hello", "world", "test", "data", "value", "key", "name", "info", "code", "type"]:
        add(f"STR_UPPER_{s}", sql=f"SELECT UPPER('{s}')", db=test_db, expect_contains=s.upper())
        add(f"STR_LOWER_{s}", sql=f"SELECT LOWER('{s}')", db=test_db, expect_contains=s.lower())
        add(f"STR_LENGTH_{s}", sql=f"SELECT LENGTH('{s}')", db=test_db, expect_contains=str(len(s)))
        add(f"STR_REVERSE_{s}", sql=f"SELECT REVERSE('{s}')", db=test_db, expect_contains=s[::-1])

    # More CONCAT variations
    add("STR_CONCAT_2", sql="SELECT CONCAT('a', 'b')", db=test_db, expect_contains="ab")
    add("STR_CONCAT_3", sql="SELECT CONCAT('x', 'y', 'z')", db=test_db, expect_contains="xyz")
    add("STR_CONCAT_NUM", sql="SELECT CONCAT('id:', 42)", db=test_db, expect_contains="id:42")
    add("STR_CONCAT_WS_COMMA", sql="SELECT CONCAT_WS(',', 'a', 'b', 'c')", db=test_db, expect_contains="a,b,c")
    add("STR_CONCAT_WS_PIPE", sql="SELECT CONCAT_WS('|', 'x', 'y')", db=test_db, expect_contains="x|y")

    # More SUBSTRING variations
    add("STR_SUB_FROM_END", sql="SELECT SUBSTRING('hello', -3)", db=test_db, expect_contains="llo")
    add("STR_SUB_LONG", sql="SELECT SUBSTRING('hello world', 7, 100)", db=test_db, expect_contains="world")
    add("STR_MID_2", sql="SELECT MID('abcdef', 2, 3)", db=test_db, expect_contains="bcd")

    # More REPLACE variations
    add("STR_REPLACE_MULTI", sql="SELECT REPLACE('aabbcc', 'b', 'x')", db=test_db, expect_contains="aaxxcc")
    add("STR_REPLACE_EMPTY", sql="SELECT REPLACE('hello', 'l', '')", db=test_db, expect_contains="heo")
    add("STR_REPLACE_SELF", sql="SELECT REPLACE('abc', 'abc', 'abc')", db=test_db, expect_contains="abc")

    # LPAD / RPAD variations
    add("STR_LPAD_SHORT", sql="SELECT LPAD('hi', 2, 'x')", db=test_db, expect_contains="hi")
    add("STR_LPAD_NUM", sql="SELECT LPAD('5', 3, '0')", db=test_db, expect_contains="005")
    add("STR_RPAD_NUM", sql="SELECT RPAD('5', 3, '0')", db=test_db, expect_contains="500")

    # LOCATE with position
    add("STR_LOCATE_POS", sql="SELECT LOCATE('l', 'hello', 4)", db=test_db, expect_contains="4")
    add("STR_LOCATE_MISS", sql="SELECT LOCATE('z', 'hello')", db=test_db, expect_contains="0")

    # =========================================================================
    # ADDITIONAL NUMERIC FUNCTION TESTS
    # =========================================================================

    # ABS variations
    add("NUM_ABS_0", sql="SELECT ABS(0)", db=test_db, expect_contains="0")
    add("NUM_ABS_FLOAT", sql="SELECT ABS(-3.14)", db=test_db, expect_contains="3.14")
    add("NUM_ABS_BIG", sql="SELECT ABS(-999999999)", db=test_db, expect_contains="999999999")

    # CEIL/FLOOR variations
    add("NUM_CEIL_NEG", sql="SELECT CEIL(-4.9)", db=test_db, expect_contains="-4")
    add("NUM_CEIL_INT", sql="SELECT CEIL(5)", db=test_db, expect_contains="5")
    add("NUM_FLOOR_NEG", sql="SELECT FLOOR(-4.1)", db=test_db, expect_contains="-5")
    add("NUM_FLOOR_INT", sql="SELECT FLOOR(5)", db=test_db, expect_contains="5")
    add("NUM_CEIL_ZERO", sql="SELECT CEIL(0)", db=test_db, expect_contains="0")
    add("NUM_FLOOR_ZERO", sql="SELECT FLOOR(0)", db=test_db, expect_contains="0")

    # ROUND variations
    add("NUM_ROUND_0DP", sql="SELECT ROUND(4.5)", db=test_db, expect_contains="5")
    add("NUM_ROUND_DOWN", sql="SELECT ROUND(4.4)", db=test_db, expect_contains="4")
    add("NUM_ROUND_NEG_DP", sql="SELECT ROUND(12345, -1)", db=test_db, expect_contains="12350")
    add("NUM_ROUND_NEG_DP2", sql="SELECT ROUND(12345, -3)", db=test_db, expect_contains="12000")

    # TRUNCATE variations
    add("NUM_TRUNC_0", sql="SELECT TRUNCATE(4.999, 0)", db=test_db, expect_contains="4")
    add("NUM_TRUNC_NEG", sql="SELECT TRUNCATE(-4.999, 2)", db=test_db, expect_contains="-4.99")
    add("NUM_TRUNC_3DP", sql="SELECT TRUNCATE(3.14159, 3)", db=test_db, expect_contains="3.141")

    # MOD variations
    add("NUM_MOD_Large", sql="SELECT MOD(1000000, 7)", db=test_db)
    add("NUM_MOD_SAME", sql="SELECT MOD(5, 5)", db=test_db, expect_contains="0")
    add("NUM_MOD_SMALL", sql="SELECT MOD(1, 5)", db=test_db, expect_contains="1")

    # POWER variations
    add("NUM_POW_0", sql="SELECT POWER(0, 5)", db=test_db, expect_contains="0")
    add("NUM_POW_1", sql="SELECT POWER(5, 0)", db=test_db, expect_contains="1")
    add("NUM_POW_NEG", sql="SELECT POWER(-2, 3)", db=test_db, expect_contains="-8")
    add("NUM_POW_FRAC", sql="SELECT POWER(4, 0.5)", db=test_db, expect_contains="2")

    # SQRT variations
    add("NUM_SQRT_0", sql="SELECT SQRT(0)", db=test_db, expect_contains="0")
    add("NUM_SQRT_1", sql="SELECT SQRT(1)", db=test_db, expect_contains="1")
    add("NUM_SQRT_LARGE", sql="SELECT SQRT(10000)", db=test_db, expect_contains="100")

    # Trigonometric variations
    add("NUM_SIN_PI", sql="SELECT SIN(PI())")
    add("NUM_COS_PI", sql="SELECT COS(PI())")
    add("NUM_TAN_ZERO", sql="SELECT TAN(0)", db=test_db, expect_contains="0")
    add("NUM_ASIN_1", sql="SELECT ASIN(1)")
    add("NUM_ACOS_0", sql="SELECT ACOS(0)")
    add("NUM_ATAN_1", sql="SELECT ATAN(1)")

    # LOG variations
    add("NUM_LN_1", sql="SELECT LN(1)", db=test_db, expect_contains="0")
    add("NUM_LOG2_1", sql="SELECT LOG2(1)", db=test_db, expect_contains="0")
    add("NUM_LOG10_1", sql="SELECT LOG10(1)", db=test_db, expect_contains="0")
    add("NUM_LOG_E", sql="SELECT LOG(EXP(1))")

    # SIGN variations
    add("NUM_SIGN_LARGE", sql="SELECT SIGN(999999)", db=test_db, expect_contains="1")
    add("NUM_SIGN_LARGE_NEG", sql="SELECT SIGN(-999999)", db=test_db, expect_contains="-1")

    # RAND determinism with seed
    add("NUM_RAND_SEED_1", sql="SELECT RAND(1) = RAND(1)", db=test_db, expect_contains="1")
    add("NUM_RAND_SEED_42", sql="SELECT RAND(42) = RAND(42)", db=test_db, expect_contains="1")

    # CRC32
    add("NUM_CRC32_EMPTY", sql="SELECT CRC32('')", db=test_db, expect_contains="0")
    add("NUM_CRC32_A", sql="SELECT CRC32('a')", db=test_db)

    # HEX/UNHEX/BIN/OCT
    add("NUM_HEX_0", sql="SELECT HEX(0)", db=test_db, expect_contains="0")
    add("NUM_HEX_16", sql="SELECT HEX(16)", db=test_db, expect_contains="10")
    add("NUM_UNHEX_FF", sql="SELECT UNHEX('FF')", db=test_db)
    add("NUM_BIN_0", sql="SELECT BIN(0)", db=test_db, expect_contains="0")
    add("NUM_BIN_1", sql="SELECT BIN(1)", db=test_db, expect_contains="1")
    add("NUM_OCT_0", sql="SELECT OCT(0)", db=test_db, expect_contains="0")

    # BIT_COUNT
    add("NUM_BITCOUNT_0", sql="SELECT BIT_COUNT(0)", db=test_db, expect_contains="0")
    add("NUM_BITCOUNT_FF", sql="SELECT BIT_COUNT(255)", db=test_db, expect_contains="8")
    add("NUM_BITCOUNT_LARGE", sql="SELECT BIT_COUNT(1023)", db=test_db, expect_contains="10")

    # =========================================================================
    # ADDITIONAL DATE FUNCTION TESTS
    # =========================================================================

    # Year/Month/Day extraction
    add("DATE_YEAR_2000", sql="SELECT YEAR('2000-01-01')", db=test_db, expect_contains="2000")
    add("DATE_YEAR_1999", sql="SELECT YEAR('1999-12-31')", db=test_db, expect_contains="1999")
    add("DATE_MONTH_JAN", sql="SELECT MONTH('2024-01-15')", db=test_db, expect_contains="1")
    add("DATE_MONTH_DEC", sql="SELECT MONTH('2024-12-15')", db=test_db, expect_contains="12")
    add("DATE_DAY_01", sql="SELECT DAY('2024-06-01')", db=test_db, expect_contains="1")
    add("DATE_DAY_31", sql="SELECT DAY('2024-01-31')", db=test_db, expect_contains="31")

    # Hour/Minute/Second
    add("DATE_HOUR_0", sql="SELECT HOUR('00:30:00')", db=test_db, expect_contains="0")
    add("DATE_HOUR_23", sql="SELECT HOUR('23:59:59')", db=test_db, expect_contains="23")
    add("DATE_MIN_0", sql="SELECT MINUTE('10:00:00')", db=test_db, expect_contains="0")
    add("DATE_MIN_59", sql="SELECT MINUTE('10:59:00')", db=test_db, expect_contains="59")
    add("DATE_SEC_0", sql="SELECT SECOND('10:30:00')", db=test_db, expect_contains="0")
    add("DATE_SEC_59", sql="SELECT SECOND('10:30:59')", db=test_db, expect_contains="59")

    # Quarter
    add("DATE_Q1", sql="SELECT QUARTER('2024-01-15')", db=test_db, expect_contains="1")
    add("DATE_Q2", sql="SELECT QUARTER('2024-04-15')", db=test_db, expect_contains="2")
    add("DATE_Q3", sql="SELECT QUARTER('2024-07-15')", db=test_db, expect_contains="3")
    add("DATE_Q4", sql="SELECT QUARTER('2024-10-15')", db=test_db, expect_contains="4")

    # DATEDIFF variations
    add("DATE_DIFF_SAME", sql="SELECT DATEDIFF('2024-06-15', '2024-06-15')", db=test_db, expect_contains="0")
    add("DATE_DIFF_NEG", sql="SELECT DATEDIFF('2024-01-01', '2024-06-15')", db=test_db, expect_contains="-166")
    add("DATE_DIFF_YEAR", sql="SELECT DATEDIFF('2025-01-01', '2024-01-01')", db=test_db, expect_contains="366")

    # DATE_ADD/SUB variations
    add("DATE_ADD_WEEK", sql="SELECT DATE_ADD('2024-06-15', INTERVAL 1 WEEK)", db=test_db, expect_contains="2024-06-22")
    add("DATE_ADD_MINUTE", sql="SELECT DATE_ADD('2024-06-15 10:00:00', INTERVAL 30 MINUTE)", db=test_db, expect_contains="10:30")
    add("DATE_ADD_SECOND", sql="SELECT DATE_ADD('2024-06-15 10:00:00', INTERVAL 45 SECOND)", db=test_db, expect_contains="10:00:45")
    add("DATE_SUB_WEEK", sql="SELECT DATE_SUB('2024-06-15', INTERVAL 1 WEEK)", db=test_db, expect_contains="2024-06-08")
    add("DATE_SUB_YEAR", sql="SELECT DATE_SUB('2024-06-15', INTERVAL 1 YEAR)", db=test_db, expect_contains="2023-06-15")
    add("DATE_SUB_HOUR", sql="SELECT DATE_SUB('2024-06-15 10:00:00', INTERVAL 3 HOUR)", db=test_db, expect_contains="07:00")

    # DATE_FORMAT variations
    add("DATE_FMT_YMD", sql="SELECT DATE_FORMAT('2024-06-15', '%Y-%m-%d')", db=test_db, expect_contains="2024-06-15")
    add("DATE_FMT_DM", sql="SELECT DATE_FORMAT('2024-06-15', '%d/%m/%Y')", db=test_db, expect_contains="15/06/2024")
    add("DATE_FMT_HIS", sql="SELECT DATE_FORMAT('2024-06-15 10:30:45', '%H:%i:%s')", db=test_db, expect_contains="10:30:45")
    add("DATE_FMT_FULL", sql="SELECT DATE_FORMAT('2024-06-15 10:30:45', '%Y-%m-%d %H:%i:%s')", db=test_db, expect_contains="2024-06-15 10:30:45")

    # LAST_DAY variations
    add("DATE_LD_JAN", sql="SELECT LAST_DAY('2024-01-15')", db=test_db, expect_contains="2024-01-31")
    add("DATE_LD_FEB_LEAP", sql="SELECT LAST_DAY('2024-02-15')", db=test_db, expect_contains="2024-02-29")
    add("DATE_LD_FEB_NONLEAP", sql="SELECT LAST_DAY('2023-02-15')", db=test_db, expect_contains="2023-02-28")
    add("DATE_LD_APR", sql="SELECT LAST_DAY('2024-04-10')", db=test_db, expect_contains="2024-04-30")
    add("DATE_LD_DEC", sql="SELECT LAST_DAY('2024-12-01')", db=test_db, expect_contains="2024-12-31")

    # WEEK variations
    add("DATE_WEEK_1", sql="SELECT WEEK('2024-01-01')", db=test_db)
    add("DATE_WEEK_MID", sql="SELECT WEEK('2024-06-15')", db=test_db)
    add("DATE_WEEK_END", sql="SELECT WEEK('2024-12-31')", db=test_db)

    # DAYOFWEEK (1=Sunday, 7=Saturday)
    add("DATE_DOW_SUN", sql="SELECT DAYOFWEEK('2024-06-16')", db=test_db, expect_contains="1")
    add("DATE_DOW_MON", sql="SELECT DAYOFWEEK('2024-06-17')", db=test_db, expect_contains="2")
    add("DATE_DOW_SAT", sql="SELECT DAYOFWEEK('2024-06-15')", db=test_db, expect_contains="7")

    # MONTHNAME / DAYNAME
    add("DATE_MN_JAN", sql="SELECT MONTHNAME('2024-01-01')", db=test_db, expect_contains="January")
    add("DATE_MN_JUN", sql="SELECT MONTHNAME('2024-06-01')", db=test_db, expect_contains="June")
    add("DATE_DN_MON", sql="SELECT DAYNAME('2024-06-17')", db=test_db, expect_contains="Monday")
    add("DATE_DN_FRI", sql="SELECT DAYNAME('2024-06-14')", db=test_db, expect_contains="Friday")

    # PERIOD functions
    add("DATE_PA_ADD1", sql="SELECT PERIOD_ADD(202401, 1)", db=test_db, expect_contains="202402")
    add("DATE_PA_ADD12", sql="SELECT PERIOD_ADD(202401, 12)", db=test_db, expect_contains="202501")
    add("DATE_PD_1", sql="SELECT PERIOD_DIFF(202406, 202401)", db=test_db, expect_contains="5")
    add("DATE_PD_12", sql="SELECT PERIOD_DIFF(202501, 202401)", db=test_db, expect_contains="12")

    # TIME_TO_SEC / SEC_TO_TIME
    add("DATE_TTS_1H", sql="SELECT TIME_TO_SEC('01:00:00')", db=test_db, expect_contains="3600")
    add("DATE_TTS_24H", sql="SELECT TIME_TO_SEC('24:00:00')", db=test_db, expect_contains="86400")
    add("DATE_STS_3600", sql="SELECT SEC_TO_TIME(3600)", db=test_db, expect_contains="01:00:00")
    add("DATE_STS_86400", sql="SELECT SEC_TO_TIME(86400)", db=test_db, expect_contains="24:00:00")

    # MAKEDATE / MAKETIME
    add("DATE_MD_1", sql="SELECT MAKEDATE(2024, 1)", db=test_db, expect_contains="2024-01-01")
    add("DATE_MD_366", sql="SELECT MAKEDATE(2024, 366)", db=test_db, expect_contains="2024-12-31")
    add("DATE_MT_NOON", sql="SELECT MAKETIME(12, 0, 0)", db=test_db, expect_contains="12:00:00")
    add("DATE_MT_MID", sql="SELECT MAKETIME(0, 0, 0)", db=test_db, expect_contains="00:00:00")

    # =========================================================================
    # ADDITIONAL SHOW TESTS
    # =========================================================================

    add("SHOW_VARIABLES_LOWER", sql="SHOW VARIABLES LIKE 'lower_case%'")
    add("SHOW_VARIABLES_CHAR", sql="SHOW VARIABLES LIKE 'character_set%'")
    add("SHOW_VARIABLES_COLL", sql="SHOW VARIABLES LIKE 'collation%'")
    add("SHOW_VARIABLES_INNODB", sql="SHOW VARIABLES LIKE 'innodb%'")
    add("SHOW_VARIABLES_LOG", sql="SHOW VARIABLES LIKE '%log%'")
    add("SHOW_VARIABLES_TIMEOUT", sql="SHOW VARIABLES LIKE '%timeout%'")
    add("SHOW_VARIABLES_BUFFER", sql="SHOW VARIABLES LIKE '%buffer%'")
    add("SHOW_VARIABLES_THREAD", sql="SHOW VARIABLES LIKE 'thread%'")
    add("SHOW_VARIABLES_SLOW", sql="SHOW VARIABLES LIKE 'slow%'")
    add("SHOW_VARIABLES_GENERAL", sql="SHOW VARIABLES LIKE 'general%'")
    add("SHOW_VARIABLES_BINLOG", sql="SHOW VARIABLES LIKE 'binlog%'")
    add("SHOW_VARIABLES_TMP", sql="SHOW VARIABLES LIKE '%tmp%'")
    add("SHOW_VARIABLES_SORT", sql="SHOW VARIABLES LIKE 'sort%'")
    add("SHOW_VARIABLES_JOIN", sql="SHOW VARIABLES LIKE 'join%'")
    add("SHOW_VARIABLES_READ", sql="SHOW VARIABLES LIKE 'read%'")
    add("SHOW_VARIABLES_QUERY", sql="SHOW VARIABLES LIKE 'query%'")
    add("SHOW_VARIABLES_TABLE", sql="SHOW VARIABLES LIKE 'table%'")
    add("SHOW_VARIABLES_NET", sql="SHOW VARIABLES LIKE 'net%'")
    add("SHOW_VARIABLES_CONNECT", sql="SHOW VARIABLES LIKE '%connect%'")
    add("SHOW_VARIABLES_WAIT", sql="SHOW VARIABLES LIKE 'wait%'")

    add("SHOW_STATUS_UPTIME", sql="SHOW STATUS LIKE 'Uptime'")
    add("SHOW_STATUS_THREADS", sql="SHOW STATUS LIKE 'Threads%'")
    add("SHOW_STATUS_QUERIES", sql="SHOW STATUS LIKE 'Queries'")
    add("SHOW_STATUS_CONNECTIONS", sql="SHOW STATUS LIKE 'Connections'")
    add("SHOW_STATUS_SLOW", sql="SHOW STATUS LIKE 'Slow_queries'")

    add("SHOW_COLLATION_LIKE_UTF8MB4", sql="SHOW COLLATION WHERE Charset = 'utf8mb4'")
    add("SHOW_CHARSET_LIKE_ASCII", sql="SHOW CHARACTER SET LIKE 'ascii%'")
    add("SHOW_CHARSET_LIKE_BINARY", sql="SHOW CHARACTER SET LIKE 'binary%'")

    add("SHOW_ENGINES_INNODB", sql="SHOW ENGINES")

    # =========================================================================
    # ADDITIONAL @@variable TESTS
    # =========================================================================

    add("VAR_VERSION_2", sql="SELECT @@version")
    add("VAR_VERSION_COMMENT_2", sql="SELECT @@version_comment")
    add("VAR_CHARSET_CLIENT", sql="SELECT @@character_set_client")
    add("VAR_CHARSET_CONN", sql="SELECT @@character_set_connection")
    add("VAR_CHARSET_RESULTS", sql="SELECT @@character_set_results")
    add("VAR_CHARSET_DB", sql="SELECT @@character_set_database")
    add("VAR_COLLATION_CONN", sql="SELECT @@collation_connection")
    add("VAR_COLLATION_DB", sql="SELECT @@collation_database")
    add("VAR_FOREIGN_KEY", sql="SELECT @@foreign_key_checks")
    add("VAR_UNIQUE_CHECKS", sql="SELECT @@unique_checks")
    add("VAR_SQL_AUTO_IS_NULL", sql="SELECT @@sql_auto_is_null")
    add("VAR_QUERY_CACHE_TYPE", sql="SELECT @@query_cache_type")
    add("VAR_NET_BUFFER_LENGTH", sql="SELECT @@net_buffer_length")
    add("VAR_AUTO_INC_INCREMENT", sql="SELECT @@auto_increment_increment")
    add("VAR_AUTO_INC_OFFSET", sql="SELECT @@auto_increment_offset")
    add("VAR_BACK_LOG", sql="SELECT @@back_log")
    add("VAR_BIG_SELECTS", sql="SELECT @@sql_big_selects")
    add("VAR_COMPLETION_TYPE", sql="SELECT @@completion_type")
    add("VAR_CONCURRENT_INSERT", sql="SELECT @@concurrent_insert")
    add("VAR_CONNECT_TIMEOUT", sql="SELECT @@connect_timeout")
    add("VAR_DEFAULT_WEEK", sql="SELECT @@default_week_format")
    add("VAR_DELAY_KEY_WRITE", sql="SELECT @@delay_key_write")
    add("VAR_DIV_PREC_INC", sql="SELECT @@div_precision_increment")
    add("VAR_END_MARKERS", sql="SELECT @@end_markers_in_json")
    add("VAR_EXP_PREALLOC", sql="SELECT @@explicit_defaults_for_timestamp")
    add("VAR_FLUSH_TIME", sql="SELECT @@flush_time")
    add("VAR_GROUP_CONCAT_LEN", sql="SELECT @@group_concat_max_len")
    add("VAR_INNODB_IO_CAP", sql="SELECT @@innodb_io_capacity")
    add("VAR_INNODB_LOCK_WAIT", sql="SELECT @@innodb_lock_wait_timeout")
    add("VAR_INNODB_PAGE_SIZE", sql="SELECT @@innodb_page_size")
    add("VAR_INSERT_ID", sql="SELECT @@insert_id")
    add("VAR_KEEP_FILES", sql="SELECT @@keep_files_on_create")
    add("VAR_MAX_SORT", sql="SELECT @@max_sort_length")
    add("VAR_NET_RETRY", sql="SELECT @@net_retry_count")
    add("VAR_OLAP", sql="SELECT @@optimizer_switch")
    add("VAR_RANGE_OPT", sql="SELECT @@range_optimizer_max_mem_size")

    # =========================================================================
    # ADDITIONAL INFORMATION_SCHEMA TESTS
    # =========================================================================

    add("INFO_TABLES_COUNT", sql="SELECT COUNT(*) FROM INFORMATION_SCHEMA.TABLES")
    add("INFO_COLUMNS_COUNT", sql="SELECT COUNT(*) FROM INFORMATION_SCHEMA.COLUMNS")
    add("INFO_SCHEMATA_COUNT", sql="SELECT COUNT(*) FROM INFORMATION_SCHEMA.SCHEMATA")
    add("INFO_TABLES_WHERE_TYPE", sql="SELECT COUNT(*) FROM INFORMATION_SCHEMA.TABLES WHERE TABLE_TYPE = 'BASE TABLE'")
    add("INFO_TABLES_WHERE_VIEW", sql="SELECT COUNT(*) FROM INFORMATION_SCHEMA.TABLES WHERE TABLE_TYPE = 'VIEW'")
    add("INFO_COLUMNS_WHERE_INT", sql="SELECT COUNT(*) FROM INFORMATION_SCHEMA.COLUMNS WHERE DATA_TYPE = 'int'")
    add("INFO_COLUMNS_WHERE_VARCHAR", sql="SELECT COUNT(*) FROM INFORMATION_SCHEMA.COLUMNS WHERE DATA_TYPE = 'varchar'")
    add("INFO_COLUMNS_WHERE_TEXT", sql="SELECT COUNT(*) FROM INFORMATION_SCHEMA.COLUMNS WHERE DATA_TYPE = 'text'")
    add("INFO_COLUMNS_WHERE_DECIMAL", sql="SELECT COUNT(*) FROM INFORMATION_SCHEMA.COLUMNS WHERE DATA_TYPE = 'decimal'")
    add("INFO_COLUMNS_WHERE_DATE", sql="SELECT COUNT(*) FROM INFORMATION_SCHEMA.COLUMNS WHERE DATA_TYPE = 'date'")
    add("INFO_COLUMNS_WHERE_DATETIME", sql="SELECT COUNT(*) FROM INFORMATION_SCHEMA.COLUMNS WHERE DATA_TYPE = 'datetime'")
    add("INFO_COLUMNS_WHERE_BOOL", sql="SELECT COUNT(*) FROM INFORMATION_SCHEMA.COLUMNS WHERE DATA_TYPE = 'tinyint'")
    add("INFO_COLUMNS_WHERE_DOUBLE", sql="SELECT COUNT(*) FROM INFORMATION_SCHEMA.COLUMNS WHERE DATA_TYPE = 'double'")
    add("INFO_COLUMNS_WHERE_FLOAT", sql="SELECT COUNT(*) FROM INFORMATION_SCHEMA.COLUMNS WHERE DATA_TYPE = 'float'")
    add("INFO_COLUMNS_WHERE_BIGINT", sql="SELECT COUNT(*) FROM INFORMATION_SCHEMA.COLUMNS WHERE DATA_TYPE = 'bigint'")
    add("INFO_COLUMNS_WHERE_BLOB", sql="SELECT COUNT(*) FROM INFORMATION_SCHEMA.COLUMNS WHERE DATA_TYPE = 'blob'")
    add("INFO_TABLES_ORDER", sql="SELECT TABLE_NAME FROM INFORMATION_SCHEMA.TABLES ORDER BY TABLE_NAME LIMIT 5")
    add("INFO_COLUMNS_ORDER", sql="SELECT COLUMN_NAME FROM INFORMATION_SCHEMA.COLUMNS ORDER BY COLUMN_NAME LIMIT 5")
    add("INFO_SCHEMATA_ORDER", sql="SELECT SCHEMA_NAME FROM INFORMATION_SCHEMA.SCHEMATA ORDER BY SCHEMA_NAME")
    add("INFO_TABLES_DISTINCT_SCHEMA", sql="SELECT DISTINCT TABLE_SCHEMA FROM INFORMATION_SCHEMA.TABLES")
    add("INFO_COLUMNS_DISTINCT_TABLE", sql="SELECT DISTINCT TABLE_NAME FROM INFORMATION_SCHEMA.COLUMNS WHERE TABLE_SCHEMA = '{test_db}'")
    add("INFO_STATS_COUNT", sql="SELECT COUNT(*) FROM INFORMATION_SCHEMA.STATISTICS")
    add("INFO_KEY_COLUMN_COUNT", sql="SELECT COUNT(*) FROM INFORMATION_SCHEMA.KEY_COLUMN_USAGE")
    add("INFO_ENGINES_COUNT", sql="SELECT COUNT(*) FROM INFORMATION_SCHEMA.ENGINES")
    add("INFO_CHARSETS_COUNT", sql="SELECT COUNT(*) FROM INFORMATION_SCHEMA.CHARACTER_SETS")
    add("INFO_COLLATIONS_COUNT", sql="SELECT COUNT(*) FROM INFORMATION_SCHEMA.COLLATIONS")
    add("INFO_TABLE_CONSTRAINTS_COUNT", sql="SELECT COUNT(*) FROM INFORMATION_SCHEMA.TABLE_CONSTRAINTS")
    add("INFO_PROCESSLIST_COUNT", sql="SELECT COUNT(*) FROM INFORMATION_SCHEMA.PROCESSLIST")

    # =========================================================================
    # ADDITIONAL TRANSACTION TESTS
    # =========================================================================

    add("CREATE_TX2_TABLE", sql="CREATE TABLE IF NOT EXISTS t_tx2 (id INT, val VARCHAR(50))", db=test_db)

    # Multiple inserts in transaction
    add("TX2_BEGIN", sql="BEGIN", db=test_db)
    for j in range(10):
        add(f"TX2_INS_{j}", sql=f"INSERT INTO t_tx2 VALUES ({j}, 'val_{j}')", db=test_db)
    add("TX2_COMMIT", sql="COMMIT", db=test_db)
    add("TX2_VERIFY", sql="SELECT COUNT(*) FROM t_tx2", db=test_db, expect_contains="10")

    # Update in transaction then rollback
    add("TX2_BEGIN2", sql="BEGIN", db=test_db)
    add("TX2_UPDATE", sql="UPDATE t_tx2 SET val = 'modified' WHERE id = 0", db=test_db)
    add("TX2_ROLLBACK", sql="ROLLBACK", db=test_db)

    # Delete in transaction then rollback
    add("TX2_BEGIN3", sql="BEGIN", db=test_db)
    add("TX2_DELETE", sql="DELETE FROM t_tx2 WHERE id = 9", db=test_db)
    add("TX2_ROLLBACK2", sql="ROLLBACK", db=test_db)
    add("TX2_VERIFY2", sql="SELECT COUNT(*) FROM t_tx2", db=test_db, expect_contains="10")

    # Transaction with SET
    add("TX2_SET_VAR", sql="SET @tx_var = 1", db=test_db)
    add("TX2_READ_VAR", sql="SELECT @tx_var", db=test_db, expect_contains="1")

    # Consecutive transactions
    for j in range(3):
        add(f"TX2_CONSEC_BEGIN_{j}", sql="BEGIN", db=test_db)
        add(f"TX2_CONSEC_INS_{j}", sql=f"INSERT INTO t_tx2 VALUES ({10+j}, 'batch_{j}')", db=test_db)
        add(f"TX2_CONSEC_COMMIT_{j}", sql="COMMIT", db=test_db)

    add("TX2_VERIFY3", sql="SELECT COUNT(*) FROM t_tx2", db=test_db, expect_contains="13")

    # Read-only style operations in transaction
    add("TX2_BEGIN_RO", sql="BEGIN", db=test_db)
    add("TX2_SELECT", sql="SELECT val FROM t_tx2 WHERE id = 0", db=test_db)
    add("TX2_COMMIT_RO", sql="COMMIT", db=test_db)

    add("DROP_TX2_TABLE", sql="DROP TABLE IF EXISTS t_tx2", db=test_db)

    # =========================================================================
    # ADDITIONAL JOIN TESTS
    # =========================================================================

    # Setup additional tables for joins
    add("CREATE_JOIN_CUST", sql="CREATE TABLE IF NOT EXISTS t_cust (id INT, name VARCHAR(50), region VARCHAR(50))", db=test_db)
    add("CREATE_JOIN_ORD", sql="CREATE TABLE IF NOT EXISTS t_ord2 (id INT, cust_id INT, amount DECIMAL(10,2), product VARCHAR(50))", db=test_db)
    add("CREATE_JOIN_ITEM", sql="CREATE TABLE IF NOT EXISTS t_item (id INT, ord_id INT, item_name VARCHAR(50), qty INT)", db=test_db)

    for j, (name, region) in enumerate([("Cust1","East"),("Cust2","West"),("Cust3","East"),("Cust4","North"),("Cust5","South")]):
        add(f"INSERT_CUST_{j}", sql=f"INSERT INTO t_cust VALUES ({j+1}, '{name}', '{region}')", db=test_db)

    orders = [(1,1,100,"Widget"),(2,1,200,"Gadget"),(3,2,150,"Widget"),(4,3,300,"Gadget"),(5,4,250,"Widget")]
    for j, (oid, cid, amt, prod) in enumerate(orders):
        add(f"INSERT_ORD2_{j}", sql=f"INSERT INTO t_ord2 VALUES ({oid}, {cid}, {amt}, '{prod}')", db=test_db)

    items = [(1,1,"Widget-A",2),(2,1,"Widget-B",1),(3,2,"Gadget-X",3),(4,3,"Widget-A",1),(5,4,"Gadget-Y",2)]
    for j, (iid, oid, iname, qty) in enumerate(items):
        add(f"INSERT_ITEM_{j}", sql=f"INSERT INTO t_item VALUES ({iid}, {oid}, '{iname}', {qty})", db=test_db)

    # 3-table join
    add("JOIN_3TABLE", sql="SELECT c.name, o.amount, i.item_name FROM t_cust c INNER JOIN t_ord2 o ON c.id = o.cust_id INNER JOIN t_item i ON o.id = i.ord_id", db=test_db)
    add("JOIN_3TABLE_COUNT", sql="SELECT COUNT(*) FROM t_cust c INNER JOIN t_ord2 o ON c.id = o.cust_id INNER JOIN t_item i ON o.id = i.ord_id", db=test_db)

    # Left join with null check
    add("JOIN_LEFT_NULL_ORD", sql="SELECT c.name FROM t_cust c LEFT JOIN t_ord2 o ON c.id = o.cust_id WHERE o.id IS NULL", db=test_db)
    add("JOIN_LEFT_NULL_ITEM", sql="SELECT o.id FROM t_ord2 o LEFT JOIN t_item i ON o.id = i.ord_id WHERE i.id IS NULL", db=test_db)

    # Join with GROUP BY and aggregate
    add("JOIN_GRP_SUM", sql="SELECT c.name, SUM(o.amount) FROM t_cust c INNER JOIN t_ord2 o ON c.id = o.cust_id GROUP BY c.name", db=test_db)
    add("JOIN_GRP_COUNT", sql="SELECT c.region, COUNT(o.id) FROM t_cust c LEFT JOIN t_ord2 o ON c.id = o.cust_id GROUP BY c.region", db=test_db)
    add("JOIN_GRP_AVG", sql="SELECT c.name, AVG(o.amount) FROM t_cust c INNER JOIN t_ord2 o ON c.id = o.cust_id GROUP BY c.name", db=test_db)

    # Join with ORDER BY
    add("JOIN_ORD_NAME", sql="SELECT c.name, o.amount FROM t_cust c INNER JOIN t_ord2 o ON c.id = o.cust_id ORDER BY c.name", db=test_db)
    add("JOIN_ORD_AMT_DESC", sql="SELECT c.name, o.amount FROM t_cust c INNER JOIN t_ord2 o ON c.id = o.cust_id ORDER BY o.amount DESC", db=test_db)
    add("JOIN_ORD_LIMIT", sql="SELECT c.name, o.amount FROM t_cust c INNER JOIN t_ord2 o ON c.id = o.cust_id ORDER BY o.amount DESC LIMIT 3", db=test_db)

    # Join with WHERE
    add("JOIN_WHERE_PROD", sql="SELECT c.name, o.amount FROM t_cust c INNER JOIN t_ord2 o ON c.id = o.cust_id WHERE o.product = 'Widget'", db=test_db)
    add("JOIN_WHERE_REGION", sql="SELECT c.name, o.amount FROM t_cust c INNER JOIN t_ord2 o ON c.id = o.cust_id WHERE c.region = 'East'", db=test_db)
    add("JOIN_WHERE_AMT_GT", sql="SELECT c.name, o.amount FROM t_cust c INNER JOIN t_ord2 o ON c.id = o.cust_id WHERE o.amount > 150", db=test_db)

    # Self join on customers
    add("JOIN_SELF_REGION", sql="SELECT a.name, b.name FROM t_cust a JOIN t_cust b ON a.region = b.region AND a.id < b.id", db=test_db)
    add("JOIN_SELF_REGION_COUNT", sql="SELECT COUNT(*) FROM t_cust a JOIN t_cust b ON a.region = b.region AND a.id < b.id", db=test_db)

    # Join with subquery
    add("JOIN_SUBQ", sql="SELECT c.name FROM t_cust c WHERE c.id IN (SELECT cust_id FROM t_ord2 WHERE amount > 200)", db=test_db)

    # Join with HAVING
    add("JOIN_HAVING", sql="SELECT c.name, SUM(o.amount) AS total FROM t_cust c INNER JOIN t_ord2 o ON c.id = o.cust_id GROUP BY c.name HAVING total > 200", db=test_db)

    # Cross join with condition
    add("JOIN_CROSS_WHERE2", sql="SELECT c.name, o.amount FROM t_cust c CROSS JOIN t_ord2 o WHERE c.id = o.cust_id AND o.amount > 100", db=test_db)

    # Right join
    add("JOIN_RIGHT_ITEM", sql="SELECT o.id, i.item_name FROM t_ord2 o RIGHT JOIN t_item i ON o.id = i.ord_id", db=test_db)

    # Join with CASE
    add("JOIN_CASE", sql="SELECT c.name, CASE WHEN o.amount > 200 THEN 'high' ELSE 'low' END FROM t_cust c INNER JOIN t_ord2 o ON c.id = o.cust_id", db=test_db)

    # Join with DISTINCT
    add("JOIN_DISTINCT", sql="SELECT DISTINCT c.region FROM t_cust c INNER JOIN t_ord2 o ON c.id = o.cust_id", db=test_db)

    # Join with COALESCE
    add("JOIN_COALESCE", sql="SELECT c.name, COALESCE(o.amount, 0) FROM t_cust c LEFT JOIN t_ord2 o ON c.id = o.cust_id", db=test_db)

    # Join with multiple conditions
    add("JOIN_MULTI_COND", sql="SELECT c.name FROM t_cust c INNER JOIN t_ord2 o ON c.id = o.cust_id WHERE c.region IN ('East','West') AND o.amount >= 100", db=test_db)

    # Cleanup join tables
    add("DROP_JOIN_ITEM", sql="DROP TABLE IF EXISTS t_item", db=test_db)
    add("DROP_JOIN_ORD2", sql="DROP TABLE IF EXISTS t_ord2", db=test_db)
    add("DROP_JOIN_CUST", sql="DROP TABLE IF EXISTS t_cust", db=test_db)

    # =========================================================================
    # ADDITIONAL EDGE CASE TESTS
    # =========================================================================

    # Nested function calls
    add("EDGE_NESTED_FUNC", sql="SELECT UPPER(SUBSTRING('hello world', 1, 5))", db=test_db, expect_contains="HELLO")
    add("EDGE_NESTED_2", sql="SELECT LENGTH(UPPER('hello'))", db=test_db, expect_contains="5")
    add("EDGE_NESTED_3", sql="SELECT ABS(ROUND(-3.7))", db=test_db, expect_contains="4")
    add("EDGE_NESTED_4", sql="SELECT CONCAT(UPPER('a'), LOWER('B'))", db=test_db, expect_contains="Ab")
    add("EDGE_NESTED_5", sql="SELECT ROUND(SQRT(2), 2)", db=test_db, expect_contains="1.41")

    # Complex CASE expressions
    add("EDGE_CASE_3WAY", sql="SELECT CASE WHEN 1=1 THEN 'a' WHEN 2=2 THEN 'b' ELSE 'c' END", db=test_db, expect_contains="a")
    add("EDGE_CASE_ELSE", sql="SELECT CASE WHEN 1=0 THEN 'a' WHEN 2=0 THEN 'b' ELSE 'c' END", db=test_db, expect_contains="c")
    add("EDGE_CASE_MATH", sql="SELECT CASE WHEN 10 > 5 THEN 10 - 5 ELSE 5 - 10 END", db=test_db, expect_contains="5")

    # Multiple UNION
    add("EDGE_UNION_3", sql="SELECT 1 AS x UNION SELECT 2 UNION SELECT 3", db=test_db)
    add("EDGE_UNION_5", sql="SELECT 'a' UNION SELECT 'b' UNION SELECT 'c' UNION SELECT 'd' UNION SELECT 'e'", db=test_db)
    add("EDGE_UNION_ALL_MULTI", sql="SELECT 1 UNION ALL SELECT 2 UNION ALL SELECT 3 UNION ALL SELECT 4", db=test_db)

    # Type coercion
    add("EDGE_INT_STR", sql="SELECT 1 + '2'", db=test_db, expect_contains="3")
    add("EDGE_STR_INT", sql="SELECT CONCAT(1, 2, 3)", db=test_db, expect_contains="123")
    add("EDGE_BOOL_INT", sql="SELECT TRUE + TRUE", db=test_db, expect_contains="2")
    add("EDGE_NULL_MATH", sql="SELECT 1 + NULL", db=test_db)
    add("EDGE_NULL_STR", sql="SELECT CONCAT('a', NULL)", db=test_db)

    # String to number conversion
    add("EDGE_CAST_INT", sql="SELECT CAST('42' AS INT)", db=test_db, expect_contains="42")
    add("EDGE_CAST_DOUBLE", sql="SELECT CAST('3.14' AS DOUBLE)", db=test_db, expect_contains="3.14")
    add("EDGE_CAST_CHAR", sql="SELECT CAST(42 AS CHAR)", db=test_db, expect_contains="42")

    # Nested subqueries
    add("EDGE_NESTED_SUB", sql="SELECT * FROM (SELECT * FROM (SELECT 1 AS x) a) b", db=test_db, expect_contains="1")
    add("EDGE_DEEP_SUB", sql="SELECT x FROM (SELECT x FROM (SELECT x FROM (SELECT 42 AS x) a) b) c", db=test_db, expect_contains="42")

    # Empty string operations
    add("EDGE_EMPTY_CONCAT", sql="SELECT CONCAT('', '', '')", db=test_db)
    add("EDGE_EMPTY_UPPER", sql="SELECT UPPER('')", db=test_db)
    add("EDGE_EMPTY_REPLACE", sql="SELECT REPLACE('', 'a', 'b')", db=test_db)
    add("EDGE_EMPTY_SUBSTRING", sql="SELECT SUBSTRING('', 1, 1)", db=test_db)

    # Very long strings
    add("EDGE_LONG_STR", sql="SELECT LENGTH(REPEAT('x', 1000))", db=test_db, expect_contains="1000")
    add("EDGE_LONG_CONCAT", sql="SELECT LENGTH(CONCAT(REPEAT('a', 500), REPEAT('b', 500)))", db=test_db, expect_contains="1000")

    # Math edge cases
    add("EDGE_NEG_ZERO", sql="SELECT -0", db=test_db, expect_contains="0")
    add("EDGE_DOUBLE_NEG", sql="SELECT --5", db=test_db, expect_contains="5")
    add("EDGE_MOD_LARGE", sql="SELECT 1000000 MOD 97", db=test_db)
    add("EDGE_POWER_0_0", sql="SELECT POWER(0, 0)", db=test_db, expect_contains="1")
    add("EDGE_MULT_NEG", sql="SELECT (-1) * (-1) * (-1)", db=test_db, expect_contains="-1")
    add("EDGE_CHAIN_ADD", sql="SELECT 1+1+1+1+1+1+1+1+1+1", db=test_db, expect_contains="10")

    # BETWEEN variations
    add("EDGE_BETWEEN_STR", sql="SELECT 'c' BETWEEN 'a' AND 'd'", db=test_db, expect_contains="1")
    add("EDGE_NOT_BETWEEN", sql="SELECT 5 NOT BETWEEN 1 AND 3", db=test_db, expect_contains="1")

    # IN with various types
    add("EDGE_IN_NUM", sql="SELECT 3 IN (1, 2, 3, 4, 5)", db=test_db, expect_contains="1")
    add("EDGE_IN_STR", sql="SELECT 'c' IN ('a', 'b', 'c')", db=test_db, expect_contains="1")
    add("EDGE_NOT_IN", sql="SELECT 6 NOT IN (1, 2, 3)", db=test_db, expect_contains="1")
    add("EDGE_IN_NULL", sql="SELECT NULL IN (1, 2, 3)", db=test_db)

    # LIKE variations
    add("EDGE_LIKE_START", sql="SELECT 'hello' LIKE 'hel%'", db=test_db, expect_contains="1")
    add("EDGE_LIKE_END", sql="SELECT 'hello' LIKE '%llo'", db=test_db, expect_contains="1")
    add("EDGE_LIKE_MID", sql="SELECT 'hello' LIKE '%ell%'", db=test_db, expect_contains="1")
    add("EDGE_LIKE_SINGLE", sql="SELECT 'cat' LIKE 'c_t'", db=test_db, expect_contains="1")
    add("EDGE_LIKE_ALL", sql="SELECT 'anything' LIKE '%'", db=test_db, expect_contains="1")
    add("EDGE_NOT_LIKE", sql="SELECT 'hello' NOT LIKE 'world%'", db=test_db, expect_contains="1")

    # IS NULL / IS NOT NULL
    add("EDGE_ISNULL_EXPR", sql="SELECT NULL IS NULL", db=test_db, expect_contains="1")
    add("EDGE_ISNOTNULL_EXPR", sql="SELECT 1 IS NOT NULL", db=test_db, expect_contains="1")
    add("EDGE_ISNULL_ZERO", sql="SELECT 0 IS NULL", db=test_db, expect_contains="0")
    add("EDGE_ISNULL_EMPTY", sql="SELECT '' IS NULL", db=test_db, expect_contains="0")

    # COALESCE / IFNULL variations
    add("EDGE_COALESCE_3", sql="SELECT COALESCE(NULL, NULL, 'third')", db=test_db, expect_contains="third")
    add("EDGE_COALESCE_FIRST", sql="SELECT COALESCE('first', 'second')", db=test_db, expect_contains="first")
    add("EDGE_COALESCE_NUM", sql="SELECT COALESCE(NULL, 42)", db=test_db, expect_contains="42")
    add("EDGE_IFNULL_NULL", sql="SELECT IFNULL(NULL, 0)", db=test_db, expect_contains="0")
    add("EDGE_IFNULL_NOTNULL", sql="SELECT IFNULL(5, 0)", db=test_db, expect_contains="5")

    # NULLIF variations
    add("EDGE_NULLIF_SAME", sql="SELECT NULLIF(1, 1)", db=test_db)
    add("EDGE_NULLIF_DIFF", sql="SELECT NULLIF(1, 2)", db=test_db, expect_contains="1")
    add("EDGE_NULLIF_STR", sql="SELECT NULLIF('a', 'a')", db=test_db)
    add("EDGE_NULLIF_STR2", sql="SELECT NULLIF('a', 'b')", db=test_db, expect_contains="a")

    # Complex boolean
    add("EDGE_BOOL_COMPLEX", sql="SELECT (1 AND 1) OR (0 AND 1)", db=test_db, expect_contains="1")
    add("EDGE_BOOL_DEMORGAN", sql="SELECT NOT (1 AND 0) = (NOT 1 OR NOT 0)", db=test_db, expect_contains="1")
    add("EDGE_BOOL_DOUBLE_NEG", sql="SELECT NOT NOT 1", db=test_db, expect_contains="1")

    # Chained comparisons
    add("EDGE_CHAIN_GT", sql="SELECT 5 > 3 AND 3 > 1", db=test_db, expect_contains="1")
    add("EDGE_CHAIN_LT", sql="SELECT 1 < 3 AND 3 < 5", db=test_db, expect_contains="1")
    add("EDGE_RANGE_CHECK", sql="SELECT 5 BETWEEN 1 AND 10", db=test_db, expect_contains="1")

    # Miscellaneous function tests
    add("EDGE_MD5", sql="SELECT MD5('hello')", db=test_db)
    add("EDGE_SHA1", sql="SELECT SHA1('hello')", db=test_db)
    add("EDGE_SHA2_256", sql="SELECT SHA2('hello', 256)", db=test_db)
    add("EDGE_SHA2_512", sql="SELECT SHA2('hello', 512)", db=test_db)

    # GREATEST / LEAST
    add("EDGE_GREATEST", sql="SELECT GREATEST(1, 5, 3)", db=test_db, expect_contains="5")
    add("EDGE_LEAST", sql="SELECT LEAST(1, 5, 3)", db=test_db, expect_contains="1")
    add("EDGE_GREATEST_STR", sql="SELECT GREATEST('a', 'c', 'b')", db=test_db, expect_contains="c")
    add("EDGE_LEAST_STR", sql="SELECT LEAST('a', 'c', 'b')", db=test_db, expect_contains="a")
    add("EDGE_GREATEST_NULL", sql="SELECT GREATEST(1, NULL, 3)", db=test_db)

    # INTERVAL
    add("EDGE_INTERVAL", sql="SELECT 5 INTERVAL 1 DAY", db=test_db)

    # VALUES function
    add("EDGE_VALUES_FUNC", sql="SELECT 1 FROM DUAL", db=test_db)

    # Multiple columns in SELECT
    add("EDGE_10COLS", sql="SELECT 1,2,3,4,5,6,7,8,9,10", db=test_db)
    add("EDGE_20COLS", sql="SELECT 1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20", db=test_db)

    # Cleanup additional tables
    add("DROP_SHOW_TABLE", sql="DROP TABLE IF EXISTS t_show", db=test_db)
    add("DROP_INFO_TABLE", sql="DROP TABLE IF EXISTS t_info", db=test_db)

    # Cleanup
    add("DROP_TEST_DB", sql=f"DROP DATABASE IF EXISTS {test_db}")

    return results

def main():
    print("Starting RorisDB ADB MySQL Protocol Tests...")
    print(f"Target: {HOST}:{PORT}")

    # Quick connectivity check
    ok, out = run_sql("SELECT 1")
    if not ok:
        print(f"FATAL: Cannot connect to server: {out}")
        sys.exit(1)

    tests = run_tests()
    total = len(tests)
    passed = 0
    failed = 0
    failures = []

    print(f"Running {total} tests...")

    for i, (name, kwargs) in enumerate(tests):
        success, msg = test(name, **kwargs)
        if success:
            passed += 1
        else:
            failed += 1
            if len(failures) < FAILURES_LIMIT:
                failures.append({"test": name, "sql": kwargs.get("sql", ""), "error": msg[:300]})

        if (i + 1) % 200 == 0:
            print(f"  Progress: {i+1}/{total} (passed={passed}, failed={failed})")

    result = {
        "protocol": "adbmysql",
        "total": total,
        "passed": passed,
        "failed": failed,
        "failures": failures
    }

    print(f"\n{'='*60}")
    print(json.dumps(result, indent=2))
    print(f"{'='*60}")

    return 0 if failed == 0 else 1

if __name__ == "__main__":
    sys.exit(main())
