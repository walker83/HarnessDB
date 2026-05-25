#!/bin/bash
# Nextcloud - File sync & share (github.com/nextcloud/server)
MYSQL="mysql -h 127.0.0.1 -P 9030 -uroot --skip-column-names"
PASS=0; FAIL=0

test_sql() {
    if output=$(echo "$2" | $MYSQL 2>&1); then
        PASS=$((PASS + 1)); echo "  ✓ $1"
    else
        FAIL=$((FAIL + 1)); echo "  ✗ $1: $output"
    fi
}

echo "=== Nextcloud File Sync Scenario ==="
test_sql "Create DB" "CREATE DATABASE IF NOT EXISTS nextcloud"
test_sql "Use DB" "USE nextcloud"
test_sql "Create users" "CREATE TABLE IF NOT EXISTS oc_users (uid VARCHAR(64), displayname VARCHAR(255), email VARCHAR(255))"
test_sql "Create filecache" "CREATE TABLE IF NOT EXISTS oc_filecache (fileid INT, storage INT, path VARCHAR(4000), size BIGINT, mtime INT)"
test_sql "Create shares" "CREATE TABLE IF NOT EXISTS oc_share (id INT, share_type INT, uid_owner VARCHAR(64), file_source INT, share_with VARCHAR(255))"
test_sql "Create activity" "CREATE TABLE IF NOT EXISTS oc_activity (activity_id INT, timestamp INT, user VARCHAR(64), app VARCHAR(255), subject VARCHAR(255))"
test_sql "Insert user" "INSERT INTO oc_users VALUES ('alice', 'Alice Smith', 'alice@example.com')"
test_sql "Insert files" "INSERT INTO oc_filecache VALUES (1, 1, '/Documents/report.pdf', 1024000, UNIX_TIMESTAMP()), (2, 1, '/Photos/vacation.jpg', 2048000, UNIX_TIMESTAMP())"
test_sql "Insert share" "INSERT INTO oc_share VALUES (1, 0, 'alice', 1, 'bob')"
test_sql "Insert activity" "INSERT INTO oc_activity VALUES (1, UNIX_TIMESTAMP(), 'alice', 'files', 'Created report.pdf')"
test_sql "List user files" "SELECT path, size FROM oc_filecache WHERE path LIKE '/Documents/%'"
test_sql "File statistics" "SELECT COUNT(*), SUM(size) FROM oc_filecache"
test_sql "Shared files" "SELECT u.displayname, s.share_with FROM oc_users u JOIN oc_share s ON u.uid = s.uid_owner"

echo "Passed: $PASS / $((PASS + FAIL))"
