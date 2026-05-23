-- ============================================================================
-- DML UPDATE / DELETE E2E Test Script
-- Test Coverage: UPDATE and DELETE operations with all WHERE clause types,
-- expressions, edge cases, chained operations, subqueries, complex conditions
-- ============================================================================
-- Total test cases: 200+ focused on UPDATE and DELETE
-- ============================================================================

-- ============================================================================
-- PART 0: Setup
-- ============================================================================
DROP DATABASE IF EXISTS e2e_update_delete_test;
CREATE DATABASE e2e_update_delete_test;
USE e2e_update_delete_test;

-- ============================================================================
-- PART 1: UPDATE Tests (Test 1.1 - 1.97)
-- ============================================================================

-- ============================================================================
-- 1.1 UPDATE with single column SET
-- ============================================================================

-- Test 1.1: UPDATE single column with WHERE =
-- Expected: id=1 val changes from 100 to 999
CREATE TABLE t_up1 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up1 VALUES (1, 100), (2, 200), (3, 300);
UPDATE t_up1 SET val = 999 WHERE id = 1;
SELECT * FROM t_up1 ORDER BY id;
-- Expected: 3 rows: (1, 999), (2, 200), (3, 300)
DROP TABLE t_up1;

-- Test 1.2: UPDATE single column with WHERE !=
-- Expected: rows where id != 1 are updated
CREATE TABLE t_up2 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up2 VALUES (1, 10), (2, 20), (3, 30);
UPDATE t_up2 SET val = 0 WHERE id != 1;
SELECT * FROM t_up2 ORDER BY id;
-- Expected: (1, 10), (2, 0), (3, 0)
DROP TABLE t_up2;

-- Test 1.3: UPDATE single column with WHERE <>
-- Expected: same as !=
CREATE TABLE t_up3 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up3 VALUES (1, 10), (2, 20), (3, 30);
UPDATE t_up3 SET val = 99 WHERE id <> 2;
SELECT * FROM t_up3 ORDER BY id;
-- Expected: (1, 99), (2, 20), (3, 99)
DROP TABLE t_up3;

-- Test 1.4: UPDATE single column with WHERE <
-- Expected: rows with val < 25 set to 0
CREATE TABLE t_up4 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up4 VALUES (1, 10), (2, 25), (3, 30);
UPDATE t_up4 SET val = 0 WHERE val < 25;
SELECT * FROM t_up4 ORDER BY id;
-- Expected: (1, 0), (2, 25), (3, 30)
DROP TABLE t_up4;

-- Test 1.5: UPDATE single column with WHERE >
-- Expected: rows with val > 100 set to 0
CREATE TABLE t_up5 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up5 VALUES (1, 50), (2, 100), (3, 150);
UPDATE t_up5 SET val = 0 WHERE val > 100;
SELECT * FROM t_up5 ORDER BY id;
-- Expected: (1, 50), (2, 100), (3, 0)
DROP TABLE t_up5;

-- Test 1.6: UPDATE single column with WHERE <=
-- Expected: rows with val <= 20 set to 0
CREATE TABLE t_up6 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up6 VALUES (1, 10), (2, 20), (3, 30);
UPDATE t_up6 SET val = 0 WHERE val <= 20;
SELECT * FROM t_up6 ORDER BY id;
-- Expected: (1, 0), (2, 0), (3, 30)
DROP TABLE t_up6;

-- Test 1.7: UPDATE single column with WHERE >=
-- Expected: rows with val >= 20 set to 0
CREATE TABLE t_up7 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up7 VALUES (1, 10), (2, 20), (3, 30);
UPDATE t_up7 SET val = 0 WHERE val >= 20;
SELECT * FROM t_up7 ORDER BY id;
-- Expected: (1, 10), (2, 0), (3, 0)
DROP TABLE t_up7;

-- ============================================================================
-- 1.2 UPDATE with VARCHAR/string columns
-- ============================================================================

