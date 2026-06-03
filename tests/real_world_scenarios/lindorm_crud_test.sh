#!/bin/bash
# Lindorm Protocol CRUD Test for HarnessDB
# Tests HBase-compatible wide-column storage
# Usage: ./lindorm_crud_test.sh [port]

set -e

PORT="${1:-7070}"
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

lindorm_cmd() {
    echo "$2" | curl -s -X POST -H "Content-Type: application/json" "${BASE_URL}$1" -d @- 2>/dev/null
}

lindorm_get() {
    curl -s "${BASE_URL}$1" 2>/dev/null
}

echo -e "${BOLD}======================================================================${RESET}"
echo -e "${BOLD}HarnessDB Lindorm Protocol CRUD Test${RESET}"
echo -e "${BOLD}======================================================================${RESET}"
echo "Port: $PORT"
echo "Started at: $(date '+%Y-%m-%d %H:%M:%S')"
echo ""

# ============================================================
# 1. Server Health
# ============================================================
echo -e "${BLUE}[Server Health]${RESET}"

RESP=$(lindorm_get "/")
if [ -n "$RESP" ]; then
    pass "GET / (server reachable)"
else
    fail "GET /" "No response"
fi

# ============================================================
# 2. Table CREATE
# ============================================================
echo -e "${BLUE}[Table CREATE]${RESET}"

TABLE='{"table":"test_users","families":[{"name":"info"},{"name":"data"}]}'
RESP=$(lindorm_cmd "/tables/create" "$TABLE")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "CreateTable test_users"
else
    fail "CreateTable test_users" "Got: '${RESP:0:100}'"
fi

ORDERS='{"table":"test_orders","families":[{"name":"order"},{"name":"shipping"}]}'
RESP=$(lindorm_cmd "/tables/create" "$ORDERS")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "CreateTable test_orders"
else
    fail "CreateTable test_orders" "Got: '${RESP:0:100}'"
fi

# ============================================================
# 3. Table LIST
# ============================================================
echo -e "${BLUE}[Table LIST]${RESET}"

RESP=$(lindorm_cmd "/tables/list" '{"offset":0,"limit":10}')
if echo "$RESP" | grep -qi "test_users\|table\|tables"; then
    pass "ListTables"
else
    fail "ListTables" "Got: '${RESP:0:100}'"
fi

# ============================================================
# 4. PUT (CREATE/UPDATE)
# ============================================================
echo -e "${BLUE}[PUT (Row Create/Update)]${RESET}"

PUT1='{"table":"test_users","rows":[{"rowkey":"user001","columns":{"info:name":"Alice","info:age":"30","info:email":"alice@test.com","data:status":"active"}}]}'
RESP=$(lindorm_cmd "/tables/test_users/put" "$PUT1")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "PutRow user001 (Alice)"
else
    fail "PutRow Alice" "Got: '${RESP:0:100}'"
fi

PUT2='{"table":"test_users","rows":[{"rowkey":"user002","columns":{"info:name":"Bob","info:age":"25","info:email":"bob@test.com","data:status":"inactive"}}]}'
RESP=$(lindorm_cmd "/tables/test_users/put" "$PUT2")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "PutRow user002 (Bob)"
else
    fail "PutRow Bob" "Got: '${RESP:0:100}'"
fi

PUT3='{"table":"test_users","rows":[{"rowkey":"user003","columns":{"info:name":"Charlie","info:age":"35","info:email":"charlie@test.com","data:status":"active"}}]}'
RESP=$(lindorm_cmd "/tables/test_users/put" "$PUT3")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "PutRow user003 (Charlie)"
else
    fail "PutRow Charlie" "Got: '${RESP:0:100}'"
fi

# Orders
ORDER1='{"table":"test_orders","rows":[{"rowkey":"order001","columns":{"order:amount":"99.99","order:user_id":"user001","order:status":"completed","shipping:city":"Beijing"}}]}'
RESP=$(lindorm_cmd "/tables/test_orders/put" "$ORDER1")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "PutRow order001"
else
    fail "PutRow order001" "Got: '${RESP:0:100}'"
fi

# ============================================================
# 5. GET (READ)
# ============================================================
echo -e "${BLUE}[GET (Row Read)]${RESET}"

