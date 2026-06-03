-- ============================================================================
-- HarnessDB Transaction and INSERT ON DUPLICATE KEY Test Suite
-- ============================================================================
-- Test Coverage:
-- 1. Transaction basics (BEGIN/COMMIT/ROLLBACK)
-- 2. SAVEPOINT functionality
-- 3. Multiple DML operations in transaction
-- 4. INSERT ON DUPLICATE KEY UPDATE (various scenarios)
-- 5. INSERT SET syntax
-- ============================================================================

DROP DATABASE IF EXISTS transaction_test;
CREATE DATABASE transaction_test;
USE transaction_test;

-- ============================================================================
-- Section 1: Transaction Basic Operations (BEGIN/COMMIT/ROLLBACK)
-- ============================================================================

-- Test 1.1: Basic BEGIN/COMMIT
-- Purpose: Verify that data is persisted after COMMIT
CREATE TABLE t_basic (
    id INT PRIMARY KEY,
    name VARCHAR(100),
    value INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;

BEGIN;
INSERT INTO t_basic VALUES (1, 'Alice', 100);
INSERT INTO t_basic VALUES (2, 'Bob', 200);
COMMIT;

SELECT * FROM t_basic ORDER BY id;
-- Expected: 2 rows (id=1, id=2)

-- Test 1.2: Basic BEGIN/ROLLBACK
-- Purpose: Verify that data is discarded after ROLLBACK
BEGIN;
INSERT INTO t_basic VALUES (3, 'Charlie', 300);
INSERT INTO t_basic VALUES (4, 'David', 400);
ROLLBACK;

SELECT * FROM t_basic ORDER BY id;
-- Expected: still 2 rows (id=3, id=4 were rolled back)

-- Test 1.3: UPDATE in transaction with COMMIT
-- Purpose: Verify UPDATE is persisted after COMMIT
BEGIN;
UPDATE t_basic SET value = 150 WHERE id = 1;
UPDATE t_basic SET name = 'Robert' WHERE id = 2;
COMMIT;

SELECT * FROM t_basic ORDER BY id;
-- Expected: id=1, name=Alice, value=150; id=2, name=Robert, value=200

-- Test 1.4: UPDATE in transaction with ROLLBACK
-- Purpose: Verify UPDATE is discarded after ROLLBACK
BEGIN;
UPDATE t_basic SET value = 999 WHERE id = 1;
UPDATE t_basic SET name = 'Charlie' WHERE id = 2;
ROLLBACK;

SELECT * FROM t_basic ORDER BY id;
-- Expected: id=1, name=Alice, value=150; id=2, name=Robert, value=200 (changes rolled back)

-- Test 1.5: DELETE in transaction with COMMIT
-- Purpose: Verify DELETE is persisted after COMMIT
BEGIN;
DELETE FROM t_basic WHERE id = 1;
COMMIT;

SELECT * FROM t_basic ORDER BY id;
-- Expected: only id=2 remains

-- Test 1.6: DELETE in transaction with ROLLBACK
-- Purpose: Verify DELETE is discarded after ROLLBACK
BEGIN;
DELETE FROM t_basic WHERE id = 2;
ROLLBACK;

SELECT * FROM t_basic ORDER BY id;
-- Expected: id=2 still exists (DELETE was rolled back)

DROP TABLE t_basic;

-- ============================================================================
-- Section 2: SAVEPOINT Functionality
-- ============================================================================

-- Test 2.1: Basic SAVEPOINT and ROLLBACK TO
-- Purpose: Verify partial rollback using SAVEPOINT
CREATE TABLE t_savepoint (
    id INT PRIMARY KEY,
    value INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;

BEGIN;
INSERT INTO t_savepoint VALUES (1, 100);
SAVEPOINT sp1;

INSERT INTO t_savepoint VALUES (2, 200);
SAVEPOINT sp2;

INSERT INTO t_savepoint VALUES (3, 300);

ROLLBACK TO sp2;
SELECT * FROM t_savepoint ORDER BY id;
-- Expected: 2 rows (id=1, id=2)

ROLLBACK TO sp1;
SELECT * FROM t_savepoint ORDER BY id;
-- Expected: 1 row (id=1)

COMMIT;

SELECT * FROM t_savepoint ORDER BY id;
-- Expected: 1 row (id=1)

DROP TABLE t_savepoint;

-- Test 2.2: Multiple SAVEPOINTs with updates
-- Purpose: Verify SAVEPOINT works with UPDATE operations
CREATE TABLE t_sp_update (
    id INT PRIMARY KEY,
    name VARCHAR(100),
    value INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_sp_update VALUES (1, 'Alice', 100);
INSERT INTO t_sp_update VALUES (2, 'Bob', 200);

BEGIN;
UPDATE t_sp_update SET value = 150 WHERE id = 1;
SAVEPOINT sp_before_delete;

DELETE FROM t_sp_update WHERE id = 2;
SAVEPOINT sp_after_delete;

INSERT INTO t_sp_update VALUES (3, 'Charlie', 300);

ROLLBACK TO sp_after_delete;
SELECT * FROM t_sp_update ORDER BY id;
-- Expected: 1 row (id=1 with value=150)

ROLLBACK TO sp_before_delete;
SELECT * FROM t_sp_update ORDER BY id;
-- Expected: 2 rows (id=1 with value=150, id=2)

COMMIT;

SELECT * FROM t_sp_update ORDER BY id;
-- Expected: 2 rows (id=1 with value=150, id=2 with value=200)

DROP TABLE t_sp_update;

-- Test 2.3: RELEASE SAVEPOINT
-- Purpose: Verify RELEASE SAVEPOINT removes savepoint
CREATE TABLE t_release_sp (
    id INT PRIMARY KEY,
    value INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;

BEGIN;
INSERT INTO t_release_sp VALUES (1, 100);
SAVEPOINT sp1;

INSERT INTO t_release_sp VALUES (2, 200);
RELEASE SAVEPOINT sp1;

-- Attempt to rollback to released savepoint should fail
-- ROLLBACK TO sp1; -- This would fail as sp1 is released
COMMIT;

SELECT * FROM t_release_sp ORDER BY id;
-- Expected: 2 rows

DROP TABLE t_release_sp;

-- ============================================================================
-- Section 3: Multiple DML Operations in Transaction
-- ============================================================================

-- Test 3.1: Mixed INSERT/UPDATE/DELETE in one transaction with COMMIT
-- Purpose: Verify complex transaction with multiple DML types
CREATE TABLE t_mixed_dml (
    id INT PRIMARY KEY,
    name VARCHAR(100),
    value INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_mixed_dml VALUES (1, 'Alice', 100);
INSERT INTO t_mixed_dml VALUES (2, 'Bob', 200);
INSERT INTO t_mixed_dml VALUES (3, 'Charlie', 300);

BEGIN;
INSERT INTO t_mixed_dml VALUES (4, 'David', 400);
UPDATE t_mixed_dml SET value = 150 WHERE id = 1;
DELETE FROM t_mixed_dml WHERE id = 2;
UPDATE t_mixed_dml SET name = 'Charles' WHERE id = 3;
COMMIT;

SELECT * FROM t_mixed_dml ORDER BY id;
-- Expected: 3 rows (id=1, id=3, id=4)

DROP TABLE t_mixed_dml;

-- Test 3.2: Mixed DML with ROLLBACK
-- Purpose: Verify all DML operations are rolled back
CREATE TABLE t_mixed_rollback (
    id INT PRIMARY KEY,
    value INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_mixed_rollback VALUES (1, 100);
INSERT INTO t_mixed_rollback VALUES (2, 200);

BEGIN;
INSERT INTO t_mixed_rollback VALUES (3, 300);
UPDATE t_mixed_rollback SET value = 999 WHERE id = 1;
DELETE FROM t_mixed_rollback WHERE id = 2;
ROLLBACK;

SELECT * FROM t_mixed_rollback ORDER BY id;
-- Expected: 2 rows (id=1 with value=100, id=2 with value=200)

DROP TABLE t_mixed_rollback;

-- Test 3.3: Nested transactions simulation with SAVEPOINT
-- Purpose: Simulate nested transactions using SAVEPOINT
CREATE TABLE t_nested (
    id INT PRIMARY KEY,
    operation VARCHAR(50),
    value INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;

BEGIN;
INSERT INTO t_nested VALUES (1, 'outer_begin', 100);
SAVEPOINT inner_tx;

INSERT INTO t_nested VALUES (2, 'inner_insert', 200);
SAVEPOINT inner_tx_2;

INSERT INTO t_nested VALUES (3, 'inner_2_insert', 300);

-- Rollback inner transaction 2
ROLLBACK TO inner_tx_2;

-- Continue with outer transaction
INSERT INTO t_nested VALUES (4, 'outer_continue', 400);

-- Rollback inner transaction 1
ROLLBACK TO inner_tx;

COMMIT;

SELECT * FROM t_nested ORDER BY id;
-- Expected: 2 rows (id=1 and id=4)

DROP TABLE t_nested;

-- ============================================================================
-- Section 4: INSERT ON DUPLICATE KEY UPDATE
-- ============================================================================

-- Test 4.1: Basic INSERT ON DUPLICATE KEY UPDATE - Insert new row
-- Purpose: Verify normal insert when key doesn't exist
CREATE TABLE t_duplicate_insert (
    id INT PRIMARY KEY,
    name VARCHAR(100),
    value INT,
    counter INT DEFAULT 0
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_duplicate_insert (id, name, value, counter)
VALUES (1, 'Alice', 100, 1)
ON DUPLICATE KEY UPDATE counter = counter + 1;

SELECT * FROM t_duplicate_insert WHERE id = 1;
-- Expected: id=1, name=Alice, value=100, counter=1

-- Test 4.2: INSERT ON DUPLICATE KEY UPDATE - Update existing row
-- Purpose: Verify update when key exists
INSERT INTO t_duplicate_insert (id, name, value, counter)
VALUES (1, 'Bob', 200, 1)
ON DUPLICATE KEY UPDATE 
    name = VALUES(name),
    value = VALUES(value),
    counter = counter + 1;

SELECT * FROM t_duplicate_insert WHERE id = 1;
-- Expected: id=1, name=Bob, value=200, counter=2

-- Test 4.3: INSERT ON DUPLICATE KEY UPDATE - Partial columns update
-- Purpose: Verify update only specified columns
INSERT INTO t_duplicate_insert (id, name, value, counter)
VALUES (1, 'Charlie', 300, 1)
ON DUPLICATE KEY UPDATE counter = counter + 1;

SELECT * FROM t_duplicate_insert WHERE id = 1;
-- Expected: id=1, name=Bob (unchanged), value=200 (unchanged), counter=3

-- Test 4.4: INSERT ON DUPLICATE KEY UPDATE with expressions
-- Purpose: Verify update with arithmetic expressions
INSERT INTO t_duplicate_insert (id, name, value, counter)
VALUES (1, 'David', 400, 1)
ON DUPLICATE KEY UPDATE 
    value = value * 2,
    counter = counter + 10;

SELECT * FROM t_duplicate_insert WHERE id = 1;
-- Expected: id=1, name=Bob (unchanged), value=400, counter=13

-- Test 4.5: INSERT ON DUPLICATE KEY UPDATE - Multiple rows batch
-- Purpose: Verify batch insert with duplicate handling
INSERT INTO t_duplicate_insert (id, name, value, counter) VALUES
    (2, 'Eve', 500, 1),
    (3, 'Frank', 600, 1),
    (1, 'Grace', 700, 1)
ON DUPLICATE KEY UPDATE 
    value = VALUES(value),
    counter = counter + 1;

SELECT * FROM t_duplicate_insert ORDER BY id;
-- Expected: 3 rows
-- id=1: name=Bob, value=700, counter=14
-- id=2: name=Eve, value=500, counter=1
-- id=3: name=Frank, value=600, counter=1

-- Test 4.6: INSERT ON DUPLICATE KEY UPDATE in transaction
-- Purpose: Verify atomicity with transaction
BEGIN;
INSERT INTO t_duplicate_insert (id, name, value, counter)
VALUES (4, 'Henry', 800, 1)
ON DUPLICATE KEY UPDATE counter = counter + 1;

INSERT INTO t_duplicate_insert (id, name, value, counter)
VALUES (2, 'Ivy', 900, 1)
ON DUPLICATE KEY UPDATE 
    name = VALUES(name),
    value = VALUES(value);

COMMIT;

SELECT * FROM t_duplicate_insert ORDER BY id;
-- Expected: 4 rows
-- id=2: name=Ivy, value=900, counter=1

DROP TABLE t_duplicate_insert;

-- Test 4.7: INSERT ON DUPLICATE KEY with NULL values
-- Purpose: Verify NULL handling in duplicate key update
CREATE TABLE t_duplicate_null (
    id INT PRIMARY KEY,
    name VARCHAR(100),
    value INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_duplicate_null VALUES (1, 'Alice', 100);

INSERT INTO t_duplicate_null (id, name, value)
VALUES (1, NULL, NULL)
ON DUPLICATE KEY UPDATE 
    name = VALUES(name),
    value = VALUES(value);

SELECT * FROM t_duplicate_null WHERE id = 1;
-- Expected: id=1, name=NULL, value=NULL

DROP TABLE t_duplicate_null;

-- ============================================================================
-- Section 5: INSERT SET Syntax
-- ============================================================================

-- Test 5.1: Basic INSERT SET syntax
-- Purpose: Verify INSERT SET syntax works
CREATE TABLE t_insert_set (
    id INT PRIMARY KEY,
    name VARCHAR(100),
    value INT,
    status VARCHAR(20) DEFAULT 'active'
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_insert_set SET id = 1, name = 'Alice', value = 100;
INSERT INTO t_insert_set SET id = 2, name = 'Bob', value = 200, status = 'inactive';

SELECT * FROM t_insert_set ORDER BY id;
-- Expected: 2 rows
-- id=1: name=Alice, value=100, status=active (default)
-- id=2: name=Bob, value=200, status=inactive

-- Test 5.2: INSERT SET with expressions
-- Purpose: Verify INSERT SET with expressions
INSERT INTO t_insert_set SET id = 3, name = 'Charlie', value = 100 + 50;

SELECT * FROM t_insert_set WHERE id = 3;
-- Expected: id=3, name=Charlie, value=150, status=active

-- Test 5.3: INSERT SET with DEFAULT values
-- Purpose: Verify INSERT SET with DEFAULT keyword
INSERT INTO t_insert_set SET id = 4, name = 'David', value = DEFAULT, status = DEFAULT;

SELECT * FROM t_insert_set WHERE id = 4;
-- Expected: id=4, name=David, value=NULL, status=active

-- Test 5.4: INSERT SET in transaction
-- Purpose: Verify INSERT SET in transaction context
BEGIN;
INSERT INTO t_insert_set SET id = 5, name = 'Eve', value = 500;
INSERT INTO t_insert_set SET id = 6, name = 'Frank', value = 600;
ROLLBACK;

SELECT COUNT(*) FROM t_insert_set;
-- Expected: 4 rows (inserts were rolled back)

BEGIN;
INSERT INTO t_insert_set SET id = 5, name = 'Eve', value = 500;
COMMIT;

SELECT COUNT(*) FROM t_insert_set;
-- Expected: 5 rows

DROP TABLE t_insert_set;

-- ============================================================================
-- Section 6: Advanced Transaction Scenarios
-- ============================================================================

-- Test 6.1: Transaction isolation test
-- Purpose: Verify data visibility within transaction
CREATE TABLE t_isolation (
    id INT PRIMARY KEY,
    value INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_isolation VALUES (1, 100);

BEGIN;
UPDATE t_isolation SET value = 200 WHERE id = 1;
SELECT * FROM t_isolation WHERE id = 1;
-- Expected: value=200 (visible within same transaction)

ROLLBACK;

SELECT * FROM t_isolation WHERE id = 1;
-- Expected: value=100 (original value)

DROP TABLE t_isolation;

-- Test 6.2: Empty transaction
-- Purpose: Verify empty transaction handling
CREATE TABLE t_empty_tx (
    id INT PRIMARY KEY,
    value INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_empty_tx VALUES (1, 100);

BEGIN;
-- No operations
COMMIT;

SELECT * FROM t_empty_tx;
-- Expected: 1 row (unchanged)

BEGIN;
-- No operations
ROLLBACK;

SELECT * FROM t_empty_tx;
-- Expected: 1 row (unchanged)

DROP TABLE t_empty_tx;

-- Test 6.3: Multiple consecutive transactions
-- Purpose: Verify multiple transactions in sequence
CREATE TABLE t_multi_tx (
    id INT PRIMARY KEY,
    value INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;

BEGIN;
INSERT INTO t_multi_tx VALUES (1, 100);
COMMIT;

BEGIN;
INSERT INTO t_multi_tx VALUES (2, 200);
ROLLBACK;

BEGIN;
INSERT INTO t_multi_tx VALUES (3, 300);
COMMIT;

BEGIN;
UPDATE t_multi_tx SET value = 150 WHERE id = 1;
COMMIT;

SELECT * FROM t_multi_tx ORDER BY id;
-- Expected: 2 rows
-- id=1: value=150 (updated)
-- id=3: value=300 (id=2 was rolled back)

DROP TABLE t_multi_tx;

-- Test 6.4: INSERT ON DUPLICATE KEY with complex expressions
-- Purpose: Verify complex expressions in ON DUPLICATE KEY UPDATE
CREATE TABLE t_complex_dup (
    id INT PRIMARY KEY,
    name VARCHAR(100),
    value1 INT,
    value2 INT,
    total INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_complex_dup VALUES (1, 'Alice', 10, 20, 30);

INSERT INTO t_complex_dup (id, name, value1, value2, total)
VALUES (1, 'Bob', 30, 40, 70)
ON DUPLICATE KEY UPDATE 
    name = CONCAT(name, '_updated'),
    value1 = value1 + VALUES(value1),
    value2 = value2 + VALUES(value2),
    total = value1 + value2;

SELECT * FROM t_complex_dup WHERE id = 1;
-- Expected: id=1, name=Alice_updated, value1=40, value2=60, total=100

DROP TABLE t_complex_dup;

-- Test 6.5: INSERT SET with ON DUPLICATE KEY UPDATE
-- Purpose: Verify INSERT SET syntax with ON DUPLICATE KEY
CREATE TABLE t_set_dup (
    id INT PRIMARY KEY,
    name VARCHAR(100),
    counter INT DEFAULT 0
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_set_dup SET id = 1, name = 'Alice', counter = 1;

INSERT INTO t_set_dup SET id = 1, name = 'Bob', counter = 1
ON DUPLICATE KEY UPDATE 
    name = VALUES(name),
    counter = counter + 1;

SELECT * FROM t_set_dup WHERE id = 1;
-- Expected: id=1, name=Bob, counter=2

DROP TABLE t_set_dup;

-- ============================================================================
-- Cleanup
-- ============================================================================

DROP DATABASE transaction_test;

SELECT 'All transaction and duplicate key tests completed successfully!' AS status;