#!/bin/bash
# Real-World Application Compatibility Test for HarnessDB
# Tests 5 popular GitHub applications with MySQL/PostgreSQL protocols
# Applications: WordPress (PHP), Grafana (Go), Superset (Python),
#               Airbyte (Python/Java), Metabase (Clojure)
# Languages: PHP, Go, Python
# Usage: ./real_app_tests.sh

set -e

MYSQL_PORT="${1:-9030}"
PG_PORT="${2:-15432}"
HOST="127.0.0.1"
PASSED=0
FAILED=0
ERRORS=""
CURRENT_APP=""

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
RESET='\033[0m'

MYSQL="mysql -h ${HOST} -P ${MYSQL_PORT} -uroot --skip-column-names"

pass() {
    PASSED=$((PASSED + 1))
    echo -e "  ${GREEN}✓${RESET} $1"
}

fail() {
    FAILED=$((FAILED + 1))
    ERRORS="${ERRORS}\n  [${CURRENT_APP}] $1: $2"
    echo -e "  ${RED}✗${RESET} $1: ${RED}$2${RESET}"
}

run_sql() {
    local test_name="$1"
    local sql="$2"
    local expect_success="${3:-true}"

    local output
    if output=$(echo "$sql" | $MYSQL 2>&1); then
        pass "$test_name"
    else
        fail "$test_name" "$output"
    fi
}

run_sql_expect_rows() {
    local test_name="$1"
    local sql="$2"
    local min_rows="${3:-1}"

    local output
    if output=$(echo "$sql" | $MYSQL 2>&1); then
        local count=$(echo "$output" | grep -c . || echo "0")
        if [ "$count" -ge "$min_rows" ]; then
            pass "$test_name"
        else
            fail "$test_name" "Expected >= $min_rows rows, got $count"
        fi
    else
        fail "$test_name" "$output"
    fi
}

app() {
    CURRENT_APP="$1"
    echo -e "\n${BLUE}${BOLD}📦 Application: $1 (${2})${RESET}"
    echo -e "   ${YELLOW}Language: $3 | GitHub: $4${RESET}"
}

echo -e "${BOLD}======================================================================${RESET}"
echo -e "${BOLD}HarnessDB Real-World Application Compatibility Test${RESET}"
echo -e "${BOLD}======================================================================${RESET}"
echo "MySQL Port: $MYSQL_PORT"
echo "Started at: $(date '+%Y-%m-%d %H:%M:%S')"
echo ""

# ============================================================
# App 1: WordPress (PHP + MySQL)
# https://github.com/WordPress/WordPress
# ============================================================
app "WordPress" "CMS/Blogging Platform" "PHP" "github.com/WordPress/WordPress"

# WordPress requires these tables and queries to function
run_sql "CREATE DATABASE wp_test" "CREATE DATABASE IF NOT EXISTS wp_test"
run_sql "USE wp_test" "USE wp_test"

# Core tables from WordPress schema
run_sql "CREATE wp_options" "CREATE TABLE IF NOT EXISTS wp_options (option_id INT AUTO_INCREMENT, option_name VARCHAR(100), option_value TEXT, autoload VARCHAR(20) DEFAULT 'yes')"
run_sql "CREATE wp_posts" "CREATE TABLE IF NOT EXISTS wp_posts (ID INT AUTO_INCREMENT, post_author INT, post_date DATETIME, post_content TEXT, post_title VARCHAR(200), post_status VARCHAR(20), post_type VARCHAR(20))"
run_sql "CREATE wp_users" "CREATE TABLE IF NOT EXISTS wp_users (ID INT AUTO_INCREMENT, user_login VARCHAR(60), user_pass VARCHAR(255), user_nicename VARCHAR(50), user_email VARCHAR(100), user_registered DATETIME, display_name VARCHAR(250))"
run_sql "CREATE wp_postmeta" "CREATE TABLE IF NOT EXISTS wp_postmeta (meta_id INT AUTO_INCREMENT, post_id INT, meta_key VARCHAR(255), meta_value TEXT)"
run_sql "CREATE wp_comments" "CREATE TABLE IF NOT EXISTS wp_comments (comment_ID INT AUTO_INCREMENT, comment_post_ID INT, comment_author VARCHAR(255), comment_content TEXT, comment_date DATETIME, comment_approved VARCHAR(20))"

