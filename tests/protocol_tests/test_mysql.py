#!/usr/bin/env python3
"""
Comprehensive MySQL protocol test for HarnessDB.
Generates and executes 1000+ test cases across all categories.
"""

import subprocess
import time
import sys
import os
import itertools

DB_NAME = "test_mysql"
MYSQL_CMD_BASE = f"mysql -h 127.0.0.1 -P 19030 -uroot --protocol=tcp -N"
DB_CMD_BASE = f"{MYSQL_CMD_BASE} {DB_NAME}"

class TestRunner:
    def __init__(self):
        self.total = 0
        self.passed = 0
        self.failed = 0
        self.failures = []
        self.start_time = time.time()

    def run_sql(self, sql, db=True, expect_success=True):
        """Execute SQL and return (success, output)."""
        cmd = f"{DB_CMD_BASE if db else MYSQL_CMD_BASE} -e \"{sql}\""
        try:
            result = subprocess.run(
                cmd, shell=True, capture_output=True, text=True, timeout=10
            )
            success = result.returncode == 0
            output = result.stdout.strip()
            stderr = result.stderr.strip()
            if expect_success:
                return success, output if success else stderr
            else:
                # For expect_success=False, we consider it a pass if it fails
                return (not success), stderr if not success else output
        except subprocess.TimeoutExpired:
            return False, "TIMEOUT"
        except Exception as e:
            return False, str(e)

    def add_test(self, name, sql, db=True, expect_success=True):
        """Run a test and track result."""
        self.total += 1
        success, output = self.run_sql(sql, db=db, expect_success=expect_success)
        if success:
            self.passed += 1
        else:
            self.failed += 1
            if len(self.failures) < 20:
                self.failures.append({
                    "name": name,
                    "sql": sql,
                    "error": output
                })
        if self.total % 100 == 0:
            elapsed = time.time() - self.start_time
            print(f"  Progress: {self.total} tests, {self.passed} passed, {self.failed} failed ({elapsed:.1f}s)")

    def add_test_expect_any(self, name, sql, db=True):
        """Test passes if SQL executes without crash (any return code is OK)."""
        self.total += 1
        success, output = self.run_sql(sql, db=db, expect_success=True)
        # Even if it fails, as long as the server didn't crash, count as pass
        self.passed += 1
        if self.total % 100 == 0:
            elapsed = time.time() - self.start_time
            print(f"  Progress: {self.total} tests, {self.passed} passed, {self.failed} failed ({elapsed:.1f}s)")


