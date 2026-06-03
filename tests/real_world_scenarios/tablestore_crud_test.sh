#!/bin/bash
# TableStore (OTS) HTTP Protocol CRUD Test for HarnessDB
# Tests TableStore REST API
# Usage: ./tablestore_crud_test.sh [port]

set -e

PORT="${1:-8087}"
HOST="127.0.0.1"
BASE_URL="http://${HOST}:${PORT}"
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

ts_get() {
    curl -s "${BASE_URL}$1" 2>/dev/null
}

ts_put() {
    curl -s -X PUT -H "Content-Type: application/json" "${BASE_URL}$1" -d "$2" 2>/dev/null
}

ts_post() {
    curl -s -X POST -H "Content-Type: application/json" "${BASE_URL}$1" -d "$2" 2>/dev/null
}

ts_delete() {
    curl -s -X DELETE "${BASE_URL}$1" 2>/dev/null
}

echo -e "${BOLD}======================================================================${RESET}"
echo -e "${BOLD}HarnessDB TableStore Protocol CRUD Test${RESET}"
echo -e "${BOLD}======================================================================${RESET}"
echo "Port: $PORT"
echo "Started at: $(date '+%Y-%m-%d %H:%M:%S')"
echo ""

# ============================================================
# 1. Instance Operations
# ============================================================
echo -e "${BLUE}[Instance & Server]${RESET}"

RESP=$(ts_get "/")
if [ -n "$RESP" ]; then
    pass "GET / (server reachable)"
else
    fail "GET /" "No response"
fi

# ============================================================
# 2. Table CREATE
# ============================================================
echo -e "${BLUE}[Table CREATE]${RESET}"

TABLE_DEF='{"table_name":"users","primary_key":[{"name":"id","type":"INTEGER"}],"defined_columns":[{"name":"name","type":"STRING"},{"name":"age","type":"INTEGER"},{"name":"email","type":"STRING"}]}'

RESP=$(ts_put "/tables/users" "$TABLE_DEF")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "CreateTable users"
else
    fail "CreateTable users" "Got: '$RESP'"
fi

ORDER_DEF='{"table_name":"orders","primary_key":[{"name":"id","type":"INTEGER"}],"defined_columns":[{"name":"user_id","type":"INTEGER"},{"name":"amount","type":"DOUBLE"},{"name":"status","type":"STRING"}]}'

RESP=$(ts_put "/tables/orders" "$ORDER_DEF")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "CreateTable orders"
else
    fail "CreateTable orders" "Got: '$RESP'"
fi

# ============================================================
# 3. Table READ
# ============================================================
echo -e "${BLUE}[Table READ]${RESET}"

RESP=$(ts_get "/tables")
if echo "$RESP" | grep -qi "users\|tables"; then
    pass "ListTables"
else
    fail "ListTables" "Got: '$RESP'"
fi

RESP=$(ts_get "/tables/users")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "DescribeTable users"
else
    fail "DescribeTable users" "Got: '$RESP'"
fi

# ============================================================
# 4. Row CREATE (PutRow)
# ============================================================
echo -e "${BLUE}[Row CREATE (PutRow)]${RESET}"

ROW1='{"primary_key":[{"name":"id","value":1}],"attributes":[{"name":"name","value":"Alice"},{"name":"age","value":30},{"name":"email","value":"alice@test.com"}]}'

RESP=$(ts_post "/tables/users/rows" "$ROW1")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "PutRow Alice"
else
    fail "PutRow Alice" "Got: '$RESP'"
fi

ROW2='{"primary_key":[{"name":"id","value":2}],"attributes":[{"name":"name","value":"Bob"},{"name":"age","value":25},{"name":"email","value":"bob@test.com"}]}'

RESP=$(ts_post "/tables/users/rows" "$ROW2")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "PutRow Bob"
else
    fail "PutRow Bob" "Got: '$RESP'"
fi