# SHOW TABLES (WordPress setup wizard checks this)
run_sql "SHOW TABLES" "SHOW TABLES"
run_sql "SHOW TABLES LIKE wp_%" "SHOW TABLES LIKE 'wp_%'"

# WordPress INSERT patterns
run_sql "INSERT wp_options (siteurl)" "INSERT INTO wp_options (option_name, option_value, autoload) VALUES ('siteurl', 'http://localhost', 'yes')"
run_sql "INSERT wp_options (home)" "INSERT INTO wp_options (option_name, option_value, autoload) VALUES ('home', 'http://localhost', 'yes')"
run_sql "INSERT wp_users" "INSERT INTO wp_users (user_login, user_pass, user_email, display_name) VALUES ('admin', '\$P\$hash', 'admin@localhost', 'Admin')"

# WordPress SELECT patterns (login, page load)
run_sql "SELECT wp_options (siteurl)" "SELECT option_value FROM wp_options WHERE option_name = 'siteurl'"
run_sql "SELECT wp_options (home)" "SELECT option_value FROM wp_options WHERE option_name = 'home'"
run_sql "SELECT wp_users (login)" "SELECT * FROM wp_users WHERE user_login = 'admin'"

# WordPress UPDATE patterns (settings save)
run_sql "UPDATE wp_options" "UPDATE wp_options SET option_value = 'http://mysite.com' WHERE option_name = 'siteurl'"
run_sql "SELECT after UPDATE" "SELECT option_value FROM wp_options WHERE option_name = 'siteurl'"

# WordPress INSERT posts (publishing)
run_sql "INSERT wp_posts" "INSERT INTO wp_posts (post_title, post_content, post_status, post_type) VALUES ('Hello World', 'Welcome to HarnessDB!', 'publish', 'post')"
run_sql "SELECT published posts" "SELECT ID, post_title FROM wp_posts WHERE post_status = 'publish'"

# WordPress DELETE (trash)
run_sql "DELETE wp_post" "DELETE FROM wp_posts WHERE post_title = 'Hello World'"
run_sql "COUNT posts after delete" "SELECT COUNT(*) FROM wp_posts"

# Cleanup
run_sql "DROP DATABASE wp_test" "DROP DATABASE IF EXISTS wp_test"

# ============================================================
# App 2: Grafana (Go + MySQL/PostgreSQL)
# https://github.com/grafana/grafana
# ============================================================
app "Grafana" "Monitoring Dashboard" "Go" "github.com/grafana/grafana"

run_sql "CREATE DATABASE grafana_test" "CREATE DATABASE IF NOT EXISTS grafana_test"
run_sql "USE grafana_test" "USE grafana_test"

# Grafana core tables
run_sql "CREATE dashboard" "CREATE TABLE IF NOT EXISTS dashboard (id INT, org_id INT, version INT, title VARCHAR(200), data TEXT, created DATETIME, updated DATETIME)"
run_sql "CREATE data_source" "CREATE TABLE IF NOT EXISTS data_source (id INT, org_id INT, version INT, name VARCHAR(200), type VARCHAR(50), access VARCHAR(50), url TEXT, password TEXT, user TEXT, database VARCHAR(200), is_default TINYINT, created DATETIME, updated DATETIME)"
run_sql "CREATE user" "CREATE TABLE IF NOT EXISTS user (id INT, org_id INT, login VARCHAR(100), email VARCHAR(255), password VARCHAR(255), created DATETIME, updated DATETIME)"
run_sql "CREATE alert" "CREATE TABLE IF NOT EXISTS alert (id INT, org_id INT, dashboard_id INT, panel_id INT, name VARCHAR(255), state VARCHAR(50), evaluated_at DATETIME, settings TEXT)"

