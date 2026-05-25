#!/bin/bash
# WordPress 6.x - Most popular CMS (github.com/WordPress/WordPress)
# Tests typical WordPress database operations

MYSQL="mysql -h 127.0.0.1 -P 9030 -uroot --skip-column-names"
PASS=0; FAIL=0

test_sql() {
    if output=$(echo "$2" | $MYSQL 2>&1); then
        PASS=$((PASS + 1)); echo "  ✓ $1"
    else
        FAIL=$((FAIL + 1)); echo "  ✗ $1: $output"
    fi
}

echo "=== WordPress 6.x Scenario ==="
test_sql "Create wordpress DB" "CREATE DATABASE IF NOT EXISTS wordpress"
test_sql "Use DB" "USE wordpress"
test_sql "Create wp_options" "CREATE TABLE IF NOT EXISTS wp_options (option_id INT AUTO_INCREMENT, option_name VARCHAR(191), option_value TEXT, autoload VARCHAR(20), PRIMARY KEY (option_id))"
test_sql "Create wp_posts" "CREATE TABLE IF NOT EXISTS wp_posts (ID INT AUTO_INCREMENT, post_author INT, post_title TEXT, post_content LONGTEXT, post_status VARCHAR(20), PRIMARY KEY (ID))"
test_sql "Create wp_users" "CREATE TABLE IF NOT EXISTS wp_users (ID INT AUTO_INCREMENT, user_login VARCHAR(60), user_pass VARCHAR(255), user_email VARCHAR(100), PRIMARY KEY (ID))"
test_sql "Insert option" "INSERT INTO wp_options (option_name, option_value, autoload) VALUES ('siteurl', 'http://localhost', 'yes')"
test_sql "Insert post" "INSERT INTO wp_posts (post_author, post_title, post_content, post_status) VALUES (1, 'Hello World', 'Welcome to WordPress', 'publish')"
test_sql "Insert user" "INSERT INTO wp_users (user_login, user_pass, user_email) VALUES ('admin', 'hashed_password', 'admin@example.com')"
test_sql "Query options" "SELECT option_value FROM wp_options WHERE option_name = 'siteurl'"
test_sql "Query posts" "SELECT post_title FROM wp_posts WHERE post_status = 'publish'"
test_sql "Update option" "UPDATE wp_options SET option_value = 'http://example.com' WHERE option_name = 'siteurl'"
test_sql "Delete post" "DELETE FROM wp_posts WHERE ID = 1"
test_sql "Count posts" "SELECT COUNT(*) FROM wp_posts"
test_sql "JOIN posts and users" "SELECT u.user_login, p.post_title FROM wp_users u JOIN wp_posts p ON u.ID = p.post_author"

echo "Passed: $PASS / $((PASS + FAIL))"
