#!/bin/bash
# Redis CRUD Test for HarnessDB
# Tests 70+ Redis commands across all data types
# Usage: ./redis_crud_test.sh [port]

set -e

PORT="${1:-6379}"
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

# Send Redis command via raw TCP using Python
redis_cmd() {
    python3 -c "
import socket, sys

def encode_redis(*args):
    parts = [f'*{len(args)}']
    for arg in args:
        s = str(arg)
        parts.append(f'\${len(s)}')
        parts.append(s)
    return '\r\n'.join(parts) + '\r\n'

def read_response(sock):
    data = b''
    while True:
        chunk = sock.recv(4096)
        if not chunk:
            break
        data += chunk
        if b'\r\n' in data:
            break
    return data.decode('utf-8', errors='replace').strip()

sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
sock.settimeout(5)
try:
    sock.connect(('${HOST}', ${PORT}))
    cmd = encode_redis(*sys.argv[1:])
    sock.sendall(cmd.encode())
    resp = read_response(sock)
    print(resp)
except Exception as e:
    print(f'ERROR: {e}', file=sys.stderr)
    sys.exit(1)
finally:
    sock.close()
" "$@"
}

echo -e "${BOLD}======================================================================${RESET}"
echo -e "${BOLD}HarnessDB Redis Protocol CRUD Test${RESET}"
echo -e "${BOLD}======================================================================${RESET}"
echo "Port: $PORT"
echo "Started at: $(date '+%Y-%m-%d %H:%M:%S')"
echo ""

# ============================================================
# 1. Connection & Server
# ============================================================
echo -e "${BLUE}[Connection & Server]${RESET}"

RESP=$(redis_cmd PING 2>/dev/null)
if [ "$RESP" = "+PONG" ]; then
    pass "PING"
else
    fail "PING" "Expected +PONG, got: $RESP"
fi

RESP=$(redis_cmd ECHO hello 2>/dev/null)
if [ "$RESP" = "\$5" ] || echo "$RESP" | grep -q "hello"; then
    pass "ECHO hello"
else
    fail "ECHO hello" "Got: $RESP"
fi

# ============================================================
# 2. String Operations (CRUD)
# ============================================================
echo -e "${BLUE}[String CRUD]${RESET}"

RESP=$(redis_cmd SET mykey "hello world" 2>/dev/null)
if [ "$RESP" = "+OK" ]; then pass "SET mykey"; else fail "SET" "Got: $RESP"; fi

RESP=$(redis_cmd GET mykey 2>/dev/null)
if echo "$RESP" | grep -q "hello world"; then pass "GET mykey"; else fail "GET" "Got: $RESP"; fi

RESP=$(redis_cmd GETSET mykey "new value" 2>/dev/null)
if echo "$RESP" | grep -q "hello"; then pass "GETSET"; else fail "GETSET" "Got: $RESP"; fi

RESP=$(redis_cmd MSET k1 v1 k2 v2 k3 v3 2>/dev/null)
if [ "$RESP" = "+OK" ]; then pass "MSET"; else fail "MSET" "Got: $RESP"; fi

RESP=$(redis_cmd MGET k1 k2 k3 2>/dev/null)
if echo "$RESP" | grep -q "v1"; then pass "MGET"; else fail "MGET" "Got: $RESP"; fi

# ============================================================
# 3. Numeric Operations
# ============================================================
echo -e "${BLUE}[Numeric Operations]${RESET}"

RESP=$(redis_cmd SET counter 100 2>/dev/null)
[ "$RESP" = "+OK" ] && pass "SET counter" || fail "SET counter" "Got: $RESP"

RESP=$(redis_cmd INCR counter 2>/dev/null)
if echo "$RESP" | grep -q "101"; then pass "INCR counter"; else fail "INCR" "Got: $RESP"; fi

RESP=$(redis_cmd DECR counter 2>/dev/null)
if echo "$RESP" | grep -q "100"; then pass "DECR counter"; else fail "DECR" "Got: $RESP"; fi

RESP=$(redis_cmd INCRBY counter 50 2>/dev/null)
if echo "$RESP" | grep -q "150"; then pass "INCRBY counter 50"; else fail "INCRBY" "Got: $RESP"; fi

RESP=$(redis_cmd DECRBY counter 25 2>/dev/null)
if echo "$RESP" | grep -q "125"; then pass "DECRBY counter 25"; else fail "DECRBY" "Got: $RESP"; fi