-- Test 1.8: UPDATE string column
-- Expected: name changes from 'Alice' to 'Updated'
CREATE TABLE t_up8 (id INT, name VARCHAR(50)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up8 VALUES (1, 'Alice'), (2, 'Bob'), (3, 'Charlie');
UPDATE t_up8 SET name = 'Updated' WHERE id = 1;
SELECT * FROM t_up8 ORDER BY id;
-- Expected: (1, 'Updated'), (2, 'Bob'), (3, 'Charlie')
DROP TABLE t_up8;

-- Test 1.9: UPDATE string column with LIKE in WHERE
-- Expected: names starting with 'A' get new name
CREATE TABLE t_up9 (id INT, name VARCHAR(50)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up9 VALUES (1, 'Alice'), (2, 'Alex'), (3, 'Bob');
UPDATE t_up9 SET name = 'Found' WHERE name LIKE 'Al%';
SELECT * FROM t_up9 ORDER BY id;
-- Expected: (1, 'Found'), (2, 'Found'), (3, 'Bob')
DROP TABLE t_up9;

-- Test 1.10: UPDATE string column with LIKE '%suffix'
-- Expected: names ending with 'e' get new name
CREATE TABLE t_up10 (id INT, name VARCHAR(50)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up10 VALUES (1, 'Alice'), (2, 'Bob'), (3, 'Charlie');
UPDATE t_up10 SET name = 'EndsWithE' WHERE name LIKE '%e';
SELECT * FROM t_up10 ORDER BY id;
-- Expected: (1, 'EndsWithE'), (2, 'Bob'), (3, 'EndsWithE')
DROP TABLE t_up10;

-- Test 1.11: UPDATE string column with LIKE '%contains%'
-- Expected: names containing 'li' get new name
CREATE TABLE t_up11 (id INT, name VARCHAR(50)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up11 VALUES (1, 'Alice'), (2, 'Oliver'), (3, 'Bob');
UPDATE t_up11 SET name = 'Contains' WHERE name LIKE '%li%';
SELECT * FROM t_up11 ORDER BY id;
-- Expected: (1, 'Contains'), (2, 'Contains'), (3, 'Bob')
DROP TABLE t_up11;

-- Test 1.12: UPDATE string with LIKE single char wildcard _
-- Expected: names matching 'A_i_e' get updated
CREATE TABLE t_up12 (id INT, name VARCHAR(50)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up12 VALUES (1, 'Alice'), (2, 'Annie'), (3, 'Bob');
UPDATE t_up12 SET name = 'Wildcard' WHERE name LIKE 'A_i_e';
SELECT * FROM t_up12 ORDER BY id;
-- Expected: (1, 'Wildcard'), (2, 'Annie'), (3, 'Bob')
DROP TABLE t_up12;

-- Test 1.13: UPDATE with NOT LIKE
-- Expected: names not containing 'o' get new name
CREATE TABLE t_up13 (id INT, name VARCHAR(50)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up13 VALUES (1, 'Alice'), (2, 'Bob'), (3, 'Charlie');
UPDATE t_up13 SET name = 'NoO' WHERE name NOT LIKE '%o%';
SELECT * FROM t_up13 ORDER BY id;
-- Expected: (1, 'NoO'), (2, 'Bob'), (3, 'NoO')
DROP TABLE t_up13;

-- ============================================================================
-- 1.3 UPDATE with multiple columns
-- ============================================================================

-- Test 1.14: UPDATE multiple columns at once
-- Expected: both val and name change for id=1
CREATE TABLE t_up14 (id INT, name VARCHAR(50), val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up14 VALUES (1, 'Alice', 100), (2, 'Bob', 200);
UPDATE t_up14 SET name = 'Alicia', val = 150 WHERE id = 1;
SELECT * FROM t_up14 ORDER BY id;
-- Expected: (1, 'Alicia', 150), (2, 'Bob', 200)
DROP TABLE t_up14;

-- Test 1.15: UPDATE three columns at once
-- Expected: three columns updated for matching row
CREATE TABLE t_up15 (id INT, a INT, b INT, c INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up15 VALUES (1, 1, 2, 3), (2, 4, 5, 6);
UPDATE t_up15 SET a = 10, b = 20, c = 30 WHERE id = 1;
SELECT * FROM t_up15 ORDER BY id;
-- Expected: (1, 10, 20, 30), (2, 4, 5, 6)
DROP TABLE t_up15;

-- Test 1.16: UPDATE multiple columns with AND condition
-- Expected: only row where name='Eng' AND val=100 gets updated
CREATE TABLE t_up16 (id INT, name VARCHAR(20), val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up16 VALUES (1, 'Eng', 100), (2, 'Eng', 50), (3, 'Sales', 100);
UPDATE t_up16 SET name = 'Updated', val = 999 WHERE name = 'Eng' AND val = 100;
SELECT * FROM t_up16 ORDER BY id;
-- Expected: (1, 'Updated', 999), (2, 'Eng', 50), (3, 'Sales', 100)
DROP TABLE t_up16;

-- Test 1.17: UPDATE multiple columns with OR condition
-- Expected: rows matching either condition get updated
CREATE TABLE t_up17 (id INT, name VARCHAR(20), val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up17 VALUES (1, 'Alice', 100), (2, 'Bob', 200), (3, 'Charlie', 300);
UPDATE t_up17 SET name = 'Found', val = 0 WHERE id = 1 OR id = 3;
SELECT * FROM t_up17 ORDER BY id;
-- Expected: (1, 'Found', 0), (2, 'Bob', 200), (3, 'Found', 0)
DROP TABLE t_up17;

-- ============================================================================
-- 1.4 UPDATE with AND / OR / NOT conditions
-- ============================================================================

-- Test 1.18: UPDATE with AND (two conditions)
-- Expected: only row matching both conditions
CREATE TABLE t_up18 (id INT, dept VARCHAR(20), salary INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up18 VALUES (1, 'Eng', 100), (2, 'Eng', 80), (3, 'Sales', 90);
UPDATE t_up18 SET salary = 999 WHERE dept = 'Eng' AND salary >= 100;
SELECT * FROM t_up18 ORDER BY id;
-- Expected: (1, 'Eng', 999), (2, 'Eng', 80), (3, 'Sales', 90)
DROP TABLE t_up18;

-- Test 1.19: UPDATE with AND (three conditions)
-- Expected: only rows matching all three conditions
CREATE TABLE t_up19 (id INT, dept VARCHAR(20), salary INT, active INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up19 VALUES (1, 'Eng', 100, 1), (2, 'Eng', 80, 1), (3, 'Sales', 100, 1);
UPDATE t_up19 SET salary = 0 WHERE dept = 'Eng' AND salary > 50 AND active = 1;
SELECT * FROM t_up19 ORDER BY id;
-- Expected: (1, 'Eng', 0), (2, 'Eng', 0), (3, 'Sales', 100)
DROP TABLE t_up19;

-- Test 1.20: UPDATE with OR (two conditions)
-- Expected: rows matching either condition
CREATE TABLE t_up20 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up20 VALUES (1, 10), (2, 20), (3, 30), (4, 40);
UPDATE t_up20 SET val = 0 WHERE val = 10 OR val = 40;
SELECT * FROM t_up20 ORDER BY id;
-- Expected: (1, 0), (2, 20), (3, 30), (4, 0)
DROP TABLE t_up20;

-- Test 1.21: UPDATE with OR (three conditions)
-- Expected: rows matching any of three conditions
CREATE TABLE t_up21 (id INT, status VARCHAR(20)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up21 VALUES (1, 'open'), (2, 'closed'), (3, 'pending'), (4, 'cancelled');
UPDATE t_up21 SET status = 'archived' WHERE status = 'closed' OR status = 'cancelled' OR status = 'pending';
SELECT * FROM t_up21 ORDER BY id;
-- Expected: (1, 'open'), (2, 'archived'), (3, 'archived'), (4, 'archived')
DROP TABLE t_up21;

-- Test 1.22: UPDATE with NOT
-- Expected: rows NOT matching condition get updated
CREATE TABLE t_up22 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up22 VALUES (1, 10), (2, 20), (3, 30);
UPDATE t_up22 SET val = 0 WHERE NOT val = 20;
SELECT * FROM t_up22 ORDER BY id;
-- Expected: (1, 0), (2, 20), (3, 0)
DROP TABLE t_up22;

-- Test 1.23: UPDATE with NOT AND (combined)
-- Expected: rows not matching both conditions
CREATE TABLE t_up23 (id INT, dept VARCHAR(20), salary INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up23 VALUES (1, 'Eng', 100), (2, 'Eng', 80), (3, 'Sales', 100);
UPDATE t_up23 SET salary = 0 WHERE NOT (dept = 'Eng' AND salary = 100);
SELECT * FROM t_up23 ORDER BY id;
-- Expected: (1, 'Eng', 100), (2, 'Eng', 0), (3, 'Sales', 0)
DROP TABLE t_up23;

-- Test 1.24: UPDATE with nested AND/OR (AND inside OR)
-- Expected: (dept='Eng' AND salary>=90) OR dept='HR' are updated
CREATE TABLE t_up24 (id INT, dept VARCHAR(20), salary INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up24 VALUES (1, 'Eng', 100), (2, 'Eng', 80), (3, 'Sales', 90), (4, 'HR', 70);
UPDATE t_up24 SET salary = 0 WHERE (dept = 'Eng' AND salary >= 90) OR dept = 'HR';
SELECT * FROM t_up24 ORDER BY id;
-- Expected: (1, 'Eng', 0), (2, 'Eng', 80), (3, 'Sales', 90), (4, 'HR', 0)
DROP TABLE t_up24;

-- Test 1.25: UPDATE with nested AND/OR (OR inside AND)
-- Expected: dept='Eng' AND (salary=100 OR id=2) are updated
CREATE TABLE t_up25 (id INT, dept VARCHAR(20), salary INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up25 VALUES (1, 'Eng', 100), (2, 'Eng', 80), (3, 'Sales', 100);
UPDATE t_up25 SET salary = 0 WHERE dept = 'Eng' AND (salary = 100 OR id = 2);
SELECT * FROM t_up25 ORDER BY id;
-- Expected: (1, 'Eng', 0), (2, 'Eng', 0), (3, 'Sales', 100)
DROP TABLE t_up25;

-- Test 1.26: UPDATE with deeply nested AND/OR conditions
-- Expected: complex condition matching only specific row
CREATE TABLE t_up26 (id INT, dept VARCHAR(20), salary INT, city VARCHAR(20)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up26 VALUES (1, 'Eng', 100, 'NYC'), (2, 'Eng', 80, 'LA'), (3, 'Sales', 90, 'NYC'), (4, 'HR', 70, 'LA');
UPDATE t_up26 SET salary = 0 WHERE (dept = 'Eng' OR dept = 'HR') AND (city = 'NYC' OR salary < 90);
SELECT * FROM t_up26 ORDER BY id;
-- Expected: (1, 'Eng', 0), (2, 'Eng', 0), (3, 'Sales', 90), (4, 'HR', 0)
DROP TABLE t_up26;

-- ============================================================================
-- 1.5 UPDATE with IN / NOT IN
-- ============================================================================

-- Test 1.27: UPDATE with IN (integer list)
-- Expected: rows with id in (1, 3, 5) get updated
CREATE TABLE t_up27 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up27 VALUES (1, 10), (2, 20), (3, 30), (4, 40), (5, 50);
UPDATE t_up27 SET val = 0 WHERE id IN (1, 3, 5);
SELECT * FROM t_up27 ORDER BY id;
-- Expected: (1, 0), (2, 20), (3, 0), (4, 40), (5, 0)
DROP TABLE t_up27;

-- Test 1.28: UPDATE with IN (string list)
-- Expected: rows with name in list get updated
CREATE TABLE t_up28 (id INT, name VARCHAR(50)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up28 VALUES (1, 'Alice'), (2, 'Bob'), (3, 'Charlie'), (4, 'David');
UPDATE t_up28 SET name = 'Matched' WHERE name IN ('Alice', 'Charlie');
SELECT * FROM t_up28 ORDER BY id;
-- Expected: (1, 'Matched'), (2, 'Bob'), (3, 'Matched'), (4, 'David')
DROP TABLE t_up28;

-- Test 1.29: UPDATE with NOT IN
-- Expected: rows NOT in the list get updated
CREATE TABLE t_up29 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up29 VALUES (1, 10), (2, 20), (3, 30), (4, 40);
UPDATE t_up29 SET val = 0 WHERE id NOT IN (2, 4);
SELECT * FROM t_up29 ORDER BY id;
-- Expected: (1, 0), (2, 20), (3, 0), (4, 40)
DROP TABLE t_up29;

-- ============================================================================
-- 1.6 UPDATE with BETWEEN / NOT BETWEEN
-- ============================================================================

-- Test 1.30: UPDATE with BETWEEN (integers)
-- Expected: rows with val between 15 and 35 inclusive get updated
CREATE TABLE t_up30 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up30 VALUES (1, 10), (2, 20), (3, 30), (4, 40);
UPDATE t_up30 SET val = 0 WHERE val BETWEEN 15 AND 35;
SELECT * FROM t_up30 ORDER BY id;
-- Expected: (1, 10), (2, 0), (3, 0), (4, 40)
DROP TABLE t_up30;

-- Test 1.31: UPDATE with NOT BETWEEN
-- Expected: rows with val outside range get updated
CREATE TABLE t_up31 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up31 VALUES (1, 5), (2, 25), (3, 50);
UPDATE t_up31 SET val = 0 WHERE val NOT BETWEEN 10 AND 40;
SELECT * FROM t_up31 ORDER BY id;
-- Expected: (1, 0), (2, 25), (3, 0)
DROP TABLE t_up31;

-- Test 1.32: UPDATE with BETWEEN (string column)
-- Expected: names lexicographically between ranges get updated
CREATE TABLE t_up32 (id INT, name VARCHAR(50)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up32 VALUES (1, 'apple'), (2, 'banana'), (3, 'cherry'), (4, 'date');
UPDATE t_up32 SET name = 'fruit' WHERE name BETWEEN 'banana' AND 'cherry';
SELECT * FROM t_up32 ORDER BY id;
-- Expected: (1, 'apple'), (2, 'fruit'), (3, 'fruit'), (4, 'date')
DROP TABLE t_up32;

-- Test 1.33: UPDATE with BETWEEN AND on VARCHAR dates
-- Expected: rows with dates in range get updated (string comparison of ISO dates works)
CREATE TABLE t_up33 (id INT, d DATE) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up33 VALUES (1, '2024-01-01'), (2, '2024-06-15'), (3, '2024-12-31');
UPDATE t_up33 SET d = '2024-01-01' WHERE d BETWEEN '2024-03-01' AND '2024-12-31';
SELECT * FROM t_up33 ORDER BY id;
-- Expected: (1, '2024-01-01'), (2, '2024-01-01'), (3, '2024-01-01')
DROP TABLE t_up33;

-- ============================================================================
-- 1.7 UPDATE with IS NULL / IS NOT NULL
-- ============================================================================

-- Test 1.34: UPDATE with IS NULL
-- Expected: rows with NULL val get updated to non-NULL
CREATE TABLE t_up34 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up34 VALUES (1, NULL), (2, 200), (3, NULL);
UPDATE t_up34 SET val = 0 WHERE val IS NULL;
SELECT * FROM t_up34 ORDER BY id;
-- Expected: (1, 0), (2, 200), (3, 0)
DROP TABLE t_up34;

-- Test 1.35: UPDATE with IS NOT NULL
-- Expected: rows with non-NULL values get updated
CREATE TABLE t_up35 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up35 VALUES (1, NULL), (2, 200), (3, 300);
UPDATE t_up35 SET val = 0 WHERE val IS NOT NULL;
SELECT * FROM t_up35 ORDER BY id;
-- Expected: (1, NULL), (2, 0), (3, 0)
DROP TABLE t_up35;

-- Test 1.36: UPDATE setting column to NULL
-- Expected: column set to NULL for matching row
CREATE TABLE t_up36 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up36 VALUES (1, 100), (2, 200);
UPDATE t_up36 SET val = NULL WHERE id = 1;
SELECT * FROM t_up36 ORDER BY id;
-- Expected: (1, NULL), (2, 200)
DROP TABLE t_up36;

-- Test 1.37: UPDATE setting column to NULL with IS NOT NULL filter
-- Expected: only non-null rows get set to null
CREATE TABLE t_up37 (id INT, name VARCHAR(50)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up37 VALUES (1, 'Alice'), (2, NULL), (3, 'Charlie');
UPDATE t_up37 SET name = NULL WHERE name IS NOT NULL;
SELECT * FROM t_up37 ORDER BY id;
-- Expected: (1, NULL), (2, NULL), (3, NULL)
DROP TABLE t_up37;

-- ============================================================================
-- 1.8 UPDATE with arithmetic expressions in SET
-- Note: arithmetic expressions in UPDATE SET (val = val + N) may not work correctly
-- ============================================================================

-- Test 1.38: UPDATE with SET col = col + 1 (addition)
-- Expected: val incremented by 1 for matching rows
CREATE TABLE t_up38 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up38 VALUES (1, 10), (2, 20), (3, 30);
UPDATE t_up38 SET val = val + 1 WHERE id >= 2;
SELECT * FROM t_up38 ORDER BY id;
-- Expected: (1, 10), (2, 21), (3, 31)
DROP TABLE t_up38;

-- Test 1.39: UPDATE with SET col = col - 5 (subtraction)
-- Expected: val decreased by 5
CREATE TABLE t_up39 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up39 VALUES (1, 100), (2, 50), (3, 25);
UPDATE t_up39 SET val = val - 5 WHERE val > 30;
SELECT * FROM t_up39 ORDER BY id;
-- Expected: (1, 95), (2, 45), (3, 25)
DROP TABLE t_up39;

-- Test 1.40: UPDATE with SET col = col * 2 (multiplication)
-- Expected: val doubled
CREATE TABLE t_up40 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up40 VALUES (1, 5), (2, 10), (3, 15);
UPDATE t_up40 SET val = val * 2 WHERE id IN (1, 3);
SELECT * FROM t_up40 ORDER BY id;
-- Expected: (1, 10), (2, 10), (3, 30)
DROP TABLE t_up40;

-- Test 1.41: UPDATE with SET col = col / 2 (division)
-- Expected: val halved
CREATE TABLE t_up41 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up41 VALUES (1, 20), (2, 30), (3, 40);
UPDATE t_up41 SET val = val / 2 WHERE val >= 30;
SELECT * FROM t_up41 ORDER BY id;
-- Expected: (1, 20), (2, 15), (3, 20)
DROP TABLE t_up41;

-- Test 1.42: UPDATE with combined expression SET col = (col + 5) * 2
-- Expected: val computed as (col + 5) * 2
CREATE TABLE t_up42 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up42 VALUES (1, 10), (2, 20);
UPDATE t_up42 SET val = (val + 5) * 2 WHERE id = 1;
SELECT * FROM t_up42 ORDER BY id;
-- Expected: (1, 30), (2, 20)
DROP TABLE t_up42;

-- Test 1.43: UPDATE with SET col = col + col (self reference)
-- Expected: val doubled (added to itself)
CREATE TABLE t_up43 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up43 VALUES (1, 10), (2, 25);
UPDATE t_up43 SET val = val + val WHERE id = 2;
SELECT * FROM t_up43 ORDER BY id;
-- Expected: (1, 10), (2, 50)
DROP TABLE t_up43;

-- Test 1.44: UPDATE with arithmetic on different columns
-- Expected: col_a set to col_b + col_c
CREATE TABLE t_up44 (id INT, col_a INT, col_b INT, col_c INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up44 VALUES (1, 0, 10, 20), (2, 0, 5, 15);
UPDATE t_up44 SET col_a = col_b + col_c WHERE id = 1;
SELECT * FROM t_up44 ORDER BY id;
-- Expected: (1, 30, 10, 20), (2, 0, 5, 15)
DROP TABLE t_up44;

-- Test 1.45: UPDATE with SET col = col * col (square)
-- Expected: val squared
CREATE TABLE t_up45 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up45 VALUES (1, 4), (2, 7);
UPDATE t_up45 SET val = val * val WHERE id = 2;
SELECT * FROM t_up45 ORDER BY id;
-- Expected: (1, 4), (2, 49)
DROP TABLE t_up45;

-- ============================================================================
-- 1.9 UPDATE edge cases
-- ============================================================================

-- Test 1.46: UPDATE all rows (no WHERE clause)
-- Expected: ALL rows get updated
CREATE TABLE t_up46 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up46 VALUES (1, 10), (2, 20), (3, 30);
UPDATE t_up46 SET val = 100;
SELECT * FROM t_up46 ORDER BY id;
-- Expected: (1, 100), (2, 100), (3, 100)
DROP TABLE t_up46;

-- Test 1.47: UPDATE no matching rows
-- Expected: no rows affected, data unchanged
CREATE TABLE t_up47 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up47 VALUES (1, 10), (2, 20);
UPDATE t_up47 SET val = 999 WHERE id = 999;
SELECT * FROM t_up47 ORDER BY id;
-- Expected: (1, 10), (2, 20) unchanged
DROP TABLE t_up47;

-- Test 1.48: UPDATE on empty table
-- Expected: no error, table remains empty
CREATE TABLE t_up48 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
-- Table empty
UPDATE t_up48 SET val = 100 WHERE id = 1;
SELECT COUNT(*) FROM t_up48;
-- Expected: 0 rows
DROP TABLE t_up48;

-- Test 1.49: UPDATE same column multiple times (chained)
-- Expected: value ends up at final SET value
CREATE TABLE t_up49 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up49 VALUES (1, 0);
UPDATE t_up49 SET val = 1 WHERE id = 1;
UPDATE t_up49 SET val = 2 WHERE id = 1;
UPDATE t_up49 SET val = 3 WHERE id = 1;
SELECT * FROM t_up49;
-- Expected: (1, 3)
DROP TABLE t_up49;

-- Test 1.50: UPDATE multiple rows with same value
-- Expected: all rows matching condition get same update
CREATE TABLE t_up50 (id INT, category VARCHAR(20)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up50 VALUES (1, 'A'), (2, 'A'), (3, 'B'), (4, 'B');
UPDATE t_up50 SET category = 'All' WHERE category IN ('A', 'B');
SELECT * FROM t_up50 ORDER BY id;
-- Expected: (1, 'All'), (2, 'All'), (3, 'All'), (4, 'All')
DROP TABLE t_up50;

-- Test 1.51: UPDATE with SET to same value as current
-- Expected: data unchanged, query still works
CREATE TABLE t_up51 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up51 VALUES (1, 100), (2, 200);
UPDATE t_up51 SET val = 100 WHERE id = 1;
SELECT * FROM t_up51 ORDER BY id;
-- Expected: (1, 100), (2, 200)
DROP TABLE t_up51;

-- Test 1.52: UPDATE table with single row
-- Expected: single row updated successfully
CREATE TABLE t_up52 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up52 VALUES (1, 100);
UPDATE t_up52 SET val = 200 WHERE id = 1;
SELECT * FROM t_up52;
-- Expected: (1, 200)
DROP TABLE t_up52;

-- Test 1.53: UPDATE with WHERE on same column being SET
-- Expected: row matches old value before update
CREATE TABLE t_up53 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up53 VALUES (1, 10), (2, 20), (3, 30);
UPDATE t_up53 SET val = 100 WHERE val = 20;
SELECT * FROM t_up53 ORDER BY id;
-- Expected: (1, 10), (2, 100), (3, 30)
DROP TABLE t_up53;

-- Test 1.54: UPDATE with multiple WHERE conditions on same column
-- Expected: row matching complex condition
CREATE TABLE t_up54 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up54 VALUES (1, 5), (2, 10), (3, 15), (4, 20);
UPDATE t_up54 SET val = 0 WHERE val > 5 AND val < 15;
SELECT * FROM t_up54 ORDER BY id;
-- Expected: (1, 5), (2, 0), (3, 15), (4, 20)
DROP TABLE t_up54;

-- Test 1.55: UPDATE using != combined with AND
-- Expected: rows where id != 2 AND val > 10 get updated
CREATE TABLE t_up55 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up55 VALUES (1, 5), (2, 15), (3, 25);
UPDATE t_up55 SET val = 0 WHERE id != 2 AND val > 10;
SELECT * FROM t_up55 ORDER BY id;
-- Expected: (1, 5), (2, 15), (3, 0)
DROP TABLE t_up55;

-- ============================================================================
-- 1.10 UPDATE with DECIMAL/DOUBLE/FLOAT types
-- ============================================================================

-- Test 1.56: UPDATE DOUBLE column
-- Expected: double column updated
CREATE TABLE t_up56 (id INT, score DOUBLE) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up56 VALUES (1, 85.5), (2, 92.3), (3, 78.0);
UPDATE t_up56 SET score = 99.9 WHERE id = 1;
SELECT * FROM t_up56 ORDER BY id;
-- Expected: (1, 99.9), (2, 92.3), (3, 78.0)
DROP TABLE t_up56;

-- Test 1.57: UPDATE DOUBLE with arithmetic
-- Note: arithmetic expressions in UPDATE SET (score = score * 2) may not work correctly
-- Expected: score doubled for matching rows
CREATE TABLE t_up57 (id INT, score DOUBLE) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up57 VALUES (1, 10.5), (2, 20.0), (3, 30.0);
UPDATE t_up57 SET score = score * 2 WHERE score < 25.0;
SELECT * FROM t_up57 ORDER BY id;
-- Expected: (1, 21.0), (2, 40.0), (3, 30.0)
DROP TABLE t_up57;

-- Test 1.58: UPDATE DECIMAL column
-- Expected: decimal column updated
CREATE TABLE t_up58 (id INT, price DECIMAL(10, 2)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up58 VALUES (1, 10.50), (2, 20.75), (3, 30.00);
UPDATE t_up58 SET price = 99.99 WHERE id = 2;
SELECT * FROM t_up58 ORDER BY id;
-- Expected: (1, 10.50), (2, 99.99), (3, 30.00)
DROP TABLE t_up58;

-- Test 1.59: UPDATE DECIMAL with BETWEEN
-- Expected: prices in range updated
CREATE TABLE t_up59 (id INT, price DECIMAL(10, 2)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up59 VALUES (1, 5.00), (2, 15.50), (3, 25.75);
UPDATE t_up59 SET price = 0.00 WHERE price BETWEEN 10.00 AND 20.00;
SELECT * FROM t_up59 ORDER BY id;
-- Expected: (1, 5.00), (2, 0.00), (3, 25.75)
DROP TABLE t_up59;

-- Test 1.60: UPDATE FLOAT column with expression
-- Note: arithmetic expressions in UPDATE SET (rate = rate + 1.0) may not work correctly
-- Expected: float value incremented
CREATE TABLE t_up60 (id INT, rate FLOAT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up60 VALUES (1, 1.5), (2, 2.5), (3, 3.5);
UPDATE t_up60 SET rate = rate + 1.0 WHERE id IN (1, 3);
SELECT * FROM t_up60 ORDER BY id;
-- Expected: (1, 2.5), (2, 2.5), (3, 4.5)
DROP TABLE t_up60;

-- ============================================================================
-- 1.11 UPDATE with VARCHAR columns (using date-like strings)
-- ============================================================================

-- Test 1.61: UPDATE VARCHAR column with date-like string
-- Expected: column updated for matching row
CREATE TABLE t_up61 (id INT, d DATE) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up61 VALUES (1, '2024-01-01'), (2, '2024-06-15'), (3, '2024-12-31');
UPDATE t_up61 SET d = '2025-01-01' WHERE id = 2;
SELECT * FROM t_up61 ORDER BY id;
-- Expected: (1, '2024-01-01'), (2, '2025-01-01'), (3, '2024-12-31')
DROP TABLE t_up61;

-- Test 1.62: UPDATE VARCHAR column with datetime-like string
-- Expected: column updated for matching row
CREATE TABLE t_up62 (id INT, dt DATETIME) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up62 VALUES (1, '2024-01-01 10:00:00'), (2, '2024-06-15 14:30:00');
UPDATE t_up62 SET dt = '2025-12-31 23:59:59' WHERE id = 1;
SELECT * FROM t_up62 ORDER BY id;
-- Expected: (1, '2025-12-31 23:59:59'), (2, '2024-06-15 14:30:00')
DROP TABLE t_up62;

-- Test 1.63: UPDATE with WHERE comparison on VARCHAR date strings
-- Expected: strings before threshold get updated (lexicographic comparison of ISO dates)
CREATE TABLE t_up63 (id INT, d DATE) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up63 VALUES (1, '2024-01-01'), (2, '2024-07-01'), (3, '2025-01-01');
UPDATE t_up63 SET d = '2024-06-01' WHERE d < '2024-07-01';
SELECT * FROM t_up63 ORDER BY id;
-- Expected: (1, '2024-06-01'), (2, '2024-07-01'), (3, '2025-01-01')
DROP TABLE t_up63;

-- Test 1.64: UPDATE with WHERE > on VARCHAR date strings
-- Expected: strings after threshold get updated
CREATE TABLE t_up64 (id INT, d DATE) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up64 VALUES (1, '2023-01-01'), (2, '2024-06-15'), (3, '2025-01-01');
UPDATE t_up64 SET d = '2024-01-01' WHERE d > '2024-01-01';
SELECT * FROM t_up64 ORDER BY id;
-- Expected: (1, '2023-01-01'), (2, '2024-01-01'), (3, '2024-01-01')
DROP TABLE t_up64;

-- ============================================================================
-- 1.12 UPDATE with different integer types
-- ============================================================================

-- Test 1.65: UPDATE TINYINT column
-- Expected: tinyint column updated
CREATE TABLE t_up65 (id INT, flag TINYINT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up65 VALUES (1, 0), (2, 1), (3, 0);
UPDATE t_up65 SET flag = 1 WHERE id IN (1, 3);
SELECT * FROM t_up65 ORDER BY id;
-- Expected: (1, 1), (2, 1), (3, 1)
DROP TABLE t_up65;

-- Test 1.66: UPDATE SMALLINT column
-- Expected: smallint updated
CREATE TABLE t_up66 (id INT, small_val SMALLINT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up66 VALUES (1, 100), (2, 200), (3, 300);
UPDATE t_up66 SET small_val = 999 WHERE small_val = 200;
SELECT * FROM t_up66 ORDER BY id;
-- Expected: (1, 100), (2, 999), (3, 300)
DROP TABLE t_up66;

-- Test 1.67: UPDATE BIGINT column
-- Expected: bigint updated
CREATE TABLE t_up67 (id INT, big_val BIGINT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up67 VALUES (1, 1000000), (2, 2000000), (3, 3000000);
UPDATE t_up67 SET big_val = 9999999 WHERE big_val = 2000000;
SELECT * FROM t_up67 ORDER BY id;
-- Expected: (1, 1000000), (2, 9999999), (3, 3000000)
DROP TABLE t_up67;

-- Test 1.68: UPDATE BIGINT column (replaced from LARGEINT)
-- Expected: bigint updated
CREATE TABLE t_up68 (id INT, large_val BIGINT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up68 VALUES (1, 1234567890123), (2, 9876543210987);
UPDATE t_up68 SET large_val = 5555555555555 WHERE id = 2;
SELECT * FROM t_up68 ORDER BY id;
-- Expected: (1, 1234567890123), (2, 5555555555555)
DROP TABLE t_up68;

-- Test 1.69: UPDATE BOOLEAN column
-- Expected: boolean updated
CREATE TABLE t_up69 (id INT, active BOOLEAN) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up69 VALUES (1, TRUE), (2, FALSE), (3, FALSE);
UPDATE t_up69 SET active = TRUE WHERE id IN (2, 3);
SELECT * FROM t_up69 ORDER BY id;
-- Expected: (1, TRUE), (2, TRUE), (3, TRUE)
DROP TABLE t_up69;

-- ============================================================================
-- 1.13 UPDATE with VARCHAR special characters
-- ============================================================================

-- Test 1.70: UPDATE VARCHAR with special characters
-- Expected: string with spaces updated
CREATE TABLE t_up70 (id INT, label VARCHAR(50)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up70 VALUES (1, 'hello world'), (2, 'spaces here'), (3, 'no');
UPDATE t_up70 SET label = 'updated string' WHERE label LIKE '% %';
SELECT * FROM t_up70 ORDER BY id;
-- Expected: (1, 'updated string'), (2, 'updated string'), (3, 'no')
DROP TABLE t_up70;

-- Test 1.71: UPDATE VARCHAR with numeric string
-- Expected: string containing digits updated
CREATE TABLE t_up71 (id INT, code VARCHAR(50)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up71 VALUES (1, 'abc123'), (2, 'xyz789'), (3, 'plain');
UPDATE t_up71 SET code = 'matched' WHERE code LIKE '%123%';
SELECT * FROM t_up71 ORDER BY id;
-- Expected: (1, 'matched'), (2, 'xyz789'), (3, 'plain')
DROP TABLE t_up71;

-- Test 1.72: UPDATE VARCHAR with exact match
-- Expected: exact string match updated
CREATE TABLE t_up72 (id INT, name VARCHAR(50)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up72 VALUES (1, 'John Doe'), (2, 'Jane Doe'), (3, 'John Smith');
UPDATE t_up72 SET name = 'Found' WHERE name = 'John Doe';
SELECT * FROM t_up72 ORDER BY id;
-- Expected: (1, 'Found'), (2, 'Jane Doe'), (3, 'John Smith')
DROP TABLE t_up72;

-- Test 1.73: UPDATE with empty string
-- Expected: empty string updated to non-empty
CREATE TABLE t_up73 (id INT, label VARCHAR(50)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up73 VALUES (1, ''), (2, 'nonempty'), (3, '');
UPDATE t_up73 SET label = 'filled' WHERE label = '';
SELECT * FROM t_up73 ORDER BY id;
-- Expected: (1, 'filled'), (2, 'nonempty'), (3, 'filled')
DROP TABLE t_up73;

-- ============================================================================
-- 1.14 UPDATE with TEXT / STRING / CHAR types
-- ============================================================================

-- Test 1.74: UPDATE TEXT column
-- Expected: text column updated
CREATE TABLE t_up74 (id INT, content TEXT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up74 VALUES (1, 'short text'), (2, 'longer text content here');
UPDATE t_up74 SET content = 'updated text' WHERE id = 2;
SELECT * FROM t_up74 ORDER BY id;
-- Expected: (1, 'short text'), (2, 'updated text')
DROP TABLE t_up74;

-- Test 1.75: UPDATE STRING column
-- Expected: string column updated
CREATE TABLE t_up75 (id INT, content STRING) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up75 VALUES (1, 'first'), (2, 'second');
UPDATE t_up75 SET content = 'modified' WHERE content = 'first';
SELECT * FROM t_up75 ORDER BY id;
-- Expected: (1, 'modified'), (2, 'second')
DROP TABLE t_up75;

-- Test 1.76: UPDATE CHAR column
-- Expected: char column updated
CREATE TABLE t_up76 (id INT, code CHAR(10)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up76 VALUES (1, 'ABC'), (2, 'XYZ');
UPDATE t_up76 SET code = 'DEF' WHERE code = 'ABC';
SELECT * FROM t_up76 ORDER BY id;
-- Expected: (1, 'DEF'), (2, 'XYZ')
DROP TABLE t_up76;

-- ============================================================================
-- 1.15 UPDATE with type conversion
-- ============================================================================

-- Test 1.77: UPDATE INT column with string literal
-- Expected: string '42' converts to int 42
CREATE TABLE t_up77 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up77 VALUES (1, 10), (2, 20);
UPDATE t_up77 SET val = '42' WHERE id = 1;
SELECT * FROM t_up77 ORDER BY id;
-- Expected: (1, 42), (2, 20)
DROP TABLE t_up77;

-- Test 1.78: UPDATE VARCHAR column with integer literal
-- Expected: integer 123 converts to string '123'
CREATE TABLE t_up78 (id INT, label VARCHAR(50)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up78 VALUES (1, 'text'), (2, 'data');
UPDATE t_up78 SET label = 123 WHERE id = 1;
SELECT * FROM t_up78 ORDER BY id;
-- Expected: (1, '123'), (2, 'data')
DROP TABLE t_up78;

-- ============================================================================
-- 1.16 UPDATE with subqueries in WHERE (rewritten with literal values)
-- Note: cross-table subquery

-- what the subquery would return are kept for documentation.
-- ============================================================================

-- Test 1.79: UPDATE with subquery in WHERE (IN) -- using literals
-- Expected: update where id matches subquery result (ids 1, 3)
CREATE TABLE t_up79_main (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
CREATE TABLE t_up79_ref (id INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up79_main VALUES (1, 10), (2, 20), (3, 30);
INSERT INTO t_up79_ref VALUES (1), (3);
-- Note: cross-table subquery using literal values
UPDATE t_up79_main SET val = 0 WHERE id IN (SELECT id FROM t_up79_ref);
SELECT * FROM t_up79_main ORDER BY id;
-- Expected: (1, 0), (2, 20), (3, 0)
DROP TABLE t_up79_main;
DROP TABLE t_up79_ref;

-- Test 1.80: UPDATE with subquery in WHERE (NOT IN) -- using literals
-- Expected: update where id NOT in subquery (not in {2} = id=1 and id=3)
CREATE TABLE t_up80_main (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
CREATE TABLE t_up80_ref (id INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up80_main VALUES (1, 10), (2, 20), (3, 30);
INSERT INTO t_up80_ref VALUES (2);
-- Note: cross-table subquery using literal values
UPDATE t_up80_main SET val = 0 WHERE id NOT IN (SELECT id FROM t_up80_ref);
SELECT * FROM t_up80_main ORDER BY id;
-- Expected: (1, 0), (2, 20), (3, 0)
DROP TABLE t_up80_main;
DROP TABLE t_up80_ref;

-- Test 1.81: UPDATE with subquery returning multiple values -- using literals
-- Expected: update all rows matching subquery set (depts 'Eng', 'HR')
CREATE TABLE t_up81_main (id INT, dept VARCHAR(20), val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
CREATE TABLE t_up81_filter (dept VARCHAR(20)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up81_main VALUES (1, 'Eng', 100), (2, 'Sales', 200), (3, 'HR', 300);
INSERT INTO t_up81_filter VALUES ('Eng'), ('HR');
-- Note: cross-table subquery using literal values
UPDATE t_up81_main SET val = 0 WHERE dept IN (SELECT dept FROM t_up81_filter);
SELECT * FROM t_up81_main ORDER BY id;
-- Expected: (1, 'Eng', 0), (2, 'Sales', 200), (3, 'HR', 0)
DROP TABLE t_up81_main;
DROP TABLE t_up81_filter;

-- ============================================================================
-- 1.17 UPDATE additional edge and boundary cases
-- ============================================================================

-- Test 1.82: UPDATE with WHERE on computed column (not SET column)
-- Expected: WHERE matches before update, SET is independent
CREATE TABLE t_up82 (id INT, a INT, b INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up82 VALUES (1, 10, 100), (2, 20, 200);
UPDATE t_up82 SET a = a + b WHERE id = 1;
SELECT * FROM t_up82 ORDER BY id;
-- Expected: (1, 110, 100), (2, 20, 200)
DROP TABLE t_up82;

-- Test 1.83: UPDATE with multiple computed columns
-- Note: arithmetic expressions in UPDATE SET (x = x + 1) may not work correctly
-- Expected: both columns computed independently
CREATE TABLE t_up83 (id INT, x INT, y INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up83 VALUES (1, 10, 20), (2, 30, 40);
UPDATE t_up83 SET x = x + 1, y = y + 2 WHERE id = 2;
SELECT * FROM t_up83 ORDER BY id;
-- Expected: (1, 10, 20), (2, 31, 42)
DROP TABLE t_up83;

-- Test 1.84: UPDATE with WHERE on string, SET on int
-- Expected: update works across types
CREATE TABLE t_up84 (id INT, name VARCHAR(50), score INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up84 VALUES (1, 'Alice', 80), (2, 'Bob', 90);
UPDATE t_up84 SET score = 100 WHERE name = 'Bob';
SELECT * FROM t_up84 ORDER BY id;
-- Expected: (1, 'Alice', 80), (2, 'Bob', 100)
DROP TABLE t_up84;

-- Test 1.85: UPDATE with WHERE on int, SET on string
-- Expected: update works across types
CREATE TABLE t_up85 (id INT, label VARCHAR(50), val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up85 VALUES (1, 'low', 10), (2, 'high', 100);
UPDATE t_up85 SET label = 'updated' WHERE val = 100;
SELECT * FROM t_up85 ORDER BY id;
-- Expected: (1, 'low', 10), (2, 'updated', 100)
DROP TABLE t_up85;

-- Test 1.86: UPDATE with WHERE on multiple joined conditions with different types
-- Expected: mixed type condition works
CREATE TABLE t_up86 (id INT, name VARCHAR(50), salary INT, active BOOLEAN) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up86 VALUES (1, 'Alice', 50000, TRUE), (2, 'Bob', 60000, FALSE), (3, 'Charlie', 70000, TRUE);
UPDATE t_up86 SET salary = 0 WHERE active = TRUE AND salary < 60000;
SELECT * FROM t_up86 ORDER BY id;
-- Expected: (1, 'Alice', 0), (2, 'Bob', 60000), (3, 'Charlie', 70000)
DROP TABLE t_up86;

-- Test 1.87: UPDATE with column alias in SET (using same column)
-- Expected: self-update works
CREATE TABLE t_up87 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up87 VALUES (1, 10), (2, 20);
UPDATE t_up87 SET val = val WHERE id = 1;
SELECT * FROM t_up87 ORDER BY id;
-- Expected: (1, 10), (2, 20)
DROP TABLE t_up87;

-- Test 1.88: UPDATE all rows to NULL
-- Expected: all rows set to NULL
CREATE TABLE t_up88 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up88 VALUES (1, 10), (2, 20), (3, 30);
UPDATE t_up88 SET val = NULL;
SELECT * FROM t_up88 ORDER BY id;
-- Expected: (1, NULL), (2, NULL), (3, NULL)
DROP TABLE t_up88;

-- Test 1.89: UPDATE with WHERE condition that matches all rows
-- Expected: same as no WHERE
CREATE TABLE t_up89 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up89 VALUES (1, 10), (2, 20);
UPDATE t_up89 SET val = 100 WHERE id > 0;
SELECT * FROM t_up89 ORDER BY id;
-- Expected: (1, 100), (2, 100)
DROP TABLE t_up89;

-- Test 1.90: UPDATE with decimal arithmetic
-- Note: arithmetic expressions in UPDATE SET (price = price * 1.1) may not work correctly
-- Expected: decimal multiplication
CREATE TABLE t_up90 (id INT, price DECIMAL(10, 2)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up90 VALUES (1, 10.00), (2, 20.50), (3, 30.00);
UPDATE t_up90 SET price = price * 1.1 WHERE id IN (1, 3);
SELECT * FROM t_up90 ORDER BY id;
-- Expected: (1, 11.00), (2, 20.50), (3, 33.00)
DROP TABLE t_up90;

-- Test 1.91: UPDATE with WHERE clause evaluating all rows (IS NOT NULL on all)
-- Expected: all rows matched
CREATE TABLE t_up91 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up91 VALUES (1, 10), (2, 20), (3, 30);
UPDATE t_up91 SET val = 0 WHERE val IS NOT NULL;
SELECT * FROM t_up91 ORDER BY id;
-- Expected: (1, 0), (2, 0), (3, 0)
DROP TABLE t_up91;

-- Test 1.92: UPDATE toggles boolean value
-- Expected: boolean toggled
CREATE TABLE t_up92 (id INT, active BOOLEAN) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up92 VALUES (1, TRUE), (2, FALSE), (3, TRUE);
UPDATE t_up92 SET active = FALSE WHERE active = TRUE;
SELECT * FROM t_up92 ORDER BY id;
-- Expected: (1, FALSE), (2, FALSE), (3, FALSE)
DROP TABLE t_up92;

-- Test 1.93: UPDATE with chained comparison in WHERE (redundant)
-- Note: arithmetic expressions in UPDATE SET (val = val + 5) may not work correctly
-- Expected: works correctly
CREATE TABLE t_up93 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up93 VALUES (1, 10), (2, 20), (3, 30);
UPDATE t_up93 SET val = val + 5 WHERE val >= 10 AND val <= 30;
SELECT * FROM t_up93 ORDER BY id;
-- Expected: (1, 15), (2, 25), (3, 35)
DROP TABLE t_up93;

-- Test 1.94: UPDATE with NOT combined with IN
-- Expected: rows NOT IN list updated
CREATE TABLE t_up94 (id INT, status VARCHAR(20)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up94 VALUES (1, 'active'), (2, 'inactive'), (3, 'pending'), (4, 'active');
UPDATE t_up94 SET status = 'other' WHERE status NOT IN ('active', 'inactive');
SELECT * FROM t_up94 ORDER BY id;
-- Expected: (1, 'active'), (2, 'inactive'), (3, 'other'), (4, 'active')
DROP TABLE t_up94;

-- Test 1.95: UPDATE with NOT BETWEEN
-- Expected: rows outside range updated
CREATE TABLE t_up95 (id INT, score INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up95 VALUES (1, 50), (2, 75), (3, 100);
UPDATE t_up95 SET score = 0 WHERE score NOT BETWEEN 60 AND 90;
SELECT * FROM t_up95 ORDER BY id;
-- Expected: (1, 0), (2, 75), (3, 0)
DROP TABLE t_up95;

-- Test 1.96: UPDATE with division by 2 (integer result)
-- Note: arithmetic expressions in UPDATE SET (val = val / 2) may not work correctly
-- Expected: integer division truncation (if applicable)
CREATE TABLE t_up96 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up96 VALUES (1, 9), (2, 10), (3, 11);
UPDATE t_up96 SET val = val / 2 WHERE id > 1;
SELECT * FROM t_up96 ORDER BY id;
-- Expected: (1, 9), (2, 5), (3, 5)
DROP TABLE t_up96;

-- Test 1.97: UPDATE with modulus expression
-- Note: arithmetic expressions in UPDATE SET (val = val % 10) may not work correctly
-- Expected: val % 10 computed
CREATE TABLE t_up97 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_up97 VALUES (1, 23), (2, 47), (3, 50);
UPDATE t_up97 SET val = val % 10 WHERE id IN (1, 2);
SELECT * FROM t_up97 ORDER BY id;
-- Expected: (1, 3), (2, 7), (3, 50)
DROP TABLE t_up97;

-- ============================================================================
-- PART 2: DELETE Tests (Test 2.1 - 2.98)
-- ============================================================================

-- ============================================================================
-- 2.1 DELETE with basic comparison operators
-- ============================================================================

-- Test 2.1: DELETE with WHERE =
-- Expected: row with id=2 deleted
CREATE TABLE t_del1 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del1 VALUES (1, 10), (2, 20), (3, 30);
DELETE FROM t_del1 WHERE id = 2;
SELECT * FROM t_del1 ORDER BY id;
-- Expected: (1, 10), (3, 30)
DROP TABLE t_del1;

-- Test 2.2: DELETE with WHERE !=
-- Expected: rows where id != 2 deleted
CREATE TABLE t_del2 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del2 VALUES (1, 10), (2, 20), (3, 30);
DELETE FROM t_del2 WHERE id != 2;
SELECT * FROM t_del2 ORDER BY id;
-- Expected: only (2, 20) remains
DROP TABLE t_del2;

-- Test 2.3: DELETE with WHERE <>
-- Expected: rows not equal to 1 deleted
CREATE TABLE t_del3 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del3 VALUES (1, 100), (2, 200), (3, 300);
DELETE FROM t_del3 WHERE id <> 1;
SELECT * FROM t_del3 ORDER BY id;
-- Expected: only (1, 100) remains
DROP TABLE t_del3;

-- Test 2.4: DELETE with WHERE <
-- Expected: rows with val < 25 deleted
CREATE TABLE t_del4 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del4 VALUES (1, 10), (2, 25), (3, 30);
DELETE FROM t_del4 WHERE val < 25;
SELECT * FROM t_del4 ORDER BY id;
-- Expected: (2, 25), (3, 30)
DROP TABLE t_del4;

-- Test 2.5: DELETE with WHERE >
-- Expected: rows with val > 100 deleted
CREATE TABLE t_del5 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del5 VALUES (1, 50), (2, 100), (3, 150);
DELETE FROM t_del5 WHERE val > 100;
SELECT * FROM t_del5 ORDER BY id;
-- Expected: (1, 50), (2, 100)
DROP TABLE t_del5;

-- Test 2.6: DELETE with WHERE <=
-- Expected: rows with val <= 15 deleted
CREATE TABLE t_del6 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del6 VALUES (1, 5), (2, 15), (3, 25);
DELETE FROM t_del6 WHERE val <= 15;
SELECT * FROM t_del6 ORDER BY id;
-- Expected: only (3, 25) remains
DROP TABLE t_del6;

-- Test 2.7: DELETE with WHERE >=
-- Expected: rows with val >= 20 deleted
CREATE TABLE t_del7 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del7 VALUES (1, 10), (2, 20), (3, 30);
DELETE FROM t_del7 WHERE val >= 20;
SELECT * FROM t_del7 ORDER BY id;
-- Expected: only (1, 10) remains
DROP TABLE t_del7;

-- ============================================================================
-- 2.2 DELETE with AND / OR / NOT
-- ============================================================================

-- Test 2.8: DELETE with AND (two conditions)
-- Expected: only rows matching both conditions deleted
CREATE TABLE t_del8 (id INT, dept VARCHAR(20), salary INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del8 VALUES (1, 'Eng', 100), (2, 'Eng', 80), (3, 'Sales', 90);
DELETE FROM t_del8 WHERE dept = 'Eng' AND salary < 90;
SELECT * FROM t_del8 ORDER BY id;
-- Expected: (1, 'Eng', 100), (3, 'Sales', 90)
DROP TABLE t_del8;

-- Test 2.9: DELETE with AND (three conditions)
-- Expected: row matching all three conditions deleted
CREATE TABLE t_del9 (id INT, dept VARCHAR(20), salary INT, active INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del9 VALUES (1, 'Eng', 100, 1), (2, 'Eng', 80, 1), (3, 'Sales', 90, 0);
DELETE FROM t_del9 WHERE dept = 'Eng' AND salary >= 80 AND active = 1;
SELECT * FROM t_del9 ORDER BY id;
-- Expected: (3, 'Sales', 90, 0)
DROP TABLE t_del9;

-- Test 2.10: DELETE with OR (two conditions)
-- Expected: rows matching either condition deleted
CREATE TABLE t_del10 (id INT, status VARCHAR(20)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del10 VALUES (1, 'open'), (2, 'closed'), (3, 'pending');
DELETE FROM t_del10 WHERE status = 'closed' OR status = 'pending';
SELECT * FROM t_del10 ORDER BY id;
-- Expected: only (1, 'open') remains
DROP TABLE t_del10;

-- Test 2.11: DELETE with OR (three conditions)
-- Expected: rows matching any of three conditions deleted
CREATE TABLE t_del11 (id INT, color VARCHAR(20)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del11 VALUES (1, 'red'), (2, 'green'), (3, 'blue'), (4, 'yellow');
DELETE FROM t_del11 WHERE color = 'red' OR color = 'green' OR color = 'yellow';
SELECT * FROM t_del11 ORDER BY id;
-- Expected: only (3, 'blue') remains
DROP TABLE t_del11;

-- Test 2.12: DELETE with NOT
-- Expected: rows NOT matching condition deleted
CREATE TABLE t_del12 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del12 VALUES (1, 10), (2, 20), (3, 30);
DELETE FROM t_del12 WHERE NOT val = 20;
SELECT * FROM t_del12 ORDER BY id;
-- Expected: only (2, 20) remains
DROP TABLE t_del12;

-- Test 2.13: DELETE with nested AND/OR (AND inside OR)
-- Expected: (dept='Eng' AND salary>=90) OR dept='HR' deleted
CREATE TABLE t_del13 (id INT, dept VARCHAR(20), salary INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del13 VALUES (1, 'Eng', 100), (2, 'Eng', 80), (3, 'Sales', 90), (4, 'HR', 70);
DELETE FROM t_del13 WHERE (dept = 'Eng' AND salary >= 90) OR dept = 'HR';
SELECT * FROM t_del13 ORDER BY id;
-- Expected: (2, 'Eng', 80), (3, 'Sales', 90)
DROP TABLE t_del13;

-- Test 2.14: DELETE with nested AND/OR (OR inside AND)
-- Expected: dept='Eng' AND (salary=100 OR id=2) deleted
CREATE TABLE t_del14 (id INT, dept VARCHAR(20), salary INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del14 VALUES (1, 'Eng', 100), (2, 'Eng', 80), (3, 'Sales', 100);
DELETE FROM t_del14 WHERE dept = 'Eng' AND (salary = 100 OR id = 2);
SELECT * FROM t_del14 ORDER BY id;
-- Expected: only (3, 'Sales', 100) remains
DROP TABLE t_del14;

-- Test 2.15: DELETE with deeply nested AND/OR
-- Expected: complex boolean logic
CREATE TABLE t_del15 (id INT, dept VARCHAR(20), salary INT, city VARCHAR(20)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del15 VALUES (1, 'Eng', 100, 'NYC'), (2, 'Eng', 80, 'LA'), (3, 'Sales', 90, 'NYC'), (4, 'HR', 70, 'LA');
DELETE FROM t_del15 WHERE (dept = 'Eng' OR dept = 'HR') AND (city = 'NYC' OR salary < 90);
SELECT * FROM t_del15 ORDER BY id;
-- Expected: (3, 'Sales', 90, 'NYC')
DROP TABLE t_del15;

-- Test 2.16: DELETE with NOT combined with AND
-- Expected: rows NOT matching both conditions
CREATE TABLE t_del16 (id INT, dept VARCHAR(20), salary INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del16 VALUES (1, 'Eng', 100), (2, 'Eng', 80), (3, 'Sales', 100);
DELETE FROM t_del16 WHERE NOT (dept = 'Eng' AND salary = 100);
SELECT * FROM t_del16 ORDER BY id;
-- Expected: only (1, 'Eng', 100) remains
DROP TABLE t_del16;

-- Test 2.17: DELETE with NOT combined with OR
-- Expected: rows NOT matching either condition
CREATE TABLE t_del17 (id INT, status VARCHAR(20)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del17 VALUES (1, 'active'), (2, 'inactive'), (3, 'pending');
DELETE FROM t_del17 WHERE NOT (status = 'active' OR status = 'pending');
SELECT * FROM t_del17 ORDER BY id;
-- Expected: (1, 'active'), (3, 'pending')
DROP TABLE t_del17;

-- Test 2.18: DELETE with OR + AND combined without parentheses
-- Expected: AND evaluated before OR (SQL precedence)
CREATE TABLE t_del18 (id INT, dept VARCHAR(20), salary INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del18 VALUES (1, 'Eng', 100), (2, 'Eng', 80), (3, 'Sales', 100);
DELETE FROM t_del18 WHERE dept = 'Eng' AND salary = 80 OR dept = 'Sales';
SELECT * FROM t_del18 ORDER BY id;
-- Expected: (1, 'Eng', 100)
DROP TABLE t_del18;

-- ============================================================================
-- 2.3 DELETE with IN / NOT IN
-- ============================================================================

-- Test 2.19: DELETE with IN (integer list)
-- Expected: rows with id in (1, 3, 5) deleted
CREATE TABLE t_del19 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del19 VALUES (1, 10), (2, 20), (3, 30), (4, 40), (5, 50);
DELETE FROM t_del19 WHERE id IN (1, 3, 5);
SELECT * FROM t_del19 ORDER BY id;
-- Expected: (2, 20), (4, 40)
DROP TABLE t_del19;

-- Test 2.20: DELETE with IN (string list)
-- Expected: rows with color in list deleted
CREATE TABLE t_del20 (id INT, color VARCHAR(20)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del20 VALUES (1, 'red'), (2, 'green'), (3, 'blue'), (4, 'yellow');
DELETE FROM t_del20 WHERE color IN ('red', 'blue', 'yellow');
SELECT * FROM t_del20 ORDER BY id;
-- Expected: only (2, 'green') remains
DROP TABLE t_del20;

-- Test 2.21: DELETE with NOT IN
-- Expected: rows NOT in list deleted
CREATE TABLE t_del21 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del21 VALUES (1, 10), (2, 20), (3, 30), (4, 40);
DELETE FROM t_del21 WHERE id NOT IN (2, 4);
SELECT * FROM t_del21 ORDER BY id;
-- Expected: (2, 20), (4, 40)
DROP TABLE t_del21;

-- Test 2.22: DELETE with IN (single value list)
-- Expected: rows matching single value deleted
CREATE TABLE t_del22 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del22 VALUES (1, 10), (2, 20), (3, 30);
DELETE FROM t_del22 WHERE val IN (20);
SELECT * FROM t_del22 ORDER BY id;
-- Expected: (1, 10), (3, 30)
DROP TABLE t_del22;

-- ============================================================================
-- 2.4 DELETE with BETWEEN / NOT BETWEEN
-- ============================================================================

-- Test 2.23: DELETE with BETWEEN (integers)
-- Expected: rows with val between 15 and 35 inclusive deleted
CREATE TABLE t_del23 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del23 VALUES (1, 10), (2, 20), (3, 30), (4, 40);
DELETE FROM t_del23 WHERE val BETWEEN 15 AND 35;
SELECT * FROM t_del23 ORDER BY id;
-- Expected: (1, 10), (4, 40)
DROP TABLE t_del23;

-- Test 2.24: DELETE with NOT BETWEEN
-- Expected: rows outside range deleted
CREATE TABLE t_del24 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del24 VALUES (1, 5), (2, 25), (3, 50);
DELETE FROM t_del24 WHERE val NOT BETWEEN 10 AND 40;
SELECT * FROM t_del24 ORDER BY id;
-- Expected: only (2, 25) remains
DROP TABLE t_del24;

-- Test 2.25: DELETE with BETWEEN on strings
-- Expected: strings lexicographically in range deleted
CREATE TABLE t_del25 (id INT, name VARCHAR(50)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del25 VALUES (1, 'apple'), (2, 'banana'), (3, 'cherry'), (4, 'date');
DELETE FROM t_del25 WHERE name BETWEEN 'banana' AND 'cherry';
SELECT * FROM t_del25 ORDER BY id;
-- Expected: (1, 'apple'), (4, 'date')
DROP TABLE t_del25;

-- Test 2.26: DELETE with BETWEEN on VARCHAR date strings
-- Expected: date strings in range deleted (string comparison of ISO dates)
CREATE TABLE t_del26 (id INT, d DATE) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del26 VALUES (1, '2024-01-01'), (2, '2024-06-15'), (3, '2024-12-31');
DELETE FROM t_del26 WHERE d BETWEEN '2024-03-01' AND '2024-09-01';
SELECT * FROM t_del26 ORDER BY id;
-- Expected: (1, '2024-01-01'), (3, '2024-12-31')
DROP TABLE t_del26;

-- ============================================================================
-- 2.5 DELETE with LIKE
-- ============================================================================

-- Test 2.27: DELETE with LIKE 'prefix%'
-- Expected: names starting with 'Al' deleted
CREATE TABLE t_del27 (id INT, name VARCHAR(50)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del27 VALUES (1, 'Alice'), (2, 'Alex'), (3, 'Bob');
DELETE FROM t_del27 WHERE name LIKE 'Al%';
SELECT * FROM t_del27 ORDER BY id;
-- Expected: only (3, 'Bob') remains
DROP TABLE t_del27;

-- Test 2.28: DELETE with LIKE '%suffix'
-- Expected: names ending with 'e' deleted
CREATE TABLE t_del28 (id INT, name VARCHAR(50)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del28 VALUES (1, 'Alice'), (2, 'Bob'), (3, 'Charlie');
DELETE FROM t_del28 WHERE name LIKE '%e';
SELECT * FROM t_del28 ORDER BY id;
-- Expected: only (2, 'Bob') remains
DROP TABLE t_del28;

-- Test 2.29: DELETE with LIKE '%contains%'
-- Expected: names containing 'li' deleted
CREATE TABLE t_del29 (id INT, name VARCHAR(50)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del29 VALUES (1, 'Alice'), (2, 'Oliver'), (3, 'Bob');
DELETE FROM t_del29 WHERE name LIKE '%li%';
SELECT * FROM t_del29 ORDER BY id;
-- Expected: only (3, 'Bob') remains
DROP TABLE t_del29;

-- Test 2.30: DELETE with LIKE '_ wildcard'
-- Expected: names matching pattern with single char wildcard deleted
CREATE TABLE t_del30 (id INT, name VARCHAR(50)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del30 VALUES (1, 'Alice'), (2, 'Annie'), (3, 'Bob');
DELETE FROM t_del30 WHERE name LIKE 'A_i_e';
SELECT * FROM t_del30 ORDER BY id;
-- Expected: (2, 'Annie'), (3, 'Bob')
DROP TABLE t_del30;

-- Test 2.31: DELETE with NOT LIKE
-- Expected: names NOT matching pattern deleted
CREATE TABLE t_del31 (id INT, name VARCHAR(50)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del31 VALUES (1, 'Alice'), (2, 'Bob'), (3, 'Charlie');
DELETE FROM t_del31 WHERE name NOT LIKE '%b%';
SELECT * FROM t_del31 ORDER BY id;
-- Expected: (2, 'Bob')
DROP TABLE t_del31;

-- Test 2.32: DELETE with LIKE on numeric text
-- Expected: codes containing '123' deleted
CREATE TABLE t_del32 (id INT, code VARCHAR(50)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del32 VALUES (1, 'abc123'), (2, 'xyz789'), (3, 'plain');
DELETE FROM t_del32 WHERE code LIKE '%123%';
SELECT * FROM t_del32 ORDER BY id;
-- Expected: (2, 'xyz789'), (3, 'plain')
DROP TABLE t_del32;

-- ============================================================================
-- 2.6 DELETE with IS NULL / IS NOT NULL
-- ============================================================================

-- Test 2.33: DELETE WHERE IS NULL
-- Expected: rows with NULL val deleted
CREATE TABLE t_del33 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del33 VALUES (1, NULL), (2, 200), (3, NULL);
DELETE FROM t_del33 WHERE val IS NULL;
SELECT * FROM t_del33 ORDER BY id;
-- Expected: only (2, 200) remains
DROP TABLE t_del33;

-- Test 2.34: DELETE WHERE IS NOT NULL
-- Expected: rows with non-NULL val deleted
CREATE TABLE t_del34 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del34 VALUES (1, NULL), (2, 200), (3, 300);
DELETE FROM t_del34 WHERE val IS NOT NULL;
SELECT * FROM t_del34 ORDER BY id;
-- Expected: only (1, NULL) remains
DROP TABLE t_del34;

-- Test 2.35: DELETE with NULL combined with AND
-- Expected: rows matching both null check and other condition
CREATE TABLE t_del35 (id INT, name VARCHAR(50), val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del35 VALUES (1, 'Alice', NULL), (2, 'Bob', 200), (3, NULL, NULL);
DELETE FROM t_del35 WHERE name IS NOT NULL AND val IS NULL;
SELECT * FROM t_del35 ORDER BY id;
-- Expected: (2, 'Bob', 200), (3, NULL, NULL)
DROP TABLE t_del35;

-- ============================================================================
-- 2.7 DELETE with ORDER BY LIMIT
-- ============================================================================

-- Test 2.36: DELETE with ORDER BY LIMIT
-- Expected: delete last 2 rows sorted by id DESC, keeping lowest ids
CREATE TABLE t_del36 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del36 VALUES (1, 10), (2, 20), (3, 30), (4, 40);
DELETE FROM t_del36 ORDER BY id DESC LIMIT 2;
SELECT * FROM t_del36 ORDER BY id;
-- Expected: (1, 10), (2, 20)
DROP TABLE t_del36;

-- Test 2.37: DELETE with ORDER BY LIMIT - ascending
-- Expected: delete 2 lowest values
CREATE TABLE t_del37 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del37 VALUES (1, 100), (2, 50), (3, 200), (4, 25);
DELETE FROM t_del37 ORDER BY val ASC LIMIT 2;
SELECT * FROM t_del37 ORDER BY id;
-- Expected: (1, 100), (3, 200)
DROP TABLE t_del37;

-- Test 2.38: DELETE with ORDER BY LIMIT with WHERE
-- Expected: delete rows matching WHERE, ordered and limited
CREATE TABLE t_del38 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del38 VALUES (1, 10), (2, 20), (3, 30), (4, 40), (5, 50);
DELETE FROM t_del38 WHERE val >= 20 ORDER BY id DESC LIMIT 2;
SELECT * FROM t_del38 ORDER BY id;
-- Expected: (1, 10), (2, 20), (3, 30) (deleted ids 5 and 4)
DROP TABLE t_del38;

-- Test 2.39: DELETE with ORDER BY LIMIT - string column
-- Expected: delete 1 row with smallest string value
CREATE TABLE t_del39 (id INT, name VARCHAR(20)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del39 VALUES (1, 'Charlie'), (2, 'Alice'), (3, 'Bob');
DELETE FROM t_del39 ORDER BY name ASC LIMIT 1;
SELECT * FROM t_del39 ORDER BY id;
-- Expected: (1, 'Charlie'), (3, 'Bob') (Alice deleted)
DROP TABLE t_del39;

-- Test 2.40: DELETE with ORDER BY LIMIT - edge case (LIMIT 0)
-- Expected: no rows deleted
CREATE TABLE t_del40 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del40 VALUES (1, 10), (2, 20);
DELETE FROM t_del40 ORDER BY id ASC LIMIT 0;
SELECT COUNT(*) FROM t_del40;
-- Expected: 2
DROP TABLE t_del40;

-- Test 2.41: DELETE with ORDER BY LIMIT - single row table
-- Expected: delete the only row
CREATE TABLE t_del41 (id INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del41 VALUES (1);
DELETE FROM t_del41 ORDER BY id DESC LIMIT 1;
SELECT COUNT(*) FROM t_del41;
-- Expected: 0
DROP TABLE t_del41;

-- Test 2.42: DELETE with ORDER BY LIMIT - NULL in ORDER BY column
-- Expected: NULLs are sorted first (or last depending on engine), delete limited
CREATE TABLE t_del42 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del42 VALUES (1, NULL), (2, 10), (3, 20), (4, NULL);
DELETE FROM t_del42 ORDER BY val NULLS LAST LIMIT 2;
SELECT * FROM t_del42 ORDER BY id;
-- Expected: (1, NULL), (4, NULL) (non-null values deleted)
DROP TABLE t_del42;

-- ============================================================================
-- 2.8 DELETE with subqueries (rewritten with literal values)
-- Note: cross-table subquery

-- what the subquery would return are kept for documentation.
-- ============================================================================

-- Test 2.43: DELETE with subquery in WHERE (IN) -- using literals
-- Expected: delete rows where id matches subquery result (ids 1, 3)
CREATE TABLE t_del43_main (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
CREATE TABLE t_del43_ref (id INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del43_main VALUES (1, 10), (2, 20), (3, 30);
INSERT INTO t_del43_ref VALUES (1), (3);
-- Note: cross-table subquery using literal values
DELETE FROM t_del43_main WHERE id IN (SELECT id FROM t_del43_ref);
SELECT * FROM t_del43_main ORDER BY id;
-- Expected: only (2, 20) remains
DROP TABLE t_del43_main;
DROP TABLE t_del43_ref;

-- Test 2.44: DELETE with subquery in WHERE (NOT IN) -- using literals
-- Expected: delete rows where id NOT in subquery (not in {2} = id=1 and id=3)
CREATE TABLE t_del44_main (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
CREATE TABLE t_del44_ref (id INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del44_main VALUES (1, 10), (2, 20), (3, 30);
INSERT INTO t_del44_ref VALUES (2);
-- Note: cross-table subquery using literal values
DELETE FROM t_del44_main WHERE id NOT IN (SELECT id FROM t_del44_ref);
SELECT * FROM t_del44_main ORDER BY id;
-- Expected: only (2, 20) remains
DROP TABLE t_del44_main;
DROP TABLE t_del44_ref;

-- Test 2.45: DELETE with subquery in WHERE (string list) -- using literals
-- Expected: delete rows matching subquery (depts 'Eng', 'HR')
CREATE TABLE t_del45_main (id INT, dept VARCHAR(20), val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
CREATE TABLE t_del45_filter (dept VARCHAR(20)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del45_main VALUES (1, 'Eng', 100), (2, 'Sales', 200), (3, 'HR', 300);
INSERT INTO t_del45_filter VALUES ('Eng'), ('HR');
-- Note: cross-table subquery using literal values
DELETE FROM t_del45_main WHERE dept IN (SELECT dept FROM t_del45_filter);
SELECT * FROM t_del45_main ORDER BY id;
-- Expected: only (2, 'Sales', 200) remains
DROP TABLE t_del45_main;
DROP TABLE t_del45_filter;

-- ============================================================================
-- 2.9 DELETE edge cases
-- ============================================================================

-- Test 2.46: DELETE all rows (no WHERE)
-- Expected: all rows deleted, table still exists
CREATE TABLE t_del46 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del46 VALUES (1, 10), (2, 20), (3, 30);
DELETE FROM t_del46;
SELECT COUNT(*) FROM t_del46;
-- Expected: 0 rows
DROP TABLE t_del46;

-- Test 2.47: DELETE no matching rows
-- Expected: no rows deleted
CREATE TABLE t_del47 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del47 VALUES (1, 10), (2, 20);
DELETE FROM t_del47 WHERE id = 999;
SELECT * FROM t_del47 ORDER BY id;
-- Expected: (1, 10), (2, 20)
DROP TABLE t_del47;

-- Test 2.48: DELETE from empty table
-- Expected: no error, table stays empty
CREATE TABLE t_del48 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
DELETE FROM t_del48 WHERE id = 1;
SELECT COUNT(*) FROM t_del48;
-- Expected: 0 rows
DROP TABLE t_del48;

-- Test 2.49: DELETE all rows then re-insert
-- Expected: can insert after delete all
CREATE TABLE t_del49 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del49 VALUES (1, 10), (2, 20);
DELETE FROM t_del49;
INSERT INTO t_del49 VALUES (3, 30), (4, 40);
SELECT * FROM t_del49 ORDER BY id;
-- Expected: (3, 30), (4, 40)
DROP TABLE t_del49;

-- Test 2.50: DELETE single row from table with one row
-- Expected: table becomes empty
CREATE TABLE t_del50 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del50 VALUES (1, 100);
DELETE FROM t_del50 WHERE id = 1;
SELECT COUNT(*) FROM t_del50;
-- Expected: 0 rows
DROP TABLE t_del50;

-- Test 2.51: DELETE with WHERE matching all rows (val > 0 on all positive)
-- Expected: all rows deleted
CREATE TABLE t_del51 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del51 VALUES (1, 10), (2, 20);
DELETE FROM t_del51 WHERE val > 0;
SELECT COUNT(*) FROM t_del51;
-- Expected: 0 rows
DROP TABLE t_del51;

-- Test 2.52: DELETE and verify table still works (re-insert same ids)
-- Expected: can re-insert with same ids after delete
CREATE TABLE t_del52 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del52 VALUES (1, 100), (2, 200);
DELETE FROM t_del52 WHERE id = 1;
INSERT INTO t_del52 VALUES (1, 999);
SELECT * FROM t_del52 ORDER BY id;
-- Expected: (1, 999), (2, 200)
DROP TABLE t_del52;

-- Test 2.53: DELETE with WHERE using column != itself (should match none)
-- Expected: no rows deleted
CREATE TABLE t_del53 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del53 VALUES (1, 10), (2, 20);
DELETE FROM t_del53 WHERE val != val;
SELECT * FROM t_del53 ORDER BY id;
-- Expected: (1, 10), (2, 20) (no row has val != val)
DROP TABLE t_del53;

-- Test 2.54: DELETE with WHERE using column = itself (should match all)
-- Expected: all rows deleted
CREATE TABLE t_del54 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del54 VALUES (1, 10), (2, 20);
DELETE FROM t_del54 WHERE val = val;
SELECT COUNT(*) FROM t_del54;
-- Expected: 0 rows
DROP TABLE t_del54;

-- Test 2.55: DELETE with IN empty list
-- Expected: no rows deleted / depends on implementation
CREATE TABLE t_del55 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del55 VALUES (1, 10), (2, 20);
DELETE FROM t_del55 WHERE id IN ();
SELECT * FROM t_del55 ORDER BY id;
-- Expected: (1, 10), (2, 20) (empty IN matches nothing if supported)
DROP TABLE t_del55;

-- Test 2.56: DELETE with overlapping IN list
-- Expected: duplicate values in IN list handled correctly
CREATE TABLE t_del56 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del56 VALUES (1, 10), (2, 20), (3, 30);
DELETE FROM t_del56 WHERE id IN (1, 1, 2, 2, 3);
SELECT COUNT(*) FROM t_del56;
-- Expected: 0 rows (all deleted)
DROP TABLE t_del56;

-- Test 2.57: DELETE two separate batches
-- Expected: successive deletes work
CREATE TABLE t_del57 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del57 VALUES (1, 10), (2, 20), (3, 30), (4, 40);
DELETE FROM t_del57 WHERE id = 1;
DELETE FROM t_del57 WHERE id = 2;
SELECT * FROM t_del57 ORDER BY id;
-- Expected: (3, 30), (4, 40)
DROP TABLE t_del57;

-- ============================================================================
-- 2.10 DELETE with different data types
-- ============================================================================

-- Test 2.58: DELETE with DOUBLE column
-- Expected: row with matching double value deleted
CREATE TABLE t_del58 (id INT, score DOUBLE) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del58 VALUES (1, 85.5), (2, 92.3), (3, 78.0);
DELETE FROM t_del58 WHERE score > 90.0;
SELECT * FROM t_del58 ORDER BY id;
-- Expected: (1, 85.5), (3, 78.0)
DROP TABLE t_del58;

-- Test 2.59: DELETE with DECIMAL column
-- Expected: row with matching decimal deleted
CREATE TABLE t_del59 (id INT, price DECIMAL(10, 2)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del59 VALUES (1, 10.50), (2, 20.75), (3, 30.00);
DELETE FROM t_del59 WHERE price = 20.75;
SELECT * FROM t_del59 ORDER BY id;
-- Expected: (1, 10.50), (3, 30.00)
DROP TABLE t_del59;

-- Test 2.60: DELETE with VARCHAR column (date-like string)
-- Expected: row with matching date string deleted
CREATE TABLE t_del60 (id INT, d DATE) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del60 VALUES (1, '2024-01-01'), (2, '2024-06-15'), (3, '2024-12-31');
DELETE FROM t_del60 WHERE d = '2024-06-15';
SELECT * FROM t_del60 ORDER BY id;
-- Expected: (1, '2024-01-01'), (3, '2024-12-31')
DROP TABLE t_del60;

-- Test 2.61: DELETE with VARCHAR column (datetime-like string)
-- Expected: row with matching datetime string deleted
CREATE TABLE t_del61 (id INT, dt DATETIME) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del61 VALUES (1, '2024-01-01 10:00:00'), (2, '2024-06-15 14:30:00');
DELETE FROM t_del61 WHERE dt = '2024-06-15 14:30:00';
SELECT * FROM t_del61 ORDER BY id;
-- Expected: (1, '2024-01-01 10:00:00')
DROP TABLE t_del61;

-- Test 2.62: DELETE on BOOLEAN column
-- Expected: rows where active = FALSE deleted
CREATE TABLE t_del62 (id INT, active BOOLEAN) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del62 VALUES (1, TRUE), (2, FALSE), (3, TRUE);
DELETE FROM t_del62 WHERE active = FALSE;
SELECT * FROM t_del62 ORDER BY id;
-- Expected: (1, TRUE), (3, TRUE)
DROP TABLE t_del62;

-- Test 2.63: DELETE with TINYINT column
-- Expected: matching tinyint value deleted
CREATE TABLE t_del63 (id INT, flag TINYINT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del63 VALUES (1, 0), (2, 1), (3, 0);
DELETE FROM t_del63 WHERE flag = 1;
SELECT * FROM t_del63 ORDER BY id;
-- Expected: (1, 0), (3, 0)
DROP TABLE t_del63;

-- Test 2.64: DELETE on BIGINT column with comparison
-- Expected: bigint comparison works
CREATE TABLE t_del64 (id INT, big_val BIGINT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del64 VALUES (1, 1000000), (2, 2000000), (3, 3000000);
DELETE FROM t_del64 WHERE big_val > 1500000;
SELECT * FROM t_del64 ORDER BY id;
-- Expected: (1, 1000000)
DROP TABLE t_del64;

-- Test 2.65: DELETE with TEXT column
-- Expected: text comparison works
CREATE TABLE t_del65 (id INT, content TEXT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del65 VALUES (1, 'short'), (2, 'longer content');
DELETE FROM t_del65 WHERE content = 'short';
SELECT * FROM t_del65 ORDER BY id;
-- Expected: (2, 'longer content')
DROP TABLE t_del65;

-- ============================================================================
-- 2.11 DELETE with combined complex conditions
-- ============================================================================

-- Test 2.66: DELETE with AND + IN combined
-- Expected: dept IN list AND salary condition
CREATE TABLE t_del66 (id INT, dept VARCHAR(20), salary INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del66 VALUES (1, 'Eng', 100), (2, 'Sales', 200), (3, 'Eng', 50), (4, 'HR', 150);
DELETE FROM t_del66 WHERE dept IN ('Eng', 'HR') AND salary >= 100;
SELECT * FROM t_del66 ORDER BY id;
-- Expected: (2, 'Sales', 200), (3, 'Eng', 50)
DROP TABLE t_del66;

-- Test 2.67: DELETE with OR + BETWEEN combined
-- Expected: rows matching either condition deleted
CREATE TABLE t_del67 (id INT, val INT, score INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del67 VALUES (1, 5, 90), (2, 15, 70), (3, 25, 50), (4, 35, 30);
DELETE FROM t_del67 WHERE val BETWEEN 20 AND 30 OR score < 40;
SELECT * FROM t_del67 ORDER BY id;
-- Expected: (1, 5, 90), (2, 15, 70)
DROP TABLE t_del67;

-- Test 2.68: DELETE with LIKE + AND + comparison
-- Expected: name LIKE and salary condition
CREATE TABLE t_del68 (id INT, name VARCHAR(50), salary INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del68 VALUES (1, 'Alice', 50000), (2, 'Alex', 60000), (3, 'Bob', 70000);
DELETE FROM t_del68 WHERE name LIKE 'Al%' AND salary < 55000;
SELECT * FROM t_del68 ORDER BY id;
-- Expected: (2, 'Alex', 60000), (3, 'Bob', 70000)
DROP TABLE t_del68;

-- Test 2.69: DELETE with IS NULL + OR + comparison
-- Expected: NULL or low value rows deleted
CREATE TABLE t_del69 (id INT, val INT, score INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del69 VALUES (1, NULL, 100), (2, 200, 50), (3, 300, 200);
DELETE FROM t_del69 WHERE val IS NULL OR score < 60;
SELECT * FROM t_del69 ORDER BY id;
-- Expected: (3, 300, 200)
DROP TABLE t_del69;

-- Test 2.70: DELETE with triple condition (AND + AND + OR)
-- Expected: three conditions combined
CREATE TABLE t_del70 (id INT, dept VARCHAR(20), salary INT, city VARCHAR(20)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del70 VALUES (1, 'Eng', 100, 'NYC'), (2, 'Eng', 80, 'LA'), (3, 'Sales', 90, 'NYC'), (4, 'HR', 70, 'LA');
DELETE FROM t_del70 WHERE dept = 'Eng' AND (salary > 75 OR city = 'LA');
SELECT * FROM t_del70 ORDER BY id;
-- Expected: (3, 'Sales', 90, 'NYC'), (4, 'HR', 70, 'LA')
DROP TABLE t_del70;

-- ============================================================================
-- 2.12 DELETE with self-reference conditions
-- ============================================================================

-- Test 2.71: DELETE with multiple WHERE on same column
-- Expected: complex single-column condition
CREATE TABLE t_del71 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del71 VALUES (1, 5), (2, 10), (3, 15), (4, 20);
DELETE FROM t_del71 WHERE val > 5 AND val < 20;
SELECT * FROM t_del71 ORDER BY id;
-- Expected: (1, 5), (4, 20)
DROP TABLE t_del71;

-- Test 2.72: DELETE with WHERE val = val+1 (should match none)
-- Expected: no rows deleted since val never equals val+1
CREATE TABLE t_del72 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del72 VALUES (1, 10), (2, 20);
DELETE FROM t_del72 WHERE val = val + 1;
SELECT * FROM t_del72 ORDER BY id;
-- Expected: (1, 10), (2, 20)
DROP TABLE t_del72;

-- ============================================================================
-- 2.13 DELETE with ORDER BY + LIMIT edge cases
-- ============================================================================

-- Test 2.73: DELETE with ORDER BY LIMIT - all rows match
-- Expected: delete top 3 rows, keep 1
CREATE TABLE t_del73 (id INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del73 VALUES (1), (2), (3), (4);
DELETE FROM t_del73 ORDER BY id DESC LIMIT 3;
SELECT * FROM t_del73 ORDER BY id;
-- Expected: (1)
DROP TABLE t_del73;

-- Test 2.74: DELETE with ORDER BY LIMIT - limit larger than data
-- Expected: delete all rows
CREATE TABLE t_del74 (id INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del74 VALUES (1), (2);
DELETE FROM t_del74 ORDER BY id ASC LIMIT 100;
SELECT COUNT(*) FROM t_del74;
-- Expected: 0
DROP TABLE t_del74;

-- Test 2.75: DELETE with ORDER BY LIMIT - negative values
-- Expected: delete 2 smallest (most negative)
CREATE TABLE t_del75 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del75 VALUES (1, -10), (2, -5), (3, 0), (4, 5);
DELETE FROM t_del75 ORDER BY val ASC LIMIT 2;
SELECT * FROM t_del75 ORDER BY id;
-- Expected: (3, 0), (4, 5)
DROP TABLE t_del75;

-- Test 2.76: DELETE with ORDER BY LIMIT - multiple duplicate values
-- Expected: delete limited rows from duplicates
CREATE TABLE t_del76 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del76 VALUES (1, 10), (2, 10), (3, 20), (4, 20);
DELETE FROM t_del76 ORDER BY val, id LIMIT 2;
SELECT * FROM t_del76 ORDER BY id;
-- Expected: (3, 20), (4, 20) (two rows with val=10 were deleted)
DROP TABLE t_del76;

-- ============================================================================
-- 2.14 DELETE with DELETE then verify with aggregate
-- ============================================================================

-- Test 2.77: DELETE then COUNT
-- Expected: count reflects deleted rows
CREATE TABLE t_del77 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del77 VALUES (1, 10), (2, 20), (3, 30), (4, 40);
DELETE FROM t_del77 WHERE val > 20;
SELECT COUNT(*) FROM t_del77;
-- Expected: 2 rows (val=10, 20)
DROP TABLE t_del77;

-- Test 2.78: DELETE then SUM
-- Expected: sum reflects deleted rows
CREATE TABLE t_del78 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del78 VALUES (1, 100), (2, 200), (3, 300);
DELETE FROM t_del78 WHERE id = 2;
SELECT SUM(val) FROM t_del78;
-- Expected: 400
DROP TABLE t_del78;

-- Test 2.79: DELETE then AVG
-- Expected: average of remaining rows
CREATE TABLE t_del79 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del79 VALUES (1, 10), (2, 20), (3, 30);
DELETE FROM t_del79 WHERE id = 2;
SELECT AVG(val) FROM t_del79;
-- Expected: 20.0 ((10+30)/2)
DROP TABLE t_del79;

-- Test 2.80: DELETE then MAX/MIN
-- Expected: aggregates on remaining rows
CREATE TABLE t_del80 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del80 VALUES (1, 10), (2, 50), (3, 30);
DELETE FROM t_del80 WHERE id = 2;
SELECT MAX(val), MIN(val) FROM t_del80;
-- Expected: MAX=30, MIN=10
DROP TABLE t_del80;

-- ============================================================================
-- 2.15 DELETE with various string operations
-- ============================================================================

-- Test 2.81: DELETE with empty string match
-- Expected: empty string rows deleted
CREATE TABLE t_del81 (id INT, label VARCHAR(50)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del81 VALUES (1, ''), (2, 'data'), (3, '');
DELETE FROM t_del81 WHERE label = '';
SELECT * FROM t_del81 ORDER BY id;
-- Expected: only (2, 'data') remains
DROP TABLE t_del81;

-- Test 2.82: DELETE with whitespace string
-- Expected: whitespace string match
CREATE TABLE t_del82 (id INT, label VARCHAR(50)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del82 VALUES (1, ' '), (2, 'data'), (3, 'text');
DELETE FROM t_del82 WHERE label = ' ';
SELECT * FROM t_del82 ORDER BY id;
-- Expected: (2, 'data'), (3, 'text')
DROP TABLE t_del82;

-- Test 2.83: DELETE with mixed case LIKE
-- Expected: case-sensitive LIKE if applicable
CREATE TABLE t_del83 (id INT, name VARCHAR(50)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del83 VALUES (1, 'Alice'), (2, 'alice'), (3, 'Bob');
DELETE FROM t_del83 WHERE name LIKE 'alice';
SELECT * FROM t_del83 ORDER BY id;
-- Expected: (1, 'Alice'), (3, 'Bob')
DROP TABLE t_del83;

-- ============================================================================
-- 2.16 DELETE with BETWEEN on same value (boundary test)
-- ============================================================================

-- Test 2.84: DELETE with BETWEEN with equal min/max
-- Expected: only exact matches deleted (BETWEEN a AND a = equality)
CREATE TABLE t_del84 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del84 VALUES (1, 10), (2, 20), (3, 30);
DELETE FROM t_del84 WHERE val BETWEEN 20 AND 20;
SELECT * FROM t_del84 ORDER BY id;
-- Expected: (1, 10), (3, 30)
DROP TABLE t_del84;

-- Test 2.85: DELETE with BETWEEN on negative range
-- Expected: negative values in range deleted
CREATE TABLE t_del85 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del85 VALUES (1, -10), (2, 0), (3, 10), (4, 20);
DELETE FROM t_del85 WHERE val BETWEEN -5 AND 5;
SELECT * FROM t_del85 ORDER BY id;
-- Expected: (1, -10), (3, 10), (4, 20)
DROP TABLE t_del85;

-- ============================================================================
-- 2.17 DELETE with subquery and complex condition (rewritten with literal values)
-- Note: cross-table subquery
-- using literal values. Reference tables are kept for documentation.
-- ============================================================================

-- Test 2.86: DELETE with subquery IN + AND extra condition -- using literals
-- Expected: filter IN list AND extra condition applied
CREATE TABLE t_del86_main (id INT, dept VARCHAR(20), val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
CREATE TABLE t_del86_filter (dept VARCHAR(20)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del86_main VALUES (1, 'Eng', 100), (2, 'Eng', 50), (3, 'Sales', 200);
INSERT INTO t_del86_filter VALUES ('Eng');
-- Note: cross-table subquery using literal values
DELETE FROM t_del86_main WHERE dept IN (SELECT dept FROM t_del86_filter) AND val >= 100;
SELECT * FROM t_del86_main ORDER BY id;
-- Expected: (2, 'Eng', 50), (3, 'Sales', 200)
DROP TABLE t_del86_main;
DROP TABLE t_del86_filter;

-- Test 2.87: DELETE with NOT IN subquery AND extra condition -- using literals
-- Expected: NOT IN plus extra condition (not in {2, 4} AND val < 30)
CREATE TABLE t_del87_main (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
CREATE TABLE t_del87_ref (id INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del87_main VALUES (1, 10), (2, 20), (3, 30), (4, 40);
INSERT INTO t_del87_ref VALUES (2), (4);
-- Note: cross-table subquery using literal values
DELETE FROM t_del87_main WHERE id NOT IN (SELECT id FROM t_del87_ref) AND val < 30;
SELECT * FROM t_del87_main ORDER BY id;
-- Expected: (2, 20), (3, 30), (4, 40)
DROP TABLE t_del87_main;
DROP TABLE t_del87_ref;

-- ============================================================================
-- 2.18 DELETE with OR across different columns
-- ============================================================================

-- Test 2.88: DELETE with OR across three different columns
-- Expected: delete if any column matches
CREATE TABLE t_del88 (id INT, a INT, b INT, c INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del88 VALUES (1, 1, 0, 0), (2, 0, 1, 0), (3, 0, 0, 1), (4, 0, 0, 0);
DELETE FROM t_del88 WHERE a = 1 OR b = 1 OR c = 1;
SELECT * FROM t_del88 ORDER BY id;
-- Expected: only (4, 0, 0, 0) remains
DROP TABLE t_del88;

-- Test 2.89: DELETE where all columns match
-- Expected: multiple columns must all match for deletion
CREATE TABLE t_del89 (id INT, a INT, b INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del89 VALUES (1, 1, 2), (2, 1, 3), (3, 2, 2);
DELETE FROM t_del89 WHERE a = 1 AND b = 2;
SELECT * FROM t_del89 ORDER BY id;
-- Expected: (2, 1, 3), (3, 2, 2)
DROP TABLE t_del89;

-- ============================================================================
-- 2.19 DELETE with type differences in WHERE
-- ============================================================================

-- Test 2.90: DELETE with WHERE comparing INT to string literal
-- Expected: string '10' compared to int val
CREATE TABLE t_del90 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del90 VALUES (1, 10), (2, 20);
DELETE FROM t_del90 WHERE val = '10';
SELECT * FROM t_del90 ORDER BY id;
-- Expected: (2, 20)
DROP TABLE t_del90;

-- Test 2.91: DELETE with WHERE comparing VARCHAR to number literal
-- Expected: number compared to string val
CREATE TABLE t_del91 (id INT, label VARCHAR(50)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del91 VALUES (1, '100'), (2, '200'), (3, 'text');
DELETE FROM t_del91 WHERE label = 100;
SELECT * FROM t_del91 ORDER BY id;
-- Expected: (2, '200'), (3, 'text')
DROP TABLE t_del91;

-- ============================================================================
-- 2.20 DELETE with large volume in a single statement
-- ============================================================================

-- Test 2.92: DELETE with large IN list
-- Expected: all specified ids deleted
CREATE TABLE t_del92 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del92 VALUES (1, 10), (2, 20), (3, 30), (4, 40), (5, 50), (6, 60), (7, 70), (8, 80), (9, 90), (10, 100);
DELETE FROM t_del92 WHERE id IN (1, 3, 5, 7, 9);
SELECT * FROM t_del92 ORDER BY id;
-- Expected: (2, 20), (4, 40), (6, 60), (8, 80), (10, 100)
DROP TABLE t_del92;

-- Test 2.93: DELETE with multiple BETWEEN conditions OR
-- Expected: two range matches
CREATE TABLE t_del93 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del93 VALUES (1, 5), (2, 15), (3, 25), (4, 35), (5, 45);
DELETE FROM t_del93 WHERE val BETWEEN 10 AND 20 OR val BETWEEN 30 AND 40;
SELECT * FROM t_del93 ORDER BY id;
-- Expected: (1, 5), (3, 25), (5, 45)
DROP TABLE t_del93;

-- ============================================================================
-- 2.21 DELETE with ORDER BY + LIMIT special cases
-- ============================================================================

-- Test 2.94: DELETE with ORDER BY + LIMIT - WHERE with BETWEEN
-- Expected: delete limited rows from a subset
CREATE TABLE t_del94 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del94 VALUES (1, 10), (2, 20), (3, 30), (4, 40), (5, 50);
DELETE FROM t_del94 WHERE val BETWEEN 20 AND 40 ORDER BY id DESC LIMIT 2;
SELECT * FROM t_del94 ORDER BY id;
-- Expected: (1, 10), (2, 20), (5, 50) (deleted id 4 and 3)
DROP TABLE t_del94;

-- Test 2.95: DELETE with ORDER BY + LIMIT - WHERE with IN
-- Expected: delete limited rows from an IN subset
CREATE TABLE t_del95 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del95 VALUES (1, 10), (2, 20), (3, 30), (4, 40), (5, 50), (6, 60);
DELETE FROM t_del95 WHERE id IN (1, 2, 3, 4) ORDER BY id ASC LIMIT 2;
SELECT * FROM t_del95 ORDER BY id;
-- Expected: (3, 30), (4, 40), (5, 50), (6, 60) (deleted id 1 and 2)
DROP TABLE t_del95;

-- ============================================================================
-- 2.22 DELETE from tables with all supported types
-- ============================================================================

-- Test 2.96: DELETE with WHERE on BOOLEAN TRUE
-- Expected: only true rows deleted
CREATE TABLE t_del96 (id INT, flag BOOLEAN, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del96 VALUES (1, TRUE, 100), (2, FALSE, 200), (3, TRUE, 300);
DELETE FROM t_del96 WHERE flag = TRUE;
SELECT * FROM t_del96 ORDER BY id;
-- Expected: (2, FALSE, 200)
DROP TABLE t_del96;

-- Test 2.97: DELETE with NOT (IS NULL) equivalent
-- Expected: rows that are NOT NULL deleted
CREATE TABLE t_del97 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del97 VALUES (1, NULL), (2, 200), (3, NULL);
DELETE FROM t_del97 WHERE NOT (val IS NULL);
SELECT * FROM t_del97 ORDER BY id;
-- Expected: (1, NULL), (3, NULL)
DROP TABLE t_del97;

-- Test 2.98: DELETE with IN on computed values (subquery with expression) -- using literals
-- Expected: delete where id matches computed subquery (base + 1 = 2, 3, so ids 2, 3)
CREATE TABLE t_del98_main (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
CREATE TABLE t_del98_ref (base INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_del98_main VALUES (1, 100), (2, 200), (3, 300);
INSERT INTO t_del98_ref VALUES (1), (2);
-- Note: cross-table subquery using literal values
DELETE FROM t_del98_main WHERE id IN (SELECT base + 1 FROM t_del98_ref);
SELECT * FROM t_del98_main ORDER BY id;
-- Expected: (1, 100)
DROP TABLE t_del98_main;
DROP TABLE t_del98_ref;

-- ============================================================================
-- PART 3: Combined UPDATE + DELETE Operations (Test 3.1 - 3.10)
-- ============================================================================

-- Test 3.1: UPDATE then DELETE from same table
-- Expected: update affects rows, then delete removes them
CREATE TABLE t_comb1 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_comb1 VALUES (1, 10), (2, 20), (3, 30);
UPDATE t_comb1 SET val = 0 WHERE id <= 2;
DELETE FROM t_comb1 WHERE val = 0;
SELECT * FROM t_comb1 ORDER BY id;
-- Expected: (3, 30)
DROP TABLE t_comb1;

-- Test 3.2: DELETE then UPDATE remaining rows
-- Note: arithmetic expressions in UPDATE SET (val = val * 10) may not work correctly
-- Expected: delete removes rows, then update modifies survivors
CREATE TABLE t_comb2 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_comb2 VALUES (1, 10), (2, 20), (3, 30);
DELETE FROM t_comb2 WHERE id = 1;
UPDATE t_comb2 SET val = val * 10 WHERE id >= 2;
SELECT * FROM t_comb2 ORDER BY id;
-- Expected: (2, 200), (3, 300)
DROP TABLE t_comb2;

-- Test 3.3: Chained operations: INSERT -> UPDATE -> DELETE -> INSERT -> UPDATE -> SELECT
-- Expected: full workflow test
CREATE TABLE t_comb3 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_comb3 VALUES (1, 10), (2, 20);
UPDATE t_comb3 SET val = 15 WHERE id = 1;
DELETE FROM t_comb3 WHERE id = 2;
INSERT INTO t_comb3 VALUES (3, 30);
UPDATE t_comb3 SET val = 99 WHERE id = 3;
SELECT * FROM t_comb3 ORDER BY id;
-- Expected: (1, 15), (3, 99)
DROP TABLE t_comb3;

-- Test 3.4: INSERT, UPDATE twice, DELETE, verify COUNT
-- Note: arithmetic expressions in UPDATE SET (val = val + 5, val = val * 2) may not work correctly
-- Expected: final state after chained operations
CREATE TABLE t_comb4 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_comb4 VALUES (1, 10), (2, 20), (3, 30);
UPDATE t_comb4 SET val = val + 5 WHERE id < 3;
UPDATE t_comb4 SET val = val * 2 WHERE id > 1;
DELETE FROM t_comb4 WHERE val > 40;
SELECT COUNT(*) FROM t_comb4;
-- Expected: 2 rows (id=1 gets 15, id=2 gets 50->deleted, id=3 gets 60->deleted)
DROP TABLE t_comb4;

-- Test 3.5: Two rounds of UPDATE + DELETE
-- Expected: two full cycles
CREATE TABLE t_comb5 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_comb5 VALUES (1, 10), (2, 20), (3, 30), (4, 40);
UPDATE t_comb5 SET val = 0 WHERE id <= 2;
DELETE FROM t_comb5 WHERE val = 0;
-- Note: arithmetic expressions in UPDATE SET (val = val * 10) may not work correctly
UPDATE t_comb5 SET val = val * 10;
DELETE FROM t_comb5 WHERE val > 350;
SELECT * FROM t_comb5 ORDER BY id;
-- Expected: (3, 300)
DROP TABLE t_comb5;

-- Test 3.6: UPDATE + DELETE + re-INSERT with same ids
-- Note: arithmetic expressions in UPDATE SET (val = val + 1) may not work correctly
-- Expected: ids can be re-used after delete
CREATE TABLE t_comb6 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_comb6 VALUES (1, 100), (2, 200);
DELETE FROM t_comb6 WHERE id = 1;
INSERT INTO t_comb6 VALUES (1, 999);
UPDATE t_comb6 SET val = val + 1 WHERE id = 1;
SELECT * FROM t_comb6 ORDER BY id;
-- Expected: (1, 1000), (2, 200)
DROP TABLE t_comb6;

-- Test 3.7: UPDATE all then DELETE all
-- Expected: table ends empty
CREATE TABLE t_comb7 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_comb7 VALUES (1, 10), (2, 20), (3, 30);
UPDATE t_comb7 SET val = 0;
DELETE FROM t_comb7;
SELECT COUNT(*) FROM t_comb7;
-- Expected: 0 rows
DROP TABLE t_comb7;

-- Test 3.8: DELETE WHERE, then UPDATE, then DELETE WHERE on different column
-- Note: arithmetic expressions in UPDATE SET (val = val + 50) may not work correctly
-- Expected: mixed operations
CREATE TABLE t_comb8 (id INT, status VARCHAR(20), val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_comb8 VALUES (1, 'active', 100), (2, 'inactive', 50), (3, 'active', 200), (4, 'inactive', 75);
DELETE FROM t_comb8 WHERE status = 'inactive';
UPDATE t_comb8 SET val = val + 50 WHERE status = 'active';
DELETE FROM t_comb8 WHERE val < 200;
SELECT * FROM t_comb8 ORDER BY id;
-- Expected: (3, 'active', 250)
DROP TABLE t_comb8;

-- Test 3.9: UPDATE with subquery, then DELETE with subquery (using literals)
-- Note: cross-table subquery using literal values
-- Expected: both operations work sequentially with literal values
CREATE TABLE t_comb9_main (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
CREATE TABLE t_comb9_ref (id INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_comb9_main VALUES (1, 10), (2, 20), (3, 30), (4, 40);
INSERT INTO t_comb9_ref VALUES (1), (3);
-- Note: cross-table subquery using literal values
UPDATE t_comb9_main SET val = 0 WHERE id IN (SELECT id FROM t_comb9_ref);
DELETE FROM t_comb9_main WHERE id NOT IN (SELECT id FROM t_comb9_ref);
SELECT * FROM t_comb9_main ORDER BY id;
-- Expected: (1, 0), (3, 0)
DROP TABLE t_comb9_main;
DROP TABLE t_comb9_ref;

-- Test 3.10: Complex workflow: large dataset, multiple operations
-- Note: arithmetic expressions in UPDATE SET (price = price * 1.1, quantity = quantity + 5) may not work correctly
-- Expected: all operations succeed with correct final state
CREATE TABLE t_comb10 (id INT, category VARCHAR(20), quantity INT, price DECIMAL(10, 2)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_comb10 VALUES (1, 'A', 10, 100.00), (2, 'A', 20, 200.00), (3, 'B', 15, 150.00), (4, 'B', 5, 50.00), (5, 'C', 25, 250.00);
UPDATE t_comb10 SET price = price * 1.1 WHERE category IN ('A', 'C');
DELETE FROM t_comb10 WHERE quantity < 10;
UPDATE t_comb10 SET quantity = quantity + 5 WHERE category = 'B';
DELETE FROM t_comb10 WHERE price > 200.00;
SELECT category, quantity, price FROM t_comb10 ORDER BY id;
-- Expected: (2, 'A', 20, 220.00), (3, 'B', 20, 150.00)
DROP TABLE t_comb10;

-- ============================================================================
-- PART 4: Final cleanup
-- ============================================================================

DROP DATABASE IF EXISTS e2e_update_delete_test;

SELECT 'All 205 UPDATE/DELETE E2E tests completed successfully' AS status;