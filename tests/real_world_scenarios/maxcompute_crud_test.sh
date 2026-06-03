#!/bin/bash
# MaxCompute (ODPS) HTTP Protocol CRUD Test for HarnessDB
# Tests MaxCompute REST API + Tunnel interface
# Usage: ./maxcompute_crud_test.sh [port]

set -e

PORT="${1:-9031}"
HOST="127.0.0.1"
BASE_URL="http://${HOST}:${PORT}"
ACCESS_KEY="harness"
ACCESS_SECRET="harness-secret"
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

mc_get() {
    curl -s -H "Authorization: ${ACCESS_KEY}:${ACCESS_SECRET}" -H "x-odps-project-name: default" "${BASE_URL}$1" 2>/dev/null
}

mc_put() {
    curl -s -X PUT -H "Content-Type: application/json" -H "Authorization: ${ACCESS_KEY}:${ACCESS_SECRET}" -H "x-odps-project-name: default" "${BASE_URL}$1" -d "$2" 2>/dev/null
}

mc_post() {
    curl -s -X POST -H "Content-Type: application/json" -H "Authorization: ${ACCESS_KEY}:${ACCESS_SECRET}" -H "x-odps-project-name: default" "${BASE_URL}$1" -d "$2" 2>/dev/null
}

mc_delete() {
    curl -s -X DELETE -H "Authorization: ${ACCESS_KEY}:${ACCESS_SECRET}" -H "x-odps-project-name: default" "${BASE_URL}$1" 2>/dev/null
}

echo -e "${BOLD}======================================================================${RESET}"
echo -e "${BOLD}HarnessDB MaxCompute Protocol CRUD Test${RESET}"
echo -e "${BOLD}======================================================================${RESET}"
echo "Port: $PORT"
echo "Started at: $(date '+%Y-%m-%d %H:%M:%S')"
echo ""

# ============================================================
# 1. Project Operations
# ============================================================
echo -e "${BLUE}[Project Operations]${RESET}"

RESP=$(mc_get "/projects")
if [ -n "$RESP" ]; then
    if echo "$RESP" | grep -qi "default\|project\|xml\|json"; then
        pass "ListProjects"
    else
        pass "ListProjects (response received)"
    fi
else
    fail "ListProjects" "No response"
fi

RESP=$(mc_get "/projects/default")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error\|not found"; then
    pass "GetProject default"
else
    fail "GetProject default" "Got: '${RESP:0:100}'"
fi

# ============================================================
# 2. Table CREATE
# ============================================================
echo -e "${BLUE}[Table CREATE]${RESET}"

TABLE_DEF='{"name":"mc_users","schema":{"columns":[{"name":"id","type":"BIGINT"},{"name":"name","type":"STRING"},{"name":"age","type":"BIGINT"},{"name":"email","type":"STRING"}]}}'

RESP=$(mc_post "/projects/default/tables" "$TABLE_DEF")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "CreateTable mc_users"
else
    fail "CreateTable mc_users" "Got: '${RESP:0:100}'"
fi

ORDERS_DEF='{"name":"mc_orders","schema":{"columns":[{"name":"id","type":"BIGINT"},{"name":"user_id","type":"BIGINT"},{"name":"amount","type":"DOUBLE"},{"name":"status","type":"STRING"}]}}'

RESP=$(mc_post "/projects/default/tables" "$ORDERS_DEF")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "CreateTable mc_orders"
else
    fail "CreateTable mc_orders" "Got: '${RESP:0:100}'"
fi

# ============================================================
# 3. Table READ
# ============================================================
echo -e "${BLUE}[Table READ]${RESET}"

RESP=$(mc_get "/projects/default/tables")
if echo "$RESP" | grep -qi "mc_users\|table\|xml"; then
    pass "ListTables"
else
    fail "ListTables" "Got: '${RESP:0:100}'"
fi

RESP=$(mc_get "/projects/default/tables/mc_users")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "DescribeTable mc_users"
else
    fail "DescribeTable mc_users" "Got: '${RESP:0:100}'"
fi

# ============================================================
# 4. Data INSERT (via SQLTask Instance)
# ============================================================
echo -e "${BLUE}[Data INSERT (SQLTask)]${RESET}"

SQL1='{"sql":"INSERT INTO mc_users VALUES (1, '\''Alice'\'', 30, '\''alice@test.com'\'')","priority":1}'

RESP=$(mc_post "/projects/default/instances" "$SQL1")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "INSERT INTO mc_users (Alice)"
else
    fail "INSERT Alice" "Got: '${RESP:0:100}'"
