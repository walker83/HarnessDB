#!/bin/bash
# Grafana - Monitoring & Observability (github.com/grafana/grafana)
MYSQL="mysql -h 127.0.0.1 -P 9030 -uroot --skip-column-names"
PASS=0; FAIL=0

test_sql() {
    if output=$(echo "$2" | $MYSQL 2>&1); then
        PASS=$((PASS + 1)); echo "  ✓ $1"
    else
        FAIL=$((FAIL + 1)); echo "  ✗ $1: $output"
    fi
}

echo "=== Grafana Monitoring Scenario ==="
test_sql "Create DB" "CREATE DATABASE IF NOT EXISTS grafana"
test_sql "Use DB" "USE grafana"
test_sql "Create dashboards" "CREATE TABLE IF NOT EXISTS dashboard (id INT, uid VARCHAR(40), title VARCHAR(255), data TEXT, created DATETIME, updated DATETIME)"
test_sql "Create data_sources" "CREATE TABLE IF NOT EXISTS data_source (id INT, name VARCHAR(190), type VARCHAR(255), url TEXT, created DATETIME)"
test_sql "Create metrics" "CREATE TABLE IF NOT EXISTS metric (id INT, source_id INT, name VARCHAR(255), value DECIMAL(20,6), timestamp DATETIME)"
test_sql "Insert dashboard" "INSERT INTO dashboard VALUES (1, 'abc123', 'System Monitoring', '{}', NOW(), NOW())"
test_sql "Insert data source" "INSERT INTO data_source VALUES (1, 'Prometheus', 'prometheus', 'http://localhost:9090', NOW())"
test_sql "Insert metrics" "INSERT INTO metric VALUES (1, 1, 'cpu_usage', 45.5, NOW()), (2, 1, 'memory_usage', 72.3, NOW())"
test_sql "Time series query" "SELECT timestamp, value FROM metric WHERE name = 'cpu_usage' AND timestamp >= DATE_SUB(NOW(), INTERVAL 1 HOUR) ORDER BY timestamp"
test_sql "Aggregate metrics" "SELECT name, AVG(value), MAX(value), MIN(value) FROM metric GROUP BY name"
test_sql "Dashboard list" "SELECT title, updated FROM dashboard ORDER BY updated DESC"
test_sql "Data source config" "SELECT name, type, url FROM data_source"

echo "Passed: $PASS / $((PASS + FAIL))"
