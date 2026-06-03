#!/usr/bin/env python3
"""
Real-world MySQL compatibility test suite for HarnessDB
Tests 15 scenarios based on real applications: Superset, Metabase, Grafana, DBeaver,
WordPress, phpMyAdmin, Flyway, dbt, Airbyte, Go driver, Node.js driver, JDBC,
SQLAlchemy, SQLancer, and IoT/MQTT patterns.
"""

import mysql.connector
from mysql.connector import Error
import sys
import json
import time
from datetime import datetime

class Colors:
    GREEN = '\033[92m'
    RED = '\033[91m'
    YELLOW = '\033[93m'
    BLUE = '\033[94m'
    RESET = '\033[0m'
    BOLD = '\033[1m'

class TestResult:
    def __init__(self):
        self.passed = 0
        self.failed = 0
        self.errors = []
        self.scenarios = {}

    def add_pass(self, scenario, test_name):
        self.passed += 1
        if scenario not in self.scenarios:
            self.scenarios[scenario] = {'passed': 0, 'failed': 0}
        self.scenarios[scenario]['passed'] += 1
        print(f"  {Colors.GREEN}✓{Colors.RESET} {test_name}")

    def add_fail(self, scenario, test_name, error):
        self.failed += 1
        if scenario not in self.scenarios:
            self.scenarios[scenario] = {'passed': 0, 'failed': 0}
        self.scenarios[scenario]['failed'] += 1
        self.errors.append((scenario, test_name, error))
        print(f"  {Colors.RED}✗{Colors.RESET} {test_name}: {Colors.RED}{error}{Colors.RESET}")

    def summary(self):
        print(f"\n{Colors.BOLD}{'='*70}{Colors.RESET}")
        print(f"{Colors.BOLD}Test Summary{Colors.RESET}")
        print(f"{Colors.BOLD}{'='*70}{Colors.RESET}")
        print(f"Total:  {self.passed + self.failed}")
        print(f"{Colors.GREEN}Passed: {self.passed}{Colors.RESET}")
        print(f"{Colors.RED}Failed: {self.failed}{Colors.RESET}")
        print(f"\n{Colors.BOLD}By Scenario:{Colors.RESET}")
        for scenario, counts in sorted(self.scenarios.items()):
            status = Colors.GREEN if counts['failed'] == 0 else Colors.RED
            print(f"  {status}{scenario}: {counts['passed']} passed, {counts['failed']} failed{Colors.RESET}")
        if self.errors:
            print(f"\n{Colors.BOLD}{Colors.RED}Failed Tests:{Colors.RESET}")
            for scenario, test, error in self.errors:
                print(f"  [{scenario}] {test}: {error}")
        return self.failed == 0

def connect_to_db():
    """Connect to HarnessDB"""
    try:
        conn = mysql.connector.connect(
            host='127.0.0.1',
            port=9030,
            user='root',
            password='',
            database='information_schema',
            buffered=True
        )
        return conn
    except Error as e:
        print(f"{Colors.RED}Failed to connect to HarnessDB: {e}{Colors.RESET}")
        sys.exit(1)

def execute_query(cursor, sql, scenario, test_name, results):
    """Execute a query and record the result"""
    try:
        cursor.execute(sql)
        if cursor.with_rows:
            rows = cursor.fetchall()
            results.add_pass(scenario, test_name)
            return rows
        else:
            results.add_pass(scenario, test_name)
            return None
    except Error as e:
        results.add_fail(scenario, test_name, str(e))
        return None

def test_superset_scenario(cursor, results):
    """Scenario 1: Apache Superset (BI/Dashboard)"""
    scenario = "Superset (BI/Dashboard)"
    print(f"\n{Colors.BLUE}Testing {scenario}...{Colors.RESET}")

    # Setup test database
    execute_query(cursor, "CREATE DATABASE IF NOT EXISTS test_superset", scenario, "CREATE DATABASE", results)
    execute_query(cursor, "USE test_superset", scenario, "USE DATABASE", results)
    execute_query(cursor, """CREATE TABLE IF NOT EXISTS sales (
        id INT, name VARCHAR(100), amount DECIMAL(10,2), created_at VARCHAR(50)
    )""", scenario, "CREATE TABLE sales", results)

    # Superset queries
    execute_query(cursor, "SHOW DATABASES", scenario, "SHOW DATABASES", results)
    execute_query(cursor, "SHOW TABLE STATUS", scenario, "SHOW TABLE STATUS", results)
    execute_query(cursor, "SELECT DATABASE()", scenario, "SELECT DATABASE()", results)
    execute_query(cursor, "SELECT COUNT(*) FROM sales", scenario, "COUNT(*)", results)
    execute_query(cursor, "SELECT * FROM sales LIMIT 100", scenario, "SELECT * LIMIT 100", results)

