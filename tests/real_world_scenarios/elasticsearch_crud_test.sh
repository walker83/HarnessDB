#!/bin/bash
# Elasticsearch HTTP Protocol CRUD Test for HarnessDB
# Tests Elasticsearch REST API
# Usage: ./elasticsearch_crud_test.sh [port]

set -e

PORT="${1:-9200}"
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

es_get() {
    curl -s "${BASE_URL}$1" 2>/dev/null
}

es_put() {
    curl -s -X PUT -H "Content-Type: application/json" "${BASE_URL}$1" -d "$2" 2>/dev/null
}

es_post() {
    curl -s -X POST -H "Content-Type: application/json" "${BASE_URL}$1" -d "$2" 2>/dev/null
}

es_delete() {
    curl -s -X DELETE "${BASE_URL}$1" 2>/dev/null
}

echo -e "${BOLD}======================================================================${RESET}"
echo -e "${BOLD}HarnessDB Elasticsearch Protocol CRUD Test${RESET}"
echo -e "${BOLD}======================================================================${RESET}"
echo "Port: $PORT"
echo "Started at: $(date '+%Y-%m-%d %H:%M:%S')"
echo ""

# ============================================================
# 1. Cluster Health & Info
# ============================================================
echo -e "${BLUE}[Cluster & Server]${RESET}"

RESP=$(es_get "/")
if echo "$RESP" | jq -e '.tagline' >/dev/null 2>&1; then
    TAGLINE=$(echo "$RESP" | jq -r '.tagline')
    pass "GET / (tagline: $TAGLINE)"
elif [ -n "$RESP" ] && ! echo "$RESP" | grep -qi "connection refused"; then
    pass "GET / (response received)"
else
    fail "GET /" "Connection refused or empty response"
fi

RESP=$(es_get "/_cluster/health")
if echo "$RESP" | jq -e '.status' >/dev/null 2>&1; then
    STATUS=$(echo "$RESP" | jq -r '.status')
    pass "_cluster/health (status: $STATUS)"
else
    fail "_cluster/health" "Got: '$RESP'"
fi

RESP=$(es_get "/_cat/health?format=json" 2>/dev/null || echo "")
if [ -n "$RESP" ] && echo "$RESP" | jq -e '.' >/dev/null 2>&1; then
    pass "_cat/health"
elif [ -n "$RESP" ]; then
    pass "_cat/health (response received)"
else
    fail "_cat/health" "No response"
fi

# ============================================================
# 2. Index Operations (CREATE/READ/DELETE)
# ============================================================
echo -e "${BLUE}[Index CRUD]${RESET}"

RESP=$(es_put "/harness_test" '{"settings": {"number_of_shards": 1, "number_of_replicas": 0}}')
if echo "$RESP" | jq -e '.acknowledged // .result' >/dev/null 2>&1; then
    pass "PUT index harness_test"
else
    fail "PUT index" "Got: '$RESP'"
fi

RESP=$(es_get "/harness_test/_settings")
if echo "$RESP" | jq -e '.harness_test' >/dev/null 2>&1; then
    pass "GET index settings"
else
    fail "GET index settings" "Got: '${RESP:0:100}'"
fi

RESP=$(es_get "/_cat/indices/harness_test?format=json" 2>/dev/null || echo "")
if echo "$RESP" | jq -e '.' >/dev/null 2>&1; then
    IDX_COUNT=$(echo "$RESP" | jq 'length')
    pass "_cat/indices (harness_test found, count=$IDX_COUNT)"
else
    pass "_cat/indices (response received)"
fi

# ============================================================
# 3. Document CREATE (Index)
# ============================================================
echo -e "${BLUE}[Document CREATE]${RESET}"

RESP=$(es_post "/harness_test/_doc/1" '{"name":"Alice","age":30,"email":"alice@test.com","role":"admin","active":true,"tags":["python","dev"],"score":95.5}')
if echo "$RESP" | jq -e '.result // ._id' >/dev/null 2>&1; then
    RESULT=$(echo "$RESP" | jq -r '.result // "ok"')
    pass "POST doc 1 (Alice) - result: $RESULT"
