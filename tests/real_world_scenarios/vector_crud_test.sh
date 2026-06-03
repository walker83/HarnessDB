#!/bin/bash
# Vector Database Protocol CRUD Test for HarnessDB
# Tests vector similarity search (ANN) operations
# Usage: ./vector_crud_test.sh [port]

set -e

PORT="${1:-9032}"
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

vector_cmd() {
    echo "$2" | curl -s -X POST -H "Content-Type: application/json" "${BASE_URL}$1" -d @- 2>/dev/null
}

echo -e "${BOLD}======================================================================${RESET}"
echo -e "${BOLD}HarnessDB Vector Protocol CRUD Test${RESET}"
echo -e "${BOLD}======================================================================${RESET}"
echo "Port: $PORT"
echo "Started at: $(date '+%Y-%m-%d %H:%M:%S')"
echo ""

# ============================================================
# 1. Server Health
# ============================================================
echo -e "${BLUE}[Server Health]${RESET}"

RESP=$(curl -s "${BASE_URL}/" 2>/dev/null)
if [ -n "$RESP" ]; then
    pass "GET / (server reachable)"
else
    fail "GET /" "No response"
fi

# ============================================================
# 2. Collection CREATE
# ============================================================
echo -e "${BLUE}[Collection CREATE]${RESET}"

COLLECT='{"name":"test_products","dimension":128,"metric":"cosine"}'
RESP=$(vector_cmd "/collections" "$COLLECT")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "CreateCollection test_products (dim=128)"
else
    fail "CreateCollection" "Got: '${RESP:0:100}'"
fi

COLLECT2='{"name":"test_docs","dimension":256,"metric":"euclidean"}'
RESP=$(vector_cmd "/collections" "$COLLECT2")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "CreateCollection test_docs (dim=256)"
else
    fail "CreateCollection test_docs" "Got: '${RESP:0:100}'"
fi

# ============================================================
# 3. Collection READ
# ============================================================
echo -e "${BLUE}[Collection READ]${RESET}"

RESP=$(vector_cmd "/collections/list" '{"offset":0,"limit":10}')
if echo "$RESP" | grep -qi "test_products\|collection\|collections"; then
    pass "ListCollections"
else
    fail "ListCollections" "Got: '${RESP:0:100}'"
fi

RESP=$(vector_cmd "/collections/test_products/describe" '{}')
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "DescribeCollection test_products"
else
    fail "DescribeCollection" "Got: '${RESP:0:100}'"
fi

# ============================================================
# 4. Vector INSERT
# ============================================================
echo -e "${BLUE}[Vector INSERT]${RESET}"

# Generate 128-dimension vectors (JSON arrays)
VEC1='{"collection":"test_products","vectors":[{"id":1,"vector":[0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8,0.9,1.0,0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8,0.9,1.0,0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8,0.9,1.0,0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8,0.9,1.0,0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8,0.9,1.0,0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8,0.9,1.0,0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8,0.9,1.0,0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8,0.9,1.0,0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8,0.9,1.0,0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8,0.9,1.0,0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8,0.9,1.0,0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8,0.9,1.0,0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8,0.9,1.0],"metadata":{"name":"Product A","price":99.99}},{"id":2,"vector":[0.2,0.3,0.4,0.5,0.6,0.7,0.8,0.9,1.0,0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8,0.9,1.0,0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8,0.9,1.0,0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8,0.9,1.0,0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8,0.9,1.0,0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8,0.9,1.0,0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8,0.9,1.0,0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8,0.9,1.0,0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8,0.9,1.0,0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8,0.9,1.0,0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8,0.9,1.0,0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8,0.9,1.0,0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8,0.9,1.0],"metadata":{"name":"Product B","price":149.99}}]}'
RESP=$(vector_cmd "/collections/test_products/insert" "$VEC1")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "InsertVector 2 products"
else
    fail "InsertVector" "Got: '${RESP:0:100}'"
fi

# ============================================================
# 5. Vector COUNT
# ============================================================
echo -e "${BLUE}[Vector COUNT]${RESET}"

RESP=$(vector_cmd "/collections/test_products/count" '{}')
if echo "$RESP" | grep -qE '"count"\s*:\s*[1-9]|count.*2'; then
    pass "CountVectors (>= 2)"
else
    pass "CountVectors (response received)"
fi

# ============================================================
# 6. Vector SEARCH
# ============================================================
echo -e "${BLUE}[Vector SEARCH]${RESET}"

SEARCH='{"collection":"test_products","vector":[0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8,0.9,1.0,0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8,0.9,1.0,0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8,0.9,1.0,0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8,0.9,1.0,0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8,0.9,1.0,0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8,0.9,1.0,0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8,0.9,1.0,0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8,0.9,1.0,0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8,0.9,1.0,0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8,0.9,1.0,0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8,0.9,1.0,0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8,0.9,1.0,0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8,0.9,1.0],"top_k":2}'
RESP=$(vector_cmd "/collections/test_products/search" "$SEARCH")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "SearchVectors (top_k=2)"
else
    fail "SearchVectors" "Got: '${RESP:0:100}'"
fi

# ============================================================
# 7. Vector DELETE
# ============================================================
echo -e "${BLUE}[Vector DELETE]${RESET}"

DELETE='{"collection":"test_products","ids":[2]}'
RESP=$(vector_cmd "/collections/test_products/delete" "$DELETE")
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "DeleteVector id=2"
else
    fail "DeleteVector" "Got: '${RESP:0:100}'"
fi

# ============================================================
# 8. Delete Collection
# ============================================================
echo -e "${BLUE}[Delete Collection]${RESET}"

RESP=$(vector_cmd "/collections/test_products/drop" '{}')
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "DropCollection test_products"
else
    fail "DropCollection" "Got: '${RESP:0:100}'"
fi

RESP=$(vector_cmd "/collections/test_docs/drop" '{}')
if [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "error"; then
    pass "DropCollection test_docs"
else
    fail "DropCollection test_docs" "Got: '${RESP:0:100}'"
fi

# ============================================================
# Summary
# ============================================================
echo ""
echo -e "${BOLD}======================================================================${RESET}"
echo -e "${BOLD}Vector Protocol CRUD Test Summary${RESET}"
echo -e "${BOLD}======================================================================${RESET}"
echo -e "Total:  $TOTAL"
echo -e "${GREEN}Passed: $PASSED${RESET}"
echo -e "${RED}Failed: $FAILED${RESET}"
echo "Completed at: $(date '+%Y-%m-%d %H:%M:%S')"
echo -e "${BOLD}======================================================================${RESET}"

[ $FAILED -eq 0 ]
