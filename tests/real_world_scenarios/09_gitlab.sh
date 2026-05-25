#!/bin/bash
# GitLab - DevOps Platform (github.com/gitlabhq/gitlabhq)
MYSQL="mysql -h 127.0.0.1 -P 9030 -uroot --skip-column-names"
PASS=0; FAIL=0

test_sql() {
    if output=$(echo "$2" | $MYSQL 2>&1); then
        PASS=$((PASS + 1)); echo "  ✓ $1"
    else
        FAIL=$((FAIL + 1)); echo "  ✗ $1: $output"
    fi
}

echo "=== GitLab DevOps Scenario ==="
test_sql "Create DB" "CREATE DATABASE IF NOT EXISTS gitlab"
test_sql "Use DB" "USE gitlab"
test_sql "Create namespaces" "CREATE TABLE IF NOT EXISTS namespaces (id INT, name VARCHAR(255), path VARCHAR(255), type VARCHAR(255))"
test_sql "Create projects" "CREATE TABLE IF NOT EXISTS projects (id INT, name VARCHAR(255), path VARCHAR(2048), namespace_id INT, created_at DATETIME)"
test_sql "Create users" "CREATE TABLE IF NOT EXISTS users (id INT, username VARCHAR(255), email VARCHAR(255), created_at DATETIME)"
test_sql "Create merge_requests" "CREATE TABLE IF NOT EXISTS merge_requests (id INT, iid INT, target_branch VARCHAR(255), source_branch VARCHAR(255), author_id INT, project_id INT, state VARCHAR(255))"
test_sql "Create issues" "CREATE TABLE IF NOT EXISTS issues (id INT, iid INT, title VARCHAR(255), description TEXT, author_id INT, project_id INT, state VARCHAR(255))"
test_sql "Create pipelines" "CREATE TABLE IF NOT EXISTS ci_pipelines (id INT, project_id INT, ref VARCHAR(255), status VARCHAR(255), created_at DATETIME)"
test_sql "Insert namespace" "INSERT INTO namespaces VALUES (1, 'Engineering', 'engineering', 'Group')"
test_sql "Insert user" "INSERT INTO users VALUES (1, 'alice', 'alice@gitlab.com', NOW())"
test_sql "Insert project" "INSERT INTO projects VALUES (1, 'myapp', 'engineering/myapp', 1, NOW())"
test_sql "Insert issue" "INSERT INTO issues VALUES (1, 1, 'Bug fix', 'Fix login issue', 1, 1, 'opened')"
test_sql "Insert MR" "INSERT INTO merge_requests VALUES (1, 1, 'main', 'feature-branch', 1, 1, 'merged')"
test_sql "Insert pipeline" "INSERT INTO ci_pipelines VALUES (1, 1, 'main', 'success', NOW())"
test_sql "Project issues" "SELECT p.name, i.title, i.state FROM projects p JOIN issues i ON p.id = i.project_id"
test_sql "Pipeline stats" "SELECT status, COUNT(*) FROM ci_pipelines GROUP BY status"
test_sql "User contributions" "SELECT u.username, COUNT(i.id) FROM users u JOIN issues i ON u.id = i.author_id GROUP BY u.id"

echo "Passed: $PASS / $((PASS + FAIL))"