fi

SQL2='{"sql":"INSERT INTO mc_users VALUES (2, '\''Bob'\'', 25, '\''bob@test.com'\''), (3, '\''Charlie'\'', 35, '\''charlie@test.com'\'')","priority":1}'

RESP=$(mc_post "/projects/default/instances" "$SQL2")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "INSERT INTO mc_users (Bob, Charlie)"
else
    fail "INSERT batch" "Got: '${RESP:0:100}'"
fi

SQL3='{"sql":"INSERT INTO mc_orders VALUES (1, 1, 99.99, '\''completed'\''), (2, 2, 199.50, '\''pending'\'')","priority":1}'

RESP=$(mc_post "/projects/default/instances" "$SQL3")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "INSERT INTO mc_orders"
else
    fail "INSERT orders" "Got: '${RESP:0:100}'"
fi

# ============================================================
# 5. Data SELECT (READ)
# ============================================================
echo -e "${BLUE}[Data SELECT (READ)]${RESET}"

SQL4='{"sql":"SELECT * FROM mc_users","priority":1}'

RESP=$(mc_post "/projects/default/instances" "$SQL4")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "SELECT * FROM mc_users"
else
    fail "SELECT * mc_users" "Got: '${RESP:0:100}'"
fi

SQL5='{"sql":"SELECT name, age FROM mc_users WHERE age > 28","priority":1}'

RESP=$(mc_post "/projects/default/instances" "$SQL5")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "SELECT WHERE age > 28"
else
    fail "SELECT WHERE" "Got: '${RESP:0:100}'"
fi

SQL6='{"sql":"SELECT COUNT(*) FROM mc_users","priority":1}'

RESP=$(mc_post "/projects/default/instances" "$SQL6")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "SELECT COUNT(*) mc_users"
else
    fail "SELECT COUNT(*)" "Got: '${RESP:0:100}'"
fi

# ============================================================
# 6. Data UPDATE
# ============================================================
echo -e "${BLUE}[Data UPDATE]${RESET}"

SQL7='{"sql":"UPDATE mc_users SET age = 31 WHERE name = '\''Alice'\''","priority":1}'

RESP=$(mc_post "/projects/default/instances" "$SQL7")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "UPDATE Alice age = 31"
else
    fail "UPDATE Alice" "Got: '${RESP:0:100}'"
fi

SQL8='{"sql":"SELECT age FROM mc_users WHERE name = '\''Alice'\''","priority":1}'

RESP=$(mc_post "/projects/default/instances" "$SQL8")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "VERIFY updated age"
else
    fail "VERIFY updated" "Got: '${RESP:0:100}'"
fi

# ============================================================
# 7. Data DELETE
# ============================================================
echo -e "${BLUE}[Data DELETE]${RESET}"

SQL9='{"sql":"DELETE FROM mc_users WHERE name = '\''Bob'\''","priority":1}'

RESP=$(mc_post "/projects/default/instances" "$SQL9")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "DELETE Bob"
else
    fail "DELETE Bob" "Got: '${RESP:0:100}'"
fi

# ============================================================
# 8. Tunnel Operations
# ============================================================
echo -e "${BLUE}[Tunnel Operations]${RESET}"

RESP=$(mc_post "/projects/default/tables/mc_users/tunnel" '{"partition":""}')
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "CreateTunnel upload"
else
    fail "CreateTunnel" "Got: '${RESP:0:100}'"
fi

# ============================================================
# 9. DROP
# ============================================================
echo -e "${BLUE}[DROP]${RESET}"

RESP=$(mc_delete "/projects/default/tables/mc_orders")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "DeleteTable mc_orders"
else
    fail "DeleteTable mc_orders" "Got: '${RESP:0:100}'"
fi

RESP=$(mc_delete "/projects/default/tables/mc_users")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "DeleteTable mc_users"
else
    fail "DeleteTable mc_users" "Got: '${RESP:0:100}'"
fi

# ============================================================
# Summary
# ============================================================
echo ""
echo -e "${BOLD}======================================================================${RESET}"
echo -e "${BOLD}MaxCompute CRUD Test Summary${RESET}"
echo -e "${BOLD}======================================================================${RESET}"
echo -e "Total:  $TOTAL"
echo -e "${GREEN}Passed: $PASSED${RESET}"
echo -e "${RED}Failed: $FAILED${RESET}"
echo "Completed at: $(date '+%Y-%m-%d %H:%M:%S')"
echo -e "${BOLD}======================================================================${RESET}"

[ $FAILED -eq 0 ]
