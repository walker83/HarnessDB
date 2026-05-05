-- ============================================================================
-- Transaction Tests: BEGIN/COMMIT/ROLLBACK
-- ============================================================================

DROP DATABASE IF EXISTS transaction_test;
CREATE DATABASE transaction_test;
USE transaction_test;

-- Test 1: BEGIN/COMMIT basic
CREATE TABLE t_tx_test (
    id INT,
    value INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;

BEGIN;
INSERT INTO t_tx_test VALUES (1, 100);
INSERT INTO t_tx_test VALUES (2, 200);
COMMIT;

SELECT COUNT(*) FROM t_tx_test;
-- Expected: 2 rows

-- Test 2: BEGIN/ROLLBACK
BEGIN;
INSERT INTO t_tx_test VALUES (3, 300);
INSERT INTO t_tx_test VALUES (4, 400);
ROLLBACK;

SELECT COUNT(*) FROM t_tx_test;
-- Expected: still 2 rows (3, 400 were rolled back)

-- Test 3: UPDATE in transaction then COMMIT
BEGIN;
UPDATE t_tx_test SET value = 999 WHERE id = 1;
COMMIT;
SELECT * FROM t_tx_test WHERE id = 1;
-- Expected: id=1, value=999

-- Test 4: UPDATE in transaction then ROLLBACK
BEGIN;
UPDATE t_tx_test SET value = 888 WHERE id = 1;
ROLLBACK;
SELECT * FROM t_tx_test WHERE id = 1;
-- Expected: id=1, value=999 (rolled back to original)

-- Test 5: DELETE in transaction then COMMIT
BEGIN;
DELETE FROM t_tx_test WHERE id = 1;
COMMIT;
SELECT COUNT(*) FROM t_tx_test;
-- Expected: 1 row (only id=2 remains)

-- Cleanup
DROP TABLE t_tx_test;
DROP DATABASE transaction_test;

SELECT 'Transaction tests passed' AS status;