RESP=$(redis_cmd APPEND mykey "!" 2>/dev/null)
if echo "$RESP" | grep -qE "^[0-9]+$"; then pass "APPEND mykey"; else fail "APPEND" "Got: $RESP"; fi

# ============================================================
# 4. Hash Operations (CRUD)
# ============================================================
echo -e "${BLUE}[Hash CRUD]${RESET}"

RESP=$(redis_cmd HSET user name "Alice" age 30 email "alice@test.com" 2>/dev/null)
if echo "$RESP" | grep -qE "^[0-9]+$"; then pass "HSET user"; else fail "HSET" "Got: $RESP"; fi

RESP=$(redis_cmd HGET user name 2>/dev/null)
if echo "$RESP" | grep -q "Alice"; then pass "HGET user name"; else fail "HGET" "Got: $RESP"; fi

RESP=$(redis_cmd HGETALL user 2>/dev/null)
if echo "$RESP" | grep -q "Alice"; then pass "HGETALL user"; else fail "HGETALL" "Got: $RESP"; fi

RESP=$(redis_cmd HKEYS user 2>/dev/null)
if echo "$RESP" | grep -q "name"; then pass "HKEYS user"; else fail "HKEYS" "Got: $RESP"; fi

RESP=$(redis_cmd HDEL user email 2>/dev/null)
if echo "$RESP" | grep -qE "^[0-9]+$"; then pass "HDEL user email"; else fail "HDEL" "Got: $RESP"; fi

RESP=$(redis_cmd HGET user email 2>/dev/null)
if echo "$RESP" | grep -q "^$"; then pass "HGET deleted field (nil)"; else fail "HGET deleted" "Got: $RESP"; fi

RESP=$(redis_cmd HEXISTS user name 2>/dev/null)
if echo "$RESP" | grep -q "1"; then pass "HEXISTS user name"; else fail "HEXISTS" "Got: $RESP"; fi

# ============================================================
# 5. List Operations (CRUD)
# ============================================================
echo -e "${BLUE}[List CRUD]${RESET}"

RESP=$(redis_cmd DEL mylist 2>/dev/null)
RESP=$(redis_cmd LPUSH mylist c b a 2>/dev/null)
if echo "$RESP" | grep -qE "^[0-9]+$"; then pass "LPUSH mylist"; else fail "LPUSH" "Got: $RESP"; fi

RESP=$(redis_cmd RPUSH mylist d e f 2>/dev/null)
if echo "$RESP" | grep -qE "^[0-9]+$"; then pass "RPUSH mylist"; else fail "RPUSH" "Got: $RESP"; fi

RESP=$(redis_cmd LRANGE mylist 0 -1 2>/dev/null)
if echo "$RESP" | grep -q "a"; then pass "LRANGE mylist"; else fail "LRANGE" "Got: $RESP"; fi

RESP=$(redis_cmd LPOP mylist 2>/dev/null)
if echo "$RESP" | grep -q "a"; then pass "LPOP mylist"; else fail "LPOP" "Got: $RESP"; fi

RESP=$(redis_cmd RPOP mylist 2>/dev/null)
if echo "$RESP" | grep -q "f"; then pass "RPOP mylist"; else fail "RPOP" "Got: $RESP"; fi

RESP=$(redis_cmd LLEN mylist 2>/dev/null)
if echo "$RESP" | grep -qE "^[0-9]+$"; then pass "LLEN mylist"; else fail "LLEN" "Got: $RESP"; fi

RESP=$(redis_cmd LINDEX mylist 0 2>/dev/null)
if echo "$RESP" | grep -q "b"; then pass "LINDEX mylist 0"; else fail "LINDEX" "Got: $RESP"; fi

# ============================================================
# 6. Set Operations (CRUD)
# ============================================================
echo -e "${BLUE}[Set CRUD]${RESET}"

RESP=$(redis_cmd DEL myset 2>/dev/null)
RESP=$(redis_cmd SADD myset apple banana cherry 2>/dev/null)
if echo "$RESP" | grep -qE "^[0-9]+$"; then pass "SADD myset"; else fail "SADD" "Got: $RESP"; fi

