#!/bin/bash
# AnalyticDB MySQL Protocol CRUD Test for HarnessDB
# Tests ADB MySQL-compatible MPP analytical queries
# Usage: ./adb_mysql_crud_test.sh [port]

set -e

PORT="${1:-8124}"
HOST="127.0.0.1"
PASSED=0
FAILED=0
TOTAL=0

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
RESET='\033[0m'

ADB="mysql -h ${HOST} -P ${PORT} -uroot --skip-column-names 2>/dev/null"

pass() {
    PASSED=$((PASSED + 1))
    TOTAL=$((TOTAL + 1))
    echo -e "  ${GREEN}✓${RESET} $1"
}

fail() {
    FAILED=$((FAILED + 1))
    TOTAL=$((TOTAL + 1))
    echo -e "  ${RED}✗${RESET} $1: ${RED}$2${RESET}"
}

run_sql() {
    echo "$1" | $ADB 2>&1
}

echo -e "${BOLD}======================================================================${RESET}"
echo -e "${BOLD}HarnessDB AnalyticDB MySQL Protocol CRUD Test${RESET}"
echo -e "${BOLD}======================================================================${RESET}"
echo "Port: $PORT"
echo "Started at: $(date '+%Y-%m-%d %H:%M:%S')"
echo ""

# ============================================================
# 1. Connection & Basic Queries
# ============================================================
echo -e "${BLUE}[Connection & Basic]${RESET}"

RESP=$(run_sql "SELECT 1")
if [ "$RESP" = "1" ]; then
    pass "SELECT 1"
else
    fail "SELECT 1" "Expected '1', got: '$RESP'"
fi

RESP=$(run_sql "SHOW DATABASES")
if [ -n "$RESP" ]; then
    pass "SHOW DATABASES"
else
    fail "SHOW DATABASES" "No response"
fi

# ============================================================
# 2. DDL - Database & Table
# ============================================================
echo -e "${BLUE}[DDL]${RESET}"

RESP=$(run_sql "CREATE DATABASE IF NOT EXISTS adb_test")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "CREATE DATABASE adb_test"
else
    fail "CREATE DATABASE" "Got: '$RESP'"
fi

RESP=$(run_sql "USE adb_test")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "USE adb_test"
else
    fail "USE adb_test" "Got: '$RESP'"
fi

RESP=$(run_sql "CREATE TABLE IF NOT EXISTS users (id INT, name VARCHAR(100), age INT, email VARCHAR(200))")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "CREATE TABLE users"
else
    fail "CREATE TABLE users" "Got: '$RESP'"
fi

RESP=$(run_sql "CREATE TABLE IF NOT EXISTS orders (id INT, user_id INT, amount DOUBLE, status VARCHAR(50))")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "CREATE TABLE orders"
else
    fail "CREATE TABLE orders" "Got: '$RESP'"
fi

RESP=$(run_sql "SHOW TABLES")
if echo "$RESP" | grep -qi "users"; then
    pass "SHOW TABLES (contains users)"
else
    fail "SHOW TABLES" "Got: '$RESP'"
fi

# ============================================================
# 3. INSERT (CREATE)
# ============================================================
echo -e "${BLUE}[INSERT]${RESET}"

RESP=$(run_sql "INSERT INTO users VALUES (1, 'Alice', 30, 'alice@test.com')")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "INSERT Alice"
else
    fail "INSERT Alice" "Got: '$RESP'"
fi

RESP=$(run_sql "INSERT INTO users VALUES (2, 'Bob', 25, 'bob@test.com'), (3, 'Charlie', 35, 'charlie@test.com'), (4, 'Diana', 28, 'diana@test.com')")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "INSERT 3 users (batch)"
else
    fail "INSERT batch" "Got: '$RESP'"
fi

RESP=$(run_sql "INSERT INTO orders VALUES (1, 1, 99.99, 'completed'), (2, 2, 199.50, 'pending')")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "INSERT orders"
else
    fail "INSERT orders" "Got: '$RESP'"
fi

# ============================================================
# 4. SELECT (READ)
# ============================================================
echo -e "${BLUE}[SELECT]${RESET}"

