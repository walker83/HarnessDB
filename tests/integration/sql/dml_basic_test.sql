-- ============================================================================
-- DML Basic Functionality Test Script
-- Test Coverage: INSERT, UPDATE, DELETE operations
-- ============================================================================

-- Setup: Create test database
DROP DATABASE IF EXISTS dml_basic_test;
CREATE DATABASE dml_basic_test;
USE dml_basic_test;

-- ============================================================================
-- Part 1: INSERT VALUES Tests
-- ============================================================================

-- Test 1.1: INSERT single row with all columns
CREATE TABLE t_insert_single (
    id INT,
    name VARCHAR(100),
    age INT,
    salary DECIMAL(10, 2)
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_insert_single VALUES (1, 'Alice', 30, 50000.50);
SELECT * FROM t_insert_single;
-- Expected: 1 row: (1, 'Alice', 30, 50000.50)

-- Test 1.2: INSERT multiple rows (batch insert)
INSERT INTO t_insert_single VALUES 
    (2, 'Bob', 25, 45000.00),
    (3, 'Charlie', 35, 60000.75),
    (4, 'Diana', 28, 52000.25);
SELECT * FROM t_insert_single ORDER BY id;
-- Expected: 4 rows with id=1,2,3,4

-- Test 1.3: INSERT with specified columns
CREATE TABLE t_insert_columns (
    id INT,
    name VARCHAR(100),
    age INT DEFAULT 0,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_insert_columns (id, name) VALUES (10, 'Test1');
INSERT INTO t_insert_columns (id, name, age) VALUES (11, 'Test2', 20);
SELECT id, name, age FROM t_insert_columns ORDER BY id;
-- Expected: 2 rows, Test1 age=0 (default), Test2 age=20

-- Test 1.4: INSERT with NULL values
CREATE TABLE t_insert_null (
    id INT,
    name VARCHAR(100),
    email VARCHAR(100),
    age INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_insert_null VALUES (100, 'NullEmail', NULL, 30);
INSERT INTO t_insert_null VALUES (101, NULL, 'test@example.com', NULL);
INSERT INTO t_insert_null VALUES (102, 'AllValues', 'all@example.com', 25);
SELECT * FROM t_insert_null ORDER BY id;
-- Expected: 3 rows with various NULL combinations

-- Test 1.5: INSERT with expression values
CREATE TABLE t_insert_expr (
    id INT,
    computed_value INT,
    concat_str VARCHAR(100)
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_insert_expr VALUES (200, 10 + 20, 'Hello' || ' World');
INSERT INTO t_insert_expr VALUES (201, 5 * 10, 'Test' || ' ' || 'Value');
SELECT * FROM t_insert_expr ORDER BY id;
-- Expected: (200, 30, 'Hello World'), (201, 50, 'Test Value')

-- ============================================================================
-- Part 2: INSERT SELECT Tests
-- ============================================================================

-- Test 2.1: Basic INSERT SELECT
CREATE TABLE t_insert_select_src (
    id INT,
    value INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;

CREATE TABLE t_insert_select_dst (
    id INT,
    value INT,
    double_value INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_insert_select_src VALUES (1, 100), (2, 200), (3, 300);
INSERT INTO t_insert_select_dst (id, value, double_value)
SELECT id, value, value * 2 FROM t_insert_select_src;
SELECT * FROM t_insert_select_dst ORDER BY id;
-- Expected: 3 rows with doubled values

-- Test 2.2: INSERT SELECT with WHERE clause
CREATE TABLE t_insert_select_filtered (
    id INT,
    value INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_insert_select_filtered
SELECT id, value FROM t_insert_select_src WHERE value > 150;
SELECT * FROM t_insert_select_filtered ORDER BY id;
-- Expected: 2 rows (id=2, id=3)

-- Test 2.3: INSERT SELECT with ORDER BY and LIMIT
CREATE TABLE t_insert_select_top (
    id INT,
    value INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_insert_select_top
SELECT id, value FROM t_insert_select_src ORDER BY value DESC LIMIT 2;
SELECT * FROM t_insert_select_top ORDER BY id;
-- Expected: 2 rows with highest values (id=3, id=2)

-- Test 2.4: INSERT SELECT with aggregation
CREATE TABLE t_insert_select_agg (
    category VARCHAR(50),
    total_value INT,
    count_rows INT
) DISTRIBUTED BY HASH(category) BUCKETS 3;

CREATE TABLE t_sales (
    id INT,
    category VARCHAR(50),
    amount INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_sales VALUES 
    (1, 'A', 100), (2, 'A', 200), (3, 'B', 150),
    (4, 'B', 250), (5, 'B', 350);

INSERT INTO t_insert_select_agg
SELECT category, SUM(amount), COUNT(*) 
FROM t_sales 
GROUP BY category;
SELECT * FROM t_insert_select_agg ORDER BY category;
-- Expected: category A: (300, 2), category B: (750, 3)

-- ============================================================================
-- Part 3: UPDATE Tests
-- ============================================================================

-- Test 3.1: Basic UPDATE with single column
CREATE TABLE t_update_basic (
    id INT,
    status VARCHAR(20),
    value INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_update_basic VALUES 
    (1, 'pending', 100),
    (2, 'pending', 200),
    (3, 'completed', 300);

UPDATE t_update_basic SET status = 'processed' WHERE id = 1;
SELECT * FROM t_update_basic WHERE id = 1;
-- Expected: status='processed'

-- Test 3.2: UPDATE with expression
UPDATE t_update_basic SET value = value * 2 WHERE status = 'pending';
SELECT * FROM t_update_basic WHERE status = 'pending' ORDER BY id;
-- Expected: id=1 value=200, id=2 value=400

-- Test 3.3: UPDATE multiple columns
CREATE TABLE t_update_multi (
    id INT,
    col1 INT,
    col2 VARCHAR(50),
    col3 DECIMAL(10, 2)
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_update_multi VALUES (1, 10, 'old1', 100.50);
INSERT INTO t_update_multi VALUES (2, 20, 'old2', 200.75);

UPDATE t_update_multi 
SET col1 = 99, col2 = 'updated', col3 = 999.99 
WHERE id = 1;
SELECT * FROM t_update_multi WHERE id = 1;
-- Expected: (1, 99, 'updated', 999.99)

-- Test 3.4: UPDATE with NULL values
CREATE TABLE t_update_null (
    id INT,
    nullable_col VARCHAR(100),
    value INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_update_null VALUES (1, 'has_value', 100);
INSERT INTO t_update_null VALUES (2, NULL, 200);

UPDATE t_update_null SET nullable_col = NULL WHERE id = 1;
UPDATE t_update_null SET nullable_col = 'now_has_value', value = NULL WHERE id = 2;
SELECT * FROM t_update_null ORDER BY id;
-- Expected: id=1 (NULL, 100), id=2 ('now_has_value', NULL)

-- Test 3.5: UPDATE with complex expressions
CREATE TABLE t_update_expr (
    id INT,
    base INT,
    bonus INT,
    total INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_update_expr VALUES (1, 1000, 100, 0);
INSERT INTO t_update_expr VALUES (2, 2000, 200, 0);
INSERT INTO t_update_expr VALUES (3, 3000, 300, 0);

UPDATE t_update_expr 
SET total = base + bonus, 
    bonus = bonus * 2 
WHERE id < 3;
SELECT * FROM t_update_expr ORDER BY id;
-- Expected: id=1 (total=1100, bonus=200), id=2 (total=2200, bonus=400), id=3 unchanged

-- Test 3.6: UPDATE with subquery (if supported)
CREATE TABLE t_update_subq (
    id INT,
    category VARCHAR(50),
    value INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_update_subq VALUES 
    (1, 'A', 100), (2, 'A', 200), (3, 'B', 300);

UPDATE t_update_subq 
SET value = (SELECT MAX(value) FROM t_update_subq) 
WHERE category = 'A';
SELECT * FROM t_update_subq ORDER BY id;
-- Expected: category A rows have value=300

-- Test 3.7: UPDATE all rows (no WHERE clause)
CREATE TABLE t_update_all (
    id INT,
    status VARCHAR(20)
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_update_all VALUES (1, 'active'), (2, 'active'), (3, 'active');
UPDATE t_update_all SET status = 'inactive';
SELECT * FROM t_update_all ORDER BY id;
-- Expected: all rows have status='inactive'

-- Test 3.8: UPDATE with IN clause
CREATE TABLE t_update_in (
    id INT,
    status VARCHAR(20)
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_update_in VALUES 
    (1, 'pending'), (2, 'pending'), (3, 'pending'),
    (4, 'pending'), (5, 'pending');

UPDATE t_update_in SET status = 'processed' WHERE id IN (1, 3, 5);
SELECT * FROM t_update_in ORDER BY id;
-- Expected: id 1,3,5 have status='processed'

-- Test 3.9: UPDATE with BETWEEN
CREATE TABLE t_update_between (
    id INT,
    range_val INT,
    category VARCHAR(20)
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_update_between VALUES 
    (1, 10, 'low'), (2, 20, 'low'), (3, 30, 'low'),
    (4, 40, 'low'), (5, 50, 'low');

UPDATE t_update_between 
SET category = 'medium' 
WHERE range_val BETWEEN 20 AND 40;
SELECT * FROM t_update_between ORDER BY id;
-- Expected: id 2,3,4 have category='medium'

-- ============================================================================
-- Part 4: DELETE Tests
-- ============================================================================

-- Test 4.1: DELETE with equality condition
CREATE TABLE t_delete_eq (
    id INT,
    value VARCHAR(50)
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_delete_eq VALUES 
    (1, 'keep'), (2, 'delete'), (3, 'keep');

DELETE FROM t_delete_eq WHERE id = 2;
SELECT * FROM t_delete_eq ORDER BY id;
-- Expected: 2 rows (id=1 and id=3)

-- Test 4.2: DELETE with comparison operators
CREATE TABLE t_delete_comp (
    id INT,
    value INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_delete_comp VALUES 
    (1, 100), (2, 200), (3, 300), (4, 400), (5, 500);

DELETE FROM t_delete_comp WHERE value > 300;
SELECT * FROM t_delete_comp ORDER BY id;
-- Expected: 3 rows (id=1,2,3)

DELETE FROM t_delete_comp WHERE value <= 100;
SELECT * FROM t_delete_comp ORDER BY id;
-- Expected: 2 rows (id=2,3)

-- Test 4.3: DELETE with IN clause
CREATE TABLE t_delete_in (
    id INT,
    category VARCHAR(20)
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_delete_in VALUES 
    (1, 'A'), (2, 'B'), (3, 'C'), (4, 'A'), (5, 'B');

DELETE FROM t_delete_in WHERE category IN ('A', 'C');
SELECT * FROM t_delete_in ORDER BY id;
-- Expected: 2 rows with category='B' (id=2,5)

-- Test 4.4: DELETE with LIKE pattern
CREATE TABLE t_delete_like (
    id INT,
    name VARCHAR(50)
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_delete_like VALUES 
    (1, 'test_001'), (2, 'prod_001'), (3, 'test_002'),
    (4, 'dev_test'), (5, 'test_003');

DELETE FROM t_delete_like WHERE name LIKE 'test%';
SELECT * FROM t_delete_like ORDER BY id;
-- Expected: 3 rows (id=2,4)

-- Test 4.5: DELETE with IS NULL
CREATE TABLE t_delete_null (
    id INT,
    value VARCHAR(50)
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_delete_null VALUES 
    (1, 'value1'), (2, NULL), (3, 'value3'),
    (4, NULL), (5, 'value5');

DELETE FROM t_delete_null WHERE value IS NULL;
SELECT * FROM t_delete_null ORDER BY id;
-- Expected: 3 rows with non-NULL values (id=1,3,5)

-- Test 4.6: DELETE with AND/OR conditions
CREATE TABLE t_delete_logical (
    id INT,
    status VARCHAR(20),
    priority INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_delete_logical VALUES 
    (1, 'active', 1), (2, 'inactive', 2), (3, 'active', 3),
    (4, 'pending', 1), (5, 'active', 2);

DELETE FROM t_delete_logical 
WHERE status = 'active' AND priority > 1;
SELECT * FROM t_delete_logical ORDER BY id;
-- Expected: 3 rows (id=1,2,4)

-- Test 4.7: DELETE with ORDER BY and LIMIT
CREATE TABLE t_delete_order_limit (
    id INT,
    value INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_delete_order_limit VALUES 
    (1, 100), (2, 200), (3, 300), (4, 400), (5, 500);

DELETE FROM t_delete_order_limit ORDER BY value DESC LIMIT 2;
SELECT * FROM t_delete_order_limit ORDER BY id;
-- Expected: 3 rows (deleted highest 2 values: id=4,5)

-- Test 4.8: DELETE with LIMIT only
CREATE TABLE t_delete_limit (
    id INT,
    batch INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_delete_limit VALUES 
    (1, 1), (2, 1), (3, 1), (4, 2), (5, 2);

DELETE FROM t_delete_limit WHERE batch = 1 LIMIT 2;
SELECT * FROM t_delete_limit ORDER BY id;
-- Expected: 4 rows (deleted 2 from batch=1)

-- Test 4.9: DELETE all rows (no WHERE clause)
CREATE TABLE t_delete_all (
    id INT,
    data VARCHAR(50)
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_delete_all VALUES (1, 'a'), (2, 'b'), (3, 'c');
DELETE FROM t_delete_all;
SELECT COUNT(*) FROM t_delete_all;
-- Expected: 0 rows

-- Test 4.10: DELETE with complex subquery condition
CREATE TABLE t_delete_subq_main (
    id INT,
    ref_id INT,
    status VARCHAR(20)
) DISTRIBUTED BY HASH(id) BUCKETS 3;

CREATE TABLE t_delete_subq_ref (
    id INT,
    active BOOLEAN
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_delete_subq_main VALUES 
    (1, 100, 'active'), (2, 200, 'active'), (3, 300, 'active');
INSERT INTO t_delete_subq_ref VALUES (100, TRUE), (200, FALSE);

DELETE FROM t_delete_subq_main 
WHERE ref_id IN (SELECT id FROM t_delete_subq_ref WHERE active = FALSE);
SELECT * FROM t_delete_subq_main ORDER BY id;
-- Expected: 2 rows (id=1,3)

-- Test 4.11: DELETE with BETWEEN
CREATE TABLE t_delete_between (
    id INT,
    value INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_delete_between VALUES 
    (1, 10), (2, 20), (3, 30), (4, 40), (5, 50);

DELETE FROM t_delete_between WHERE value BETWEEN 20 AND 40;
SELECT * FROM t_delete_between ORDER BY id;
-- Expected: 2 rows (id=1,5)

-- ============================================================================
-- Part 5: Edge Cases and Boundary Tests
-- ============================================================================

-- Test 5.1: INSERT into empty table then UPDATE
CREATE TABLE t_edge_empty (
    id INT,
    value INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;

-- Table is empty
SELECT COUNT(*) FROM t_edge_empty;
-- Expected: 0

-- UPDATE on empty table (should not error)
UPDATE t_edge_empty SET value = 100 WHERE id = 1;
SELECT COUNT(*) FROM t_edge_empty;
-- Expected: 0

-- DELETE from empty table (should not error)
DELETE FROM t_edge_empty WHERE id = 1;
SELECT COUNT(*) FROM t_edge_empty;
-- Expected: 0

-- Now insert data
INSERT INTO t_edge_empty VALUES (1, 10), (2, 20);
SELECT * FROM t_edge_empty ORDER BY id;
-- Expected: 2 rows

-- Test 5.2: INSERT with type conversion
CREATE TABLE t_edge_types (
    id INT,
    int_col INT,
    str_col VARCHAR(50),
    dec_col DECIMAL(10, 2)
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_edge_types VALUES 
    (1, 100, '100', 100.50),
    (2, '200', '200', '200.75');
SELECT * FROM t_edge_types ORDER BY id;
-- Expected: successful type conversions

-- Test 5.3: UPDATE with type conversion
UPDATE t_edge_types SET int_col = '300' WHERE id = 1;
SELECT * FROM t_edge_types WHERE id = 1;
-- Expected: int_col=300

-- Test 5.4: INSERT/UPDATE with string functions
CREATE TABLE t_edge_string (
    id INT,
    name VARCHAR(100),
    upper_name VARCHAR(100)
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_edge_string (id, name) VALUES (1, 'alice'), (2, 'bob');
UPDATE t_edge_string SET upper_name = UPPER(name);
SELECT * FROM t_edge_string ORDER BY id;
-- Expected: upper_name='ALICE', 'BOB'

-- Test 5.5: DELETE preserving referential integrity pattern
CREATE TABLE t_edge_parent (
    id INT,
    name VARCHAR(50)
) DISTRIBUTED BY HASH(id) BUCKETS 3;

CREATE TABLE t_edge_child (
    id INT,
    parent_id INT,
    value INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_edge_parent VALUES (1, 'Parent1'), (2, 'Parent2');
INSERT INTO t_edge_child VALUES (1, 1, 100), (2, 1, 200), (3, 2, 300);

-- Delete children first, then parent
DELETE FROM t_edge_child WHERE parent_id = 1;
DELETE FROM t_edge_parent WHERE id = 1;
SELECT * FROM t_edge_parent ORDER BY id;
SELECT * FROM t_edge_child ORDER BY id;
-- Expected: parent_id=1 deleted from both tables

-- Test 5.6: Multiple consecutive operations
CREATE TABLE t_edge_multi (
    id INT,
    version INT,
    data VARCHAR(50)
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_edge_multi VALUES (1, 1, 'v1');
UPDATE t_edge_multi SET version = 2, data = 'v2' WHERE id = 1;
UPDATE t_edge_multi SET version = 3, data = 'v3' WHERE id = 1;
DELETE FROM t_edge_multi WHERE version = 3;
SELECT COUNT(*) FROM t_edge_multi;
-- Expected: 0

-- Test 5.7: Large batch INSERT and UPDATE
CREATE TABLE t_edge_large (
    id INT,
    batch INT,
    processed BOOLEAN
) DISTRIBUTED BY HASH(id) BUCKETS 3;

-- Insert 100 rows in batches
INSERT INTO t_edge_large VALUES 
    (1, 1, FALSE), (2, 1, FALSE), (3, 1, FALSE), (4, 1, FALSE), (5, 1, FALSE),
    (6, 2, FALSE), (7, 2, FALSE), (8, 2, FALSE), (9, 2, FALSE), (10, 2, FALSE);

-- Update all batch 1
UPDATE t_edge_large SET processed = TRUE WHERE batch = 1;
SELECT COUNT(*) FROM t_edge_large WHERE processed = TRUE;
-- Expected: 5

-- Delete unprocessed from batch 2
DELETE FROM t_edge_large WHERE batch = 2 AND processed = FALSE;
SELECT COUNT(*) FROM t_edge_large;
-- Expected: 5 (only batch 1 remains)

-- ============================================================================
-- Cleanup
-- ============================================================================
DROP TABLE IF EXISTS t_insert_single;
DROP TABLE IF EXISTS t_insert_columns;
DROP TABLE IF EXISTS t_insert_null;
DROP TABLE IF EXISTS t_insert_expr;
DROP TABLE IF EXISTS t_insert_select_src;
DROP TABLE IF EXISTS t_insert_select_dst;
DROP TABLE IF EXISTS t_insert_select_filtered;
DROP TABLE IF EXISTS t_insert_select_top;
DROP TABLE IF EXISTS t_insert_select_agg;
DROP TABLE IF EXISTS t_sales;
DROP TABLE IF EXISTS t_update_basic;
DROP TABLE IF EXISTS t_update_multi;
DROP TABLE IF EXISTS t_update_null;
DROP TABLE IF EXISTS t_update_expr;
DROP TABLE IF EXISTS t_update_subq;
DROP TABLE IF EXISTS t_update_all;
DROP TABLE IF EXISTS t_update_in;
DROP TABLE IF EXISTS t_update_between;
DROP TABLE IF EXISTS t_delete_eq;
DROP TABLE IF EXISTS t_delete_comp;
DROP TABLE IF EXISTS t_delete_in;
DROP TABLE IF EXISTS t_delete_like;
DROP TABLE IF EXISTS t_delete_null;
DROP TABLE IF EXISTS t_delete_logical;
DROP TABLE IF EXISTS t_delete_order_limit;
DROP TABLE IF EXISTS t_delete_limit;
DROP TABLE IF EXISTS t_delete_all;
DROP TABLE IF EXISTS t_delete_subq_main;
DROP TABLE IF EXISTS t_delete_subq_ref;
DROP TABLE IF EXISTS t_delete_between;
DROP TABLE IF EXISTS t_edge_empty;
DROP TABLE IF EXISTS t_edge_types;
DROP TABLE IF EXISTS t_edge_string;
DROP TABLE IF EXISTS t_edge_parent;
DROP TABLE IF EXISTS t_edge_child;
DROP TABLE IF EXISTS t_edge_multi;
DROP TABLE IF EXISTS t_edge_large;

DROP DATABASE dml_basic_test;

-- ============================================================================
-- Summary
-- ============================================================================
SELECT 'DML Basic Test Completed Successfully' AS status;