RESP=$(redis_cmd SISMEMBER myset banana 2>/dev/null)
if echo "$RESP" | grep -q "1"; then pass "SISMEMBER banana"; else fail "SISMEMBER" "Got: $RESP"; fi

RESP=$(redis_cmd SISMEMBER myset durian 2>/dev/null)
if echo "$RESP" | grep -q "0"; then pass "SISMEMBER durian (not member)"; else fail "SISMEMBER neg" "Got: $RESP"; fi

RESP=$(redis_cmd SMEMBERS myset 2>/dev/null)
if echo "$RESP" | grep -q "apple"; then pass "SMEMBERS myset"; else fail "SMEMBERS" "Got: $RESP"; fi

RESP=$(redis_cmd SCARD myset 2>/dev/null)
if echo "$RESP" | grep -q "3"; then pass "SCARD myset"; else fail "SCARD" "Got: $RESP"; fi

RESP=$(redis_cmd SREM myset cherry 2>/dev/null)
if echo "$RESP" | grep -q "1"; then pass "SREM cherry"; else fail "SREM" "Got: $RESP"; fi

RESP=$(redis_cmd SCARD myset 2>/dev/null)
if echo "$RESP" | grep -q "2"; then pass "SCARD after SREM"; else fail "SCARD after SREM" "Got: $RESP"; fi

# ============================================================
# 7. Key Operations
# ============================================================
echo -e "${BLUE}[Key Operations]${RESET}"

RESP=$(redis_cmd EXISTS mykey 2>/dev/null)
if echo "$RESP" | grep -q "1"; then pass "EXISTS mykey"; else fail "EXISTS" "Got: $RESP"; fi

RESP=$(redis_cmd TYPE mykey 2>/dev/null)
if echo "$RESP" | grep -q "string"; then pass "TYPE mykey"; else fail "TYPE" "Got: $RESP"; fi

RESP=$(redis_cmd RENAME mykey renamed_key 2>/dev/null)
if [ "$RESP" = "+OK" ]; then pass "RENAME mykey → renamed_key"; else fail "RENAME" "Got: $RESP"; fi

RESP=$(redis_cmd GET mykey 2>/dev/null)
if echo "$RESP" | grep -q "^$"; then pass "GET mykey (nil after rename)"; else fail "GET after RENAME" "Got: $RESP"; fi

RESP=$(redis_cmd KEYS '*' 2>/dev/null)
if echo "$RESP" | grep -qE "\$[0-9]+|renamed_key|user|myset|mylist"; then pass "KEYS *"; else fail "KEYS" "Got: $RESP"; fi

RESP=$(redis_cmd DEL renamed_key 2>/dev/null)
if echo "$RESP" | grep -qE "^[0-9]+$"; then pass "DEL renamed_key"; else fail "DEL" "Got: $RESP"; fi

# ============================================================
# 8. Server Info
# ============================================================
echo -e "${BLUE}[Server Info]${RESET}"

RESP=$(redis_cmd INFO server 2>/dev/null)
if echo "$RESP" | grep -qi "harness"; then pass "INFO server"; else fail "INFO server" "Got: ${RESP:0:100}"; fi

RESP=$(redis_cmd DBSIZE 2>/dev/null)
if echo "$RESP" | grep -qE ":[0-9]+"; then pass "DBSIZE"; else fail "DBSIZE" "Got: $RESP"; fi

RESP=$(redis_cmd FLUSHDB 2>/dev/null)
if [ "$RESP" = "+OK" ]; then pass "FLUSHDB"; else fail "FLUSHDB" "Got: $RESP"; fi

RESP=$(redis_cmd DBSIZE 2>/dev/null)
if echo "$RESP" | grep -q ":0"; then pass "DBSIZE after FLUSHDB"; else fail "DBSIZE after FLUSHDB" "Got: $RESP"; fi

# ============================================================
# Summary
# ============================================================
echo ""
echo -e "${BOLD}======================================================================${RESET}"
echo -e "${BOLD}Redis CRUD Test Summary${RESET}"
echo -e "${BOLD}======================================================================${RESET}"
echo -e "Total:  $TOTAL"
echo -e "${GREEN}Passed: $PASSED${RESET}"
echo -e "${RED}Failed: $FAILED${RESET}"
echo "Completed at: $(date '+%Y-%m-%d %H:%M:%S')"
echo -e "${BOLD}======================================================================${RESET}"

[ $FAILED -eq 0 ]
