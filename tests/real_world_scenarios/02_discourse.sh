#!/bin/bash
# Discourse - Popular forum software (github.com/discourse/discourse)
MYSQL="mysql -h 127.0.0.1 -P 9030 -uroot --skip-column-names"
PASS=0; FAIL=0

test_sql() {
    if output=$(echo "$2" | $MYSQL 2>&1); then
        PASS=$((PASS + 1)); echo "  ✓ $1"
    else
        FAIL=$((FAIL + 1)); echo "  ✗ $1: $output"
    fi
}

echo "=== Discourse Forum Scenario ==="
test_sql "Create DB" "CREATE DATABASE IF NOT EXISTS discourse"
test_sql "Use DB" "USE discourse"
test_sql "Create users" "CREATE TABLE IF NOT EXISTS users (id INT, username VARCHAR(60), email VARCHAR(255), created_at DATETIME)"
test_sql "Create topics" "CREATE TABLE IF NOT EXISTS topics (id INT, title VARCHAR(255), user_id INT, created_at DATETIME, views INT, posts_count INT)"
test_sql "Create posts" "CREATE TABLE IF NOT EXISTS posts (id INT, topic_id INT, user_id INT, raw TEXT, cooked TEXT, created_at DATETIME)"
test_sql "Create categories" "CREATE TABLE IF NOT EXISTS categories (id INT, name VARCHAR(50), description TEXT, parent_category_id INT)"
test_sql "Insert users" "INSERT INTO users VALUES (1, 'alice', 'alice@example.com', NOW()), (2, 'bob', 'bob@example.com', NOW())"
test_sql "Insert categories" "INSERT INTO categories VALUES (1, 'General', 'General discussion', NULL)"
test_sql "Insert topics" "INSERT INTO topics VALUES (1, 'Welcome to Discourse', 1, NOW(), 100, 5)"
test_sql "Insert posts" "INSERT INTO posts VALUES (1, 1, 1, 'Hello world', '<p>Hello world</p>', NOW())"
test_sql "Query topics with user" "SELECT t.title, u.username FROM topics t JOIN users u ON t.user_id = u.id"
test_sql "Count posts per topic" "SELECT topic_id, COUNT(*) FROM posts GROUP BY topic_id"
test_sql "Recent posts" "SELECT * FROM posts ORDER BY created_at DESC LIMIT 10"
test_sql "User activity" "SELECT u.username, COUNT(p.id) FROM users u LEFT JOIN posts p ON u.id = p.user_id GROUP BY u.id"

echo "Passed: $PASS / $((PASS + FAIL))"