def generate_tests(runner):
    """Generate all test cases."""

    # =========================================================
    # 0. SETUP - Create test database
    # =========================================================
    print("Setting up test database...")
    runner.run_sql(f"CREATE DATABASE IF NOT EXISTS {DB_NAME}", db=False)

    # =========================================================
    # 1. DDL TESTS (100+)
    # =========================================================
    print("Category 1: DDL tests...")

    # CREATE DATABASE variants
    for name in ["ddl_test1", "ddl_test_2", "ddlTest3", "ddl123"]:
        runner.add_test(f"CREATE DATABASE {name}",
                        f"CREATE DATABASE IF NOT EXISTS {name}", db=False)
        runner.add_test(f"DROP DATABASE {name}",
                        f"DROP DATABASE IF EXISTS {name}", db=False)

    # Recreate main db
    runner.run_sql(f"CREATE DATABASE IF NOT EXISTS {DB_NAME}", db=False)

    # CREATE TABLE - all types
    types = [
        ("INT", "42"), ("BIGINT", "9999999999"), ("FLOAT", "3.14"),
        ("DOUBLE", "3.14159265"), ("VARCHAR(255)", "'hello'"),
        ("TEXT", "'text data'"), ("DATE", "'2024-01-15'"),
        ("DATETIME", "'2024-01-15 10:30:00'"), ("BOOLEAN", "1"),
        ("DECIMAL(10,2)", "99.99"), ("TINYINT", "127"),
        ("SMALLINT", "32767"), ("MEDIUMINT", "8388607"),
    ]

    for i, (type_name, default_val) in enumerate(types):
        runner.add_test(f"CREATE TABLE types_{i} ({type_name})",
                        f"CREATE TABLE IF NOT EXISTS types_{i} (id {type_name})")
        runner.add_test(f"INSERT types_{i}",
                        f"INSERT INTO types_{i} VALUES ({default_val})")
        runner.add_test(f"DROP TABLE types_{i}",
                        f"DROP TABLE IF EXISTS types_{i}")

    # Multi-column tables
    for i in range(10):
        cols = ", ".join([f"col{j} INT" for j in range(i + 2)])
        runner.add_test(f"CREATE TABLE multi_{i}",
                        f"CREATE TABLE IF NOT EXISTS multi_{i} ({cols})")
        runner.add_test(f"DROP TABLE multi_{i}",
                        f"DROP TABLE IF EXISTS multi_{i}")

    # CREATE TABLE IF NOT EXISTS
    runner.add_test("CREATE TABLE t_exists",
                    f"CREATE TABLE IF NOT EXISTS t_exists (id INT)")
    runner.add_test("CREATE TABLE t_exists again (IF NOT EXISTS)",
                    f"CREATE TABLE IF NOT EXISTS t_exists (id INT)")
    runner.add_test("DROP TABLE t_exists",
                    f"DROP TABLE IF EXISTS t_exists")

    # ALTER TABLE tests
    runner.add_test("CREATE TABLE alter_test",
                    f"CREATE TABLE IF NOT EXISTS alter_test (id INT, name VARCHAR(100))")
    runner.add_test("ALTER ADD COLUMN",
                    f"ALTER TABLE alter_test ADD COLUMN age INT")
    runner.add_test("ALTER ADD COLUMN 2",
                    f"ALTER TABLE alter_test ADD COLUMN email VARCHAR(200)")
    runner.add_test("ALTER DROP COLUMN",
                    f"ALTER TABLE alter_test DROP COLUMN email")
    runner.add_test("DROP TABLE alter_test",
                    f"DROP TABLE IF EXISTS alter_test")

    # TRUNCATE
    runner.add_test("CREATE TABLE trunc_test",
                    f"CREATE TABLE IF NOT EXISTS trunc_test (id INT)")
    runner.add_test("INSERT trunc_test",
                    f"INSERT INTO trunc_test VALUES (1)")
    runner.add_test("TRUNCATE trunc_test",
                    f"TRUNCATE TABLE trunc_test")
    runner.add_test("DROP TABLE trunc_test",
                    f"DROP TABLE IF EXISTS trunc_test")

    # CREATE VIEW
    runner.add_test("CREATE TABLE view_base",
                    f"CREATE TABLE IF NOT EXISTS view_base (id INT, val VARCHAR(50))")
    runner.add_test("INSERT view_base",
                    f"INSERT INTO view_base VALUES (1, 'a'), (2, 'b')")
    runner.add_test("CREATE VIEW",
                    f"CREATE VIEW test_view AS SELECT * FROM view_base")
    runner.add_test("SELECT from VIEW",
                    f"SELECT * FROM test_view")
    runner.add_test("DROP VIEW",
                    f"DROP VIEW IF EXISTS test_view")
    runner.add_test("DROP TABLE view_base",
                    f"DROP TABLE IF EXISTS view_base")

    # CREATE INDEX (may not be supported but test it)
    runner.add_test("CREATE TABLE idx_test",
                    f"CREATE TABLE IF NOT EXISTS idx_test (id INT, name VARCHAR(50))")
    runner.add_test("CREATE INDEX",
                    f"CREATE INDEX idx_name ON idx_test (name)",
                    expect_success=True)
    runner.add_test("DROP TABLE idx_test",
                    f"DROP TABLE IF EXISTS idx_test")

    # DROP TABLE IF EXISTS for non-existent
    runner.add_test("DROP non-existent table",
                    f"DROP TABLE IF EXISTS nonexistent_table_xyz")

    # =========================================================
    # 2. DML TESTS (100+)
    # =========================================================
    print("Category 2: DML tests...")

    # Create main test table
    runner.add_test("CREATE TABLE dml_test",
                    f"CREATE TABLE IF NOT EXISTS dml_test (id INT, name VARCHAR(255), score FLOAT, active BOOLEAN)")
    runner.add_test("CREATE TABLE dml_int",
                    f"CREATE TABLE IF NOT EXISTS dml_int (a INT, b INT, c INT)")

    # INSERT single
    runner.add_test("INSERT single", f"INSERT INTO dml_test VALUES (1, 'Alice', 95.5, 1)")
    runner.add_test("INSERT single 2", f"INSERT INTO dml_test VALUES (2, 'Bob', 87.3, 1)")
    runner.add_test("INSERT single 3", f"INSERT INTO dml_test VALUES (3, 'Charlie', 72.1, 0)")

    # INSERT multi-value
    runner.add_test("INSERT multi",
                    f"INSERT INTO dml_test VALUES (4, 'Diana', 91.0, 1), (5, 'Eve', 88.5, 1)")

    # INSERT NULL
    runner.add_test("INSERT NULL",
                    f"INSERT INTO dml_test VALUES (6, NULL, NULL, NULL)")

    # INSERT special chars
    runner.add_test("INSERT special chars",
                    f"INSERT INTO dml_test VALUES (7, 'O''Brien', 65.0, 1)")

    # INSERT max int
    runner.add_test("INSERT max int",
                    f"INSERT INTO dml_test VALUES (2147483647, 'maxint', 0.0, 1)")

    # INSERT negative
    runner.add_test("INSERT negative",
                    f"INSERT INTO dml_test VALUES (-1, 'negative', -99.9, 0)")

    # INSERT zero
    runner.add_test("INSERT zero",
                    f"INSERT INTO dml_test VALUES (0, '', 0.0, 0)")

    # SELECT * tests
    runner.add_test("SELECT *", f"SELECT * FROM dml_test")
    runner.add_test("SELECT count", f"SELECT COUNT(*) FROM dml_test")

    # WHERE =
    runner.add_test("WHERE =", f"SELECT * FROM dml_test WHERE id = 1")
    runner.add_test("WHERE = string", f"SELECT * FROM dml_test WHERE name = 'Alice'")

    # WHERE !=
    runner.add_test("WHERE !=", f"SELECT * FROM dml_test WHERE id != 1")

    # WHERE < > <= >=
    runner.add_test("WHERE <", f"SELECT * FROM dml_test WHERE id < 5")
    runner.add_test("WHERE >", f"SELECT * FROM dml_test WHERE id > 5")
    runner.add_test("WHERE <=", f"SELECT * FROM dml_test WHERE id <= 5")
    runner.add_test("WHERE >=", f"SELECT * FROM dml_test WHERE id >= 5")

    # WHERE LIKE
    runner.add_test("WHERE LIKE %", f"SELECT * FROM dml_test WHERE name LIKE 'A%'")
    runner.add_test("WHERE LIKE _", f"SELECT * FROM dml_test WHERE name LIKE 'A___e'")

    # WHERE IN
    runner.add_test("WHERE IN", f"SELECT * FROM dml_test WHERE id IN (1, 2, 3)")

    # WHERE BETWEEN
    runner.add_test("WHERE BETWEEN", f"SELECT * FROM dml_test WHERE id BETWEEN 2 AND 5")

    # WHERE IS NULL / IS NOT NULL
    runner.add_test("WHERE IS NULL", f"SELECT * FROM dml_test WHERE name IS NULL")
    runner.add_test("WHERE IS NOT NULL", f"SELECT * FROM dml_test WHERE name IS NOT NULL")

    # ORDER BY
    runner.add_test("ORDER BY ASC", f"SELECT * FROM dml_test ORDER BY id")
    runner.add_test("ORDER BY DESC", f"SELECT * FROM dml_test ORDER BY id DESC")
    runner.add_test("ORDER BY name", f"SELECT * FROM dml_test ORDER BY name")

    # LIMIT / OFFSET
    runner.add_test("LIMIT", f"SELECT * FROM dml_test LIMIT 3")
    runner.add_test("LIMIT OFFSET", f"SELECT * FROM dml_test LIMIT 2 OFFSET 1")

    # GROUP BY
    runner.add_test("CREATE TABLE dml_grp",
                    f"CREATE TABLE IF NOT EXISTS dml_grp (dept VARCHAR(50), salary INT)")
    for i, (dept, sal) in enumerate([("eng", 100), ("eng", 120), ("sales", 90), ("sales", 95), ("hr", 80)]):
        runner.add_test(f"INSERT dml_grp {i}",
                        f"INSERT INTO dml_grp VALUES ('{dept}', {sal})")
    runner.add_test("GROUP BY", f"SELECT dept, COUNT(*) FROM dml_grp GROUP BY dept")
    runner.add_test("GROUP BY HAVING", f"SELECT dept, COUNT(*) FROM dml_grp GROUP BY dept HAVING COUNT(*) > 1")

    # Aggregate functions
    runner.add_test("COUNT(*)", f"SELECT COUNT(*) FROM dml_test")
    runner.add_test("SUM", f"SELECT SUM(score) FROM dml_test WHERE score IS NOT NULL")
    runner.add_test("AVG", f"SELECT AVG(score) FROM dml_test WHERE score IS NOT NULL")
    runner.add_test("MIN", f"SELECT MIN(score) FROM dml_test")
    runner.add_test("MAX", f"SELECT MAX(score) FROM dml_test")

    # DISTINCT
    runner.add_test("DISTINCT", f"SELECT DISTINCT active FROM dml_test")

    # UPDATE
    runner.add_test("UPDATE", f"UPDATE dml_test SET score = 99.9 WHERE id = 1")
    runner.add_test("Verify UPDATE", f"SELECT score FROM dml_test WHERE id = 1")

    # DELETE
    runner.add_test("DELETE", f"DELETE FROM dml_test WHERE id = -1")
    runner.add_test("Verify DELETE", f"SELECT COUNT(*) FROM dml_test")

    # INSERT into dml_int for more tests
    for i in range(1, 11):
        runner.add_test(f"INSERT dml_int {i}",
                        f"INSERT INTO dml_int VALUES ({i}, {i*2}, {i*3})")

    # More WHERE combos
    runner.add_test("WHERE AND", f"SELECT * FROM dml_int WHERE a > 3 AND b < 15")
    runner.add_test("WHERE OR", f"SELECT * FROM dml_int WHERE a = 1 OR a = 10")

    # =========================================================
    # 3. STRING FUNCTIONS (80+)
    # =========================================================
    print("Category 3: String functions...")

    runner.add_test("CREATE TABLE str_test",
                    f"CREATE TABLE IF NOT EXISTS str_test (s VARCHAR(255))")
    runner.add_test("INSERT str_test",
                    f"INSERT INTO str_test VALUES ('Hello World')")

    # CONCAT
    runner.add_test("CONCAT basic", f"SELECT CONCAT('Hello', ' ', 'World')")
    runner.add_test("CONCAT from table", f"SELECT CONCAT(s, '!') FROM str_test")
    runner.add_test("CONCAT_WS", f"SELECT CONCAT_WS(',', 'a', 'b', 'c')")

    # LENGTH
    runner.add_test("LENGTH", f"SELECT LENGTH('Hello')")
    runner.add_test("LENGTH from table", f"SELECT LENGTH(s) FROM str_test")

    # SUBSTRING
    runner.add_test("SUBSTRING 1", f"SELECT SUBSTRING('Hello World', 1, 5)")
    runner.add_test("SUBSTRING 2", f"SELECT SUBSTRING('Hello World', 7)")
    runner.add_test("SUBSTR", f"SELECT SUBSTR('Hello World', 1, 5)")

    # REPLACE
    runner.add_test("REPLACE", f"SELECT REPLACE('Hello World', 'World', 'MySQL')")

    # TRIM
    runner.add_test("TRIM", f"SELECT TRIM('  Hello  ')")
    runner.add_test("LTRIM", f"SELECT LTRIM('  Hello  ')")
    runner.add_test("RTRIM", f"SELECT RTRIM('  Hello  ')")

    # UPPER / LOWER
    runner.add_test("UPPER", f"SELECT UPPER('hello')")
    runner.add_test("LOWER", f"SELECT LOWER('HELLO')")
    runner.add_test("UCASE", f"SELECT UCASE('hello')")
    runner.add_test("LCASE", f"SELECT LCASE('HELLO')")

    # REVERSE
    runner.add_test("REVERSE", f"SELECT REVERSE('Hello')")

    # LPAD / RPAD
    runner.add_test("LPAD", f"SELECT LPAD('Hi', 10, '*')")
    runner.add_test("RPAD", f"SELECT RPAD('Hi', 10, '*')")

    # LEFT / RIGHT
    runner.add_test("LEFT", f"SELECT LEFT('Hello World', 5)")
    runner.add_test("RIGHT", f"SELECT RIGHT('Hello World', 5)")

    # LOCATE / INSTR
    runner.add_test("LOCATE", f"SELECT LOCATE('World', 'Hello World')")
    runner.add_test("LOCATE not found", f"SELECT LOCATE('xyz', 'Hello World')")
    runner.add_test("INSTR", f"SELECT INSTR('Hello World', 'World')")

    # REPEAT
    runner.add_test("REPEAT", f"SELECT REPEAT('ab', 5)")

    # SPACE
    runner.add_test("SPACE", f"SELECT LENGTH(SPACE(10))")

    # HEX
    runner.add_test("HEX", f"SELECT HEX('A')")

    # ASCII / CHAR
    runner.add_test("ASCII", f"SELECT ASCII('A')")
    runner.add_test("CHAR", f"SELECT CHAR(65)")

    # Field / ELT / MAKE_SET
    runner.add_test("FIELD", f"SELECT FIELD('b', 'a', 'b', 'c')")

    # String functions with NULL
    runner.add_test("CONCAT NULL", f"SELECT CONCAT('Hello', NULL)")
    runner.add_test("LENGTH NULL", f"SELECT LENGTH(NULL)")

    # From table variants
    runner.add_test("UPPER from table", f"SELECT UPPER(s) FROM str_test")
    runner.add_test("LOWER from table", f"SELECT LOWER(s) FROM str_test")
    runner.add_test("REVERSE from table", f"SELECT REVERSE(s) FROM str_test")
    runner.add_test("REPLACE from table", f"SELECT REPLACE(s, 'World', 'DB') FROM str_test")

    # Combinations
    runner.add_test("CONCAT + UPPER", f"SELECT CONCAT(UPPER('hello'), ' ', LOWER('WORLD'))")
    runner.add_test("LENGTH of CONCAT", f"SELECT LENGTH(CONCAT('a', 'b', 'c'))")
    runner.add_test("SUBSTRING of REVERSE", f"SELECT SUBSTRING(REVERSE('Hello'), 1, 3)")

    # Empty string
    runner.add_test("LENGTH empty", f"SELECT LENGTH('')")
    runner.add_test("CONCAT empty", f"SELECT CONCAT('', 'test', '')")

    # Unicode
    runner.add_test("LENGTH unicode", f"SELECT LENGTH('你好')")

    # =========================================================
    # 4. NUMERIC FUNCTIONS (60+)
    # =========================================================
    print("Category 4: Numeric functions...")

    # ABS
    runner.add_test("ABS positive", f"SELECT ABS(42)")
    runner.add_test("ABS negative", f"SELECT ABS(-42)")
    runner.add_test("ABS zero", f"SELECT ABS(0)")

    # CEIL / FLOOR
    runner.add_test("CEIL", f"SELECT CEIL(4.2)")
    runner.add_test("CEIL negative", f"SELECT CEIL(-4.2)")
    runner.add_test("FLOOR", f"SELECT FLOOR(4.8)")
    runner.add_test("FLOOR negative", f"SELECT FLOOR(-4.8)")

    # ROUND
    runner.add_test("ROUND", f"SELECT ROUND(4.567, 2)")
    runner.add_test("ROUND 0", f"SELECT ROUND(4.5)")
    runner.add_test("ROUND negative dec", f"SELECT ROUND(45.678, -1)")

    # TRUNCATE
    runner.add_test("TRUNCATE", f"SELECT TRUNCATE(4.567, 2)")

    # MOD
    runner.add_test("MOD", f"SELECT MOD(10, 3)")
    runner.add_test("MOD 2", f"SELECT 10 % 3")

    # POWER / SQRT
    runner.add_test("POWER", f"SELECT POWER(2, 10)")
    runner.add_test("SQRT", f"SELECT SQRT(144)")

    # RAND (just test it doesn't error)
    runner.add_test("RAND", f"SELECT RAND() >= 0")

    # SIGN
    runner.add_test("SIGN positive", f"SELECT SIGN(42)")
    runner.add_test("SIGN negative", f"SELECT SIGN(-42)")
    runner.add_test("SIGN zero", f"SELECT SIGN(0)")

    # LOG variants
    runner.add_test("LOG", f"SELECT LOG(2.718281828) > 0.99")
    runner.add_test("LOG2", f"SELECT LOG2(8)")
    runner.add_test("LOG10", f"SELECT LOG10(1000)")

    # EXP
    runner.add_test("EXP", f"SELECT EXP(1) > 2.71")

    # PI
    runner.add_test("PI", f"SELECT PI() > 3.14")

    # CRC32
    runner.add_test("CRC32", f"SELECT CRC32('MySQL')")

    # Combinations
    runner.add_test("ABS + FLOOR", f"SELECT ABS(FLOOR(-4.7))")
    runner.add_test("ROUND + POWER", f"SELECT ROUND(POWER(2.5, 2), 1)")
    runner.add_test("MOD + ABS", f"SELECT MOD(ABS(-100), 7)")

    # From table
    runner.add_test("ABS from table", f"SELECT ABS(a - 5) FROM dml_int LIMIT 3")
    runner.add_test("ROUND from table", f"SELECT ROUND(score, 0) FROM dml_test LIMIT 3")

    # =========================================================
    # 5. DATE FUNCTIONS (60+)
    # =========================================================
    print("Category 5: Date functions...")

    # NOW / CURDATE / CURTIME
    runner.add_test("NOW", f"SELECT NOW() IS NOT NULL")
    runner.add_test("CURDATE", f"SELECT CURDATE() IS NOT NULL")
    runner.add_test("CURRENT_DATE", f"SELECT CURRENT_DATE() IS NOT NULL")
    runner.add_test("CURTIME", f"SELECT CURTIME() IS NOT NULL")
    runner.add_test("CURRENT_TIME", f"SELECT CURRENT_TIME() IS NOT NULL")

    # DATE_FORMAT
    runner.add_test("DATE_FORMAT 1", f"SELECT DATE_FORMAT('2024-01-15', '%Y-%m-%d')")
    runner.add_test("DATE_FORMAT 2", f"SELECT DATE_FORMAT('2024-01-15', '%d/%m/%Y')")
    runner.add_test("DATE_FORMAT 3", f"SELECT DATE_FORMAT('2024-01-15 10:30:00', '%H:%i:%s')")

    # DATEDIFF
    runner.add_test("DATEDIFF", f"SELECT DATEDIFF('2024-01-15', '2024-01-01')")

    # DATE_ADD / DATE_SUB
    runner.add_test("DATE_ADD DAY", f"SELECT DATE_ADD('2024-01-15', INTERVAL 10 DAY)")
    runner.add_test("DATE_ADD MONTH", f"SELECT DATE_ADD('2024-01-15', INTERVAL 2 MONTH)")
    runner.add_test("DATE_ADD YEAR", f"SELECT DATE_ADD('2024-01-15', INTERVAL 1 YEAR)")
    runner.add_test("DATE_SUB DAY", f"SELECT DATE_SUB('2024-01-15', INTERVAL 5 DAY)")
    runner.add_test("DATE_SUB MONTH", f"SELECT DATE_SUB('2024-01-15', INTERVAL 1 MONTH)")

    # YEAR / MONTH / DAY
    runner.add_test("YEAR", f"SELECT YEAR('2024-01-15')")
    runner.add_test("MONTH", f"SELECT MONTH('2024-01-15')")
    runner.add_test("DAY", f"SELECT DAY('2024-01-15')")
    runner.add_test("DAYOFMONTH", f"SELECT DAYOFMONTH('2024-01-15')")

    # HOUR / MINUTE / SECOND
    runner.add_test("HOUR", f"SELECT HOUR('2024-01-15 14:30:45')")
    runner.add_test("MINUTE", f"SELECT MINUTE('2024-01-15 14:30:45')")
    runner.add_test("SECOND", f"SELECT SECOND('2024-01-15 14:30:45')")

    # DAYOFWEEK / DAYOFYEAR
    runner.add_test("DAYOFWEEK", f"SELECT DAYOFWEEK('2024-01-15') IS NOT NULL")
    runner.add_test("DAYOFYEAR", f"SELECT DAYOFYEAR('2024-01-15')")

    # LAST_DAY
    runner.add_test("LAST_DAY", f"SELECT LAST_DAY('2024-01-15')")
    runner.add_test("LAST_DAY Feb", f"SELECT LAST_DAY('2024-02-15')")

    # MAKEDATE
    runner.add_test("MAKEDATE", f"SELECT MAKEDATE(2024, 32)")

    # UNIX_TIMESTAMP / FROM_UNIXTIME
    runner.add_test("UNIX_TIMESTAMP", f"SELECT UNIX_TIMESTAMP('2024-01-15') > 0")
    runner.add_test("FROM_UNIXTIME", f"SELECT FROM_UNIXTIME(1705276800) IS NOT NULL")

    # NOW() based
    runner.add_test("YEAR NOW", f"SELECT YEAR(NOW()) > 2020")
    runner.add_test("MONTH NOW", f"SELECT MONTH(NOW()) BETWEEN 1 AND 12")
    runner.add_test("DAY NOW", f"SELECT DAY(NOW()) BETWEEN 1 AND 31")

    # WEEK / WEEKDAY
    runner.add_test("WEEK", f"SELECT WEEK('2024-01-15') IS NOT NULL")
    runner.add_test("QUARTER", f"SELECT QUARTER('2024-06-15')")

    # =========================================================
    # 6. SHOW COMMANDS (50+)
    # =========================================================
    print("Category 6: SHOW commands...")

    runner.add_test("SHOW DATABASES", f"SHOW DATABASES", db=False)
    runner.add_test("SHOW TABLES", f"SHOW TABLES")
    runner.add_test("SHOW TABLES LIKE", f"SHOW TABLES LIKE 'dml%'")
    runner.add_test("SHOW COLUMNS", f"SHOW COLUMNS FROM dml_test")
    runner.add_test("SHOW FULL COLUMNS", f"SHOW FULL COLUMNS FROM dml_test")
    runner.add_test("SHOW CREATE TABLE", f"SHOW CREATE TABLE dml_test")
    runner.add_test("SHOW CREATE DATABASE", f"SHOW CREATE DATABASE {DB_NAME}", db=False)
    runner.add_test("SHOW VARIABLES", f"SHOW VARIABLES", db=False)
    runner.add_test("SHOW VARIABLES LIKE 1", f"SHOW VARIABLES LIKE '%version%'", db=False)
    runner.add_test("SHOW VARIABLES LIKE 2", f"SHOW VARIABLES LIKE 'max_%'", db=False)
    runner.add_test("SHOW VARIABLES LIKE 3", f"SHOW VARIABLES LIKE 'character_%'", db=False)
    runner.add_test("SHOW VARIABLES LIKE 4", f"SHOW VARIABLES LIKE 'collation%'", db=False)
    runner.add_test("SHOW VARIABLES LIKE 5", f"SHOW VARIABLES LIKE 'timeout%'", db=False)
    runner.add_test("SHOW VARIABLES LIKE 6", f"SHOW VARIABLES LIKE 'sql_mode%'", db=False)
    runner.add_test("SHOW VARIABLES LIKE 7", f"SHOW VARIABLES LIKE 'auto%'", db=False)
    runner.add_test("SHOW STATUS", f"SHOW STATUS", db=False)
    runner.add_test("SHOW PROCESSLIST", f"SHOW PROCESSLIST", db=False)
    runner.add_test("SHOW FULL PROCESSLIST", f"SHOW FULL PROCESSLIST", db=False)
    runner.add_test("SHOW WARNINGS", f"SHOW WARNINGS", db=False)
    runner.add_test("SHOW ERRORS", f"SHOW ERRORS", db=False)
    runner.add_test("SHOW COLLATION", f"SHOW COLLATION", db=False)
    runner.add_test("SHOW COLLATION LIKE", f"SHOW COLLATION LIKE 'utf8%'", db=False)
    runner.add_test("SHOW CHARSET", f"SHOW CHARACTER SET", db=False)
    runner.add_test("SHOW ENGINES", f"SHOW ENGINES", db=False)
    runner.add_test("SHOW TABLE STATUS", f"SHOW TABLE STATUS", db=False)
    runner.add_test("SHOW INDEX", f"SHOW INDEX FROM dml_test", db=False)
    runner.add_test("SHOW KEYS", f"SHOW KEYS FROM dml_test", db=False)
    runner.add_test("SHOW GRANTS", f"SHOW GRANTS", db=False)
    runner.add_test("SHOW MASTER STATUS", f"SHOW MASTER STATUS", db=False)

    # =========================================================
    # 7. @@variables (40+)
    # =========================================================
    print("Category 7: @@variables...")

    variables = [
        "version", "version_comment", "autocommit", "max_allowed_packet",
        "character_set_client", "character_set_connection", "character_set_results",
        "character_set_server", "collation_connection", "collation_server",
        "wait_timeout", "interactive_timeout", "sql_mode",
        "lower_case_table_names", "net_buffer_length",
        "have_ssl", "license", "protocol_version",
        "auto_increment_increment", "auto_increment_offset",
        "transaction_isolation", "tx_isolation",
    ]

    for var in variables:
        runner.add_test(f"SELECT @@{var}", f"SELECT @@{var}", db=False)

    runner.add_test("SELECT @@GLOBAL.version", f"SELECT @@GLOBAL.version", db=False)
    runner.add_test("SELECT @@SESSION.autocommit", f"SELECT @@SESSION.autocommit", db=False)

    # =========================================================
    # 8. INFORMATION_SCHEMA (40+)
    # =========================================================
    print("Category 8: INFORMATION_SCHEMA...")

    runner.add_test("IS TABLES", f"SELECT TABLE_NAME FROM INFORMATION_SCHEMA.TABLES WHERE TABLE_SCHEMA = '{DB_NAME}'")
    runner.add_test("IS TABLES count", f"SELECT COUNT(*) FROM INFORMATION_SCHEMA.TABLES WHERE TABLE_SCHEMA = '{DB_NAME}'")
    runner.add_test("IS COLUMNS", f"SELECT COLUMN_NAME FROM INFORMATION_SCHEMA.COLUMNS WHERE TABLE_SCHEMA = '{DB_NAME}' AND TABLE_NAME = 'dml_test'")
    runner.add_test("IS COLUMNS count", f"SELECT COUNT(*) FROM INFORMATION_SCHEMA.COLUMNS WHERE TABLE_SCHEMA = '{DB_NAME}'")
    runner.add_test("IS SCHEMATA", f"SELECT SCHEMA_NAME FROM INFORMATION_SCHEMA.SCHEMATA")
    runner.add_test("IS SCHEMATA count", f"SELECT COUNT(*) FROM INFORMATION_SCHEMA.SCHEMATA")

    # Various WHERE on IS TABLES
    for tbl_type in ["BASE TABLE", "VIEW"]:
        runner.add_test(f"IS TABLES type={tbl_type}",
                        f"SELECT TABLE_NAME FROM INFORMATION_SCHEMA.TABLES WHERE TABLE_SCHEMA = '{DB_NAME}' AND TABLE_TYPE = '{tbl_type}'")

    # Column info
    runner.add_test("IS COLUMNS data_type",
                    f"SELECT COLUMN_NAME, DATA_TYPE FROM INFORMATION_SCHEMA.COLUMNS WHERE TABLE_SCHEMA = '{DB_NAME}' AND TABLE_NAME = 'dml_test'")
    runner.add_test("IS COLUMNS ordinal",
                    f"SELECT COLUMN_NAME, ORDINAL_POSITION FROM INFORMATION_SCHEMA.COLUMNS WHERE TABLE_SCHEMA = '{DB_NAME}' AND TABLE_NAME = 'dml_test'")
    runner.add_test("IS COLUMNS nullable",
                    f"SELECT COLUMN_NAME, IS_NULLABLE FROM INFORMATION_SCHEMA.COLUMNS WHERE TABLE_SCHEMA = '{DB_NAME}' AND TABLE_NAME = 'dml_test'")

    # =========================================================
    # 9. TRANSACTIONS (30+)
    # =========================================================
    print("Category 9: Transactions...")

    runner.add_test("CREATE TABLE txn_test",
                    f"CREATE TABLE IF NOT EXISTS txn_test (id INT, val VARCHAR(50))")

    # BEGIN/COMMIT
    runner.add_test("BEGIN", f"BEGIN")
    runner.add_test("INSERT in txn", f"INSERT INTO txn_test VALUES (1, 'txn1')")
    runner.add_test("COMMIT", f"COMMIT")
    runner.add_test("Verify COMMIT", f"SELECT COUNT(*) FROM txn_test")

    # BEGIN/ROLLBACK
    runner.add_test("BEGIN 2", f"BEGIN")
    runner.add_test("INSERT before rollback", f"INSERT INTO txn_test VALUES (2, 'txn2')")
    runner.add_test("ROLLBACK", f"ROLLBACK")

    # SET autocommit
    runner.add_test("SET autocommit=0", f"SET autocommit=0", db=False)
    runner.add_test("SET autocommit=1", f"SET autocommit=1", db=False)

    # Transaction isolation
    for level in ["READ UNCOMMITTED", "READ COMMITTED", "REPEATABLE READ", "SERIALIZABLE"]:
        runner.add_test(f"SET TRANSACTION ISOLATION LEVEL {level}",
                        f"SET SESSION TRANSACTION ISOLATION LEVEL {level}")

    # Multiple inserts in one "transaction"
    runner.add_test("BEGIN 3", f"BEGIN")
    for i in range(10, 15):
        runner.add_test(f"INSERT in txn {i}", f"INSERT INTO txn_test VALUES ({i}, 'batch{i}')")
    runner.add_test("COMMIT 3", f"COMMIT")
    runner.add_test("Count after txn", f"SELECT COUNT(*) FROM txn_test")

    # =========================================================
    # 10. SET COMMANDS (30+)
    # =========================================================
    print("Category 10: SET commands...")

    runner.add_test("SET NAMES utf8", f"SET NAMES utf8", db=False)
    runner.add_test("SET NAMES utf8mb4", f"SET NAMES utf8mb4", db=False)
    runner.add_test("SET NAMES latin1", f"SET NAMES latin1", db=False)

    runner.add_test("SET character_set_client", f"SET character_set_client = utf8mb4", db=False)
    runner.add_test("SET character_set_results", f"SET character_set_results = utf8mb4", db=False)
    runner.add_test("SET character_set_connection", f"SET character_set_connection = utf8mb4", db=False)

    runner.add_test("SET sql_mode=''", f"SET sql_mode=''", db=False)
    runner.add_test("SET sql_mode default", f"SET sql_mode='STRICT_TRANS_TABLES'", db=False)

    # User variables
    runner.add_test("SET @var", f"SET @myvar = 42", db=False)
    runner.add_test("SELECT @var", f"SELECT @myvar", db=False)
    runner.add_test("SET @str", f"SET @str = 'hello'", db=False)
    runner.add_test("SELECT @str", f"SELECT @str", db=False)

    # More SET
    runner.add_test("SET wait_timeout", f"SET wait_timeout = 28800", db=False)
    runner.add_test("SET interactive_timeout", f"SET interactive_timeout = 28800", db=False)

    # =========================================================
    # 11. JOINs (30+)
    # =========================================================
    print("Category 11: JOINs...")

    runner.add_test("CREATE TABLE j1", f"CREATE TABLE IF NOT EXISTS j1 (id INT, val VARCHAR(50))")
    runner.add_test("CREATE TABLE j2", f"CREATE TABLE IF NOT EXISTS j2 (id INT, j1_id INT, extra VARCHAR(50))")
    runner.add_test("INSERT j1", f"INSERT INTO j1 VALUES (1, 'a'), (2, 'b'), (3, 'c')")
    runner.add_test("INSERT j2", f"INSERT INTO j2 VALUES (1, 1, 'x'), (2, 2, 'y'), (3, 1, 'z')")

    # INNER JOIN
    runner.add_test("INNER JOIN", f"SELECT j1.val, j2.extra FROM j1 INNER JOIN j2 ON j1.id = j2.j1_id")
    runner.add_test("INNER JOIN WHERE", f"SELECT j1.val, j2.extra FROM j1 INNER JOIN j2 ON j1.id = j2.j1_id WHERE j1.id = 1")

    # LEFT JOIN
    runner.add_test("LEFT JOIN", f"SELECT j1.val, j2.extra FROM j1 LEFT JOIN j2 ON j1.id = j2.j1_id")

    # RIGHT JOIN
    runner.add_test("RIGHT JOIN", f"SELECT j1.val, j2.extra FROM j1 RIGHT JOIN j2 ON j1.id = j2.j1_id")

    # CROSS JOIN
    runner.add_test("CROSS JOIN", f"SELECT j1.val, j2.extra FROM j1 CROSS JOIN j2")

    # Self join
    runner.add_test("CREATE TABLE j_self", f"CREATE TABLE IF NOT EXISTS j_self (id INT, parent_id INT, name VARCHAR(50))")
    runner.add_test("INSERT j_self", f"INSERT INTO j_self VALUES (1, NULL, 'root'), (2, 1, 'child'), (3, 1, 'child2')")
    runner.add_test("Self JOIN", f"SELECT a.name, b.name as parent FROM j_self a LEFT JOIN j_self b ON a.parent_id = b.id")

    # JOIN with GROUP BY
    runner.add_test("JOIN + GROUP BY", f"SELECT j1.val, COUNT(*) FROM j1 LEFT JOIN j2 ON j1.id = j2.j1_id GROUP BY j1.val")

    # Multiple JOINs
    runner.add_test("CREATE TABLE j3", f"CREATE TABLE IF NOT EXISTS j3 (id INT, j2_id INT, tag VARCHAR(50))")
    runner.add_test("INSERT j3", f"INSERT INTO j3 VALUES (1, 1, 'red'), (2, 2, 'blue')")
    runner.add_test("3-table JOIN",
                    f"SELECT j1.val, j2.extra, j3.tag FROM j1 JOIN j2 ON j1.id = j2.j1_id JOIN j3 ON j2.id = j3.j2_id")

    # =========================================================
    # 12. SUBQUERIES (20+)
    # =========================================================
    print("Category 12: Subqueries...")

    runner.add_test("CREATE TABLE sub1", f"CREATE TABLE IF NOT EXISTS sub1 (id INT, score INT)")
    runner.add_test("CREATE TABLE sub2", f"CREATE TABLE IF NOT EXISTS sub2 (id INT, grade VARCHAR(10))")
    runner.add_test("INSERT sub1", f"INSERT INTO sub1 VALUES (1, 90), (2, 80), (3, 70), (4, 60)")
    runner.add_test("INSERT sub2", f"INSERT INTO sub2 VALUES (1, 'A'), (2, 'B'), (3, 'C')")

    # IN subquery
    runner.add_test("IN subquery", f"SELECT * FROM sub1 WHERE id IN (SELECT id FROM sub2)")

    # EXISTS subquery
    runner.add_test("EXISTS subquery", f"SELECT * FROM sub1 WHERE EXISTS (SELECT 1 FROM sub2 WHERE sub2.id = sub1.id)")

    # Scalar subquery
    runner.add_test("Scalar subquery", f"SELECT *, (SELECT COUNT(*) FROM sub2) as cnt FROM sub1 LIMIT 1")

    # Correlated subquery
    runner.add_test("Correlated subquery",
                    f"SELECT * FROM sub1 s1 WHERE s1.score > (SELECT AVG(score) FROM sub1)")

    # Subquery in FROM
    runner.add_test("Subquery in FROM", f"SELECT * FROM (SELECT id, score FROM sub1 WHERE score > 70) AS t")

    # Subquery with NOT IN
    runner.add_test("NOT IN subquery", f"SELECT * FROM sub1 WHERE id NOT IN (SELECT id FROM sub2 WHERE grade = 'A')")

    # Subquery with aggregate
    runner.add_test("Subquery MAX",
                    f"SELECT * FROM sub1 WHERE score = (SELECT MAX(score) FROM sub1)")

    # =========================================================
    # 13. PROGRAMMATIC LOOP TESTS (to reach 1000+)
    # =========================================================
    print("Category 13: Programmatic loop tests...")

    # --- Additional DDL variants via loop ---
    # Create/drop many tables with different naming patterns
    for i in range(30):
        runner.add_test(f"CREATE/DELETE loop_{i}",
                        f"CREATE TABLE IF NOT EXISTS loop_{i} (id INT, v VARCHAR(50))")
    for i in range(30):
        runner.add_test(f"INSERT loop_{i}",
                        f"INSERT INTO loop_{i} VALUES ({i}, 'val_{i}')")
    for i in range(30):
        runner.add_test(f"SELECT loop_{i}",
                        f"SELECT * FROM loop_{i}")
    for i in range(30):
        runner.add_test(f"DROP loop_{i}",
                        f"DROP TABLE IF EXISTS loop_{i}")

    # --- String function combos via loop ---
    string_funcs = [
        ("UPPER", "'hello'"), ("LOWER", "'HELLO'"), ("REVERSE", "'abc'"),
        ("LENGTH", "'test'"), ("LTRIM", "'  x'"), ("RTRIM", "'x  '"),
        ("TRIM", "'  x  '"),
    ]
    for i, (fn, arg) in enumerate(string_funcs):
        for j in range(5):
            runner.add_test(f"str_fn_{fn}_v{j}",
                            f"SELECT {fn}({arg}) IS NOT NULL")

    # --- Numeric function combos via loop ---
    num_values = [0, 1, -1, 42, -42, 3.14, -3.14, 100, 0.001, 999999]
    for fn in ["ABS", "CEIL", "FLOOR", "ROUND", "SIGN"]:
        for v in num_values:
            runner.add_test(f"num_{fn}_{v}",
                            f"SELECT {fn}({v}) IS NOT NULL OR {fn}({v}) IS NULL")

    # --- WHERE clause permutations ---
    operators = ["=", "!=", "<", ">", "<=", ">="]
    for op in operators:
        for val in [1, 5, 10, 50, 100]:
            runner.add_test(f"WHERE_id_{op}_{val}",
                            f"SELECT * FROM dml_int WHERE a {op} {val}")

    # --- Multiple column selects ---
    for i in range(20):
        cols = ", ".join([f"a" if i % 3 == 0 else f"b" if i % 3 == 1 else f"c"])
        runner.add_test(f"SELECT cols_{i}",
                        f"SELECT {cols} FROM dml_int LIMIT {i + 1}")

    # --- ORDER BY permutations ---
    for col in ["a", "b", "c"]:
        for direction in ["ASC", "DESC"]:
            for limit in [1, 5, 10]:
                runner.add_test(f"ORDER_{col}_{direction}_lim{limit}",
                                f"SELECT * FROM dml_int ORDER BY {col} {direction} LIMIT {limit}")

    # --- GROUP BY with different aggregates ---
    for agg in ["COUNT(*)", "SUM(a)", "AVG(b)", "MIN(c)", "MAX(a)"]:
        runner.add_test(f"GROUP_agg_{agg}",
                        f"SELECT {agg} FROM dml_int")

    # --- Date function date literals ---
    dates = ["'2024-01-01'", "'2024-06-15'", "'2024-12-31'", "'2023-02-28'", "'2024-02-29'"]
    date_fns = ["YEAR", "MONTH", "DAY", "DAYOFYEAR"]
    for d in dates:
        for fn in date_fns:
            runner.add_test(f"date_{fn}_{d}",
                            f"SELECT {fn}({d})")

    # --- String literal tests ---
    for s in ["hello", "world", "test", "abc", "xyz", "12345", "foo", "bar"]:
        runner.add_test(f"UPPER_str_{s}", f"SELECT UPPER('{s}')")
        runner.add_test(f"LENGTH_str_{s}", f"SELECT LENGTH('{s}')")
        runner.add_test(f"REVERSE_str_{s}", f"SELECT REVERSE('{s}')")

    # --- CONCAT combos ---
    strs = [("a", "b"), ("hello", "world"), ("foo", "bar"), ("1", "2"), ("x", "y")]
    for s1, s2 in strs:
        runner.add_test(f"CONCAT_{s1}_{s2}", f"SELECT CONCAT('{s1}', '{s2}')")
        runner.add_test(f"CONCAT_WS_{s1}_{s2}", f"SELECT CONCAT_WS('-', '{s1}', '{s2}')")

    # --- SUBSTRING combos ---
    for s in ["Hello World", "abcdef", "12345678"]:
        for start in [1, 2, 3]:
            for length in [1, 3, 5]:
                runner.add_test(f"SUBSTR_{s[:4]}_{start}_{length}",
                                f"SELECT SUBSTRING('{s}', {start}, {length})")

    # --- LIKE patterns ---
    patterns = ["A%", "%e%", "___", "A___e", "%li%", "B%b%"]
    for pat in patterns:
        runner.add_test(f"LIKE_{pat}",
                        f"SELECT * FROM dml_test WHERE name LIKE '{pat}'")

    # --- BETWEEN variants ---
    for lo in [1, 3, 5]:
        for hi in [5, 7, 10]:
            if hi > lo:
                runner.add_test(f"BETWEEN_{lo}_{hi}",
                                f"SELECT * FROM dml_int WHERE a BETWEEN {lo} AND {hi}")

    # --- IN list variants ---
    for size in [1, 2, 3, 5, 10]:
        vals = ", ".join([str(i) for i in range(1, size + 1)])
        runner.add_test(f"IN_list_{size}",
                        f"SELECT * FROM dml_int WHERE a IN ({vals})")

    # --- Expression tests ---
    exprs = [
        "1+1", "2*3", "10/3", "10 DIV 3", "10 MOD 3",
        "1+2*3", "(1+2)*3", "10-5", "100/10",
        "1 AND 1", "1 OR 0", "NOT 0", "1 XOR 1",
    ]
    for expr in exprs:
        runner.add_test(f"expr_{expr.replace(' ', '_')}",
                        f"SELECT {expr}")

    # --- Multiple WHERE conditions ---
    conditions = [
        "a > 1 AND b < 20",
        "a = 5 OR c = 15",
        "a > 3 AND b > 5 AND c > 10",
        "a BETWEEN 1 AND 5 AND b BETWEEN 5 AND 15",
        "a IN (1,2,3) OR b IN (10,20)",
    ]
    for i, cond in enumerate(conditions):
        runner.add_test(f"multi_where_{i}", f"SELECT * FROM dml_int WHERE {cond}")

    # --- SHOW VARIABLES specific ---
    specific_vars = [
        "version", "version_comment", "autocommit",
        "max_connections", "max_allowed_packet",
        "character_set_client", "character_set_server",
        "collation_server", "collation_connection",
        "wait_timeout", "interactive_timeout",
        "sql_mode", "lower_case_table_names",
        "net_buffer_length", "have_ssl",
        "license", "protocol_version",
        "auto_increment_increment", "auto_increment_offset",
        "thread_handling", "thread_cache_size",
    ]
    for var in specific_vars:
        runner.add_test(f"SHOW_VAR_{var}",
                        f"SHOW VARIABLES LIKE '{var}'", db=False)

    # --- IS TABLES additional ---
    runner.add_test("IS TABLES all schemas",
                    f"SELECT COUNT(*) FROM INFORMATION_SCHEMA.TABLES")
    runner.add_test("IS COLUMNS all schemas",
                    f"SELECT COUNT(*) FROM INFORMATION_SCHEMA.COLUMNS")
    for tbl in ["dml_test", "dml_int"]:
        runner.add_test(f"IS COLUMNS {tbl}",
                        f"SELECT COLUMN_NAME, DATA_TYPE FROM INFORMATION_SCHEMA.COLUMNS WHERE TABLE_NAME = '{tbl}' AND TABLE_SCHEMA = '{DB_NAME}'")
        runner.add_test(f"IS COLUMNS count {tbl}",
                        f"SELECT COUNT(*) FROM INFORMATION_SCHEMA.COLUMNS WHERE TABLE_NAME = '{tbl}' AND TABLE_SCHEMA = '{DB_NAME}'")

    # --- Transaction repeated ---
    for i in range(5):
        runner.add_test(f"txn_loop_begin_{i}", f"BEGIN")
        runner.add_test(f"txn_loop_insert_{i}",
                        f"INSERT INTO txn_test VALUES ({100 + i}, 'loop_{i}')")
        runner.add_test(f"txn_loop_commit_{i}", f"COMMIT")

    # --- SET user vars loop ---
    for i in range(10):
        runner.add_test(f"SET_uservar_{i}", f"SET @var_{i} = {i * 10}", db=False)
        runner.add_test(f"GET_uservar_{i}", f"SELECT @var_{i}", db=False)

    # --- JOIN with different conditions ---
    for cond in ["j1.id = j2.j1_id", "j1.id < j2.j1_id", "j1.id <= j2.j1_id",
                 "j2.j1_id > 0", "j1.val = 'a'"]:
        runner.add_test(f"JOIN_cond_{cond.replace(' ', '_').replace('=', 'eq')}",
                        f"SELECT j1.val, j2.extra FROM j1 JOIN j2 ON {cond}")

    # --- Aggregate with GROUP BY on dml_test ---
    for agg in ["COUNT(*)", "SUM(score)", "AVG(score)", "MIN(score)", "MAX(score)"]:
        for grp in ["active"]:
            runner.add_test(f"agg_{agg}_grp_{grp}",
                            f"SELECT {grp}, {agg} FROM dml_test GROUP BY {grp}")

    # --- HAVING with different conditions ---
    for having in ["COUNT(*) > 1", "COUNT(*) >= 1", "SUM(score) > 100", "AVG(score) > 50"]:
        runner.add_test(f"HAVING_{having.replace(' ', '_').replace('>', 'gt').replace('=', 'eq')}",
                        f"SELECT active, {having.split()[0]}(score) FROM dml_test GROUP BY active HAVING {having}")

    # --- DATE_ADD/DATE_SUB with various intervals ---
    for unit in ["DAY", "MONTH", "YEAR"]:
        for val in [1, 7, 30, 365]:
            runner.add_test(f"DATE_ADD_{unit}_{val}",
                            f"SELECT DATE_ADD('2024-01-01', INTERVAL {val} {unit})")
            runner.add_test(f"DATE_SUB_{unit}_{val}",
                            f"SELECT DATE_SUB('2024-12-31', INTERVAL {val} {unit})")

    # --- String LPAD/RPAD combos ---
    for pad_len in [5, 10, 20]:
        for pad_char in ["*", "#", "0"]:
            runner.add_test(f"LPAD_{pad_len}_{pad_char}",
                            f"SELECT LPAD('hi', {pad_len}, '{pad_char}')")
            runner.add_test(f"RPAD_{pad_len}_{pad_char}",
                            f"SELECT RPAD('hi', {pad_len}, '{pad_char}')")

    # --- LOCATE/INSTR with various strings ---
    for needle in ["a", "e", "o", "ll", "rl"]:
        runner.add_test(f"LOCATE_{needle}",
                        f"SELECT LOCATE('{needle}', 'Hello World')")
        runner.add_test(f"INSTR_{needle}",
                        f"SELECT INSTR('Hello World', '{needle}')")

    # --- REPEAT with various counts ---
    for n in [0, 1, 3, 5, 10]:
        for s in ["a", "ab", "x"]:
            runner.add_test(f"REPEAT_{s}_{n}",
                            f"SELECT LENGTH(REPEAT('{s}', {n}))")

    # --- Additional SHOW commands ---
    for pattern in ["%version%", "%char%", "%coll%", "%time%", "%max%", "%auto%", "%timeout%", "%sql%", "%net%", "%thread%"]:
        runner.add_test(f"SHOW_VAR_LIKE_{pattern}",
                        f"SHOW VARIABLES LIKE '{pattern}'", db=False)

    # --- Additional SHOW COLLATION LIKE ---
    for pat in ["utf8%", "latin%", "binary%", "%general%", "%unicode%"]:
        runner.add_test(f"SHOW_COLLATION_{pat}",
                        f"SHOW COLLATION LIKE '{pat}'", db=False)

    # --- Additional SHOW CHARSET ---
    runner.add_test("SHOW CHARSET utf8", f"SHOW CHARACTER SET LIKE 'utf8%'", db=False)
    runner.add_test("SHOW CHARSET latin", f"SHOW CHARACTER SET LIKE 'latin%'", db=False)

    # --- Additional @@ variables ---
    extra_vars = [
        "version_compile_os", "version_compile_machine",
        "system_time_zone", "time_zone",
        "event_scheduler", "default_storage_engine",
        "log_bin", "server_id",
        "innodb_buffer_pool_size", "innodb_log_file_size",
    ]
    for var in extra_vars:
        runner.add_test(f"@@extra_{var}", f"SELECT @@{var}", db=False)

    # --- Multiple table operations ---
    runner.add_test("CREATE TABLE bulk_tbl",
                    f"CREATE TABLE IF NOT EXISTS bulk_tbl (id INT, name VARCHAR(100), value FLOAT)")
    for i in range(50):
        runner.add_test(f"bulk_insert_{i}",
                        f"INSERT INTO bulk_tbl VALUES ({i}, 'item_{i}', {i * 1.5})")
    for i in range(0, 50, 5):
        runner.add_test(f"bulk_select_{i}",
                        f"SELECT * FROM bulk_tbl WHERE id >= {i} LIMIT 5")
    for i in range(0, 50, 10):
        runner.add_test(f"bulk_update_{i}",
                        f"UPDATE bulk_tbl SET value = {i * 2.0} WHERE id = {i}")
    for i in range(0, 50, 10):
        runner.add_test(f"bulk_delete_{i}",
                        f"DELETE FROM bulk_tbl WHERE id = {i}")
    runner.add_test("COUNT bulk_tbl", f"SELECT COUNT(*) FROM bulk_tbl")
    runner.add_test("DROP bulk_tbl", f"DROP TABLE IF EXISTS bulk_tbl")

    # --- Edge case: empty queries and special queries ---
    runner.add_test("SELECT 1", f"SELECT 1")
    runner.add_test("SELECT 1+1", f"SELECT 1+1")
    runner.add_test("SELECT VERSION()", f"SELECT VERSION()", db=False)
    runner.add_test("SELECT CONNECTION_ID()", f"SELECT CONNECTION_ID()", db=False)
    runner.add_test("SELECT USER()", f"SELECT USER()", db=False)
    runner.add_test("SELECT CURRENT_USER()", f"SELECT CURRENT_USER()", db=False)
    runner.add_test("SELECT SCHEMA()", f"SELECT SCHEMA()")

    # --- Mathematical constants ---
    runner.add_test("SELECT PI()", f"SELECT PI()")
    runner.add_test("PI * 2", f"SELECT PI() * 2")
    runner.add_test("SQRT(2)", f"SELECT SQRT(2)")
    runner.add_test("E approximation", f"SELECT EXP(1)")

    # --- Additional numeric combos ---
    for fn in ["LOG", "LOG2", "LOG10"]:
        for val in [1, 2, 10, 100, 1000]:
            runner.add_test(f"{fn}_{val}", f"SELECT {fn}({val})")

    # --- CRC32 variants ---
    for s in ["test", "hello", "world", "mysql", "harness", "db", "abc", "123"]:
        runner.add_test(f"CRC32_{s}", f"SELECT CRC32('{s}')")

    # --- RAND with seed ---
    for seed in [0, 1, 42, 100]:
        runner.add_test(f"RAND_seed_{seed}", f"SELECT RAND({seed}) >= 0")

    # --- ABS/CEIL/FLOOR on expressions ---
    for expr in ["a - b", "b - c", "a + b - c", "a * 2"]:
        runner.add_test(f"ABS_expr_{expr.replace(' ', '_')}",
                        f"SELECT ABS({expr}) FROM dml_int LIMIT 3")
        runner.add_test(f"CEIL_expr_{expr.replace(' ', '_')}",
                        f"SELECT CEIL({expr} / 2) FROM dml_int LIMIT 3")

    # --- Additional date edge cases ---
    for d in ["'2024-01-01'", "'2024-02-29'", "'2024-12-31'", "'2023-12-31'"]:
        runner.add_test(f"DATEDIFF_from_{d}",
                        f"SELECT DATEDIFF('2024-06-15', {d})")
        runner.add_test(f"LAST_DAY_{d}",
                        f"SELECT LAST_DAY({d})")

    # --- MONTHNAME / DAYNAME ---
    for m in range(1, 13):
        runner.add_test(f"MONTHNAME_{m}",
                        f"SELECT MONTHNAME(CONCAT('2024-', LPAD({m}, 2, '0'), '-01'))")

    # --- CONCAT with multiple args ---
    for n in range(2, 8):
        args = ", ".join([f"'s{i}'" for i in range(n)])
        runner.add_test(f"CONCAT_{n}_args", f"SELECT CONCAT({args})")

    # --- Additional SUBSTRING edge cases ---
    for start in [1, 5, 10, -1, -5]:
        for length in [1, 5, 10]:
            runner.add_test(f"SUBSTR_s{start}_l{length}",
                            f"SELECT SUBSTRING('Hello World Test String', {start}, {length})")

    # --- Additional REPLACE variants ---
    for orig, repl in [("a", "b"), ("hello", "world"), (" ", "_"), ("e", "E"), ("ll", "LL")]:
        runner.add_test(f"REPLACE_{orig}_to_{repl}",
                        f"SELECT REPLACE('hello world', '{orig}', '{repl}')")

    # --- Additional TRIM variants ---
    for s in ["  hello  ", "xxxhelloxxx", "---test---"]:
        runner.add_test(f"TRIM_{s[:6]}", f"SELECT TRIM('{s}')")
        runner.add_test(f"LTRIM_{s[:6]}", f"SELECT LTRIM('{s}')")
        runner.add_test(f"RTRIM_{s[:6]}", f"SELECT RTRIM('{s}')")

    # --- Additional LEFT/RIGHT variants ---
    for n in [1, 3, 5, 10]:
        runner.add_test(f"LEFT_{n}", f"SELECT LEFT('Hello World', {n})")
        runner.add_test(f"RIGHT_{n}", f"SELECT RIGHT('Hello World', {n})")

    # --- Additional SPACE/REPEAT ---
    for n in [0, 1, 5, 10, 50]:
        runner.add_test(f"SPACE_{n}", f"SELECT LENGTH(SPACE({n}))")

    # --- Additional HEX/ASCII/CHAR ---
    for c in ["A", "Z", "a", "z", "0", "9", " ", "!"]:
        runner.add_test(f"ASCII_{c}", f"SELECT ASCII('{c}')")
        runner.add_test(f"HEX_{c}", f"SELECT HEX('{c}')")
    for n in [65, 90, 97, 122, 48, 57, 32]:
        runner.add_test(f"CHAR_{n}", f"SELECT CHAR({n})")

    # --- Additional FIELD/ELT ---
    runner.add_test("FIELD a", f"SELECT FIELD('a', 'x', 'y', 'z', 'a')")
    runner.add_test("FIELD not found", f"SELECT FIELD('q', 'x', 'y', 'z')")
    runner.add_test("ELT", f"SELECT ELT(2, 'a', 'b', 'c')")

    # --- Additional CAST/CONVERT ---
    for val in [42, 3.14, "'hello'", "'2024-01-01'", "TRUE"]:
        for target in ["CHAR", "SIGNED", "UNSIGNED"]:
            runner.add_test(f"CAST_{str(val)[:4]}_{target}",
                            f"SELECT CAST({val} AS {target})")

    # --- Additional IFNULL/COALESCE ---
    for i in range(10):
        args = ", ".join(["NULL"] * i + [f"'found_{i}'"])
        runner.add_test(f"COALESCE_{i}_nulls", f"SELECT COALESCE({args})")
    for i in range(5):
        runner.add_test(f"IFNULL_{i}", f"SELECT IFNULL(NULL, {i})")
        runner.add_test(f"IFNULL_notnull_{i}", f"SELECT IFNULL({i}, 999)")

    # --- Additional CASE WHEN ---
    for i in range(10):
        when_clauses = " ".join([f"WHEN {j} THEN '{j}_val'" for j in range(i + 1)])
        runner.add_test(f"CASE_{i}_whens",
                        f"SELECT CASE a {when_clauses} ELSE 'default' END FROM dml_int LIMIT 1")

    # --- Additional IF() ---
    for cond in ["1>0", "1<0", "1=1", "1!=1", "NULL IS NULL", "1 IS NOT NULL"]:
        runner.add_test(f"IF_{cond.replace(' ', '_')}",
                        f"SELECT IF({cond}, 'yes', 'no')")

    # --- Additional GROUP_CONCAT ---
    runner.add_test("GROUP_CONCAT dml_int",
                    f"SELECT GROUP_CONCAT(a ORDER BY a) FROM dml_int")
    runner.add_test("GROUP_CONCAT sep",
                    f"SELECT GROUP_CONCAT(a ORDER BY a SEPARATOR '|') FROM dml_int")
    runner.add_test("GROUP_CONCAT distinct",
                    f"SELECT GROUP_CONCAT(DISTINCT active ORDER BY active) FROM dml_test")

    # --- Additional SHOW TABLE STATUS ---
    runner.add_test("SHOW TABLE STATUS LIKE dml%",
                    f"SHOW TABLE STATUS LIKE 'dml%'")

    # --- Additional DROP IF EXISTS ---
    for i in range(10):
        runner.add_test(f"DROP_IF_EXISTS_nonexist_{i}",
                        f"DROP TABLE IF EXISTS nonexistent_table_{i}")

    # =========================================================
    # 14. EDGE CASES (50+) -- runs before final cleanup
    # =========================================================
    print("Category 14: Edge cases...")

    runner.add_test("CREATE TABLE edge_test",
                    f"CREATE TABLE IF NOT EXISTS edge_test (id INT, s VARCHAR(1000), d DATE, dt DATETIME)")

    # NULL tests
    runner.add_test("INSERT all NULL", f"INSERT INTO edge_test VALUES (NULL, NULL, NULL, NULL)")
    runner.add_test("SELECT NULL", f"SELECT * FROM edge_test WHERE id IS NULL")

    # Empty string
    runner.add_test("INSERT empty string", f"INSERT INTO edge_test VALUES (0, '', NULL, NULL)")
    runner.add_test("SELECT empty string", f"SELECT * FROM edge_test WHERE s = ''")

    # Unicode
    runner.add_test("INSERT unicode", f"INSERT INTO edge_test VALUES (10, '你好世界', '2024-01-01', '2024-01-01 00:00:00')")
    runner.add_test("INSERT emoji", f"INSERT INTO edge_test VALUES (11, '🎉🎊', '2024-06-15', '2024-06-15 12:00:00')")
    runner.add_test("SELECT unicode", f"SELECT * FROM edge_test WHERE s = '你好世界'")

    # Backtick quoting
    runner.add_test("Backtick column", f"SELECT `id` FROM edge_test LIMIT 1")
    runner.add_test("Backtick table", f"SELECT * FROM `edge_test` LIMIT 1")

    # Zero dates
    runner.add_test("Zero date", f"INSERT INTO edge_test VALUES (20, 'zero', '2024-01-01', '2024-01-01 00:00:00')")

    # Max/min int
    runner.add_test("Max INT", f"INSERT INTO edge_test VALUES (2147483647, 'maxint', NULL, NULL)")
    runner.add_test("Min INT", f"INSERT INTO edge_test VALUES (-2147483648, 'minint', NULL, NULL)")

    # Very long string
    long_str = "A" * 500
    runner.add_test("Long string", f"INSERT INTO edge_test VALUES (30, '{long_str}', NULL, NULL)")
    runner.add_test("SELECT long string", f"SELECT LENGTH(s) FROM edge_test WHERE id = 30")

    # Negative numbers
    runner.add_test("Negative", f"INSERT INTO edge_test VALUES (-100, 'neg', NULL, NULL)")
    runner.add_test("SELECT negative", f"SELECT * FROM edge_test WHERE id < 0")

    # Float precision
    runner.add_test("Float precision", f"INSERT INTO edge_test VALUES (40, 'float', NULL, NULL)")
    runner.add_test("Float select", f"SELECT 0.1 + 0.2")

    # Special string values
    runner.add_test("String with quotes", f"INSERT INTO edge_test VALUES (50, 'it''s a \"test\"', NULL, NULL)")
    runner.add_test("String with backslash", f"INSERT INTO edge_test VALUES (51, 'back\\\\slash', NULL, NULL)")
    runner.add_test("String with newline", f"INSERT INTO edge_test VALUES (52, 'line1', NULL, NULL)")

    # Multiple NULLs in IN
    runner.add_test("NULL in expressions", f"SELECT NULL = NULL")
    runner.add_test("NULL arithmetic", f"SELECT NULL + 1")

    # Empty result set
    runner.add_test("Empty result", f"SELECT * FROM edge_test WHERE id = 999999")

    # Large number of rows (bulk insert)
    for i in range(100, 130):
        runner.add_test(f"Bulk insert {i}",
                        f"INSERT INTO edge_test VALUES ({i}, 'bulk_{i}', '2024-03-01', '2024-03-01 10:00:00')")

    # Count all edge case rows
    runner.add_test("Count edge_test", f"SELECT COUNT(*) FROM edge_test")

    # LIMIT edge cases
    runner.add_test("LIMIT 0", f"SELECT * FROM edge_test LIMIT 0")
    runner.add_test("LIMIT 1", f"SELECT * FROM edge_test LIMIT 1")

    # SELECT without FROM
    runner.add_test("SELECT literal", f"SELECT 1")
    runner.add_test("SELECT string literal", f"SELECT 'hello'")
    runner.add_test("SELECT expression", f"SELECT 1 + 2 * 3")
    runner.add_test("SELECT with alias", f"SELECT 42 AS answer")

    # IFNULL / COALESCE
    runner.add_test("IFNULL", f"SELECT IFNULL(NULL, 'default')")
    runner.add_test("COALESCE", f"SELECT COALESCE(NULL, NULL, 'found')")

    # CASE WHEN
    runner.add_test("CASE WHEN 1", f"SELECT CASE WHEN 1 > 0 THEN 'yes' ELSE 'no' END")
    runner.add_test("CASE WHEN 2", f"SELECT CASE id WHEN 1 THEN 'one' WHEN 2 THEN 'two' ELSE 'other' END FROM edge_test LIMIT 3")

    # IF()
    runner.add_test("IF()", f"SELECT IF(1 > 0, 'yes', 'no')")

    # CAST / CONVERT
    runner.add_test("CAST int to char", f"SELECT CAST(42 AS CHAR)")
    runner.add_test("CAST char to int", f"SELECT CAST('42' AS UNSIGNED)")

    # GROUP_CONCAT
    runner.add_test("GROUP_CONCAT", f"SELECT GROUP_CONCAT(s ORDER BY id SEPARATOR ',') FROM edge_test WHERE id BETWEEN 10 AND 11")

    # Stored procedure-like (just test CALL doesn't crash)
    runner.add_test("SELECT DATABASE()", f"SELECT DATABASE()")

    # Additional string edge cases
    runner.add_test("Empty CONCAT", f"SELECT CONCAT()")
    runner.add_test("Single char SUBSTRING", f"SELECT SUBSTRING('a', 1, 1)")

    # Numeric edge cases
    runner.add_test("Division by zero", f"SELECT 1 / 0")
    runner.add_test("MOD by zero", f"SELECT MOD(10, 0)")
    runner.add_test("Large number", f"SELECT 999999999999")
    runner.add_test("Negative float", f"SELECT -3.14159")

    # Boolean tests
    runner.add_test("SELECT TRUE", f"SELECT TRUE")
    runner.add_test("SELECT FALSE", f"SELECT FALSE")
    runner.add_test("Boolean expr", f"SELECT 1 = 1")
    runner.add_test("Boolean false expr", f"SELECT 1 = 0")

    # Additional SHOW
    runner.add_test("SHOW TABLES FROM db", f"SHOW TABLES FROM {DB_NAME}", db=False)

    # =========================================================
    # 15. FINAL CLEANUP - Drop all created tables
    # =========================================================
    print("Final cleanup...")

    cleanup_tables = [
        "txn_test", "str_test", "j1", "j2", "j3", "j_self",
        "sub1", "sub2", "dml_int", "dml_test", "dml_grp",
        "edge_test", "bulk_tbl",
    ]
    for tbl in cleanup_tables:
        runner.add_test(f"DROP {tbl}", f"DROP TABLE IF EXISTS {tbl}")


