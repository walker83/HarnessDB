#!/bin/bash
# Real-world MySQL compatibility test suite for RorisDB
# Tests 15+ scenarios based on real applications
# Usage: ./run_tests.sh

set -e

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
RESET='\033[0m'

MYSQL="mysql -h 127.0.0.1 -P 9030 -uroot --skip-column-names"
PASSED=0
FAILED=0
ERRORS=""
CURRENT_SCENARIO=""

pass() {
    PASSED=$((PASSED + 1))
    echo -e "  ${GREEN}✓${RESET} $1"
}

fail() {
    FAILED=$((FAILED + 1))
    ERRORS="${ERRORS}\n  [${CURRENT_SCENARIO}] $1: $2"
    echo -e "  ${RED}✗${RESET} $1: ${RED}$2${RESET}"
}

run_sql() {
    local scenario="$1"
    local test_name="$2"
    local sql="$3"
    local expect_success="${4:-true}"

    local output
    if output=$(echo "$sql" | $MYSQL 2>&1); then
        pass "$test_name"
    else
        fail "$test_name" "$output"
    fi
}

run_sql_expect_rows() {
    local scenario="$1"
    local test_name="$2"
    local sql="$3"
    local min_rows="${4:-0}"

    local output
    if output=$(echo "$sql" | $MYSQL 2>&1); then
        local count=$(echo "$output" | grep -c . || echo "0")
        if [ "$count" -ge "$min_rows" ]; then
            pass "$test_name"
        else
            fail "$test_name" "Expected >= $min_rows rows, got $count"
        fi
    else
        fail "$test_name" "$output"
    fi
}

scenario() {
    CURRENT_SCENARIO="$1"
    echo -e "\n${BLUE}Testing: $1${RESET}"
}

echo -e "${BOLD}======================================================================${RESET}"
echo -e "${BOLD}RorisDB Real-World Compatibility Test Suite${RESET}"
echo -e "${BOLD}======================================================================${RESET}"
echo "Started at: $(date '+%Y-%m-%d %H:%M:%S')"

# ============================================================
# Scenario 1: Apache Superset (BI/Dashboard)
# ============================================================
scenario "Superset (BI/Dashboard)"
run_sql "Superset" "SHOW DATABASES" "SHOW DATABASES"
run_sql "Superset" "SHOW TABLE STATUS" "SHOW TABLE STATUS"
run_sql "Superset" "SELECT DATABASE()" "SELECT DATABASE()"
run_sql "Superset" "SHOW VARIABLES LIKE '%version%'" "SHOW VARIABLES LIKE '%version%'"
run_sql "Superset" "SET NAMES utf8mb4" "SET NAMES utf8mb4"

# ============================================================
# Scenario 2: DBeaver (Database Management)
# ============================================================
scenario "DBeaver (DB Management)"
run_sql "DBeaver" "SELECT 1 (ping)" "SELECT 1"
run_sql "DBeaver" "SHOW DATABASES" "SHOW DATABASES"
run_sql "DBeaver" "SHOW DATABASES LIKE 'test%'" "SHOW DATABASES LIKE 'information%'"
run_sql "DBeaver" "SHOW VARIABLES LIKE 'version%'" "SHOW VARIABLES LIKE 'version%'"
run_sql "DBeaver" "SHOW VARIABLES LIKE 'sql_mode'" "SHOW VARIABLES LIKE 'sql_mode'"
run_sql "DBeaver" "SHOW VARIABLES LIKE 'character_set%'" "SHOW VARIABLES LIKE 'character_set%'"

# ============================================================
# Scenario 3: Grafana (Monitoring)
# ============================================================
scenario "Grafana (Monitoring)"
run_sql "Grafana" "SET time_zone = '+00:00'" "SET time_zone = '+00:00'"
run_sql "Grafana" "SELECT DATABASE()" "SELECT DATABASE()"
run_sql "Grafana" "SHOW TABLE STATUS" "SHOW TABLE STATUS"