def test_metabase_scenario(cursor, results):
    """Scenario 2: Metabase (Self-service Analytics)"""
    scenario = "Metabase (Analytics)"
    print(f"\n{Colors.BLUE}Testing {scenario}...{Colors.RESET}")

    execute_query(cursor, "CREATE DATABASE IF NOT EXISTS test_metabase", scenario, "CREATE DATABASE", results)
    execute_query(cursor, "USE test_metabase", scenario, "USE DATABASE", results)
    execute_query(cursor, """CREATE TABLE IF NOT EXISTS orders (
        id INT, customer_id INT, total DECIMAL(10,2), created_at VARCHAR(50)
    )""", scenario, "CREATE TABLE orders", results)

    execute_query(cursor, "SHOW DATABASES", scenario, "SHOW DATABASES", results)
    execute_query(cursor, "SHOW TABLES", scenario, "SHOW TABLES", results)
    execute_query(cursor, "DESCRIBE orders", scenario, "DESCRIBE table", results)
    execute_query(cursor, "SELECT COUNT(*) AS count FROM orders", scenario, "COUNT with alias", results)

def test_grafana_scenario(cursor, results):
    """Scenario 3: Grafana (Monitoring)"""
    scenario = "Grafana (Monitoring)"
    print(f"\n{Colors.BLUE}Testing {scenario}...{Colors.RESET}")

    execute_query(cursor, "SET SESSION time_zone = '+00:00'", scenario, "SET time_zone", results)
    execute_query(cursor, """CREATE DATABASE IF NOT EXISTS test_grafana""", scenario, "CREATE DATABASE", results)
    execute_query(cursor, "USE test_grafana", scenario, "USE DATABASE", results)
    execute_query(cursor, """CREATE TABLE IF NOT EXISTS events (
        id INT, value INT, created_at VARCHAR(50)
    )""", scenario, "CREATE TABLE events", results)

    # Time series queries
    execute_query(cursor, "SELECT COUNT(*) FROM events", scenario, "COUNT for time series", results)
    execute_query(cursor, "SELECT * FROM events LIMIT 1000", scenario, "SELECT with LIMIT", results)

def test_dbeaver_scenario(cursor, results):
    """Scenario 4: DBeaver (Database Management)"""
    scenario = "DBeaver (DB Management)"
    print(f"\n{Colors.BLUE}Testing {scenario}...{Colors.RESET}")

    execute_query(cursor, "SELECT 1", scenario, "SELECT 1 (ping)", results)
    execute_query(cursor, "SHOW DATABASES", scenario, "SHOW DATABASES", results)
    execute_query(cursor, "SHOW DATABASES LIKE 'test%'", scenario, "SHOW DATABASES LIKE", results)
    execute_query(cursor, "SHOW VARIABLES LIKE 'version%'", scenario, "SHOW VARIABLES LIKE", results)
    execute_query(cursor, "SHOW VARIABLES LIKE 'sql_mode'", scenario, "SHOW VARIABLES sql_mode", results)

def test_wordpress_scenario(cursor, results):
    """Scenario 5: WordPress (CMS)"""
    scenario = "WordPress (CMS)"
    print(f"\n{Colors.BLUE}Testing {scenario}...{Colors.RESET}")

    execute_query(cursor, "CREATE DATABASE IF NOT EXISTS test_wordpress", scenario, "CREATE DATABASE", results)
    execute_query(cursor, "USE test_wordpress", scenario, "USE DATABASE", results)
    execute_query(cursor, """CREATE TABLE IF NOT EXISTS wp_options (
        id INT, option_name VARCHAR(100), option_value TEXT
    )""", scenario, "CREATE TABLE wp_options", results)
    execute_query(cursor, """CREATE TABLE IF NOT EXISTS wp_posts (
        id INT, post_title VARCHAR(200), post_status VARCHAR(20)
    )""", scenario, "CREATE TABLE wp_posts", results)

    # WordPress queries
    execute_query(cursor, "SHOW TABLES LIKE 'wp_%'", scenario, "SHOW TABLES LIKE", results)
    execute_query(cursor, "SELECT * FROM wp_options WHERE option_name = 'siteurl'", scenario, "SELECT with WHERE", results)
    execute_query(cursor, "INSERT INTO wp_options VALUES (1, 'test', 'value')", scenario, "INSERT INTO", results)
    execute_query(cursor, "UPDATE wp_options SET option_value = 'new_value' WHERE id = 1", scenario, "UPDATE with WHERE", results)
    execute_query(cursor, "DELETE FROM wp_options WHERE id = 1", scenario, "DELETE with WHERE", results)

