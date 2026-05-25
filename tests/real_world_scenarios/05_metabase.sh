#!/bin/bash
# Metabase - Analytics platform (github.com/metabase/metabase)
MYSQL="mysql -h 127.0.0.1 -P 9030 -uroot --skip-column-names"
PASS=0; FAIL=0

test_sql() {
    if output=$(echo "$2" | $MYSQL 2>&1); then
        PASS=$((PASS + 1)); echo "  ✓ $1"
    else
        FAIL=$((FAIL + 1)); echo "  ✗ $1: $output"
    fi
}

echo "=== Metabase Analytics Scenario ==="
test_sql "Create DB" "CREATE DATABASE IF NOT EXISTS metabase"
test_sql "Use DB" "USE metabase"
test_sql "Create queries" "CREATE TABLE IF NOT EXISTS queries (id INT, name VARCHAR(255), query TEXT, created_at DATETIME)"
test_sql "Create cards" "CREATE TABLE IF NOT EXISTS cards (id INT, name VARCHAR(255), query_id INT, display VARCHAR(255))"
test_sql "Create databases" "CREATE TABLE IF NOT EXISTS databases (id INT, name VARCHAR(255), engine VARCHAR(255))"
test_sql "Insert query" "INSERT INTO queries VALUES (1, 'Monthly Revenue', 'SELECT SUM(revenue) FROM sales', NOW())"
test_sql "Insert card" "INSERT INTO cards VALUES (1, 'Revenue Chart', 1, 'line')"
test_sql "Insert database" "INSERT INTO databases VALUES (1, 'Production', 'mysql')"
test_sql "List saved questions" "SELECT q.name, c.display FROM queries q JOIN cards c ON q.id = c.query_id"
test_sql "Question count" "SELECT COUNT(*) FROM queries"
test_sql "Card types" "SELECT display, COUNT(*) FROM cards GROUP BY display"

echo "Passed: $PASS / $((PASS + FAIL))"
