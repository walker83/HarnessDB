-- Test 1: INSERT → UPDATE → SELECT verification
CREATE TABLE t_update_verify (
    id INT,
    name VARCHAR(100),
    value INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_update_verify VALUES (1, 'One', 100);
INSERT INTO t_update_verify VALUES (2, 'Two', 200);
INSERT INTO t_update_verify VALUES (3, 'Three', 300);

UPDATE t_update_verify SET value = 999 WHERE id = 1;
SELECT * FROM t_update_verify WHERE id = 1;
-- Expected: id=1, name=One, value=999

UPDATE t_update_verify SET value = value * 2 WHERE id > 1;
SELECT * FROM t_update_verify WHERE id = 2;
-- Expected: id=2, name=Two, value=400

-- Test 2: INSERT → DELETE → SELECT verification
CREATE TABLE t_delete_verify (
    id INT,
    name VARCHAR(100),
    value INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_delete_verify VALUES (1, 'One', 100);
INSERT INTO t_delete_verify VALUES (2, 'Two', 200);
INSERT INTO t_delete_verify VALUES (3, 'Three', 300);

DELETE FROM t_delete_verify WHERE id = 2;
SELECT * FROM t_delete_verify;
-- Expected: rows with id=1 and id=3 remain

DELETE FROM t_delete_verify WHERE id IN (1, 3);
SELECT COUNT(*) FROM t_delete_verify;
-- Expected: 0 rows

-- Cleanup
DROP TABLE t_update_verify;
DROP TABLE t_delete_verify;

SELECT 'DML verification tests passed' AS status;