def test_phpmyadmin_scenario(cursor, results):
    """Scenario 6: phpMyAdmin (Web Admin)"""
    scenario = "phpMyAdmin (Web Admin)"
    print(f"\n{Colors.BLUE}Testing {scenario}...{Colors.RESET}")

    execute_query(cursor, "SELECT 1", scenario, "SELECT 1", results)
    execute_query(cursor, "SHOW DATABASES", scenario, "SHOW DATABASES", results)
    execute_query(cursor, "SHOW TABLE STATUS", scenario, "SHOW TABLE STATUS", results)
    execute_query(cursor, "SHOW VARIABLES LIKE 'max_allowed_packet'", scenario, "SHOW VARIABLES max_allowed_packet", results)
    execute_query(cursor, "SET NAMES 'utf8mb4'", scenario, "SET NAMES", results)

def test_flyway_scenario(cursor, results):
    """Scenario 7: Flyway (Migration Tool)"""
    scenario = "Flyway (Migration)"
    print(f"\n{Colors.BLUE}Testing {scenario}...{Colors.RESET}")

    execute_query(cursor, "CREATE DATABASE IF NOT EXISTS test_flyway", scenario, "CREATE DATABASE", results)
    execute_query(cursor, "USE test_flyway", scenario, "USE DATABASE", results)

    # Migration DDL
    execute_query(cursor, """CREATE TABLE IF NOT EXISTS users (
        id INT, email VARCHAR(255), name VARCHAR(100)
    )""", scenario, "CREATE TABLE users", results)

    execute_query(cursor, """ALTER TABLE users ADD COLUMN status VARCHAR(20)""", scenario, "ALTER TABLE ADD COLUMN", results)

    execute_query(cursor, "SHOW TABLES LIKE 'flyway_schema_history'", scenario, "SHOW TABLES LIKE", results)

def test_dbt_scenario(cursor, results):
    """Scenario 8: dbt (Data Transformation)"""
    scenario = "dbt (Data Transform)"
    print(f"\n{Colors.BLUE}Testing {scenario}...{Colors.RESET}")

    execute_query(cursor, "CREATE DATABASE IF NOT EXISTS test_dbt", scenario, "CREATE DATABASE", results)
    execute_query(cursor, "USE test_dbt", scenario, "USE DATABASE", results)
    execute_query(cursor, """CREATE TABLE IF NOT EXISTS raw_sessions (
        id INT, user_id INT, duration INT, created_at VARCHAR(50)
    )""", scenario, "CREATE TABLE raw_sessions", results)

    # Aggregation queries
    execute_query(cursor, "SELECT COUNT(DISTINCT user_id) FROM raw_sessions", scenario, "COUNT DISTINCT", results)
    execute_query(cursor, "SELECT user_id, COUNT(*) FROM raw_sessions GROUP BY user_id", scenario, "GROUP BY", results)
    execute_query(cursor, "SELECT * FROM raw_sessions HAVING duration > 100", scenario, "HAVING clause", results)

def test_airbyte_scenario(cursor, results):
    """Scenario 9: Airbyte (ETL)"""
    scenario = "Airbyte (ETL)"
    print(f"\n{Colors.BLUE}Testing {scenario}...{Colors.RESET}")

    execute_query(cursor, "CREATE DATABASE IF NOT EXISTS test_airbyte", scenario, "CREATE DATABASE", results)
    execute_query(cursor, "USE test_airbyte", scenario, "USE DATABASE", results)
    execute_query(cursor, """CREATE TABLE IF NOT EXISTS orders (
        id INT, updated_at VARCHAR(50), amount DECIMAL(10,2)
    )""", scenario, "CREATE TABLE orders", results)

    # CDC queries
    execute_query(cursor, "SELECT * FROM orders WHERE updated_at > '2024-01-01'", scenario, "CDC time filter", results)
    execute_query(cursor, "SELECT MAX(updated_at) FROM orders", scenario, "MAX for watermark", results)
    execute_query(cursor, "SELECT COUNT(*) FROM orders", scenario, "COUNT for validation", results)