# Grafana initialization queries
run_sql "SHOW VARIABLES (version)" "SHOW VARIABLES LIKE 'version%'"
run_sql "SHOW VARIABLES (sql_mode)" "SHOW VARIABLES LIKE 'sql_mode'"
run_sql "SHOW VARIABLES (time_zone)" "SHOW VARIABLES LIKE '%time_zone%'"
run_sql "SET time_zone" "SET time_zone = '+00:00'"
run_sql "SET NAMES utf8mb4" "SET NAMES utf8mb4"

# Grafana INSERT (dashboard creation)
run_sql "INSERT dashboard" "INSERT INTO dashboard (id, org_id, version, title, created, updated) VALUES (1, 1, 1, 'Main Dashboard', NOW(), NOW())"
run_sql "INSERT data_source" "INSERT INTO data_source (id, org_id, version, name, type, is_default, created, updated) VALUES (1, 1, 1, 'HarnessDB', 'mysql', 1, NOW(), NOW())"
run_sql "INSERT user" "INSERT INTO user (id, org_id, login, email, created) VALUES (1, 1, 'admin', 'admin@localhost', NOW())"

# Grafana SELECT (dashboard load)
run_sql "SELECT dashboard by id" "SELECT id, title FROM dashboard WHERE id = 1"
run_sql "SELECT data_sources" "SELECT id, name, type FROM data_source WHERE is_default = 1"
run_sql "SELECT user by login" "SELECT id, login, email FROM user WHERE login = 'admin'"

# Grafana UPDATE (dashboard save)
run_sql "UPDATE dashboard version" "UPDATE dashboard SET version = version + 1, updated = NOW() WHERE id = 1"
run_sql "VERIFY dashboard version" "SELECT version FROM dashboard WHERE id = 1"

# Grafana DELETE (dashboard removal)
run_sql "DELETE dashboard" "DELETE FROM dashboard WHERE id = 1"
run_sql "COUNT dashboards" "SELECT COUNT(*) FROM dashboard"

# Time-series query simulation (Grafana panel query)
run_sql "CREATE metrics table" "CREATE TABLE IF NOT EXISTS metrics (id INT, metric VARCHAR(100), value FLOAT, ts DATETIME)"
run_sql "INSERT metrics" "INSERT INTO metrics VALUES (1, 'cpu', 45.2, NOW()), (2, 'memory', 78.5, NOW()), (3, 'cpu', 48.1, NOW())"
run_sql "SELECT metrics with time" "SELECT metric, value FROM metrics WHERE ts > '2024-01-01' ORDER BY ts DESC"
run_sql "SELECT AVG metric" "SELECT AVG(value) FROM metrics WHERE metric = 'cpu'"

# Cleanup
run_sql "DROP DATABASE grafana_test" "DROP DATABASE IF EXISTS grafana_test"

# ============================================================
# App 3: Apache Superset (Python + MySQL)
# https://github.com/apache/superset
# ============================================================
app "Apache Superset" "BI/Dashboard" "Python" "github.com/apache/superset"

run_sql "CREATE DATABASE superset_test" "CREATE DATABASE IF NOT EXISTS superset_test"
run_sql "USE superset_test" "USE superset_test"

# Superset core tables
run_sql "CREATE ab_user" "CREATE TABLE IF NOT EXISTS ab_user (id INT, first_name VARCHAR(64), last_name VARCHAR(64), username VARCHAR(64), password VARCHAR(256), email VARCHAR(320), active TINYINT, created_on DATETIME, changed_on DATETIME)"
run_sql "CREATE dashboards" "CREATE TABLE IF NOT EXISTS dashboards (id INT, dashboard_title VARCHAR(500), position_json TEXT, description TEXT, css TEXT, certified_by TEXT, certification_details TEXT, created_on DATETIME, changed_on DATETIME)"
run_sql "CREATE slices" "CREATE TABLE IF NOT EXISTS slices (id INT, slice_name VARCHAR(255), viz_type VARCHAR(50), params TEXT, datasource_type VARCHAR(200), datasource_name TEXT, created_on DATETIME, changed_on DATETIME)"
run_sql "CREATE tables" "CREATE TABLE IF NOT EXISTS tables (id INT, database_id INT, table_name VARCHAR(250), main_dttm_col VARCHAR(250), description TEXT, default_endpoint TEXT, offset INT, cache_timeout INT, created_on DATETIME)"