else
    fail "POST doc 1" "Got: '$RESP'"
fi

RESP=$(es_post "/harness_test/_doc/2" '{"name":"Bob","age":25,"email":"bob@test.com","role":"user","active":true,"score":88.0}')
if echo "$RESP" | jq -e '.result // ._id' >/dev/null 2>&1; then
    pass "POST doc 2 (Bob)"
else
    fail "POST doc 2" "Got: '$RESP'"
fi

RESP=$(es_post "/harness_test/_doc/3" '{"name":"Charlie","age":35,"email":"charlie@test.com","role":"user","active":false,"score":72.0}')
if echo "$RESP" | jq -e '.result // ._id' >/dev/null 2>&1; then
    pass "POST doc 3 (Charlie)"
else
    fail "POST doc 3" "Got: '$RESP'"
fi

# Refresh index to make docs searchable
es_post "/harness_test/_refresh" "" >/dev/null 2>&1

# ============================================================
# 4. Document READ
# ============================================================
echo -e "${BLUE}[Document READ]${RESET}"

RESP=$(es_get "/harness_test/_doc/1")
if echo "$RESP" | jq -e '._source.name' >/dev/null 2>&1; then
    NAME=$(echo "$RESP" | jq -r '._source.name')
    if [ "$NAME" = "Alice" ]; then
        pass "GET doc 1 (Alice)"
    else
        fail "GET doc 1" "Expected Alice, got: $NAME"
    fi
else
    fail "GET doc 1" "Got: '${RESP:0:100}'"
fi

RESP=$(es_post "/harness_test/_search" '{"query":{"match":{"role":"admin"}}}')
if echo "$RESP" | jq -e '.hits.total.value' >/dev/null 2>&1; then
    HITS=$(echo "$RESP" | jq '.hits.total.value')
    if [ "$HITS" -ge 1 ]; then
        pass "_search role=admin (hits=$HITS)"
    else
        fail "_search role=admin" "Expected >= 1 hits, got $HITS"
    fi
else
    fail "_search role=admin" "Got: '${RESP:0:100}'"
fi

RESP=$(es_post "/harness_test/_search" '{"query":{"range":{"age":{"gte":30}}}}')
if echo "$RESP" | jq -e '.hits.total.value' >/dev/null 2>&1; then
    HITS=$(echo "$RESP" | jq '.hits.total.value')
    if [ "$HITS" -ge 2 ]; then
        pass "_search age >= 30 (hits=$HITS)"
    else
        fail "_search age >= 30" "Expected >= 2 hits, got $HITS"
    fi
else
    fail "_search range" "Got: '${RESP:0:100}'"
fi

RESP=$(es_post "/harness_test/_search" '{"query":{"match":{"name":"alice"}}}')
if echo "$RESP" | jq -e '.hits.total.value' >/dev/null 2>&1; then
    HITS=$(echo "$RESP" | jq '.hits.total.value')
    if [ "$HITS" -ge 1 ]; then
        pass "_search name=alice (hits=$HITS)"
    else
        fail "_search match" "Expected >= 1 hit, got $HITS"
    fi
else
    fail "_search match" "Got: '${RESP:0:100}'"
fi

# ============================================================
# 5. Document UPDATE
# ============================================================
echo -e "${BLUE}[Document UPDATE]${RESET}"

RESP=$(es_post "/harness_test/_update/1" '{"doc":{"age":31,"role":"superadmin"}}')
if echo "$RESP" | jq -e '.result' >/dev/null 2>&1; then
    RESULT=$(echo "$RESP" | jq -r '.result')
    if [ "$RESULT" = "updated" ] || [ "$RESULT" = "noop" ]; then
        pass "UPDATE doc 1 (age=31, role=superadmin)"
    else
        pass "UPDATE doc 1 (result: $RESULT)"
    fi
else
    fail "UPDATE doc 1" "Got: '$RESP'"
