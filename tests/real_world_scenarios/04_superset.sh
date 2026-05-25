#!/bin/bash
# Apache Superset - Business Intelligence (github.com/apache/superset)
MYSQL="mysql -h 127.0.0.1 -P 9030 -uroot --skip-column-names"
PASS=0; FAIL=0

test_sql() {
    if output=$(echo "$2" | $MYSQL 2>&1); then
        PASS=$((PASS + 1)); echo "  ✓ $1"
    else
        FAIL=$((FAIL + 1)); echo "  ✗ $1: $output"
    fi
}

echo "=== Apache Superset BI Scenario ==="
test_sql "Create DB" "CREATE DATABASE IF NOT EXISTS superset"
test_sql "Use DB" "USE superset"
test_sql "Create tables" "CREATE TABLE IF NOT EXISTS tables (id INT, table_name VARCHAR(250), database_id INT)"
test_sql "Create columns" "CREATE TABLE IF NOT EXISTS columns (id INT, table_id INT, column_name VARCHAR(255), type VARCHAR(32))"
test_sql "Create slices" "CREATE TABLE IF NOT EXISTS slices (id INT, slice_name VARCHAR(250), viz_type VARCHAR(250), datasource_id INT)"
test_sql "Create dashboards" "CREATE TABLE IF NOT EXISTS dashboards (id INT, dashboard_title VARCHAR(500), slug VARCHAR(255))"
test_sql "Insert table" "INSERT INTO tables VALUES (1, 'sales_data', 1)"
test_sql "Insert columns" "INSERT INTO columns VALUES (1, 1, 'revenue', 'DECIMAL'), (2, 1, 'date', 'DATE')"
test_sql "Insert slice" "INSERT INTO slices VALUES (1, 'Revenue Chart', 'line', 1)"
test_sql "Insert dashboard" "INSERT INTO dashboards VALUES (1, 'Sales Overview', 'sales-overview')"
test_sql "Query table metadata" "SELECT t.table_name, c.column_name, c.type FROM tables t JOIN columns c ON t.id = c.table_id"
test_sql "List dashboards" "SELECT dashboard_title FROM dashboards"
test_sql "Complex aggregation" "SELECT viz_type, COUNT(*) FROM slices GROUP BY viz_type"

echo "Passed: $PASS / $((PASS + FAIL))"