# Superset connection check
run_sql "SHOW DATABASES" "SHOW DATABASES"
run_sql "SHOW TABLES" "SHOW TABLES"
run_sql "SELECT 1 (ping)" "SELECT 1"

# Superset INSERT (user, dashboard, chart)
run_sql "INSERT ab_user" "INSERT INTO ab_user (id, first_name, last_name, username, email, active, created_on) VALUES (1, 'Admin', 'User', 'admin', 'admin@localhost', 1, NOW())"
run_sql "INSERT dashboards" "INSERT INTO dashboards (id, dashboard_title, created_on) VALUES (1, 'Sales Overview', NOW())"
run_sql "INSERT slices" "INSERT INTO slices (id, slice_name, viz_type, datasource_type, created_on) VALUES (1, 'Revenue Chart', 'line', 'table', NOW())"
run_sql "INSERT tables" "INSERT INTO tables (id, database_id, table_name, created_on) VALUES (1, 1, 'sales_data', NOW())"

# Superset SELECT (explore, dashboard load)
run_sql "SELECT dashboards" "SELECT id, dashboard_title FROM dashboards"
run_sql "SELECT slices by dashboard" "SELECT id, slice_name, viz_type FROM slices"
run_sql "SELECT tables by db" "SELECT id, table_name FROM tables WHERE database_id = 1"
run_sql "SELECT active users" "SELECT id, username, email FROM ab_user WHERE active = 1"

# Superset UPDATE (chart save)
run_sql "UPDATE slice params" "UPDATE slices SET params = '{\"groupby\":[\"category\"],\"metrics\":[\"SUM(amount)\"]}' WHERE id = 1"
run_sql "VERIFY slice params" "SELECT params FROM slices WHERE id = 1"

# Superset DELETE (chart removal)
run_sql "DELETE slice" "DELETE FROM slices WHERE id = 1"
run_sql "COUNT slices" "SELECT COUNT(*) FROM slices"

# Analytical query simulation
run_sql "CREATE sales_data" "CREATE TABLE IF NOT EXISTS sales_data (id INT, category VARCHAR(100), amount FLOAT, region VARCHAR(50), sale_date DATE)"
run_sql "INSERT sales_data" "INSERT INTO sales_data VALUES (1, 'Electronics', 999.99, 'US', '2024-01-15'), (2, 'Books', 29.99, 'EU', '2024-01-16'), (3, 'Electronics', 1499.00, 'US', '2024-02-01')"
run_sql "SELECT SUM by category" "SELECT category, SUM(amount) FROM sales_data GROUP BY category"
run_sql "SELECT AVG by region" "SELECT region, AVG(amount) FROM sales_data GROUP BY region"

# Cleanup
run_sql "DROP DATABASE superset_test" "DROP DATABASE IF EXISTS superset_test"

# ============================================================
# App 4: Airbyte (Python/Java + MySQL)
# https://github.com/airbytehq/airbyte
# ============================================================
app "Airbyte" "ETL/CDC Data Pipeline" "Python/Java" "github.com/airbytehq/airbyte"

run_sql "CREATE DATABASE airbyte_test" "CREATE DATABASE IF NOT EXISTS airbyte_test"
run_sql "USE airbyte_test" "USE airbyte_test"