ROW3='{"primary_key":[{"name":"id","value":3}],"attributes":[{"name":"name","value":"Charlie"},{"name":"age","value":35},{"name":"email","value":"charlie@test.com"}]}'

RESP=$(ts_post "/tables/users/rows" "$ROW3")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "PutRow Charlie"
else
    fail "PutRow Charlie" "Got: '$RESP'"
fi

# ============================================================
# 5. Row READ (GetRow)
# ============================================================
echo -e "${BLUE}[Row READ (GetRow)]${RESET}"

RESP=$(ts_get "/tables/users/rows?id=1")
if echo "$RESP" | grep -qi "Alice"; then
    pass "GetRow id=1 (Alice)"
else
    fail "GetRow id=1" "Got: '$RESP'"
fi

RESP=$(ts_get "/tables/users/rows?id=2")
if echo "$RESP" | grep -qi "Bob"; then
    pass "GetRow id=2 (Bob)"
else
    fail "GetRow id=2" "Got: '$RESP'"
fi

# ============================================================
# 6. Row READ (GetRange)
# ============================================================
echo -e "${BLUE}[Row READ (GetRange)]${RESET}"

RANGE='{"start":{"id":1},"end":{"id":4}}'

RESP=$(ts_post "/tables/users/range" "$RANGE")
if echo "$RESP" | grep -qi "Alice\|Bob\|rows"; then
    pass "GetRange id 1-4"
else
    fail "GetRange" "Got: '$RESP'"
fi

# ============================================================
# 7. Row UPDATE
# ============================================================
echo -e "${BLUE}[Row UPDATE]${RESET}"

UPDATE='{"primary_key":[{"name":"id","value":1}],"attributes":[{"name":"age","value":31},{"name":"email","value":"alice.new@test.com"}]}'

RESP=$(ts_post "/tables/users/rows/1" "$UPDATE")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "UpdateRow id=1 (age=31)"
else
    fail "UpdateRow id=1" "Got: '$RESP'"
fi

RESP=$(ts_get "/tables/users/rows?id=1")
if echo "$RESP" | grep -qi "31"; then
    pass "VERIFY updated age=31"
else
    fail "VERIFY updated age" "Got: '$RESP'"
fi

# ============================================================
# 8. Row DELETE
# ============================================================
echo -e "${BLUE}[Row DELETE]${RESET}"

RESP=$(ts_delete "/tables/users/rows?id=2")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "DeleteRow id=2 (Bob)"
else
    fail "DeleteRow id=2" "Got: '$RESP'"
fi

RESP=$(ts_get "/tables/users/rows?id=2")
if echo "$RESP" | grep -qi "error\|not.*found\|null"; then
    pass "VERIFY id=2 deleted"
else
    fail "VERIFY deleted" "Got: '$RESP'"
fi

# ============================================================
# 9. Delete Table
# ============================================================
echo -e "${BLUE}[Delete Table]${RESET}"

RESP=$(ts_delete "/tables/orders")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "DeleteTable orders"
else
    fail "DeleteTable orders" "Got: '$RESP'"
fi

RESP=$(ts_delete "/tables/users")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "DeleteTable users"
else
    fail "DeleteTable users" "Got: '$RESP'"
fi

RESP=$(ts_get "/tables")
if ! echo "$RESP" | grep -qi "users"; then
    pass "VERIFY users table deleted"
else
    fail "VERIFY table deleted" "users still in ListTables"
fi

# ============================================================
# Summary
# ============================================================
echo ""
echo -e "${BOLD}======================================================================${RESET}"
echo -e "${BOLD}TableStore CRUD Test Summary${RESET}"
echo -e "${BOLD}======================================================================${RESET}"
echo -e "Total:  $TOTAL"
echo -e "${GREEN}Passed: $PASSED${RESET}"
echo -e "${RED}Failed: $FAILED${RESET}"
echo "Completed at: $(date '+%Y-%m-%d %H:%M:%S')"
echo -e "${BOLD}======================================================================${RESET}"

[ $FAILED -eq 0 ]