RESP=$(run_sql "SELECT COUNT(*) FROM users")
if [ "$RESP" = "4" ]; then
    pass "SELECT COUNT(*) users (4)"
else
    fail "SELECT COUNT(*)" "Expected '4', got: '$RESP'"
fi

RESP=$(run_sql "SELECT * FROM users")
ROW_COUNT=$(echo "$RESP" | grep -c . || echo "0")
if [ "$ROW_COUNT" -ge 4 ]; then
    pass "SELECT * users ($ROW_COUNT rows)"
else
    fail "SELECT * users" "Expected >= 4 rows, got $ROW_COUNT"
fi

RESP=$(run_sql "SELECT name FROM users WHERE age > 28")
if echo "$RESP" | grep -qi "Alice"; then
    pass "SELECT WHERE age > 28"
else
    fail "SELECT WHERE" "Got: '$RESP'"
fi

RESP=$(run_sql "SELECT * FROM users ORDER BY age DESC LIMIT 2")
ROW_COUNT=$(echo "$RESP" | grep -c . || echo "0")
if [ "$ROW_COUNT" -eq 2 ]; then
    pass "SELECT ORDER BY DESC LIMIT 2"
else
    fail "SELECT ORDER BY LIMIT" "Expected 2 rows, got $ROW_COUNT"
fi

# Analytical queries (ADB specialty)
RESP=$(run_sql "SELECT name, COUNT(*) as cnt FROM users GROUP BY name")
if echo "$RESP" | grep -qi "Alice"; then
    pass "SELECT GROUP BY (analytical)"
else
    fail "SELECT GROUP BY" "Got: '$RESP'"
fi

# ============================================================
# 5. UPDATE
# ============================================================
echo -e "${BLUE}[UPDATE]${RESET}"

RESP=$(run_sql "UPDATE users SET age = 31 WHERE name = 'Alice'")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "UPDATE Alice age = 31"
else
    fail "UPDATE Alice" "Got: '$RESP'"
fi

RESP=$(run_sql "SELECT age FROM users WHERE name = 'Alice'")
if [ "$RESP" = "31" ]; then
    pass "VERIFY updated age = 31"
else
    fail "VERIFY updated age" "Expected '31', got: '$RESP'"
fi

# ============================================================
# 6. DELETE
# ============================================================
echo -e "${BLUE}[DELETE]${RESET}"

RESP=$(run_sql "DELETE FROM users WHERE name = 'Bob'")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "DELETE Bob"
else
    fail "DELETE Bob" "Got: '$RESP'"
fi

RESP=$(run_sql "SELECT COUNT(*) FROM users")
if [ "$RESP" = "3" ]; then
    pass "VERIFY count after delete (3)"
else
    fail "VERIFY count after delete" "Expected '3', got: '$RESP'"
fi

# ============================================================
# 7. DROP
# ============================================================
echo -e "${BLUE}[DROP]${RESET}"

RESP=$(run_sql "DROP TABLE IF EXISTS orders")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "DROP TABLE orders"
else
    fail "DROP TABLE orders" "Got: '$RESP'"
fi

RESP=$(run_sql "DROP TABLE IF EXISTS users")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "DROP TABLE users"
else
    fail "DROP TABLE users" "Got: '$RESP'"
fi

RESP=$(run_sql "DROP DATABASE IF EXISTS adb_test")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "DROP DATABASE adb_test"
else
    fail "DROP DATABASE" "Got: '$RESP'"
fi

# ============================================================
# Summary
# ============================================================
echo ""
echo -e "${BOLD}======================================================================${RESET}"
echo -e "${BOLD}AnalyticDB MySQL CRUD Test Summary${RESET}"
echo -e "${BOLD}======================================================================${RESET}"
echo -e "Total:  $TOTAL"
echo -e "${GREEN}Passed: $PASSED${RESET}"
echo -e "${RED}Failed: $FAILED${RESET}"
echo "Completed at: $(date '+%Y-%m-%d %H:%M:%S')"
echo -e "${BOLD}======================================================================${RESET}"

[ $FAILED -eq 0 ]
