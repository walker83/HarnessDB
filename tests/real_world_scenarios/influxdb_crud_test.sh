#!/bin/bash
# InfluxDB HTTP Protocol CRUD Test for HarnessDB
# Tests InfluxDB line protocol and HTTP API
# Usage: ./influxdb_crud_test.sh [port]

set -e

PORT="${1:-8086}"
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

influx_query() {
    curl -s "http://${HOST}:${PORT}/query?db=harness_test&q=$1" 2>/dev/null
}

influx_write() {
    curl -s -X POST "http://${HOST}:${PORT}/write?db=harness_test" -d "$1" 2>/dev/null
}

echo -e "${BOLD}======================================================================${RESET}"
echo -e "${BOLD}HarnessDB InfluxDB Protocol CRUD Test${RESET}"
echo -e "${BOLD}======================================================================${RESET}"
echo "Port: $PORT"
echo "Started at: $(date '+%Y-%m-%d %H:%M:%S')"
echo ""

# ============================================================
# 1. Connection & Ping
# ============================================================
echo -e "${BLUE}[Connection & Ping]${RESET}"

RESP=$(curl -s -o /dev/null -w "%{http_code}" "${BASE_URL}/ping" 2>/dev/null)
if [ "$RESP" = "200" ] || [ "$RESP" = "204" ]; then
    pass "GET /ping (HTTP $RESP)"
else
    fail "GET /ping" "HTTP $RESP (expected 200 or 204)"
fi

# ============================================================
# 2. Database Operations
# ============================================================
echo -e "${BLUE}[Database Operations]${RESET}"

RESP=$(curl -s -X POST "${BASE_URL}/query" --data-urlencode "q=CREATE DATABASE harness_test" 2>/dev/null)
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "CREATE DATABASE harness_test"
else
    fail "CREATE DATABASE" "Got: '$RESP'"
fi

RESP=$(curl -s "${BASE_URL}/query?q=SHOW+DATABASES" 2>/dev/null)
if echo "$RESP" | grep -qi "harness_test"; then
    pass "SHOW DATABASES (contains harness_test)"
else
    fail "SHOW DATABASES" "Got: '$RESP'"
fi

# ============================================================
# 3. Line Protocol WRITE
# ============================================================
echo -e "${BLUE}[Line Protocol WRITE]${RESET}"

RESP=$(influx_write "temperature,location=beijing value=22.5,humidity=60 $(date +%s)000000000")
if [ -z "$RESP" ] || ! echo "$RESP" | grep -qi "error"; then
    pass "WRITE temperature point 1"
else
    fail "WRITE point 1" "Got: '$RESP'"
fi

RESP=$(influx_write "temperature,location=beijing value=23.0,humidity=58 $(date +%s)001000000")
if [ -z "$RESP" ] || ! echo "$RESP" | grep -qi "error"; then
    pass "WRITE temperature point 2"
else
    fail "WRITE point 2" "Got: '$RESP'"
fi

RESP=$(influx_write "temperature,location=shanghai value=25.0,humidity=70 $(date +%s)002000000")
if [ -z "$RESP" ] || ! echo "$RESP" | grep -qi "error"; then
    pass "WRITE temperature point 3 (shanghai)"
else
    fail "WRITE point 3" "Got: '$RESP'"
fi

RESP=$(influx_write "cpu,host=server1 usage=45.2,system=12.0 $(date +%s)003000000")
if [ -z "$RESP" ] || ! echo "$RESP" | grep -qi "error"; then
    pass "WRITE cpu measurement"
else
    fail "WRITE cpu" "Got: '$RESP'"
fi

# ============================================================
# 4. SELECT (READ)
# ============================================================
echo -e "${BLUE}[SELECT (READ)]${RESET}"

RESP=$(influx_query "SELECT * FROM temperature")
if echo "$RESP" | grep -qi "beijing\|temperature\|value"; then
    pass "SELECT * FROM temperature"
else
    fail "SELECT * temperature" "Got: '${RESP:0:200}'"
fi

RESP=$(influx_query "SELECT value FROM temperature WHERE location=%27beijing%27")
if echo "$RESP" | grep -qi "22.5\|23"; then
    pass "SELECT WHERE location=beijing"
else
    fail "SELECT WHERE location" "Got: '${RESP:0:200}'"
fi

RESP=$(influx_query "SHOW MEASUREMENTS")
if echo "$RESP" | grep -qi "temperature\|cpu"; then
    pass "SHOW MEASUREMENTS"
else
    fail "SHOW MEASUREMENTS" "Got: '${RESP:0:200}'"
fi

# ============================================================
# 5. Retention Policy
# ============================================================
echo -e "${BLUE}[Retention Policy]${RESET}"

RESP=$(curl -s -X POST "${BASE_URL}/query" --data-urlencode "q=CREATE RETENTION POLICY rp_7d ON harness_test DURATION 7d REPLICATION 1 DEFAULT" 2>/dev/null)
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "CREATE RETENTION POLICY rp_7d"
else
    fail "CREATE RETENTION POLICY" "Got: '$RESP'"
fi

RESP=$(curl -s "${BASE_URL}/query?q=SHOW+RETENTION+POLICIES+ON+harness_test" 2>/dev/null)
if echo "$RESP" | grep -qi "rp_7d\|autogen\|default"; then
    pass "SHOW RETENTION POLICIES"
else
    fail "SHOW RETENTION POLICIES" "Got: '${RESP:0:200}'"
fi

# ============================================================
# 6. Aggregation Queries
# ============================================================
echo -e "${BLUE}[Aggregation Queries]${RESET}"

RESP=$(influx_query "SELECT COUNT(value) FROM temperature")
if echo "$RESP" | grep -qi "count\|3"; then
    pass "SELECT COUNT(value)"
else
    fail "SELECT COUNT" "Got: '${RESP:0:200}'"
fi

RESP=$(influx_query "SELECT MEAN(value) FROM temperature")
if echo "$RESP" | grep -qi "mean\|23"; then
    pass "SELECT MEAN(value)"
else
    fail "SELECT MEAN" "Got: '${RESP:0:200}'"
fi

RESP=$(influx_query "SELECT MAX(value) FROM temperature")
if echo "$RESP" | grep -qi "max\|25"; then
    pass "SELECT MAX(value)"
else
    fail "SELECT MAX" "Got: '${RESP:0:200}'"
fi

# ============================================================
# 7. DROP
# ============================================================
echo -e "${BLUE}[DROP]${RESET}"

RESP=$(curl -s -X POST "${BASE_URL}/query" --data-urlencode "q=DROP DATABASE harness_test" 2>/dev/null)
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "DROP DATABASE harness_test"
else
    fail "DROP DATABASE" "Got: '$RESP'"
fi

RESP=$(curl -s "${BASE_URL}/query?q=SHOW+DATABASES" 2>/dev/null)
if ! echo "$RESP" | grep -qi "harness_test"; then
    pass "VERIFY harness_test dropped"
else
    fail "VERIFY dropped" "harness_test still in SHOW DATABASES"
fi

# ============================================================
# Summary
# ============================================================
echo ""
echo -e "${BOLD}======================================================================${RESET}"
echo -e "${BOLD}InfluxDB CRUD Test Summary${RESET}"
echo -e "${BOLD}======================================================================${RESET}"
echo -e "Total:  $TOTAL"
echo -e "${GREEN}Passed: $PASSED${RESET}"
echo -e "${RED}Failed: $FAILED${RESET}"
echo "Completed at: $(date '+%Y-%m-%d %H:%M:%S')"
echo -e "${BOLD}======================================================================${RESET}"

[ $FAILED -eq 0 ]