def test_go_driver_scenario(cursor, results):
    """Scenario 10: Go MySQL Driver"""
    scenario = "Go Driver"
    print(f"\n{Colors.BLUE}Testing {scenario}...{Colors.RESET}")

    execute_query(cursor, "SET NAMES utf8mb4", scenario, "SET NAMES utf8mb4", results)
    execute_query(cursor, "SET SESSION sql_mode = 'STRICT_TRANS_TABLES'", scenario, "SET sql_mode", results)
    execute_query(cursor, "SELECT 1", scenario, "SELECT 1 (ping)", results)
    execute_query(cursor, "SET SESSION wait_timeout = 28800", scenario, "SET wait_timeout", results)

def test_nodejs_driver_scenario(cursor, results):
    """Scenario 11: Node.js mysql2 Driver"""
    scenario = "Node.js Driver"
    print(f"\n{Colors.BLUE}Testing {scenario}...{Colors.RESET}")

    execute_query(cursor, "SET NAMES utf8mb4", scenario, "SET NAMES", results)
    execute_query(cursor, "SET time_zone = '+00:00'", scenario, "SET time_zone", results)

    execute_query(cursor, "CREATE DATABASE IF NOT EXISTS test_nodejs", scenario, "CREATE DATABASE", results)
    execute_query(cursor, "USE test_nodejs", scenario, "USE DATABASE", results)
    execute_query(cursor, """CREATE TABLE IF NOT EXISTS users (
        id INT, username VARCHAR(100)
    )""", scenario, "CREATE TABLE", results)

    execute_query(cursor, "SELECT * FROM users LIMIT 1", scenario, "SELECT LIMIT 1", results)

def test_jdbc_scenario(cursor, results):
    """Scenario 12: JDBC Connector/J"""
    scenario = "JDBC Driver"
    print(f"\n{Colors.BLUE}Testing {scenario}...{Colors.RESET}")

    execute_query(cursor, "SELECT 1", scenario, "SELECT 1 (ping)", results)
    execute_query(cursor, "SET @jdbc_variable = 42", scenario, "SET @variable", results)
    execute_query(cursor, "SELECT @jdbc_variable", scenario, "SELECT @variable", results)

    execute_query(cursor, "START TRANSACTION", scenario, "START TRANSACTION", results)
    execute_query(cursor, "COMMIT", scenario, "COMMIT", results)

def test_sqlalchemy_scenario(cursor, results):
    """Scenario 13: SQLAlchemy (Python ORM)"""
    scenario = "SQLAlchemy (Python ORM)"
    print(f"\n{Colors.BLUE}Testing {scenario}...{Colors.RESET}")

    execute_query(cursor, "CREATE DATABASE IF NOT EXISTS test_sqlalchemy", scenario, "CREATE DATABASE", results)
    execute_query(cursor, "USE test_sqlalchemy", scenario, "USE DATABASE", results)
    execute_query(cursor, """CREATE TABLE IF NOT EXISTS accounts (
        id INT, name VARCHAR(100), balance DECIMAL(10,2)
    )""", scenario, "CREATE TABLE accounts", results)

    execute_query(cursor, "SELECT * FROM accounts WHERE balance >= 100.00", scenario, "SELECT with decimal WHERE", results)
    execute_query(cursor, "SELECT * FROM accounts ORDER BY balance DESC LIMIT 50", scenario, "ORDER BY DESC LIMIT", results)

def test_sqlancer_scenario(cursor, results):
    """Scenario 14: SQLancer (Fuzzing)"""
    scenario = "SQLancer (Fuzzing)"
    print(f"\n{Colors.BLUE}Testing {scenario}...{Colors.RESET}")

    execute_query(cursor, "CREATE DATABASE IF NOT EXISTS test_sqlancer", scenario, "CREATE DATABASE", results)
    execute_query(cursor, "USE test_sqlancer", scenario, "USE DATABASE", results)
    execute_query(cursor, "CREATE TABLE IF NOT EXISTS t1 (c1 INT, c2 VARCHAR(100))", scenario, "CREATE TABLE t1", results)

    execute_query(cursor, "INSERT INTO t1 VALUES (1, 'hello')", scenario, "INSERT values", results)
    execute_query(cursor, "SELECT * FROM t1 WHERE c1 IS NULL OR c2 LIKE '%o%'", scenario, "SELECT with IS NULL and LIKE", results)
    execute_query(cursor, "SELECT c1, COUNT(*) FROM t1 GROUP BY c1 HAVING COUNT(*) > 0", scenario, "GROUP BY HAVING", results)

