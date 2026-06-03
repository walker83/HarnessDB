#!/bin/bash
# Master Test Runner for HarnessDB 14 Protocol CRUD Tests
# Runs all protocol tests in sequence, reports overall results
# Usage: ./run_all_protocol_tests.sh [--skip-real-apps]

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PASSED_PROTOCOLS=0
FAILED_PROTOCOLS=0
TOTAL_PROTOCOLS=0

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
RESET='\033[0m'

run_test() {
    local name="$1"
    local cmd="$2"
    local port="$3"

    TOTAL_PROTOCOLS=$((TOTAL_PROTOCOLS + 1))

    echo ""
    echo -e "${BOLD}======================================================================${RESET}"
    echo -e "${BOLD}Protocol $TOTAL_PROTOCOLS: $name (port $port)${RESET}"
    echo -e "${BOLD}======================================================================${RESET}"

    if eval "$cmd" "$port"; then
        echo -e "\n${GREEN}✓ $name PASSED${RESET}"
        PASSED_PROTOCOLS=$((PASSED_PROTOCOLS + 1))
    else
        echo -e "\n${RED}✗ $name FAILED${RESET}"
        FAILED_PROTOCOLS=$((FAILED_PROTOCOLS + 1))
    fi
}

echo -e "${BOLD}######################################################################${RESET}"
echo -e "${BOLD}# HarnessDB 14 Protocol CRUD Test Suite                           #${RESET}"
echo -e "${BOLD}######################################################################${RESET}"
echo "Started at: $(date '+%Y-%m-%d %H:%M:%S')"
echo ""

# ============================================================
# Protocol Tests (TCP/HTTP)
# ============================================================

# 1. MySQL Protocol (primary - always running)
run_test "MySQL Protocol" "bash ${SCRIPT_DIR}/run_tests.sh" "9030"

# 2. PostgreSQL Protocol
if command -v python3 &>/dev/null; then
    run_test "PostgreSQL Protocol" "python3 ${SCRIPT_DIR}/pg_crud_test.py" "15432"
fi

# 3. MaxCompute Protocol
run_test "MaxCompute Protocol" "bash ${SCRIPT_DIR}/maxcompute_crud_test.sh" "9031"

# 4. Redis Protocol
run_test "Redis Protocol" "bash ${SCRIPT_DIR}/redis_crud_test.sh" "6379"

# 5. MongoDB Protocol
if command -v python3 &>/dev/null; then
    pip3 install pymongo -q 2>/dev/null
    run_test "MongoDB Protocol" "python3 ${SCRIPT_DIR}/mongodb_crud_test.py" "27017"
fi

# 6. ClickHouse Protocol
run_test "ClickHouse Protocol" "bash ${SCRIPT_DIR}/clickhouse_crud_test.sh" "8123"

# 7. Elasticsearch Protocol
run_test "Elasticsearch Protocol" "bash ${SCRIPT_DIR}/elasticsearch_crud_test.sh" "9200"

# 8. InfluxDB Protocol
run_test "InfluxDB Protocol" "bash ${SCRIPT_DIR}/influxdb_crud_test.sh" "8086"

# 9. Cassandra Protocol
if command -v python3 &>/dev/null; then
    run_test "Cassandra Protocol" "python3 ${SCRIPT_DIR}/cassandra_crud_test.py" "9042"
fi

# 10. Oracle Protocol (read-only simulation)
if command -v python3 &>/dev/null; then
    run_test "Oracle Protocol" "python3 ${SCRIPT_DIR}/oracle_crud_test.py" "1521"
fi

# 11. TableStore Protocol
run_test "TableStore Protocol" "bash ${SCRIPT_DIR}/tablestore_crud_test.sh" "8087"

# 12. AnalyticDB MySQL Protocol
run_test "AnalyticDB MySQL Protocol" "bash ${SCRIPT_DIR}/adb_mysql_crud_test.sh" "8124"

# 13. Lindorm Protocol
run_test "Lindorm Protocol" "bash ${SCRIPT_DIR}/lindorm_crud_test.sh" "7070"

# 14. Vector Protocol
run_test "Vector Protocol" "bash ${SCRIPT_DIR}/vector_crud_test.sh" "9032"

# ============================================================
# Real-World Application Tests
# ============================================================
if [ "$1" != "--skip-real-apps" ]; then
    echo ""
    echo -e "${BOLD}######################################################################${RESET}"
    echo -e "${BOLD}# Real-World Application Tests                                      #${RESET}"
    echo -e "${BOLD}######################################################################${RESET}"
    bash ${SCRIPT_DIR}/real_app_tests.sh 9030 15432 || true
fi

# ============================================================
# Final Summary
# ============================================================
echo ""
echo -e "${BOLD}######################################################################${RESET}"
echo -e "${BOLD}# Final Summary                                                     #${RESET}"
echo -e "${BOLD}######################################################################${RESET}"
echo "Protocols Tested: $TOTAL_PROTOCOLS"
echo -e "${GREEN}Passed: $PASSED_PROTOCOLS${RESET}"
echo -e "${RED}Failed: $FAILED_PROTOCOLS${RESET}"
echo ""

if [ $FAILED_PROTOCOLS -eq 0 ]; then
    echo -e "${GREEN}${BOLD}🎉 All $TOTAL_PROTOCOLS protocol tests passed!${RESET}"
else
    echo -e "${RED}${BOLD}⚠️  $FAILED_PROTOCOLS protocol(s) failed.${RESET}"
fi

echo "Completed at: $(date '+%Y-%m-%d %H:%M:%S')"

[ $FAILED_PROTOCOLS -eq 0 ]