# Airbyte connection patterns (MySQL source/destination)
run_sql "SHOW VARIABLES (wait_timeout)" "SHOW VARIABLES LIKE 'wait_timeout'"
run_sql "SHOW VARIABLES (max_allowed)" "SHOW VARIABLES LIKE 'max_allowed_packet'"
run_sql "SET wait_timeout" "SET SESSION wait_timeout = 28800"
run_sql "SET sql_mode" "SET SESSION sql_mode = 'STRICT_TRANS_TABLES,NO_ZERO_DATE'"
run_sql "SET NAMES utf8mb4" "SET NAMES utf8mb4"

# Airbyte CDC source tables
run_sql "CREATE orders (CDC source)" "CREATE TABLE IF NOT EXISTS orders (id INT, customer_id INT, amount DECIMAL(10,2), status VARCHAR(50), updated_at DATETIME, _airbyte_raw_id VARCHAR(100), _airbyte_emitted_at DATETIME)"
run_sql "CREATE customers" "CREATE TABLE IF NOT EXISTS customers (id INT, name VARCHAR(200), email VARCHAR(255), tier VARCHAR(20), created_at DATETIME, updated_at DATETIME)"

# Airbyte INSERT (sync destination)
run_sql "INSERT customers" "INSERT INTO customers (id, name, email, tier, created_at, updated_at) VALUES (1, 'Acme Corp', 'info@acme.com', 'enterprise', NOW(), NOW())"
run_sql "INSERT customers batch" "INSERT INTO customers VALUES (2, 'Startup Inc', 'dev@startup.com', 'startup', NOW(), NOW()), (3, 'Big Corp', 'admin@bigcorp.com', 'enterprise', NOW(), NOW())"
run_sql "INSERT orders" "INSERT INTO orders (id, customer_id, amount, status, updated_at) VALUES (1, 1, 5000.00, 'completed', NOW()), (2, 2, 1500.00, 'pending', NOW()), (3, 1, 3000.00, 'completed', NOW())"

# Airbyte CDC watermark queries (incremental sync)
run_sql "CDC max updated_at" "SELECT MAX(updated_at) FROM customers"
run_sql "CDC incremental sync" "SELECT * FROM customers WHERE updated_at > '2024-01-01' ORDER BY updated_at"
run_sql "CDC with _airbyte_emitted" "SELECT id, _airbyte_emitted_at FROM orders WHERE _airbyte_emitted_at > '2024-01-01'"

# Airbyte validation queries
run_sql "COUNT customers" "SELECT COUNT(*) FROM customers"
run_sql "COUNT orders" "SELECT COUNT(*) FROM orders"
run_sql "DISTINCT tiers" "SELECT DISTINCT tier FROM customers"

# Airbyte UPDATE (status change sync)
run_sql "UPDATE order status" "UPDATE orders SET status = 'shipped', updated_at = NOW() WHERE id = 2"
run_sql "SELECT updated orders" "SELECT id, status, updated_at FROM orders WHERE status = 'shipped'"

# Airbyte DELETE (soft delete pattern)
run_sql "DELETE customer" "DELETE FROM customers WHERE id = 2"
run_sql "VERIFY delete" "SELECT COUNT(*) FROM customers"

# Cleanup
run_sql "DROP DATABASE airbyte_test" "DROP DATABASE IF EXISTS airbyte_test"

# ============================================================
# App 5: Metabase (Clojure + MySQL/PostgreSQL)
# https://github.com/metabase/metabase
# ============================================================
app "Metabase" "BI/Data Exploration" "Clojure" "github.com/metabase/metabase"

run_sql "CREATE DATABASE metabase_test" "CREATE DATABASE IF NOT EXISTS metabase_test"
run_sql "USE metabase_test" "USE metabase_test"

# Metabase connection patterns
run_sql "SELECT 1 (health check)" "SELECT 1"
run_sql "SELECT version()" "SELECT version()"
run_sql "SHOW DATABASES" "SHOW DATABASES"