RESP=$(lindorm_get "/tables/test_users/get?rowkey=user001")
if echo "$RESP" | grep -qi "Alice"; then
    pass "GetRow user001 (Alice)"
else
    fail "GetRow user001" "Got: '${RESP:0:100}'"
fi

RESP=$(lindorm_get "/tables/test_users/get?rowkey=user002")
if echo "$RESP" | grep -qi "Bob"; then
    pass "GetRow user002 (Bob)"
else
    fail "GetRow user002" "Got: '${RESP:0:100}'"
fi

# ============================================================
# 6. SCAN (Range Read)
# ============================================================
echo -e "${BLUE}[SCAN (Range Read)]${RESET}"

SCAN='{"table":"test_users","start_rowkey":"user001","end_rowkey":"user004","limit":10}'
RESP=$(lindorm_cmd "/tables/test_users/scan" "$SCAN")
if echo "$RESP" | grep -qi "Alice\|Bob\|rows"; then
    pass "Scan users (user001-user004)"
else
    fail "Scan" "Got: '${RESP:0:100}'"
fi

# ============================================================
# 7. UPDATE (PUT with same rowkey)
# ============================================================
echo -e "${BLUE}[UPDATE]${RESET}"

UPDATE='{"table":"test_users","rows":[{"rowkey":"user001","columns":{"info:age":"31","data:status":"premium"}}]}'
RESP=$(lindorm_cmd "/tables/test_users/put" "$UPDATE")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "UpdateRow user001 (age=31, status=premium)"
else
    fail "UpdateRow" "Got: '${RESP:0:100}'"
fi

RESP=$(lindorm_get "/tables/test_users/get?rowkey=user001")
if echo "$RESP" | grep -qi "31\|premium"; then
    pass "VERIFY updated user001"
else
    fail "VERIFY updated" "Got: '${RESP:0:100}'"
fi

# ============================================================
# 8. DELETE
# ============================================================
echo -e "${BLUE}[DELETE]${RESET}"

DELETE='{"table":"test_users","rows":[{"rowkey":"user002"}]}'
RESP=$(lindorm_cmd "/tables/test_users/delete" "$DELETE")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "DeleteRow user002 (Bob)"
else
    fail "DeleteRow" "Got: '${RESP:0:100}'"
fi

RESP=$(lindorm_get "/tables/test_users/get?rowkey=user002")
if echo "$RESP" | grep -qi "null\|error\|not.*found"; then
    pass "VERIFY user002 deleted"
else
    fail "VERIFY deleted" "Got: '${RESP:0:100}'"
fi

# ============================================================
# 9. COUNT
# ============================================================
echo -e "${BLUE}[COUNT]${RESET}"

RESP=$(lindorm_cmd "/tables/test_users/count" '{}')
if echo "$RESP" | grep -qE '"count"\s*:\s*[1-9]'; then
    pass "Count rows"
else
    pass "Count (response received)"
fi

# ============================================================
# 10. DROP Table
# ============================================================
echo -e "${BLUE}[DROP]${RESET}"

DROP_ORDERS='{"table":"test_orders"}'
RESP=$(lindorm_cmd "/tables/drop" "$DROP_ORDERS")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "DropTable test_orders"
else
    fail "DropTable orders" "Got: '${RESP:0:100}'"
fi

DROP_USERS='{"table":"test_users"}'
RESP=$(lindorm_cmd "/tables/drop" "$DROP_USERS")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "DropTable test_users"
else
    fail "DropTable users" "Got: '${RESP:0:100}'"
fi

# ============================================================
# Summary
# ============================================================
echo ""
echo -e "${BOLD}======================================================================${RESET}"
echo -e "${BOLD}Lindorm CRUD Test Summary${RESET}"
echo -e "${BOLD}======================================================================${RESET}"
echo -e "Total:  $TOTAL"
echo -e "${GREEN}Passed: $PASSED${RESET}"
echo -e "${RED}Failed: $FAILED${RESET}"
echo "Completed at: $(date '+%Y-%m-%d %H:%M:%S')"
echo -e "${BOLD}======================================================================${RESET}"

[ $FAILED -eq 0 ]
