#!/bin/bash
# ClickHouse HTTP Protocol CRUD Test for HarnessDB
# Tests ClickHouse HTTP interface with TSV format
# Usage: ./clickhouse_crud_test.sh [port]

set -e

PORT="${1:-8123}"
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

ch_query() {
    curl -s "http://${HOST}:${PORT}/" -d "$1" 2>/dev/null
}

echo -e "${BOLD}======================================================================${RESET}"
echo -e "${BOLD}HarnessDB ClickHouse Protocol CRUD Test${RESET}"
echo -e "${BOLD}======================================================================${RESET}"
echo "Port: $PORT"
echo "Started at: $(date '+%Y-%m-%d %H:%M:%S')"
echo ""

# ============================================================
# 1. Connection & Version
# ============================================================
echo -e "${BLUE}[Connection & Version]${RESET}"

RESP=$(ch_query "SELECT 1")
if [ "$RESP" = "1" ]; then
    pass "SELECT 1"
else
    fail "SELECT 1" "Expected '1', got: '$RESP'"
fi

RESP=$(ch_query "SELECT version()")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "SELECT version()"
else
    fail "SELECT version()" "Got: '$RESP'"
fi

# ============================================================
# 2. Database Operations
# ============================================================
echo -e "${BLUE}[Database Operations]${RESET}"

RESP=$(ch_query "CREATE DATABASE IF NOT EXISTS ch_test")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "CREATE DATABASE ch_test"
else
    fail "CREATE DATABASE" "Got: '$RESP'"
fi

RESP=$(ch_query "SHOW DATABASES")
if echo "$RESP" | grep -qi "ch_test"; then
    pass "SHOW DATABASES (contains ch_test)"
else
    fail "SHOW DATABASES" "Got: '$RESP'"
fi

# ============================================================
# 3. Table Operations (DDL)
# ============================================================
echo -e "${BLUE}[Table DDL]${RESET}"

RESP=$(ch_query "CREATE TABLE IF NOT EXISTS ch_test.users (id UInt32, name String, age UInt32, email String) ENGINE = Memory")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "CREATE TABLE users"
else
    fail "CREATE TABLE users" "Got: '$RESP'"
fi

RESP=$(ch_query "CREATE TABLE IF NOT EXISTS ch_test.orders (id UInt32, user_id UInt32, amount Float64, status String) ENGINE = Memory")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "CREATE TABLE orders"
else
    fail "CREATE TABLE orders" "Got: '$RESP'"
fi

RESP=$(ch_query "SHOW TABLES FROM ch_test")
if echo "$RESP" | grep -qi "users"; then
    pass "SHOW TABLES (contains users)"
else
    fail "SHOW TABLES" "Got: '$RESP'"
fi

RESP=$(ch_query "DESCRIBE ch_test.users")
if echo "$RESP" | grep -qi "id"; then
    pass "DESCRIBE users"
else
    fail "DESCRIBE users" "Got: '$RESP'"
fi

# ============================================================
# 4. INSERT (CREATE)
# ============================================================
echo -e "${BLUE}[INSERT (CREATE)]${RESET}"

RESP=$(ch_query "INSERT INTO ch_test.users VALUES (1, 'Alice', 30, 'alice@test.com')")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "INSERT Alice"
else
    fail "INSERT Alice" "Got: '$RESP'"
fi

RESP=$(ch_query "INSERT INTO ch_test.users VALUES (2, 'Bob', 25, 'bob@test.com'), (3, 'Charlie', 35, 'charlie@test.com'), (4, 'Diana', 28, 'diana@test.com')")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "INSERT 3 users (batch)"
else
    fail "INSERT batch" "Got: '$RESP'"
fi

RESP=$(ch_query "INSERT INTO ch_test.orders VALUES (1, 1, 99.99, 'completed'), (2, 2, 199.50, 'pending'), (3, 1, 50.00, 'completed')")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "INSERT orders"
else
    fail "INSERT orders" "Got: '$RESP'"
fi

# ============================================================
# 5. SELECT (READ)
# ============================================================
echo -e "${BLUE}[SELECT (READ)]${RESET}"

RESP=$(ch_query "SELECT * FROM ch_test.users")
ROW_COUNT=$(echo "$RESP" | grep -c . || echo "0")
if [ "$ROW_COUNT" -ge 4 ]; then
    pass "SELECT * users ($ROW_COUNT rows)"
else
    fail "SELECT * users" "Expected >= 4 rows, got $ROW_COUNT"
fi