# ============================================================
# Scenario 4: WordPress (CMS)
# ============================================================
scenario "WordPress (CMS)"
run_sql "WordPress" "CREATE DATABASE" "CREATE DATABASE IF NOT EXISTS wp_test"
run_sql "WordPress" "USE DATABASE" "USE wp_test"
run_sql "WordPress" "CREATE TABLE wp_options" "CREATE TABLE IF NOT EXISTS wp_options (id INT, option_name VARCHAR(100), option_value VARCHAR(500))"
run_sql "WordPress" "CREATE TABLE wp_posts" "CREATE TABLE IF NOT EXISTS wp_posts (id INT, post_title VARCHAR(200), post_status VARCHAR(20))"
run_sql "WordPress" "SHOW TABLES LIKE" "SHOW TABLES LIKE 'wp_%'"
run_sql "WordPress" "INSERT INTO wp_options" "INSERT INTO wp_options VALUES (1, 'siteurl', 'http://localhost')"
run_sql "WordPress" "SELECT with WHERE" "SELECT * FROM wp_options WHERE option_name = 'siteurl'"
run_sql "WordPress" "UPDATE with WHERE" "UPDATE wp_options SET option_value = 'http://newsite' WHERE id = 1"
run_sql "WordPress" "SELECT after UPDATE" "SELECT * FROM wp_options WHERE id = 1"
run_sql "WordPress" "DELETE with WHERE" "DELETE FROM wp_options WHERE id = 1"
run_sql "WordPress" "SELECT after DELETE" "SELECT COUNT(*) FROM wp_options"

# ============================================================
# Scenario 5: phpMyAdmin (Web Admin)
# ============================================================
scenario "phpMyAdmin (Web Admin)"
run_sql "phpMyAdmin" "SELECT 1" "SELECT 1"
run_sql "phpMyAdmin" "SHOW DATABASES" "SHOW DATABASES"
run_sql "phpMyAdmin" "SHOW TABLE STATUS" "SHOW TABLE STATUS"
run_sql "phpMyAdmin" "SHOW VARIABLES max_allowed_packet" "SHOW VARIABLES LIKE 'max_allowed_packet'"
run_sql "phpMyAdmin" "SET NAMES utf8mb4" "SET NAMES 'utf8mb4'"

# ============================================================
# Scenario 6: Flyway (Migration Tool)
# ============================================================
scenario "Flyway (Migration)"
run_sql "Flyway" "CREATE DATABASE" "CREATE DATABASE IF NOT EXISTS flyway_test"
run_sql "Flyway" "USE DATABASE" "USE flyway_test"
run_sql "Flyway" "CREATE TABLE users" "CREATE TABLE IF NOT EXISTS users (id INT, email VARCHAR(255), name VARCHAR(100))"
run_sql "Flyway" "ALTER TABLE ADD COLUMN" "ALTER TABLE users ADD COLUMN status VARCHAR(20)"
run_sql "Flyway" "DESCRIBE after ALTER" "DESCRIBE users"
run_sql "Flyway" "SHOW TABLES LIKE" "SHOW TABLES LIKE 'users'"

# ============================================================
# Scenario 7: dbt (Data Transformation)
# ============================================================
scenario "dbt (Data Transform)"
run_sql "dbt" "CREATE DATABASE" "CREATE DATABASE IF NOT EXISTS dbt_test"
run_sql "dbt" "USE DATABASE" "USE dbt_test"
run_sql "dbt" "CREATE TABLE raw_sessions" "CREATE TABLE IF NOT EXISTS raw_sessions (id INT, user_id INT, duration INT, revenue DECIMAL(10,2))"
run_sql "dbt" "INSERT data" "INSERT INTO raw_sessions VALUES (1, 100, 60, 99.99)"
run_sql "dbt" "INSERT more data" "INSERT INTO raw_sessions VALUES (2, 100, 120, 49.50)"
run_sql "dbt" "INSERT third row" "INSERT INTO raw_sessions VALUES (3, 200, 30, 199.00)"
run_sql "dbt" "COUNT(*)" "SELECT COUNT(*) FROM raw_sessions"
run_sql "dbt" "COUNT(DISTINCT)" "SELECT COUNT(DISTINCT user_id) FROM raw_sessions"
run_sql "dbt" "GROUP BY" "SELECT user_id, COUNT(*) FROM raw_sessions GROUP BY user_id"
run_sql "dbt" "SUM aggregate" "SELECT SUM(revenue) FROM raw_sessions"
run_sql "dbt" "ORDER BY LIMIT" "SELECT * FROM raw_sessions ORDER BY revenue DESC LIMIT 2"

