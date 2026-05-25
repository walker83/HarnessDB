#!/bin/bash
# dbt - Data Build Tool (github.com/dbt-labs/dbt-core)
MYSQL="mysql -h 127.0.0.1 -P 9030 -uroot --skip-column-names"
PASS=0; FAIL=0

test_sql() {
    if output=$(echo "$2" | $MYSQL 2>&1); then
        PASS=$((PASS + 1)); echo "  ✓ $1"
    else
        FAIL=$((FAIL + 1)); echo "  ✗ $1: $output"
    fi
}

echo "=== dbt Data Transformation Scenario ==="
test_sql "Create DB" "CREATE DATABASE IF NOT EXISTS dbt"
test_sql "Use DB" "USE dbt"
test_sql "Create raw events" "CREATE TABLE IF NOT EXISTS raw_events (id INT, user_id INT, event_type VARCHAR(50), occurred_at DATETIME)"
test_sql "Create stg events" "CREATE TABLE IF NOT EXISTS stg_events (event_id INT, user_id INT, event_type VARCHAR(50), event_date DATE)"
test_sql "Create fct events" "CREATE TABLE IF NOT EXISTS fct_user_events (user_id INT, event_count INT, first_event DATE, last_event DATE)"
test_sql "Insert raw data" "INSERT INTO raw_events VALUES (1, 100, 'page_view', NOW()), (2, 100, 'click', NOW()), (3, 200, 'page_view', NOW())"
test_sql "Stage transform" "INSERT INTO stg_events SELECT id, user_id, event_type, DATE(occurred_at) FROM raw_events"
test_sql "Aggregate transform" "INSERT INTO fct_user_events SELECT user_id, COUNT(*), MIN(event_date), MAX(event_date) FROM stg_events GROUP BY user_id"
test_sql "Query models" "SELECT * FROM fct_user_events ORDER BY event_count DESC"
test_sql "Model lineage" "SELECT COUNT(*) as raw_count FROM raw_events"
test_sql "Model lineage 2" "SELECT COUNT(*) as stg_count FROM stg_events"
test_sql "Model lineage 3" "SELECT COUNT(*) as fct_count FROM fct_user_events"

echo "Passed: $PASS / $((PASS + FAIL))"
