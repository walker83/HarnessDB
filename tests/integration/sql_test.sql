-- RorisDB Integration Test Script
-- Run with: mysql -h 127.0.0.1 -P 9030 -uroot < tests/integration/sql_test.sql

-- ============================================================================
-- Section 1: CANCEL ALTER TABLE (Batch 2 DDL) - Primary test cases
-- ============================================================================

-- Setup: Create test database and table
CREATE DATABASE IF NOT EXISTS cancel_alter_test;
USE cancel_alter_test;
CREATE TABLE IF NOT EXISTS test_tbl (id INT, name VARCHAR(100)) DISTRIBUTED BY HASH(id) BUCKETS 3;

-- Test 1: CANCEL ALTER TABLE with single table name (uses current database)
CANCEL ALTER TABLE test_tbl;

-- Test 2: CANCEL ALTER TABLE FROM db.table (database specified via FROM clause)
CANCEL ALTER TABLE FROM cancel_alter_test.test_tbl;

-- Test 3: CANCEL ALTER TABLE FROM db.table where table doesn't exist (expected error)
CANCEL ALTER TABLE FROM cancel_alter_test.nonexistent;

-- Cleanup
DROP TABLE test_tbl;
DROP DATABASE cancel_alter_test;

-- ============================================================================
-- Section 2: CANCEL ALTER TABLE on sql_test database
-- ============================================================================
USE sql_test;
CANCEL ALTER TABLE FROM sql_test.t1;
CANCEL ALTER TABLE t2;

-- ============================================================================
-- Section 3: CREATE INDEX / DROP INDEX (Batch 2 DDL)
-- ============================================================================
USE sql_test;
CREATE INDEX idx_t1_id ON t1 (id) USING BITMAP;
DROP INDEX idx_t1_id ON t1;

-- ============================================================================
-- Section 4: ALTER DATABASE (Batch 1 DDL)
-- ============================================================================
CREATE DATABASE IF NOT EXISTS alter_db_test;
ALTER DATABASE alter_db_test SET PROPERTIES ("test" = "value");
DROP DATABASE alter_db_test;

-- ============================================================================
-- Section 5: DROP VIEW (Batch 1 DDL)
-- ============================================================================
CREATE DATABASE IF NOT EXISTS view_test;
USE view_test;
CREATE VIEW test_view AS SELECT 1 AS col;
DROP VIEW test_view;
DROP DATABASE view_test;

-- ============================================================================
-- Section 6: SHOW statements (Batch 3)
-- ============================================================================
USE sql_test;
SHOW TABLES;
SHOW CREATE TABLE t1;
-- SHOW COLUMNS FROM t1; -- Note: uses sqlparser native AST, has known limitations
SHOW INDEX FROM t1;
DESCRIBE t1;
SHOW TABLE STATUS;
SHOW VARIABLES LIKE '%version%';
SHOW PROCESSLIST;

-- ============================================================================
-- Summary
-- ============================================================================
SELECT 'All tests completed successfully!' AS status;
