#!/bin/bash
# Mattermost - Team collaboration (github.com/mattermost/mattermost-server)
MYSQL="mysql -h 127.0.0.1 -P 9030 -uroot --skip-column-names"
PASS=0; FAIL=0

test_sql() {
    if output=$(echo "$2" | $MYSQL 2>&1); then
        PASS=$((PASS + 1)); echo "  ✓ $1"
    else
        FAIL=$((FAIL + 1)); echo "  ✗ $1: $output"
    fi
}

echo "=== Mattermost Team Chat Scenario ==="
test_sql "Create DB" "CREATE DATABASE IF NOT EXISTS mattermost"
test_sql "Use DB" "USE mattermost"
test_sql "Create users" "CREATE TABLE IF NOT EXISTS users (id VARCHAR(26), username VARCHAR(64), email VARCHAR(128), create_at BIGINT)"
test_sql "Create teams" "CREATE TABLE IF NOT EXISTS teams (id VARCHAR(26), name VARCHAR(64), display_name VARCHAR(64), type VARCHAR(255))"
test_sql "Create channels" "CREATE TABLE IF NOT EXISTS channels (id VARCHAR(26), team_id VARCHAR(26), type VARCHAR(1), display_name VARCHAR(64), name VARCHAR(64))"
test_sql "Create posts" "CREATE TABLE IF NOT EXISTS posts (id VARCHAR(26), channel_id VARCHAR(26), user_id VARCHAR(26), message TEXT, create_at BIGINT)"
test_sql "Create reactions" "CREATE TABLE IF NOT EXISTS reactions (user_id VARCHAR(26), post_id VARCHAR(26), emoji_name VARCHAR(64))"
test_sql "Insert user" "INSERT INTO users VALUES ('u1', 'alice', 'alice@example.com', UNIX_TIMESTAMP())"
test_sql "Insert team" "INSERT INTO teams VALUES ('t1', 'engineering', 'Engineering Team', 'O')"
test_sql "Insert channel" "INSERT INTO channels VALUES ('c1', 't1', 'O', 'General', 'general')"
test_sql "Insert post" "INSERT INTO posts VALUES ('p1', 'c1', 'u1', 'Hello team!', UNIX_TIMESTAMP())"
test_sql "Insert reaction" "INSERT INTO reactions VALUES ('u1', 'p1', 'thumbsup')"
test_sql "Channel messages" "SELECT c.display_name, p.message FROM channels c JOIN posts p ON c.id = p.channel_id"
test_sql "User activity" "SELECT u.username, COUNT(p.id) FROM users u LEFT JOIN posts p ON u.id = p.user_id GROUP BY u.id"
test_sql "Popular posts" "SELECT p.message, COUNT(r.emoji_name) FROM posts p LEFT JOIN reactions r ON p.id = r.post_id GROUP BY p.id"

echo "Passed: $PASS / $((PASS + FAIL))"