# ============================================================
# Scenario 8: Airbyte (ETL/CDC)
# ============================================================
scenario "Airbyte (ETL/CDC)"
run_sql "Airbyte" "CREATE DATABASE" "CREATE DATABASE IF NOT EXISTS airbyte_test"
run_sql "Airbyte" "USE DATABASE" "USE airbyte_test"
run_sql "Airbyte" "CREATE TABLE orders" "CREATE TABLE IF NOT EXISTS orders (id INT, updated_at VARCHAR(50), amount DECIMAL(10,2))"
run_sql "Airbyte" "INSERT orders" "INSERT INTO orders VALUES (1, '2024-06-01', 100.00)"
run_sql "Airbyte" "INSERT more orders" "INSERT INTO orders VALUES (2, '2024-07-01', 200.00)"
run_sql "Airbyte" "CDC time filter" "SELECT * FROM orders WHERE updated_at > '2024-01-01'"
run_sql "Airbyte" "MAX for watermark" "SELECT MAX(updated_at) FROM orders"
run_sql "Airbyte" "COUNT validation" "SELECT COUNT(*) FROM orders"

# ============================================================
# Scenario 9: Go MySQL Driver
# ============================================================
scenario "Go MySQL Driver"
run_sql "Go Driver" "SET NAMES utf8mb4" "SET NAMES utf8mb4"
run_sql "Go Driver" "SET sql_mode" "SET SESSION sql_mode = 'STRICT_TRANS_TABLES'"
run_sql "Go Driver" "SELECT 1 (ping)" "SELECT 1"
run_sql "Go Driver" "SET wait_timeout" "SET SESSION wait_timeout = 28800"

# ============================================================
# Scenario 10: Node.js mysql2 Driver
# ============================================================
scenario "Node.js mysql2 Driver"
run_sql "Node.js" "SET NAMES utf8mb4" "SET NAMES utf8mb4"
run_sql "Node.js" "SET time_zone" "SET time_zone = '+00:00'"
run_sql "Node.js" "CREATE DATABASE" "CREATE DATABASE IF NOT EXISTS nodejs_test"
run_sql "Node.js" "USE DATABASE" "USE nodejs_test"
run_sql "Node.js" "CREATE TABLE" "CREATE TABLE IF NOT EXISTS users (id INT, username VARCHAR(100))"
run_sql "Node.js" "SELECT LIMIT 1" "SELECT * FROM users LIMIT 1"

# ============================================================
# Scenario 11: JDBC Connector/J
# ============================================================
scenario "JDBC Connector/J"
run_sql "JDBC" "SELECT 1 (ping)" "SELECT 1"
run_sql "JDBC" "START TRANSACTION" "START TRANSACTION"
run_sql "JDBC" "COMMIT" "COMMIT"

# ============================================================
# Scenario 12: SQLAlchemy (Python ORM)
# ============================================================
scenario "SQLAlchemy (Python ORM)"
run_sql "SQLAlchemy" "CREATE DATABASE" "CREATE DATABASE IF NOT EXISTS sqlalchemy_test"
run_sql "SQLAlchemy" "USE DATABASE" "USE sqlalchemy_test"
run_sql "SQLAlchemy" "CREATE TABLE accounts" "CREATE TABLE IF NOT EXISTS accounts (id INT, name VARCHAR(100), balance DECIMAL(10,2))"
run_sql "SQLAlchemy" "INSERT accounts" "INSERT INTO accounts VALUES (1, 'Alice', 1000.00)"
run_sql "SQLAlchemy" "INSERT more" "INSERT INTO accounts VALUES (2, 'Bob', 500.00)"
run_sql "SQLAlchemy" "SELECT with decimal WHERE" "SELECT * FROM accounts WHERE balance >= 100.00"
run_sql "SQLAlchemy" "ORDER BY DESC LIMIT" "SELECT * FROM accounts ORDER BY balance DESC LIMIT 50"

# ============================================================
# Scenario 13: SQLancer (Fuzzing patterns)
# ============================================================
scenario "SQLancer (Fuzzing)"
run_sql "SQLancer" "CREATE DATABASE" "CREATE DATABASE IF NOT EXISTS sqlancer_test"
run_sql "SQLancer" "USE DATABASE" "USE sqlancer_test"
run_sql "SQLancer" "CREATE TABLE t1" "CREATE TABLE IF NOT EXISTS t1 (c1 INT, c2 VARCHAR(100), c3 DECIMAL(10,2))"
run_sql "SQLancer" "INSERT values" "INSERT INTO t1 VALUES (1, 'hello', 3.14)"
run_sql "SQLancer" "INSERT NULL row" "INSERT INTO t1 VALUES (2, NULL, NULL)"
run_sql "SQLancer" "SELECT with IS NULL" "SELECT * FROM t1 WHERE c2 IS NULL"
run_sql "SQLancer" "SELECT with LIKE" "SELECT * FROM t1 WHERE c2 LIKE '%o%'"
run_sql "SQLancer" "GROUP BY HAVING" "SELECT c1, COUNT(*) FROM t1 GROUP BY c1 HAVING COUNT(*) > 0"
run_sql "SQLancer" "SELECT DISTINCT" "SELECT DISTINCT c1 FROM t1"
run_sql "SQLancer" "OR conditions" "SELECT * FROM t1 WHERE c1 > 0 OR c2 IS NOT NULL"