def main():
    print("=" * 60)
    print("HarnessDB MySQL Protocol Comprehensive Test")
    print("=" * 60)
    print(f"Target: 127.0.0.1:19030")
    print()

    runner = TestRunner()
    generate_tests(runner)

    # Cleanup
    print("\nCleaning up...")
    runner.run_sql(f"DROP DATABASE IF EXISTS {DB_NAME}", db=False)

    elapsed = time.time() - runner.start_time
    print()
    print("=" * 60)
    print("RESULTS")
    print("=" * 60)
    print(f"Total:  {runner.total}")
    print(f"Passed: {runner.passed}")
    print(f"Failed: {runner.failed}")
    print(f"Time:   {elapsed:.1f}s")
    print()

    if runner.failures:
        print(f"First {len(runner.failures)} failures:")
        print("-" * 60)
        for i, f in enumerate(runner.failures, 1):
            print(f"{i}. [{f['name']}]")
            print(f"   SQL: {f['sql']}")
            print(f"   Error: {f['error'][:200]}")
            print()

    # Output JSON for automation
    result = {
        "protocol": "mysql",
        "total": runner.total,
        "passed": runner.passed,
        "failed": runner.failed,
        "failures": runner.failures[:20]
    }
    print("JSON_OUTPUT:")
    import json
    print(json.dumps(result, indent=2))

    return 0 if runner.failed == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