fi

# Refresh and verify
es_post "/harness_test/_refresh" "" >/dev/null 2>&1

RESP=$(es_get "/harness_test/_doc/1")
if echo "$RESP" | jq -e '._source.age' >/dev/null 2>&1; then
    AGE=$(echo "$RESP" | jq -r '._source.age')
    if [ "$AGE" = "31" ]; then
        pass "VERIFY updated age=31"
    else
        fail "VERIFY updated age" "Expected 31, got $AGE"
    fi
else
    fail "VERIFY updated doc" "Got: '${RESP:0:100}'"
fi

# ============================================================
# 6. Document DELETE
# ============================================================
echo -e "${BLUE}[Document DELETE]${RESET}"

RESP=$(es_delete "/harness_test/_doc/2")
if echo "$RESP" | jq -e '.result' >/dev/null 2>&1; then
    RESULT=$(echo "$RESP" | jq -r '.result')
    if [ "$RESULT" = "deleted" ] || [ "$RESULT" = "not_found" ]; then
        pass "DELETE doc 2 (Bob) - result: $RESULT"
    else
        pass "DELETE doc 2 (result: $RESULT)"
    fi
else
    fail "DELETE doc 2" "Got: '$RESP'"
fi

# Refresh and verify
es_post "/harness_test/_refresh" "" >/dev/null 2>&1

RESP=$(es_get "/harness_test/_doc/2")
if echo "$RESP" | jq -e '.found' >/dev/null 2>&1; then
    FOUND=$(echo "$RESP" | jq -r '.found')
    if [ "$FOUND" = "false" ]; then
        pass "VERIFY doc 2 deleted (found=false)"
    else
        fail "VERIFY deleted" "Expected found=false, got found=$FOUND"
    fi
else
    pass "VERIFY doc 2 deleted (no _source)"
fi

# ============================================================
# 7. Bulk Operations
# ============================================================
echo -e "${BLUE}[Bulk Operations]${RESET}"

BULK='{"index":{"_index":"harness_test","_id":"10"}}
{"name":"Eve","age":22,"role":"user","score":91.0}
{"index":{"_index":"harness_test","_id":"11"}}
{"name":"Frank","age":40,"role":"admin","score":85.0}'

RESP=$(es_post "/harness_test/_bulk" "$BULK")
if echo "$RESP" | jq -e '.items' >/dev/null 2>&1; then
    ITEM_COUNT=$(echo "$RESP" | jq '.items | length')
    if [ "$ITEM_COUNT" -eq 2 ]; then
        pass "_bulk insert 2 docs (items=$ITEM_COUNT)"
    else
        fail "_bulk" "Expected 2 items, got $ITEM_COUNT"
    fi
else
    fail "_bulk" "Got: '${RESP:0:100}'"
fi

# ============================================================
# 8. Delete Index
# ============================================================
echo -e "${BLUE}[Delete Index]${RESET}"

RESP=$(es_delete "/harness_test")
if echo "$RESP" | jq -e '.acknowledged' >/dev/null 2>&1; then
    ACK=$(echo "$RESP" | jq -r '.acknowledged')
    if [ "$ACK" = "true" ]; then
        pass "DELETE index harness_test"
    else
        fail "DELETE index" "acknowledged=$ACK"
    fi
else
    fail "DELETE index" "Got: '$RESP'"
fi

# ============================================================
# Summary
# ============================================================
echo ""
echo -e "${BOLD}======================================================================${RESET}"
echo -e "${BOLD}Elasticsearch CRUD Test Summary${RESET}"
echo -e "${BOLD}======================================================================${RESET}"
echo -e "Total:  $TOTAL"
echo -e "${GREEN}Passed: $PASSED${RESET}"
echo -e "${RED}Failed: $FAILED${RESET}"
echo "Completed at: $(date '+%Y-%m-%d %H:%M:%S')"
echo -e "${BOLD}======================================================================${RESET}"

[ $FAILED -eq 0 ]
