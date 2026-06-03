#!/bin/bash
# Airbyte - Data Integration (github.com/airbytehq/airbyte)
MYSQL="mysql -h 127.0.0.1 -P 9030 -uroot --skip-column-names"
PASS=0; FAIL=0

test_sql() {
    if output=$(echo "$2" | $MYSQL 2>&1); then
        PASS=$((PASS + 1)); echo "  ✓ $1"
    else
        FAIL=$((FAIL + 1)); echo "  ✗ $1: $output"
    fi
}

echo "=== Airbyte ETL Scenario ==="
test_sql "Create DB" "CREATE DATABASE IF NOT EXISTS airbyte"
test_sql "Use DB" "USE airbyte"
test_sql "Create connections" "CREATE TABLE IF NOT EXISTS connections (id INT, name VARCHAR(255), source_id INT, destination_id INT, status VARCHAR(50))"
test_sql "Create sources" "CREATE TABLE IF NOT EXISTS sources (id INT, name VARCHAR(255), source_type VARCHAR(50), config TEXT)"
test_sql "Create destinations" "CREATE TABLE IF NOT EXISTS destinations (id INT, name VARCHAR(255), destination_type VARCHAR(50), config TEXT)"
test_sql "Create syncs" "CREATE TABLE IF NOT EXISTS syncs (id INT, connection_id INT, status VARCHAR(50), started_at DATETIME, completed_at DATETIME, bytes_synced BIGINT)"
test_sql "Create raw_tables" "CREATE TABLE IF NOT EXISTS raw_customers (id INT, name VARCHAR(255), email VARCHAR(255), updated_at DATETIME)"
test_sql "Insert source" "INSERT INTO sources VALUES (1, 'Production DB', 'postgres', '{}')"
test_sql "Insert destination" "INSERT INTO destinations VALUES (1, 'Warehouse', 'bigquery', '{}')"
test_sql "Insert connection" "INSERT INTO connections VALUES (1, 'Customer Sync', 1, 1, 'active')"
test_sql "Insert sync" "INSERT INTO syncs VALUES (1, 1, 'success', NOW(), NOW(), 1024000)"
test_sql "Insert raw data" "INSERT INTO raw_customers VALUES (1, 'Alice', 'alice@example.com', NOW()), (2, 'Bob', 'bob@example.com', NOW())"
test_sql "Sync history" "SELECT c.name, s.status, s.bytes_synced FROM connections c JOIN syncs s ON c.id = s.connection_id"
test_sql "Data volume" "SELECT SUM(bytes_synced) FROM syncs"
test_sql "Raw data query" "SELECT COUNT(*) FROM raw_customers"
test_sql "CDC watermark" "SELECT MAX(updated_at) FROM raw_customers"

echo "Passed: $PASS / $((PASS + FAIL))"