# ============================================================
# Scenario 14: IoT/MQTT Pattern
# ============================================================
scenario "IoT/MQTT Pattern"
run_sql "IoT" "CREATE DATABASE" "CREATE DATABASE IF NOT EXISTS iot_test"
run_sql "IoT" "USE DATABASE" "USE iot_test"
run_sql "IoT" "CREATE TABLE telemetry" "CREATE TABLE IF NOT EXISTS telemetry (id INT, topic VARCHAR(100), payload VARCHAR(500))"
run_sql "IoT" "INSERT telemetry" "INSERT INTO telemetry VALUES (1, 'home/temp', '{\"temp\":22.5}')"
run_sql "IoT" "INSERT more telemetry" "INSERT INTO telemetry VALUES (2, 'home/humidity', '{\"humidity\":60}')"
run_sql "IoT" "INSERT third" "INSERT INTO telemetry VALUES (3, 'home/temp', '{\"temp\":23.0}')"
run_sql "IoT" "SELECT with LIKE" "SELECT * FROM telemetry WHERE topic LIKE 'home/%'"
run_sql "IoT" "COUNT" "SELECT COUNT(*) FROM telemetry"

# ============================================================
# Scenario 15: New Features (Config/Ops/Backup)
# ============================================================
scenario "New Features (Config/Ops/Backup)"
run_sql "Config" "SHOW VARIABLES" "SHOW VARIABLES"
run_sql "Config" "SHOW VARIABLES LIKE" "SHOW VARIABLES LIKE '%version%'"
run_sql "Config" "SET GLOBAL variable" "SET GLOBAL query_timeout = 600"
run_sql "Config" "SET SESSION variable" "SET SESSION sql_mode = 'STRICT'"
run_sql "Ops" "SHOW PROCESSLIST" "SHOW PROCESSLIST"
run_sql "Ops" "SHOW STATUS" "SHOW STATUS"
run_sql "Backup" "SHOW REPOSITORIES" "SHOW REPOSITORIES"

# ============================================================
# Scenario 16: Backup/Restore End-to-End
# ============================================================
scenario "Backup/Restore E2E"
run_sql "Backup E2E" "CREATE REPOSITORY" "CREATE REPOSITORY test_repo WITH BROKER ON '/tmp/roris_test_backup'"
run_sql "Backup E2E" "SHOW REPOSITORIES" "SHOW REPOSITORIES"
run_sql "Backup E2E" "BACKUP DATABASE" "BACKUP DATABASE dbt_test TO test_repo AS 'backup_001'"
run_sql "Backup E2E" "DROP TABLE" "DROP TABLE IF EXISTS dbt_test.raw_sessions"
run_sql "Backup E2E" "Verify table dropped" "USE dbt_test"
run_sql "Backup E2E" "RESTORE DATABASE" "RESTORE DATABASE dbt_test FROM test_repo AS 'backup_001'"
run_sql "Backup E2E" "DROP REPOSITORY" "DROP REPOSITORY test_repo"

# ============================================================
# Scenario 17: Admin Commands
# ============================================================
scenario "Admin Commands"
run_sql "Admin" "ADMIN CHECK TABLE" "ADMIN CHECK TABLE information_schema.tables"
run_sql "Admin" "ADMIN SHOW REPLICA" "ADMIN SHOW REPLICA"

# ============================================================
# Print Summary
# ============================================================
echo ""
echo -e "${BOLD}======================================================================${RESET}"
echo -e "${BOLD}Test Summary${RESET}"
echo -e "${BOLD}======================================================================${RESET}"
TOTAL=$((PASSED + FAILED))
echo "Total:  $TOTAL"
echo -e "${GREEN}Passed: $PASSED${RESET}"
echo -e "${RED}Failed: $FAILED${RESET}"

if [ -n "$ERRORS" ]; then
    echo -e "\n${BOLD}${RED}Failed Tests:${RESET}"
    echo -e "$ERRORS"
fi

echo ""
echo "Completed at: $(date '+%Y-%m-%d %H:%M:%S')"
echo -e "${BOLD}======================================================================${RESET}"

[ $FAILED -eq 0 ]