RESP=$(ch_query "SELECT name FROM ch_test.users WHERE age > 28")
if echo "$RESP" | grep -qi "Alice"; then
    pass "SELECT WHERE age > 28"
else
    fail "SELECT WHERE" "Got: '$RESP'"
fi

RESP=$(ch_query "SELECT COUNT(*) FROM ch_test.users")
if [ "$RESP" = "4" ]; then
    pass "SELECT COUNT(*)"
else
    fail "SELECT COUNT(*)" "Expected '4', got: '$RESP'"
fi

RESP=$(ch_query "SELECT * FROM ch_test.users ORDER BY age DESC LIMIT 2")
ROW_COUNT=$(echo "$RESP" | grep -c . || echo "0")
if [ "$ROW_COUNT" -eq 2 ]; then
    pass "SELECT ORDER BY DESC LIMIT 2"
else
    fail "SELECT ORDER BY LIMIT" "Expected 2 rows, got $ROW_COUNT"
fi

RESP=$(ch_query "SELECT name, COUNT(*) as cnt FROM ch_test.users GROUP BY name")
if echo "$RESP" | grep -qi "Alice"; then
    pass "SELECT GROUP BY"
else
    fail "SELECT GROUP BY" "Got: '$RESP'"
fi

RESP=$(ch_query "SELECT * FROM ch_test.users WHERE name LIKE '%li%'")
if echo "$RESP" | grep -qi "Alice"; then
    pass "SELECT WHERE name LIKE '%li%'"
else
    fail "SELECT LIKE" "Got: '$RESP'"
fi

# ============================================================
# 6. UPDATE (via ALTER in ClickHouse style)
# ============================================================
echo -e "${BLUE}[UPDATE]${RESET}"

RESP=$(ch_query "ALTER TABLE ch_test.users UPDATE age = 31 WHERE name = 'Alice'")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "ALTER TABLE UPDATE age=31 WHERE Alice"
else
    fail "ALTER UPDATE" "Got: '$RESP'"
fi

RESP=$(ch_query "SELECT age FROM ch_test.users WHERE name = 'Alice'")
if [ "$RESP" = "31" ]; then
    pass "VERIFY updated age=31"
else
    fail "VERIFY updated age" "Expected '31', got: '$RESP'"
fi

# ============================================================
# 7. DELETE
# ============================================================
echo -e "${BLUE}[DELETE]${RESET}"

RESP=$(ch_query "ALTER TABLE ch_test.users DELETE WHERE name = 'Bob'")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "ALTER TABLE DELETE WHERE Bob"
else
    fail "ALTER DELETE" "Got: '$RESP'"
fi

RESP=$(ch_query "SELECT COUNT(*) FROM ch_test.users")
if [ "$RESP" = "3" ]; then
    pass "VERIFY count after delete (3)"
else
    fail "VERIFY count after delete" "Expected '3', got: '$RESP'"
fi

RESP=$(ch_query "SELECT * FROM ch_test.users WHERE name = 'Bob'")
if [ -z "$RESP" ]; then
    pass "VERIFY Bob deleted (empty result)"
else
    fail "VERIFY Bob deleted" "Expected empty, got: '$RESP'"
fi

# ============================================================
# 8. DROP
# ============================================================
echo -e "${BLUE}[DROP]${RESET}"

RESP=$(ch_query "DROP TABLE IF EXISTS ch_test.orders")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "DROP TABLE orders"
else
    fail "DROP TABLE orders" "Got: '$RESP'"
fi

RESP=$(ch_query "DROP DATABASE IF EXISTS ch_test")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "DROP DATABASE ch_test"
else
    fail "DROP DATABASE" "Got: '$RESP'"
fi

RESP=$(ch_query "SHOW DATABASES")
if ! echo "$RESP" | grep -qi "ch_test"; then
    pass "VERIFY ch_test dropped"
else
    fail "VERIFY dropped" "ch_test still in SHOW DATABASES"
fi

# ============================================================
# Summary
# ============================================================
echo ""
echo -e "${BOLD}======================================================================${RESET}"
echo -e "${BOLD}ClickHouse CRUD Test Summary${RESET}"
echo -e "${BOLD}======================================================================${RESET}"
echo -e "Total:  $TOTAL"
echo -e "${GREEN}Passed: $PASSED${RESET}"
echo -e "${RED}Failed: $FAILED${RESET}"
echo "Completed at: $(date '+%Y-%m-%d %H:%M:%S')"
echo -e "${BOLD}======================================================================${RESET}"

[ $FAILED -eq 0 ]