def test_iot_scenario(cursor, results):
    """Scenario 15: IoT/MQTT Pattern"""
    scenario = "IoT/MQTT"
    print(f"\n{Colors.BLUE}Testing {scenario}...{Colors.RESET}")

    execute_query(cursor, "CREATE DATABASE IF NOT EXISTS test_iot", scenario, "CREATE DATABASE", results)
    execute_query(cursor, "USE test_iot", scenario, "USE DATABASE", results)
    execute_query(cursor, """CREATE TABLE IF NOT EXISTS telemetry (
        id INT, topic VARCHAR(100), payload VARCHAR(500)
    )""", scenario, "CREATE TABLE telemetry", results)

    execute_query(cursor, "INSERT INTO telemetry VALUES (1, 'home/temp', '{\"temp\":22.5}')", scenario, "INSERT telemetry", results)
    execute_query(cursor, "SELECT * FROM telemetry WHERE topic LIKE 'home/%'", scenario, "SELECT with LIKE", results)
    execute_query(cursor, "SELECT COUNT(*) FROM telemetry", scenario, "COUNT telemetry", results)

def test_new_features(cursor, results):
    """Test new features: config, ops, backup, web editor"""
    scenario = "New Features"
    print(f"\n{Colors.BLUE}Testing New Features (config/ops/backup)...{Colors.RESET}")

    # Configuration system
    execute_query(cursor, "SHOW VARIABLES", scenario, "SHOW VARIABLES", results)
    execute_query(cursor, "SHOW VARIABLES LIKE '%version%'", scenario, "SHOW VARIABLES LIKE", results)
    execute_query(cursor, "SET GLOBAL query_timeout = 600", scenario, "SET GLOBAL variable", results)
    execute_query(cursor, "SET SESSION sql_mode = 'STRICT'", scenario, "SET SESSION variable", results)

    # Operations
    execute_query(cursor, "SHOW PROCESSLIST", scenario, "SHOW PROCESSLIST", results)
    execute_query(cursor, "SHOW STATUS", scenario, "SHOW STATUS", results)

    # Backup/Repository
    execute_query(cursor, "SHOW REPOSITORIES", scenario, "SHOW REPOSITORIES", results)

def main():
    print(f"{Colors.BOLD}{'='*70}{Colors.RESET}")
    print(f"{Colors.BOLD}HarnessDB Real-World Compatibility Test Suite{Colors.RESET}")
    print(f"{Colors.BOLD}{'='*70}{Colors.RESET}")
    print(f"Testing 15 real application scenarios + new features")
    print(f"Started at: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n")

    conn = connect_to_db()
    cursor = conn.cursor()
    results = TestResult()

    try:
        # Run all scenarios
        test_superset_scenario(cursor, results)
        test_metabase_scenario(cursor, results)
        test_grafana_scenario(cursor, results)
        test_dbeaver_scenario(cursor, results)
        test_wordpress_scenario(cursor, results)
        test_phpmyadmin_scenario(cursor, results)
        test_flyway_scenario(cursor, results)
        test_dbt_scenario(cursor, results)
        test_airbyte_scenario(cursor, results)
        test_go_driver_scenario(cursor, results)
        test_nodejs_driver_scenario(cursor, results)
        test_jdbc_scenario(cursor, results)
        test_sqlalchemy_scenario(cursor, results)
        test_sqlancer_scenario(cursor, results)
        test_iot_scenario(cursor, results)
        test_new_features(cursor, results)

        # Print summary
        success = results.summary()

        print(f"\n{Colors.BOLD}{'='*70}{Colors.RESET}")
        print(f"Completed at: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
        print(f"{Colors.BOLD}{'='*70}{Colors.RESET}")

        sys.exit(0 if success else 1)

    finally:
        cursor.close()
        conn.close()

if __name__ == "__main__":
    main()