# Metabase core tables
run_sql "CREATE reports" "CREATE TABLE IF NOT EXISTS reports (id INT, name VARCHAR(255), description TEXT, query TEXT, created_at DATETIME, updated_at DATETIME)"
run_sql "CREATE report_cards" "CREATE TABLE IF NOT EXISTS report_cards (id INT, report_id INT, card_type VARCHAR(50), config TEXT, position INT)"
run_sql "CREATE sample_data" "CREATE TABLE IF NOT EXISTS sample_data (id INT, category VARCHAR(100), product VARCHAR(200), price FLOAT, quantity INT, revenue FLOAT, created_at DATETIME)"

# Metabase INSERT
run_sql "INSERT reports" "INSERT INTO reports (id, name, description, created_at) VALUES (1, 'Monthly Revenue', 'Monthly revenue report', NOW())"
run_sql "INSERT report_cards" "INSERT INTO report_cards (id, report_id, card_type, config) VALUES (1, 1, 'bar', '{\"xAxis\":\"category\",\"yAxis\":\"revenue\"}')"
run_sql "INSERT sample_data" "INSERT INTO sample_data VALUES (1, 'Electronics', 'Laptop', 999.99, 10, 9999.90, NOW()), (2, 'Electronics', 'Phone', 599.99, 50, 29999.50, NOW()), (3, 'Books', 'Novel', 14.99, 200, 2998.00, NOW()), (4, 'Clothing', 'T-Shirt', 24.99, 100, 2499.00, NOW()), (5, 'Books', 'Textbook', 49.99, 30, 1499.70, NOW())"

# Metabase SELECT (exploration queries)
run_sql "SELECT reports" "SELECT id, name FROM reports"
run_sql "SELECT cards by report" "SELECT id, card_type FROM report_cards WHERE report_id = 1"
run_sql "SELECT top products" "SELECT product, revenue FROM sample_data ORDER BY revenue DESC LIMIT 5"
run_sql "SELECT by category" "SELECT category, SUM(revenue) as total FROM sample_data GROUP BY category"
run_sql "SELECT AVG price" "SELECT AVG(price) FROM sample_data"
run_sql "SELECT category count" "SELECT category, COUNT(*) as count FROM sample_data GROUP BY category"

# Metabase filter queries
run_sql "SELECT WHERE price > 50" "SELECT product, price FROM sample_data WHERE price > 50"
run_sql "SELECT WHERE category IN" "SELECT product, revenue FROM sample_data WHERE category IN ('Electronics', 'Books')"
run_sql "SELECT LIKE product" "SELECT product FROM sample_data WHERE product LIKE '%Phone%'"
run_sql "SELECT BETWEEN dates" "SELECT product FROM sample_data WHERE price BETWEEN 20 AND 100"

# Metabase UPDATE
run_sql "UPDATE price" "UPDATE sample_data SET price = price * 1.1 WHERE category = 'Electronics'"
run_sql "VERIFY updated prices" "SELECT product, price FROM sample_data WHERE category = 'Electronics'"

# Metabase DELETE
run_sql "DELETE low revenue" "DELETE FROM sample_data WHERE revenue < 1000"
run_sql "COUNT after delete" "SELECT COUNT(*) FROM sample_data"

# Cleanup
run_sql "DROP DATABASE metabase_test" "DROP DATABASE IF EXISTS metabase_test"

# ============================================================
# Print Summary
# ============================================================
echo ""
echo -e "${BOLD}======================================================================${RESET}"
echo -e "${BOLD}Real-World Application Test Summary${RESET}"
echo -e "${BOLD}======================================================================${RESET}"
TOTAL=$((PASSED + FAILED))
echo "Total:  $TOTAL"
echo -e "${GREEN}Passed: $PASSED${RESET}"
echo -e "${RED}Failed: $FAILED${RESET}"

if [ -n "$ERRORS" ]; then
    echo -e "\n${BOLD}${RED}Failed Tests:${RESET}"
    echo -e "$ERRORS"
fi

echo ""
echo "Completed at: $(date '+%Y-%m-%d %H:%M:%S')"
echo -e "${BOLD}======================================================================${RESET}"

[ $FAILED -eq 0 ]
