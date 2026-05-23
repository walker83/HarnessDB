-- ============================================================================
-- E2E SELECT Queries Test Script
-- ============================================================================
-- Covers: basic SELECT, aliases, DISTINCT, WHERE (all operators),
-- ORDER BY (variations, expressions, NULL handling), LIMIT/OFFSET (including
-- edge cases), CASE WHEN (simple + searched), CAST, COALESCE, arithmetic &
-- string expressions, subqueries (IN, scalar, EXISTS, correlated, derived
-- tables), and complex combinations of multiple features.
-- ============================================================================

DROP DATABASE IF EXISTS e2e_select_test;
CREATE DATABASE e2e_select_test;
USE e2e_select_test;

-- ============================================================================
-- Section 1: Basic SELECT and Column Selection
-- ============================================================================

-- Test 1.1: SELECT * from a table with multiple rows
CREATE TABLE t_basic (
    id INT,
    name VARCHAR(50),
    age INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_basic VALUES (1, 'Alice', 30), (2, 'Bob', 25), (3, 'Charlie', 35);
SELECT * FROM t_basic ORDER BY id;
-- Expected: 3 rows: (1, Alice, 30), (2, Bob, 25), (3, Charlie, 35)

-- Test 1.2: SELECT specific columns
SELECT id, name FROM t_basic ORDER BY id;
-- Expected: (1, Alice), (2, Bob), (3, Charlie)

-- Test 1.3: SELECT single column
SELECT name FROM t_basic ORDER BY id;
-- Expected: 'Alice', 'Bob', 'Charlie'

-- Test 1.4: SELECT with reordered columns
SELECT name, id, age FROM t_basic ORDER BY id;
-- Expected: (Alice, 1, 30), (Bob, 2, 25), (Charlie, 3, 35)

-- Test 1.5: SELECT constant literal
SELECT 1;
-- Expected: 1

-- Test 1.6: SELECT multiple constants
SELECT 1, 'hello', 3.14;
-- Expected: (1, hello, 3.14)

-- Test 1.7: SELECT with computed constant expression
SELECT 1 + 2, 3 * 4, 10 / 2, 10 - 3;
-- Expected: (3, 12, 5, 7)

-- Test 1.8: SELECT with string expression
SELECT 'Hello' || ' ' || 'World';
-- Expected: 'Hello World'

-- Test 1.9: SELECT * with no WHERE
SELECT * FROM t_basic ORDER BY id;
-- Expected: 3 rows

-- Test 1.10: SELECT * from empty table (edge case)
CREATE TABLE t_empty (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
SELECT * FROM t_empty;
-- Expected: empty result set

-- Test 1.11: SELECT with BOOLEAN column
CREATE TABLE t_bool (id INT, active BOOLEAN) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_bool VALUES (1, TRUE), (2, FALSE), (3, TRUE);
SELECT * FROM t_bool ORDER BY id;
-- Expected: (1, true), (2, false), (3, true)

-- Test 1.12: SELECT with VARCHAR column (variable length)
SELECT id, name FROM t_basic ORDER BY id;
-- Expected: same as Test 1.2

-- Test 1.13: SELECT with CHAR column
CREATE TABLE t_char (id INT, code CHAR(3)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_char VALUES (1, 'ABC'), (2, 'DEF');
SELECT * FROM t_char ORDER BY id;
-- Expected: (1, ABC), (2, DEF)

-- Test 1.14: SELECT with DATE column
CREATE TABLE t_date (id INT, dt DATE) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_date VALUES (1, '2024-01-15'), (2, '2024-06-30'), (3, '2025-12-25');
SELECT * FROM t_date ORDER BY id;
-- Expected: 3 rows with dates

-- Test 1.15: SELECT with DATETIME column
CREATE TABLE t_datetime (id INT, ts DATETIME) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_datetime VALUES (1, '2024-01-15 10:30:00'), (2, '2024-06-30 23:59:59');
SELECT * FROM t_datetime ORDER BY id;
-- Expected: 2 rows with datetimes

-- Test 1.16: SELECT with DECIMAL column
CREATE TABLE t_decimal (id INT, price DECIMAL(10, 2)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_decimal VALUES (1, 123.45), (2, 0.99), (3, 999999.99);
SELECT * FROM t_decimal ORDER BY id;
-- Expected: (1, 123.45), (2, 0.99), (3, 999999.99)

-- Test 1.17: SELECT with FLOAT column
CREATE TABLE t_float (id INT, val FLOAT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_float VALUES (1, 3.14), (2, 2.718);
SELECT * FROM t_float ORDER BY id;
-- Expected: (1, 3.14), (2, 2.718)

-- Test 1.18: SELECT with DOUBLE column
CREATE TABLE t_double (id INT, val DOUBLE) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_double VALUES (1, 1.23456789), (2, 9.87654321);
SELECT * FROM t_double ORDER BY id;

-- Test 1.19: SELECT with TINYINT column
CREATE TABLE t_tiny (id INT, val TINYINT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_tiny VALUES (1, 127), (2, -128);
SELECT * FROM t_tiny ORDER BY id;
-- Expected: (1, 127), (2, -128)

-- Test 1.20: SELECT with SMALLINT column
CREATE TABLE t_small (id INT, val SMALLINT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_small VALUES (1, 32767), (2, -32768);
SELECT * FROM t_small ORDER BY id;

-- Test 1.21: SELECT with BIGINT column
CREATE TABLE t_big (id INT, val BIGINT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_big VALUES (1, 9223372036854775807), (2, -9223372036854775808);
SELECT * FROM t_big ORDER BY id;

-- Test 1.22: SELECT with LARGEINT column
CREATE TABLE t_large (id INT, val BIGINT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_large VALUES (1, 12345678901234567890), (2, -9876543210987654321);
SELECT * FROM t_large ORDER BY id;

-- Test 1.23: SELECT with TEXT column
CREATE TABLE t_text (id INT, content TEXT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_text VALUES (1, 'This is a long text value'), (2, 'Short text');
SELECT * FROM t_text ORDER BY id;

-- Test 1.24: SELECT one row by constant filter
SELECT * FROM t_basic WHERE id = 2;
-- Expected: (2, Bob, 25)

-- Test 1.25: SELECT with no matching rows
SELECT * FROM t_basic WHERE id = 999;
-- Expected: empty result set

DROP TABLE t_basic;
DROP TABLE t_empty;
DROP TABLE t_bool;
DROP TABLE t_char;
DROP TABLE t_date;
DROP TABLE t_datetime;
DROP TABLE t_decimal;
DROP TABLE t_float;
DROP TABLE t_double;
DROP TABLE t_tiny;
DROP TABLE t_small;
DROP TABLE t_big;
DROP TABLE t_large;
DROP TABLE t_text;

-- ============================================================================
-- Section 2: Column Aliases
-- ============================================================================

-- Test 2.1: Simple column alias
CREATE TABLE t_alias (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_alias VALUES (1, 100), (2, 200);
SELECT id AS employee_id, val AS salary FROM t_alias ORDER BY employee_id;
-- Expected: (1, 100), (2, 200) with aliased column names

-- Test 2.2: Alias with expression
SELECT id, val * 2 AS doubled FROM t_alias ORDER BY id;
-- Expected: (1, 200), (2, 400)

-- Test 2.3: Alias with string expression
SELECT id, 'Prefix: ' || name FROM (SELECT 1 AS id, 'test' AS name) AS sub ORDER BY id;
-- Note: uses subquery for constant

-- Test 2.4: Table alias
CREATE TABLE t_alias2 (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_alias2 VALUES (1, 10), (2, 20);
SELECT a.id, a.val, b.val AS other_val FROM t_alias a JOIN t_alias2 b ON a.id = b.id ORDER BY a.id;
-- Expected: (1, 100, 10), (2, 200, 20)

-- Test 2.5: Multiple aliases
SELECT id AS x, val AS y, val * 2 AS z FROM t_alias ORDER BY x;
-- Expected: (1, 100, 200), (2, 200, 400)

-- Test 2.6: Alias with complex expression
SELECT id, (val + val) * 3 AS computed FROM t_alias ORDER BY id;
-- Expected: id=1: 600, id=2: 1200

-- Test 2.7: Alias reused in ORDER BY
SELECT id AS a, val AS b FROM t_alias ORDER BY b DESC;
-- Expected: (2, 200), (1, 100)

-- Test 2.8: Alias with function expression
SELECT id, CAST(val AS VARCHAR) AS str_val FROM t_alias ORDER BY id;

-- Test 2.9: Alias with COALESCE
SELECT id, COALESCE(val, 0) AS coalesced_val FROM t_alias ORDER BY id;

-- Test 2.10: AS keyword optional (implicit alias)
SELECT id employee_no, val amount FROM t_alias ORDER BY id;

DROP TABLE t_alias;
DROP TABLE t_alias2;

-- ============================================================================
-- Section 3: DISTINCT
-- ============================================================================

-- Test 3.1: DISTINCT on single column
CREATE TABLE t_dist (id INT, dept VARCHAR(20), salary INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_dist VALUES
    (1, 'Engineering', 100),
    (2, 'Engineering', 200),
    (3, 'Marketing', 150),
    (4, 'Marketing', 250),
    (5, 'Sales', 300);
SELECT DISTINCT dept FROM t_dist ORDER BY dept;
-- Expected: Engineering, Marketing, Sales (3 rows)

-- Test 3.2: DISTINCT on multiple columns
SELECT DISTINCT dept, salary FROM t_dist ORDER BY dept, salary;
-- Expected: 5 rows (all unique combinations)

-- Test 3.3: DISTINCT on single column with duplicates
INSERT INTO t_dist VALUES (6, 'Engineering', 100);
SELECT DISTINCT dept FROM t_dist ORDER BY dept;
-- Expected: still 3 rows

-- Test 3.4: DISTINCT with WHERE
SELECT DISTINCT dept FROM t_dist WHERE salary > 150 ORDER BY dept;
-- Expected: Engineering (200), Marketing (250), Sales (300)

-- Test 3.5: DISTINCT with expression
SELECT DISTINCT salary / 100 AS bracket FROM t_dist ORDER BY bracket;
-- Expected: (1), (2), (3)

-- Test 3.6: DISTINCT with NULL handling
CREATE TABLE t_dist_null (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_dist_null VALUES (1, NULL), (2, NULL), (3, 10), (4, 20);
SELECT DISTINCT val FROM t_dist_null ORDER BY val;
-- Expected: NULL, 10, 20

-- Test 3.7: DISTINCT with ORDER BY on same column
SELECT DISTINCT salary FROM t_dist ORDER BY salary DESC;
-- Expected: 300, 250, 200, 150, 100

-- Test 3.8: DISTINCT with LIMIT
SELECT DISTINCT dept FROM t_dist ORDER BY dept LIMIT 2;
-- Expected: Engineering, Marketing

-- Test 3.9: DISTINCT on all columns
SELECT DISTINCT id, dept, salary FROM t_dist WHERE id <= 3 ORDER BY id;
-- Expected: 3 rows (id 1,2,3)

-- Test 3.10: DISTINCT with string column
CREATE TABLE t_dist_str (id INT, label VARCHAR(20)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_dist_str VALUES (1, 'a'), (2, 'a'), (3, 'b'), (4, 'c');
SELECT DISTINCT label FROM t_dist_str ORDER BY label;
-- Expected: a, b, c

-- Test 3.11: DISTINCT on single-row results
SELECT DISTINCT dept FROM t_dist WHERE dept = 'Sales';
-- Expected: Sales (1 row)

-- Test 3.12: DISTINCT on empty table
CREATE TABLE t_dist_empty (x INT) DISTRIBUTED BY HASH(x) BUCKETS 3;
SELECT DISTINCT x FROM t_dist_empty;
-- Expected: empty result set

-- Test 3.13: DISTINCT with COUNT
SELECT COUNT(DISTINCT dept) FROM t_dist;
-- Expected: 3

DROP TABLE t_dist;
DROP TABLE t_dist_null;
DROP TABLE t_dist_str;
DROP TABLE t_dist_empty;

-- ============================================================================
-- Section 4: WHERE with Comparison Operators
-- ============================================================================

CREATE TABLE t_where (
    id INT,
    name VARCHAR(50),
    age INT,
    salary DECIMAL(10, 2),
    active BOOLEAN
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_where VALUES
    (1, 'Alice', 30, 50000.00, TRUE),
    (2, 'Bob', 25, 45000.00, FALSE),
    (3, 'Charlie', 35, 60000.00, TRUE),
    (4, 'Diana', 28, 52000.00, TRUE),
    (5, 'Eve', 22, 38000.00, FALSE),
    (6, 'Frank', 40, 75000.00, TRUE),
    (7, 'Grace', 33, 55000.00, FALSE),
    (8, 'Henry', 45, 80000.00, TRUE),
    (9, 'Ivy', 27, 48000.00, TRUE),
    (10, 'Jack', 38, 62000.00, FALSE);

-- Test 4.1: WHERE with = (equality)
SELECT * FROM t_where WHERE id = 5;
-- Expected: (5, Eve, 22, 38000, false)

-- Test 4.2: WHERE with <> (not equal)
SELECT * FROM t_where WHERE id <> 5 ORDER BY id;
-- Expected: all rows except id=5 (9 rows)

-- Test 4.3: WHERE with != (not equal)
SELECT * FROM t_where WHERE id != 5 ORDER BY id;
-- Expected: all rows except id=5

-- Test 4.4: WHERE with < (less than)
SELECT * FROM t_where WHERE age < 25 ORDER BY id;
-- Expected: (5, Eve, 22)

-- Test 4.5: WHERE with <= (less than or equal)
SELECT * FROM t_where WHERE age <= 25 ORDER BY id;
-- Expected: (2, Bob, 25), (5, Eve, 22)

-- Test 4.6: WHERE with > (greater than)
SELECT * FROM t_where WHERE age > 40 ORDER BY id;
-- Expected: (8, Henry, 45)

-- Test 4.7: WHERE with >= (greater than or equal)
SELECT * FROM t_where WHERE age >= 40 ORDER BY id;
-- Expected: (6, Frank, 40), (8, Henry, 45)

-- Test 4.8: WHERE with string equality
SELECT * FROM t_where WHERE name = 'Alice';
-- Expected: (1, Alice, 30, 50000, true)

-- Test 4.9: WHERE with string inequality
SELECT * FROM t_where WHERE name > 'M' ORDER BY id;
-- Expected: names starting with N-Z

-- Test 4.10: WHERE with decimal comparison
SELECT * FROM t_where WHERE salary > 60000 ORDER BY id;
-- Expected: (6, Frank, 75000), (8, Henry, 80000), (10, Jack, 62000)

-- Test 4.11: WHERE with boolean comparison
SELECT * FROM t_where WHERE active = TRUE ORDER BY id;
-- Expected: rows with active=true (ids: 1,3,4,6,8,9)

-- Test 4.12: WHERE with boolean comparison (implicit)
SELECT * FROM t_where WHERE active ORDER BY id;
-- Expected: same as 4.11

-- Test 4.13: WHERE with NOT boolean
SELECT * FROM t_where WHERE NOT active ORDER BY id;
-- Expected: rows with active=false (ids: 2,5,7,10)

-- Test 4.14: WHERE with composite numeric
SELECT * FROM t_where WHERE age >= 30 AND salary >= 55000 ORDER BY id;
-- Expected: (3, Charlie), (6, Frank), (7, Grace), (10, Jack)

-- Test 4.15: WHERE with string comparison (less than)
SELECT * FROM t_where WHERE name < 'D' ORDER BY id;
-- Expected: (1, Alice), (2, Bob)

DROP TABLE t_where;

-- ============================================================================
-- Section 5: WHERE with AND/OR/NOT
-- ============================================================================

CREATE TABLE t_logic (
    id INT,
    status VARCHAR(20),
    priority INT,
    score INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_logic VALUES
    (1, 'active', 1, 90),
    (2, 'active', 2, 80),
    (3, 'inactive', 1, 70),
    (4, 'pending', 3, 95),
    (5, 'active', 3, 85),
    (6, 'inactive', 2, 60),
    (7, 'pending', 1, 75),
    (8, 'active', 1, 88),
    (9, 'inactive', 3, 92),
    (10, 'pending', 2, 78);

-- Test 5.1: WHERE with AND (two conditions)
SELECT * FROM t_logic WHERE status = 'active' AND priority = 1 ORDER BY id;
-- Expected: (1, active, 1, 90), (8, active, 1, 88)

-- Test 5.2: WHERE with AND (three conditions)
SELECT * FROM t_logic WHERE status = 'active' AND priority >= 2 AND score >= 85 ORDER BY id;
-- Expected: (5, active, 3, 85)

-- Test 5.3: WHERE with OR
SELECT * FROM t_logic WHERE status = 'active' OR status = 'pending' ORDER BY id;
-- Expected: all except inactive (7 rows)

-- Test 5.4: WHERE with NOT
SELECT * FROM t_logic WHERE NOT status = 'inactive' ORDER BY id;
-- Expected: all except inactive (7 rows)

-- Test 5.5: WHERE with AND and OR combined
SELECT * FROM t_logic WHERE (status = 'active' AND priority = 1) OR (status = 'pending' AND score > 80) ORDER BY id;
-- Expected: (1, active, 1, 90), (4, pending, 3, 95), (8, active, 1, 88)

-- Test 5.6: WHERE with nested AND/OR
SELECT * FROM t_logic WHERE (status = 'active' OR status = 'pending') AND score >= 80 ORDER BY id;
-- Expected: (1, 90), (4, 95), (5, 85), (8, 88)

-- Test 5.7: WHERE with NOT AND
SELECT * FROM t_logic WHERE NOT (status = 'inactive' AND priority = 2) ORDER BY id;
-- Expected: all rows except (6, inactive, 2, 60)

-- Test 5.8: WHERE with NOT OR
SELECT * FROM t_logic WHERE NOT (status = 'active' OR priority = 1) ORDER BY id;
-- Expected: (4, pending, 3, 95), (6, inactive, 2, 60), (10, pending, 2, 78)

-- Test 5.9: WHERE with complex boolean logic
SELECT * FROM t_logic WHERE (priority = 1 AND score > 80) OR (priority = 3 AND status = 'inactive') ORDER BY id;
-- Expected: (1, 90), (8, 88), (9, inactive, 3, 92)

-- Test 5.10: WHERE with multiple OR conditions on same column
SELECT * FROM t_logic WHERE status = 'active' OR status = 'pending' OR status = 'inactive' ORDER BY id;
-- Expected: all 10 rows

-- Test 5.11: WHERE with XOR-like logic using AND/OR/NOT
SELECT * FROM t_logic WHERE (status = 'active' OR status = 'inactive') AND NOT (status = 'active' AND priority = 2) ORDER BY id;
-- Expected: all active not priority 2 + all inactive

-- Test 5.12: WHERE with chained AND
SELECT * FROM t_logic WHERE id >= 3 AND id <= 7 AND priority >= 2 ORDER BY id;
-- Expected: (4, pending, 3, 95), (5, active, 3, 85), (6, inactive, 2, 60)

-- Test 5.13: WHERE with AND and expression
SELECT * FROM t_logic WHERE score > 80 AND priority + 1 > 2 ORDER BY id;
-- Expected: (4, 95), (5, 85), (9, 92)

-- Test 5.14: WHERE with chained conditions (4+)
SELECT * FROM t_logic WHERE id > 2 AND id < 9 AND status != 'inactive' AND score > 80 ORDER BY id;
-- Expected: (4, pending, 3, 95), (5, active, 3, 85)

-- Test 5.15: WHERE with all false
SELECT * FROM t_logic WHERE status = 'active' AND priority = 99;
-- Expected: empty result set

DROP TABLE t_logic;

-- ============================================================================
-- Section 6: WHERE with LIKE
-- ============================================================================

CREATE TABLE t_like (
    id INT,
    name VARCHAR(50),
    email VARCHAR(100)
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_like VALUES
    (1, 'Alice', 'alice@example.com'),
    (2, 'Bob', 'bob@test.org'),
    (3, 'Charlie', 'charlie@example.com'),
    (4, 'David', 'david@test.org'),
    (5, 'Eve', 'eve@example.com'),
    (6, 'Frank', 'frank@test.org'),
    (7, 'Grace', 'grace@example.com'),
    (8, 'Henry', 'henry@test.org'),
    (9, 'Ivy', 'ivy@example.com'),
    (10, 'Jack', 'jack@test.org');

-- Test 6.1: LIKE with prefix pattern
SELECT * FROM t_like WHERE name LIKE 'A%';
-- Expected: (1, Alice)

-- Test 6.2: LIKE with suffix pattern
SELECT * FROM t_like WHERE email LIKE '%.org';
-- Expected: even-numbered rows (5 rows)

-- Test 6.3: LIKE with contains pattern
SELECT * FROM t_like WHERE email LIKE '%example%';
-- Expected: odd-numbered rows (5 rows)

-- Test 6.4: LIKE with single character wildcard
SELECT * FROM t_like WHERE name LIKE 'A_ice';
-- Expected: (1, Alice)

-- Test 6.5: LIKE with multiple wildcards
SELECT * FROM t_like WHERE name LIKE '_a%';
-- Expected: names with 'a' as 2nd char

-- Test 6.6: LIKE with NOT LIKE
SELECT * FROM t_like WHERE name NOT LIKE 'A%' ORDER BY id;
-- Expected: all except Alice (9 rows)

-- Test 6.7: LIKE combined with AND
SELECT * FROM t_like WHERE name LIKE '%e%' AND email LIKE '%.org' ORDER BY id;
-- Expected: Eve, Henry, Jack (3 rows)

-- Test 6.8: LIKE with strings containing underscore
CREATE TABLE t_like_underscore (id INT, code VARCHAR(20)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_like_underscore VALUES (1, 'test_code'), (2, 'test_code_x'), (3, 'testXcode');
SELECT * FROM t_like_underscore WHERE code LIKE 'test\_code' ESCAPE '\' ORDER BY id;
-- Expected: (1, test_code) only (exact match with literal underscore)

-- Test 6.9: LIKE with starts-with pattern
SELECT * FROM t_like WHERE email LIKE 'b%';
-- Expected: (2, Bob)

-- Test 6.10: LIKE with ends-with and starts-with
SELECT * FROM t_like WHERE name LIKE 'C%e';
-- Expected: (3, Charlie)

-- Test 6.11: LIKE with middle wildcard
SELECT * FROM t_like WHERE name LIKE '%r%' ORDER BY id;
-- Expected: names containing 'r' (3: Charlie, 6: Frank, 8: Henry)

-- Test 6.12: LIKE case sensitivity
SELECT * FROM t_like WHERE name LIKE 'alice';
-- Expected: case-dependent result (Alice or empty)

-- Test 6.13: LIKE on empty result
SELECT * FROM t_like WHERE name LIKE 'ZZZ%';
-- Expected: empty set

-- Test 6.14: LIKE with multiple %
SELECT * FROM t_like WHERE name LIKE '%i%c%' ORDER BY id;
-- Expected: Alice, Charlie (2 rows)

-- Test 6.15: LIKE combined with other conditions
SELECT * FROM t_like WHERE name LIKE 'A%' OR name LIKE 'J%' ORDER BY id;
-- Expected: (1, Alice), (10, Jack)

DROP TABLE t_like;
DROP TABLE t_like_underscore;

-- ============================================================================
-- Section 7: WHERE with IN
-- ============================================================================

CREATE TABLE t_in (
    id INT,
    category VARCHAR(20),
    value INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_in VALUES
    (1, 'A', 100), (2, 'B', 200), (3, 'C', 300),
    (4, 'A', 400), (5, 'B', 500), (6, 'C', 600),
    (7, 'A', 700), (8, 'B', 800), (9, 'C', 900),
    (10, 'D', 1000);

-- Test 7.1: WHERE IN with list of values
SELECT * FROM t_in WHERE category IN ('A', 'C') ORDER BY id;
-- Expected: id: 1,3,4,6,7,9 (6 rows)

-- Test 7.2: WHERE IN with numeric list
SELECT * FROM t_in WHERE id IN (1, 5, 9) ORDER BY id;
-- Expected: (1, A, 100), (5, B, 500), (9, C, 900)

-- Test 7.3: WHERE NOT IN
SELECT * FROM t_in WHERE category NOT IN ('A', 'B') ORDER BY id;
-- Expected: C and D rows (ids: 3,6,9,10)

-- Test 7.4: WHERE IN with single value
SELECT * FROM t_in WHERE category IN ('D') ORDER BY id;
-- Expected: (10, D, 1000)

-- Test 7.5: WHERE NOT IN with single value
SELECT * FROM t_in WHERE category NOT IN ('D') ORDER BY id;
-- Expected: all except D (9 rows)

-- Test 7.6: WHERE IN combined with AND
SELECT * FROM t_in WHERE category IN ('A', 'B') AND value > 300 ORDER BY id;
-- Expected: (4, A, 400), (5, B, 500), (7, A, 700), (8, B, 800)

-- Test 7.7: WHERE IN with ORDER BY
SELECT * FROM t_in WHERE id IN (10, 1, 5) ORDER BY value DESC;
-- Expected: (10, D, 1000), (5, B, 500), (1, A, 100)

-- Test 7.8: WHERE IN with empty list
-- Note: IN () is generally not valid SQL; skip this test

-- Test 7.9: WHERE IN with no match
SELECT * FROM t_in WHERE id IN (999, 888);
-- Expected: empty result set

-- Test 7.10: WHERE IN with all matching
SELECT * FROM t_in WHERE id IN (1, 2, 3, 4, 5, 6, 7, 8, 9, 10) ORDER BY id;
-- Expected: all 10 rows

DROP TABLE t_in;

-- ============================================================================
-- Section 8: WHERE with BETWEEN
-- ============================================================================

CREATE TABLE t_between (
    id INT,
    score INT,
    price DECIMAL(10, 2)
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_between VALUES
    (1, 10, 5.50), (2, 20, 15.75), (3, 30, 25.00),
    (4, 40, 35.25), (5, 50, 45.50), (6, 60, 55.75),
    (7, 70, 65.00), (8, 80, 75.25), (9, 90, 85.50),
    (10, 100, 95.75);

-- Test 8.1: BETWEEN inclusive (lower and upper bound)
SELECT * FROM t_between WHERE score BETWEEN 30 AND 60 ORDER BY id;
-- Expected: ids 3,4,5,6 (4 rows)

-- Test 8.2: NOT BETWEEN
SELECT * FROM t_between WHERE score NOT BETWEEN 30 AND 60 ORDER BY id;
-- Expected: ids 1,2,7,8,9,10 (6 rows)

-- Test 8.3: BETWEEN with decimal columns
SELECT * FROM t_between WHERE price BETWEEN 25.00 AND 65.00 ORDER BY id;
-- Expected: ids 3,4,5,6,7 (5 rows)

-- Test 8.4: BETWEEN with same lower and upper bound
SELECT * FROM t_between WHERE score BETWEEN 50 AND 50 ORDER BY id;
-- Expected: (5, 50, 45.50)

-- Test 8.5: BETWEEN where lower > upper
SELECT * FROM t_between WHERE score BETWEEN 60 AND 30 ORDER BY id;
-- Expected: empty result set (invalid range)

-- Test 8.6: BETWEEN combined with AND
SELECT * FROM t_between WHERE score BETWEEN 20 AND 80 AND price BETWEEN 25 AND 55 ORDER BY id;
-- Expected: ids 3,4,5,6

-- Test 8.7: BETWEEN inclusive boundary test (left edge)
SELECT * FROM t_between WHERE score BETWEEN 10 AND 20 ORDER BY id;
-- Expected: ids 1,2

-- Test 8.8: BETWEEN inclusive boundary test (right edge)
SELECT * FROM t_between WHERE score BETWEEN 90 AND 100 ORDER BY id;
-- Expected: ids 9,10

-- Test 8.9: BETWEEN with no match
SELECT * FROM t_between WHERE score BETWEEN 200 AND 300;
-- Expected: empty

-- Test 8.10: BETWEEN with LIMIT
SELECT * FROM t_between WHERE score BETWEEN 30 AND 80 ORDER BY score LIMIT 3;
-- Expected: (3, 30), (4, 40), (5, 50)

DROP TABLE t_between;

-- ============================================================================
-- Section 9: WHERE with IS NULL / IS NOT NULL
-- ============================================================================

CREATE TABLE t_null (
    id INT,
    name VARCHAR(50),
    age INT,
    email VARCHAR(100),
    salary DECIMAL(10, 2)
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_null VALUES
    (1, 'Alice', 30, 'alice@example.com', 50000.00),
    (2, 'Bob', NULL, 'bob@example.com', 45000.00),
    (3, NULL, 35, 'charlie@example.com', NULL),
    (4, 'Diana', 28, NULL, 52000.00),
    (5, NULL, NULL, NULL, NULL);

-- Test 9.1: IS NULL on string column
SELECT * FROM t_null WHERE name IS NULL ORDER BY id;
-- Expected: ids 3,5 (2 rows)

-- Test 9.2: IS NOT NULL on string column
SELECT * FROM t_null WHERE name IS NOT NULL ORDER BY id;
-- Expected: ids 1,2,4 (3 rows)

-- Test 9.3: IS NULL on numeric column
SELECT * FROM t_null WHERE age IS NULL ORDER BY id;
-- Expected: ids 2,5 (2 rows)

-- Test 9.4: IS NOT NULL on numeric column
SELECT * FROM t_null WHERE age IS NOT NULL ORDER BY id;
-- Expected: ids 1,3,4 (3 rows)

-- Test 9.5: IS NULL on nullable column with some nulls
SELECT * FROM t_null WHERE email IS NULL ORDER BY id;
-- Expected: ids 4,5 (2 rows)

-- Test 9.6: IS NULL combined with AND
SELECT * FROM t_null WHERE name IS NULL AND age IS NULL ORDER BY id;
-- Expected: (5, all nulls)

-- Test 9.7: IS NOT NULL combined with AND
SELECT * FROM t_null WHERE name IS NOT NULL AND age IS NOT NULL ORDER BY id;
-- Expected: ids 1,4 (2 rows)

-- Test 9.8: IS NULL combined with OR
SELECT * FROM t_null WHERE name IS NULL OR email IS NULL ORDER BY id;
-- Expected: ids 3,4,5 (3 rows)

-- Test 9.9: IS NULL with numeric comparison on other column
SELECT * FROM t_null WHERE age IS NULL AND salary > 40000 ORDER BY id;
-- Expected: (2, Bob, null age, 45000)

-- Test 9.10: IS NOT NULL on all nullable columns
SELECT * FROM t_null WHERE name IS NOT NULL AND age IS NOT NULL AND email IS NOT NULL AND salary IS NOT NULL ORDER BY id;
-- Expected: (1, Alice, complete row)

-- Test 9.11: IS NULL with ORDER BY
SELECT * FROM t_null WHERE age IS NULL ORDER BY id DESC;
-- Expected: ids 5,2

-- Test 9.12: IS NULL with DISTINCT
SELECT DISTINCT age FROM t_null ORDER BY age;
-- Expected: NULL, 28, 30, 35

-- Test 9.13: IS NULL with LIMIT
SELECT * FROM t_null WHERE name IS NULL ORDER BY id LIMIT 1;
-- Expected: (3, null, 35, charlie@..., null) only

-- Test 9.14: WHERE clause comparing against NULL (should not match)
SELECT * FROM t_null WHERE name = NULL;
-- Expected: empty (NULL comparison semantics)

-- Test 9.15: WHERE clause with != NULL (should not match)
SELECT * FROM t_null WHERE name != NULL;
-- Expected: empty (NULL comparison semantics)

DROP TABLE t_null;

-- ============================================================================
-- Section 10: ORDER BY
-- ============================================================================

CREATE TABLE t_order (
    id INT,
    name VARCHAR(50),
    age INT,
    salary DECIMAL(10, 2),
    category VARCHAR(20)
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_order VALUES
    (1, 'Alice', 30, 50000.00, 'B'),
    (2, 'Bob', 25, 45000.00, 'A'),
    (3, 'Charlie', 35, 60000.00, 'C'),
    (4, 'Diana', 28, 52000.00, 'A'),
    (5, 'Eve', 22, 38000.00, 'B'),
    (6, 'Frank', 40, 75000.00, 'C'),
    (7, 'Grace', 33, 55000.00, 'A'),
    (8, 'Henry', 45, 80000.00, 'B'),
    (9, 'Ivy', 27, 48000.00, 'C'),
    (10, 'Jack', 38, 62000.00, 'A');

-- Test 10.1: ORDER BY single column ASC (default)
SELECT * FROM t_order ORDER BY id;
-- Expected: ascending by id (1-10)

-- Test 10.2: ORDER BY single column explicitly ASC
SELECT * FROM t_order ORDER BY name ASC;
-- Expected: alphabetical names

-- Test 10.3: ORDER BY single column DESC
SELECT * FROM t_order ORDER BY salary DESC;
-- Expected: highest salary first

-- Test 10.4: ORDER BY multiple columns
SELECT * FROM t_order ORDER BY category ASC, salary DESC;
-- Expected: grouped by category, then high salary first within each group

-- Test 10.5: ORDER BY with mixed ASC and DESC
SELECT * FROM t_order ORDER BY category ASC, age DESC, name ASC;
-- Expected: category A asc, age desc, name asc; then B; then C

-- Test 10.6: ORDER BY with expression
SELECT id, salary, salary * 1.1 AS raised FROM t_order ORDER BY raised DESC;
-- Expected: ordered by computed salary * 1.1

-- Test 10.7: ORDER BY with alias
SELECT id, name, age AS years FROM t_order ORDER BY years DESC;
-- Expected: ordered by alias 'years' which is age

-- Test 10.8: ORDER BY with column position (ordinal)
SELECT id, name, age FROM t_order ORDER BY 3 DESC;
-- Expected: ordered by 3rd column (age) descending

-- Test 10.9: ORDER BY DESC with LIMIT (top N)
SELECT id, name, salary FROM t_order ORDER BY salary DESC LIMIT 3;
-- Expected: Henry 80000, Frank 75000, Jack 62000

-- Test 10.10: ORDER BY ASC with LIMIT (bottom N)
SELECT id, name, salary FROM t_order ORDER BY salary ASC LIMIT 3;
-- Expected: Eve 38000, Bob 45000, Ivy 48000

-- Test 10.11: ORDER BY with string column
SELECT * FROM t_order ORDER BY name DESC;
-- Expected: names reverse alphabetical

-- Test 10.12: ORDER BY with multiple columns (all DESC)
SELECT * FROM t_order ORDER BY category DESC, id DESC;
-- Expected: category C first (desc), then B, then A; within each, id descending

-- Test 10.13: ORDER BY with WHERE filter
SELECT * FROM t_order WHERE age >= 30 ORDER BY salary DESC;
-- Expected: filtered (age>=30) then sorted by salary descending

-- Test 10.14: ORDER BY with WHERE and LIMIT
SELECT * FROM t_order WHERE category = 'A' ORDER BY salary DESC LIMIT 2;
-- Expected: top 2 salaries in category A (Jack 62000, Grace 55000)

-- Test 10.15: ORDER BY on single row result
SELECT * FROM t_order WHERE id = 5 ORDER BY name;
-- Expected: single row (Eve)

-- Test 10.16: ORDER BY on empty result
SELECT * FROM t_order WHERE id = 999 ORDER BY name;
-- Expected: empty

-- Test 10.17: ORDER BY with expression (arithmetic)
SELECT id, salary, salary / 12 AS monthly FROM t_order ORDER BY monthly DESC;
-- Expected: ordered by computed monthly salary

-- Test 10.18: ORDER BY on DECIMAL column
SELECT id, salary FROM t_order ORDER BY salary;
-- Expected: salaries ascending

-- Test 10.19: ORDER BY with computed string
SELECT id, name, LENGTH(name) AS name_len FROM t_order ORDER BY name_len DESC;
-- Expected: ordered by name length descending

-- Test 10.20: ORDER BY with DISTINCT
SELECT DISTINCT category FROM t_order ORDER BY category DESC;
-- Expected: C, B, A

-- Test 10.21: ORDER BY with multiple sort keys, some with NULL data
CREATE TABLE t_order_null (id INT, val INT, label VARCHAR(20)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_order_null VALUES (1, 100, 'a'), (2, NULL, 'b'), (3, 200, NULL), (4, NULL, NULL), (5, 50, 'c');
SELECT * FROM t_order_null ORDER BY val;
-- Expected: default NULL handling (typically NULLS LAST in standard SQL)

-- Test 10.22: ORDER BY DESC with NULL values
SELECT * FROM t_order_null ORDER BY val DESC;
-- Expected: NULL values ordering (typically NULLS LAST for ASC, NULLS FIRST for DESC in some databases)

-- Test 10.23: ORDER BY with label containing NULL
SELECT * FROM t_order_null ORDER BY label;
-- Expected: ordered by label, NULLs positioned according to default behavior

-- Test 10.24: ORDER BY two columns with NULLs
SELECT * FROM t_order_null ORDER BY val, label;

-- Test 10.25: ORDER BY with large offset
SELECT * FROM t_order ORDER BY id LIMIT 5 OFFSET 100;
-- Expected: empty (beyond result set)

DROP TABLE t_order;
DROP TABLE t_order_null;

-- ============================================================================
-- Section 11: LIMIT and OFFSET
-- ============================================================================

CREATE TABLE t_limit (
    id INT,
    val INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_limit VALUES
    (1, 10), (2, 20), (3, 30), (4, 40), (5, 50),
    (6, 60), (7, 70), (8, 80), (9, 90), (10, 100);

-- Test 11.1: LIMIT with number less than total
SELECT * FROM t_limit ORDER BY id LIMIT 3;
-- Expected: ids 1,2,3

-- Test 11.2: LIMIT with number exceeding total
SELECT * FROM t_limit ORDER BY id LIMIT 100;
-- Expected: all 10 rows

-- Test 11.3: LIMIT 0
SELECT * FROM t_limit ORDER BY id LIMIT 0;
-- Expected: empty result set

-- Test 11.4: LIMIT 1 (single row)
SELECT * FROM t_limit ORDER BY id LIMIT 1;
-- Expected: (1, 10)

-- Test 11.5: OFFSET without LIMIT
SELECT * FROM t_limit ORDER BY id OFFSET 5;
-- Expected: ids 6,7,8,9,10

-- Test 11.6: LIMIT with OFFSET
SELECT * FROM t_limit ORDER BY id LIMIT 3 OFFSET 4;
-- Expected: ids 5,6,7

-- Test 11.7: LIMIT exact number of rows
SELECT * FROM t_limit ORDER BY id LIMIT 10;
-- Expected: all 10 rows

-- Test 11.8: OFFSET at boundary
SELECT * FROM t_limit ORDER BY id OFFSET 9;
-- Expected: (10, 100)

-- Test 11.9: OFFSET beyond total rows
SELECT * FROM t_limit ORDER BY id OFFSET 20;
-- Expected: empty result set

-- Test 11.10: LIMIT with OFFSET beyond total
SELECT * FROM t_limit ORDER BY id LIMIT 5 OFFSET 100;
-- Expected: empty

-- Test 11.11: LIMIT with ORDER BY DESC
SELECT * FROM t_limit ORDER BY id DESC LIMIT 3;
-- Expected: (10, 100), (9, 90), (8, 80)

-- Test 11.12: LIMIT with WHERE filter
SELECT * FROM t_limit WHERE val > 50 ORDER BY id LIMIT 2;
-- Expected: (6, 60), (7, 70)

-- Test 11.13: OFFSET with WHERE
SELECT * FROM t_limit WHERE val > 30 ORDER BY id OFFSET 2;
-- Expected: (6, 60), (7, 70), (8, 80), (9, 90), (10, 100)

-- Test 11.14: LIMIT with DISTINCT
SELECT DISTINCT val FROM t_limit ORDER BY val LIMIT 4;
-- Expected: 10, 20, 30, 40

-- Test 11.15: LIMIT/OFFSET syntax with comma (MySQL-style)
SELECT * FROM t_limit ORDER BY id LIMIT 3, 2;
-- Expected: LIMIT 2 OFFSET 3 => ids 4,5 (if MySQL-compatible LIMIT a,b syntax)

-- Test 11.16: LIMIT with negative value (should be error, so we skip)

-- Test 11.17: OFFSET 0
SELECT * FROM t_limit ORDER BY id LIMIT 3 OFFSET 0;
-- Expected: ids 1,2,3

-- Test 11.18: LIMIT with aggregated result
SELECT COUNT(*) AS cnt FROM t_limit;
-- Expected: 10

-- Test 11.19: LIMIT with expression ORDER BY
SELECT id, val, val * 2 AS doubled FROM t_limit ORDER BY val DESC LIMIT 5;
-- Expected: top 5 by val desc: (10, 100, 200), (9, 90, 180), etc.

-- Test 11.20: OFFSET with single row fetch
SELECT * FROM t_limit ORDER BY id LIMIT 1 OFFSET 9;
-- Expected: (10, 100)

DROP TABLE t_limit;

-- ============================================================================
-- Section 12: CASE WHEN
-- ============================================================================

-- Test 12.1: Simple CASE WHEN with equality
CREATE TABLE t_case (id INT, score INT, grade VARCHAR(2)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_case VALUES
    (1, 95, NULL), (2, 85, NULL), (3, 75, NULL),
    (4, 65, NULL), (5, 55, NULL), (6, 45, NULL);
UPDATE t_case SET grade = CASE
    WHEN score >= 90 THEN 'A'
    WHEN score >= 80 THEN 'B'
    WHEN score >= 70 THEN 'C'
    WHEN score >= 60 THEN 'D'
    ELSE 'F'
END;
SELECT * FROM t_case ORDER BY id;
-- Expected: ids 1-6: A, B, C, D, F, F

-- Test 12.2: Searched CASE WHEN in SELECT (not UPDATE)
SELECT id, score,
    CASE
        WHEN score >= 90 THEN 'A'
        WHEN score >= 80 THEN 'B'
        WHEN score >= 70 THEN 'C'
        WHEN score >= 60 THEN 'D'
        ELSE 'F'
    END AS grade
FROM t_case ORDER BY id;
-- Expected: same as 12.1 but in SELECT

-- Test 12.3: Simple CASE expression (CASE x WHEN ...)
SELECT id, score,
    CASE score
        WHEN 95 THEN 'Excellent'
        WHEN 85 THEN 'Great'
        WHEN 75 THEN 'Good'
        ELSE 'OK'
    END AS rating
FROM t_case ORDER BY id;
-- Expected: Excellent, Great, Good, OK, OK, OK

-- Test 12.4: CASE WHEN with ELSE NULL
SELECT id, score,
    CASE
        WHEN score >= 80 THEN 'Pass'
    END AS pass_fail
FROM t_case ORDER BY id;
-- Expected: ids 1,2 have Pass; ids 3-6 have NULL

-- Test 12.5: CASE WHEN with multiple conditions AND/OR
SELECT id, score,
    CASE
        WHEN score >= 90 OR score < 50 THEN 'Extreme'
        WHEN score >= 70 AND score < 90 THEN 'Mid'
        ELSE 'Average'
    END AS band
FROM t_case ORDER BY id;
-- Expected: 95->Extreme, 85->Mid, 75->Mid, 65->Average, 55->Average, 45->Extreme

-- Test 12.6: CASE WHEN in WHERE clause
SELECT * FROM t_case WHERE
    CASE WHEN score >= 70 THEN TRUE ELSE FALSE END ORDER BY id;
-- Expected: ids 1,2,3 (scores >= 70)

-- Test 12.7: CASE WHEN in ORDER BY
SELECT * FROM t_case ORDER BY
    CASE WHEN score >= 80 THEN 0 ELSE 1 END, score DESC;
-- Expected: scores >= 80 first (95,85), then rest descending (75,65,55,45)

-- Test 12.8: Nested CASE WHEN
SELECT id, score,
    CASE
        WHEN score >= 60 THEN
            CASE
                WHEN score >= 80 THEN 'High Pass'
                ELSE 'Low Pass'
            END
        ELSE 'Fail'
    END AS result
FROM t_case ORDER BY id;
-- Expected: (95, High Pass), (85, High Pass), (75, Low Pass), (65, Low Pass), (55, Fail), (45, Fail)

-- Test 12.9: CASE WHEN with arithmetic expression
SELECT id, score,
    CASE
        WHEN score % 2 = 0 THEN 'Even'
        WHEN score % 2 = 1 THEN 'Odd'
    END AS parity
FROM t_case ORDER BY id;

-- Test 12.10: CASE WHEN with string column
CREATE TABLE t_case_str (id INT, status VARCHAR(20)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_case_str VALUES (1, 'active'), (2, 'inactive'), (3, 'pending'), (4, 'active');
SELECT id, status,
    CASE status
        WHEN 'active' THEN 'Online'
        WHEN 'inactive' THEN 'Offline'
        WHEN 'pending' THEN 'Starting'
        ELSE 'Unknown'
    END AS display_status
FROM t_case_str ORDER BY id;
-- Expected: (1, active, Online), (2, inactive, Offline), (3, pending, Starting), (4, active, Online)

-- Test 12.11: CASE WHEN in aggregate
SELECT
    SUM(CASE WHEN score >= 80 THEN 1 ELSE 0 END) AS high_count,
    SUM(CASE WHEN score >= 60 AND score < 80 THEN 1 ELSE 0 END) AS mid_count,
    SUM(CASE WHEN score < 60 THEN 1 ELSE 0 END) AS low_count
FROM t_case;
-- Expected: high=2, mid=2, low=2

-- Test 12.12: CASE WHEN with NULL handling
CREATE TABLE t_case_null (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_case_null VALUES (1, 10), (2, NULL), (3, 30);
SELECT id, val,
    CASE
        WHEN val IS NULL THEN 'Missing'
        WHEN val > 20 THEN 'Large'
        ELSE 'Small'
    END AS size
FROM t_case_null ORDER BY id;
-- Expected: (1, 10, Small), (2, null, Missing), (3, 30, Large)

-- Test 12.13: CASE WHEN with BETWEEN
SELECT id, score,
    CASE
        WHEN score BETWEEN 80 AND 100 THEN 'A'
        WHEN score BETWEEN 60 AND 79 THEN 'B'
        ELSE 'C'
    END AS letter
FROM t_case ORDER BY id;
-- Expected: (95, A), (85, A), (75, B), (65, B), (55, C), (45, C)

-- Test 12.14: CASE WHEN with IN
CREATE TABLE t_case_in (id INT, color VARCHAR(20)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_case_in VALUES (1, 'red'), (2, 'blue'), (3, 'green'), (4, 'yellow');
SELECT id, color,
    CASE color
        WHEN 'red' THEN 'Primary'
        WHEN 'blue' THEN 'Primary'
        WHEN 'green' THEN 'Secondary'
        ELSE 'Other'
    END AS color_type
FROM t_case_in ORDER BY id;
-- Expected: (red, Primary), (blue, Primary), (green, Secondary), (yellow, Other)

-- Test 12.15: CASE WHEN with COALESCE-like behavior
SELECT id, val,
    CASE
        WHEN val IS NOT NULL THEN val
        ELSE 0
    END AS safe_val
FROM t_case_null ORDER BY id;
-- Expected: (1, 10), (2, 0), (3, 30)

DROP TABLE t_case;
DROP TABLE t_case_str;
DROP TABLE t_case_null;
DROP TABLE t_case_in;

-- ============================================================================
-- Section 13: CAST
-- ============================================================================

CREATE TABLE t_cast (
    id INT,
    int_val INT,
    str_val VARCHAR(50),
    dec_val DECIMAL(10, 2),
    float_val FLOAT
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_cast VALUES
    (1, 42, '123', 45.67, 3.14),
    (2, -10, '-99', 0.99, 2.718),
    (3, 0, '0', 100.00, 1.618);

-- Test 13.1: CAST INT to VARCHAR
SELECT id, CAST(int_val AS VARCHAR) AS str FROM t_cast ORDER BY id;
-- Expected: '42', '-10', '0'

-- Test 13.2: CAST VARCHAR to INT
SELECT id, CAST(str_val AS INT) AS num FROM t_cast ORDER BY id;
-- Expected: 123, -99, 0

-- Test 13.3: CAST INT to DECIMAL
SELECT id, CAST(int_val AS DECIMAL(10, 2)) AS dec_val FROM t_cast ORDER BY id;
-- Expected: 42.00, -10.00, 0.00

-- Test 13.4: CAST DECIMAL to INT
SELECT id, CAST(dec_val AS INT) AS int_val FROM t_cast ORDER BY id;
-- Expected: 45, 0, 100 (truncation)

-- Test 13.5: CAST FLOAT to INT
SELECT id, CAST(float_val AS INT) AS int_val FROM t_cast ORDER BY id;
-- Expected: 3, 2, 1 (truncation)

-- Test 13.6: CAST INT to DOUBLE
SELECT id, CAST(int_val AS DOUBLE) AS dbl FROM t_cast ORDER BY id;
-- Expected: 42.0, -10.0, 0.0

-- Test 13.7: CAST VARCHAR to DOUBLE
SELECT id, CAST(str_val AS DOUBLE) AS dbl FROM t_cast ORDER BY id;
-- Expected: 123.0, -99.0, 0.0

-- Test 13.8: CAST INT to DATE (if supported)
-- Note: INT to DATE may not be supported; skip if not

-- Test 13.9: CAST in WHERE
SELECT * FROM t_cast WHERE CAST(str_val AS INT) > 0 ORDER BY id;
-- Expected: (1, 42, '123', 45.67, 3.14)

-- Test 13.10: CAST in ORDER BY
SELECT * FROM t_cast ORDER BY CAST(str_val AS INT) DESC;
-- Expected: 123, 0, -99

-- Test 13.11: CAST with negative numbers
SELECT CAST('-100' AS INT) AS neg_int;
-- Expected: -100

-- Test 13.12: CAST boolean to INT
CREATE TABLE t_cast_bool (id INT, flag BOOLEAN) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_cast_bool VALUES (1, TRUE), (2, FALSE);
SELECT id, CAST(flag AS INT) AS flag_int FROM t_cast_bool ORDER BY id;
-- Expected: (1, 1), (2, 0)

-- Test 13.13: CAST in arithmetic expression
SELECT id, CAST(str_val AS INT) * 2 AS doubled FROM t_cast ORDER BY id;
-- Expected: 246, -198, 0

-- Test 13.14: CAST on constant
SELECT CAST(3.14159 AS INT) AS pi;
-- Expected: 3

-- Test 13.15: CAST NULL
SELECT CAST(NULL AS INT);
-- Expected: NULL

DROP TABLE t_cast;
DROP TABLE t_cast_bool;

-- ============================================================================
-- Section 14: COALESCE
-- ============================================================================

CREATE TABLE t_coalesce (
    id INT,
    a INT,
    b INT,
    c INT,
    label VARCHAR(50)
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_coalesce VALUES
    (1, 10, 20, 30, 'first'),
    (2, NULL, 20, 30, 'second'),
    (3, NULL, NULL, 30, 'third'),
    (4, NULL, NULL, NULL, NULL),
    (5, 10, NULL, 30, 'fifth');

-- Test 14.1: COALESCE with first value non-null
SELECT id, COALESCE(a, b, c) AS result FROM t_coalesce ORDER BY id;
-- Expected: id1=10, id2=20, id3=30, id4=NULL, id5=10

-- Test 14.2: COALESCE with all NULLs
SELECT id, COALESCE(a, b, c) AS result FROM t_coalesce WHERE id = 4;
-- Expected: NULL

-- Test 14.3: COALESCE with default value
SELECT id, COALESCE(a, b, c, 999) AS result FROM t_coalesce ORDER BY id;
-- Expected: id1=10, id2=20, id3=30, id4=999, id5=10

-- Test 14.4: COALESCE with string columns
SELECT id, COALESCE(label, 'N/A') AS display FROM t_coalesce ORDER BY id;
-- Expected: first, second, third, N/A, fifth

-- Test 14.5: COALESCE with expression
SELECT id, COALESCE(a * 2, b + 5, 0) AS computed FROM t_coalesce ORDER BY id;
-- Expected: id1=20, id2=25, id3=0, id4=0, id5=20

-- Test 14.6: COALESCE in WHERE
SELECT * FROM t_coalesce WHERE COALESCE(a, b, 0) > 5 ORDER BY id;
-- Expected: ids 1,2,5

-- Test 14.7: COALESCE in ORDER BY
SELECT * FROM t_coalesce ORDER BY COALESCE(a, 999);
-- Expected: NULL a values sorted after non-null (with default 999)

-- Test 14.8: COALESCE in SELECT with alias
SELECT id, COALESCE(a, 0) AS a_non_null, COALESCE(b, 0) AS b_non_null FROM t_coalesce ORDER BY id;
-- Expected: id3: (3, 0, 0, 30), id4: (4, 0, 0, NULL)

-- Test 14.9: COALESCE with 2 arguments
SELECT id, COALESCE(a, b) AS result FROM t_coalesce ORDER BY id;
-- Expected: id1=10, id2=20, id3=NULL, id4=NULL, id5=10

-- Test 14.10: COALESCE with single non-nullable column
SELECT id, COALESCE(id, 0) AS result FROM t_coalesce ORDER BY id;
-- Expected: id values (always non-null)

-- Test 14.11: COALESCE with CAST
SELECT id, COALESCE(CAST(a AS VARCHAR), 'missing') AS label FROM t_coalesce ORDER BY id;
-- Expected: '10', 'missing', 'missing', 'missing', '10'

-- Test 14.12: COALESCE in computed expression
SELECT id, COALESCE(a, 0) + COALESCE(b, 0) + COALESCE(c, 0) AS total FROM t_coalesce ORDER BY id;
-- Expected: id1=60, id2=50, id3=30, id4=0, id5=40

-- Test 14.13: COALESCE with nested expression
SELECT id, a, COALESCE(a, COALESCE(b, COALESCE(c, -1))) AS nested FROM t_coalesce ORDER BY id;
-- Expected: id1=10, id2=20, id3=30, id4=-1, id5=10

-- Test 14.14: COALESCE with constant fallback in SELECT list
SELECT COALESCE(NULL, NULL, 'fallback');
-- Expected: 'fallback'

-- Test 14.15: COALESCE with integer fallback
SELECT COALESCE(NULL, 42);
-- Expected: 42

DROP TABLE t_coalesce;

-- ============================================================================
-- Section 15: Arithmetic and Expression SELECTs
-- ============================================================================

CREATE TABLE t_expr (
    id INT,
    x INT,
    y INT,
    price DECIMAL(10, 2),
    qty INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_expr VALUES
    (1, 10, 3, 5.50, 2),
    (2, 20, 5, 12.75, 4),
    (3, 30, 7, 25.00, 3),
    (4, 40, 9, 35.50, 1),
    (5, 50, 11, 45.25, 5);

-- Test 15.1: Addition in SELECT
SELECT id, x + y AS sum FROM t_expr ORDER BY id;
-- Expected: 13, 25, 37, 49, 61

-- Test 15.2: Subtraction in SELECT
SELECT id, x - y AS diff FROM t_expr ORDER BY id;
-- Expected: 7, 15, 23, 31, 39

-- Test 15.3: Multiplication in SELECT
SELECT id, x * y AS product FROM t_expr ORDER BY id;
-- Expected: 30, 100, 210, 360, 550

-- Test 15.4: Division in SELECT
SELECT id, x / y AS quotient FROM t_expr ORDER BY id;
-- Expected: 3, 4, 4, 4, 4 (integer division)

-- Test 15.5: Modulo in SELECT
SELECT id, x % y AS remainder FROM t_expr ORDER BY id;
-- Expected: 1, 0, 2, 4, 6

-- Test 15.6: Complex arithmetic expression
SELECT id, (x + y) * 2 - x / y AS result FROM t_expr ORDER BY id;
-- Expected: computed values

-- Test 15.7: Decimal multiplication
SELECT id, price * qty AS total FROM t_expr ORDER BY id;
-- Expected: 11.00, 51.00, 75.00, 35.50, 226.25

-- Test 15.8: Multiple arithmetic operations
SELECT id, ((x + y) * (x - y)) / 2 AS result FROM t_expr ORDER BY id;
-- Expected: computed

-- Test 15.9: Expression in WHERE
SELECT * FROM t_expr WHERE x * y > 200 ORDER BY id;
-- Expected: ids 3,4,5

-- Test 15.10: Expression in ORDER BY
SELECT * FROM t_expr ORDER BY price * qty DESC;
-- Expected: id5 (226.25), id2 (51.00), id3 (75.00), id1 (11.00), id4 (35.50)
-- Actually correct: 5=226.25, 2=51, 3=75, 4=35.5, 1=11 -> 5,3,2,4,1

-- Test 15.11: Expression in WHERE with AND
SELECT * FROM t_expr WHERE x + y > 30 AND price * qty > 50 ORDER BY id;
-- Expected: ids 3,5

-- Test 15.12: Negative numbers in expressions
SELECT id, -x AS neg_x, x + (-y) AS diff2 FROM t_expr ORDER BY id;
-- Expected: -10, 7; -20, 15; etc.

-- Test 15.13: Compound expression in SELECT
SELECT id, x + y * 2 AS compound FROM t_expr ORDER BY id;
-- Expected: 10+6=16, 20+10=30, 30+14=44, 40+18=58, 50+22=72

-- Test 15.14: Expression with parenthesized precedence
SELECT id, (x + y) * 2 AS paren FROM t_expr ORDER BY id;
-- Expected: 13*2=26, 25*2=50, 37*2=74, 49*2=98, 61*2=122

-- Test 15.15: Arithmetic on constants
SELECT 10 + 20 AS thirty, 100 - 25 AS seventy_five, 5 * 6 AS thirty_again, 100 / 4 AS twenty_five;
-- Expected: (30, 75, 30, 25)

-- Test 15.16: Division by zero (should error or return NULL depending on DB)
-- Skipping: depends on database behavior

-- Test 15.17: Expression with all numeric types
SELECT CAST(x AS DOUBLE) / CAST(y AS DOUBLE) AS precise FROM t_expr ORDER BY id;
-- Expected: 3.333..., 4.0, 4.285..., 4.444..., 4.545...

-- Test 15.18: String concatenation with || operator
CREATE TABLE t_concat (id INT, first VARCHAR(20), last VARCHAR(20)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_concat VALUES (1, 'John', 'Doe'), (2, 'Jane', 'Smith'), (3, 'Bob', NULL);
SELECT id, first || ' ' || last AS full_name FROM t_concat ORDER BY id;
-- Expected: (1, 'John Doe'), (2, 'Jane Smith'), (3, 'Bob ' || NULL -> result depends on NULL handling)

-- Test 15.19: String concatenation with constants
SELECT 'Hello, ' || 'world!' AS greeting;
-- Expected: 'Hello, world!'

-- Test 15.20: String concatenation in WHERE
SELECT * FROM t_concat WHERE (first || ' ' || last) LIKE 'John%' ORDER BY id;
-- Expected: (1, John, Doe)

DROP TABLE t_expr;
DROP TABLE t_concat;

-- ============================================================================
-- Section 16: Subqueries - IN (SELECT ...)
-- ============================================================================

CREATE TABLE t_sub_in_main (
    id INT,
    name VARCHAR(50),
    dept_id INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;

CREATE TABLE t_sub_in_dept (
    id INT,
    dept_name VARCHAR(50),
    active BOOLEAN
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_sub_in_main VALUES
    (1, 'Alice', 10),
    (2, 'Bob', 20),
    (3, 'Charlie', 10),
    (4, 'Diana', 30),
    (5, 'Eve', 20),
    (6, 'Frank', 40),
    (7, 'Grace', 10);

INSERT INTO t_sub_in_dept VALUES
    (10, 'Engineering', TRUE),
    (20, 'Marketing', TRUE),
    (30, 'Sales', FALSE),
    (40, 'HR', TRUE);

-- Test 16.1: Basic IN subquery
SELECT * FROM t_sub_in_main WHERE dept_id IN (SELECT id FROM t_sub_in_dept) ORDER BY id;
-- Expected: all rows (all dept_ids exist in dept table)

-- Test 16.2: IN subquery with WHERE condition in subquery
SELECT * FROM t_sub_in_main WHERE dept_id IN (SELECT id FROM t_sub_in_dept WHERE active = TRUE) ORDER BY id;
-- Expected: employees in active depts (10,20,40): ids 1,2,3,5,6,7

-- Test 16.3: NOT IN subquery
SELECT * FROM t_sub_in_main WHERE dept_id NOT IN (SELECT id FROM t_sub_in_dept WHERE active = FALSE) ORDER BY id;
-- Expected: employees NOT in Sales (dept 30): ids 1,2,3,5,6,7

-- Test 16.4: IN subquery with numeric comparison
SELECT * FROM t_sub_in_main WHERE dept_id IN (SELECT id FROM t_sub_in_dept WHERE id > 20) ORDER BY id;
-- Expected: depts 30 and 40: ids 4,6

-- Test 16.5: IN subquery with string column
SELECT * FROM t_sub_in_dept WHERE dept_name IN (SELECT name FROM (SELECT 'Engineering' AS name) AS d) ORDER BY id;
-- Expected: (10, Engineering, TRUE)

-- Test 16.6: IN subquery combining with other WHERE conditions
SELECT * FROM t_sub_in_main WHERE dept_id IN (SELECT id FROM t_sub_in_dept WHERE active = TRUE) AND name LIKE 'A%' ORDER BY id;
-- Expected: (1, Alice, 10)

-- Test 16.7: Nested IN subquery
SELECT * FROM t_sub_in_main WHERE dept_id IN (
    SELECT id FROM t_sub_in_dept WHERE active IN (SELECT TRUE)
) ORDER BY id;
-- Expected: all rows in active depts

-- Test 16.8: IN subquery returning no rows
SELECT * FROM t_sub_in_main WHERE dept_id IN (SELECT id FROM t_sub_in_dept WHERE id = 999);
-- Expected: empty

-- Test 16.9: IN subquery with DISTINCT in subquery
SELECT * FROM t_sub_in_main WHERE dept_id IN (SELECT DISTINCT id FROM t_sub_in_dept WHERE active = TRUE) ORDER BY id;
-- Expected: same as 16.2

-- Test 16.10: IN subquery with ORDER BY in outer query
SELECT * FROM t_sub_in_main WHERE dept_id IN (SELECT id FROM t_sub_in_dept WHERE active = TRUE) ORDER BY name DESC;
-- Expected: same rows as 16.2 but ordered by name descending

DROP TABLE t_sub_in_main;
DROP TABLE t_sub_in_dept;

-- ============================================================================
-- Section 17: Subqueries - Scalar Subquery
-- ============================================================================

CREATE TABLE t_sub_scalar_emp (
    id INT,
    name VARCHAR(50),
    salary DECIMAL(10, 2),
    dept_id INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;

CREATE TABLE t_sub_scalar_dept (
    id INT,
    name VARCHAR(50),
    budget DECIMAL(12, 2)
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_sub_scalar_emp VALUES
    (1, 'Alice', 50000, 10),
    (2, 'Bob', 45000, 20),
    (3, 'Charlie', 60000, 10),
    (4, 'Diana', 52000, 30),
    (5, 'Eve', 38000, 20),
    (6, 'Frank', 75000, 10);

INSERT INTO t_sub_scalar_dept VALUES
    (10, 'Engineering', 500000),
    (20, 'Marketing', 300000),
    (30, 'Sales', 200000);

-- Test 17.1: Scalar subquery in SELECT
SELECT id, name, salary,
    (SELECT AVG(salary) FROM t_sub_scalar_emp) AS avg_salary
FROM t_sub_scalar_emp ORDER BY id;
-- Expected: each row has the overall avg salary as extra column

-- Test 17.2: Scalar subquery in WHERE (comparison)
SELECT * FROM t_sub_scalar_emp WHERE salary > (SELECT AVG(salary) FROM t_sub_scalar_emp) ORDER BY id;
-- Expected: employees with salary > avg

-- Test 17.3: Scalar subquery with correlated reference
SELECT id, name, salary,
    (SELECT name FROM t_sub_scalar_dept WHERE id = t_sub_scalar_emp.dept_id) AS dept_name
FROM t_sub_scalar_emp ORDER BY id;
-- Expected: each employee gets their department name

-- Test 17.4: Scalar subquery as computed column
SELECT id, name, salary,
    (SELECT MAX(salary) FROM t_sub_scalar_emp) - salary AS gap_to_max
FROM t_sub_scalar_emp ORDER BY id;
-- Expected: difference between each salary and max

-- Test 17.5: Scalar subquery with aggregate function
SELECT * FROM t_sub_scalar_emp WHERE salary > (SELECT MAX(salary) FROM t_sub_scalar_emp WHERE dept_id = 10) ORDER BY id;
-- Expected: employees with salary > 75000 (none)

-- Test 17.6: Scalar subquery returning NULL (no rows)
SELECT id, name,
    (SELECT budget FROM t_sub_scalar_dept WHERE id = 999) AS missing_budget
FROM t_sub_scalar_emp ORDER BY id;
-- Expected: NULL for missing_budget column

-- Test 17.7: Multiple scalar subqueries in SELECT
SELECT id, name,
    (SELECT AVG(salary) FROM t_sub_scalar_emp) AS avg_sal,
    (SELECT MAX(salary) FROM t_sub_scalar_emp) AS max_sal,
    (SELECT MIN(salary) FROM t_sub_scalar_emp) AS min_sal
FROM t_sub_scalar_emp ORDER BY id;
-- Expected: each row has all three aggregate values

-- Test 17.8: Scalar subquery in ORDER BY
SELECT id, name, dept_id FROM t_sub_scalar_emp
ORDER BY (SELECT name FROM t_sub_scalar_dept WHERE id = t_sub_scalar_emp.dept_id), name;
-- Expected: ordered by department name, then employee name

-- Test 17.9: Scalar subquery with arithmetic
SELECT id, name, salary,
    salary - (SELECT AVG(salary) FROM t_sub_scalar_emp) AS diff_from_avg
FROM t_sub_scalar_emp ORDER BY id;

-- Test 17.10: Scalar subquery with HAVING (if aggregation)
SELECT dept_id, AVG(salary) AS avg_dept_sal
FROM t_sub_scalar_emp
GROUP BY dept_id
HAVING AVG(salary) > (SELECT AVG(salary) FROM t_sub_scalar_emp)
ORDER BY dept_id;
-- Expected: depts where avg salary > overall avg (Engineering: ~61667 > avg ~53333)

DROP TABLE t_sub_scalar_emp;
DROP TABLE t_sub_scalar_dept;

-- ============================================================================
-- Section 18: Subqueries - EXISTS
-- ============================================================================

CREATE TABLE t_exists_customers (
    id INT,
    name VARCHAR(50),
    city VARCHAR(50)
) DISTRIBUTED BY HASH(id) BUCKETS 3;

CREATE TABLE t_exists_orders (
    id INT,
    customer_id INT,
    amount DECIMAL(10, 2),
    status VARCHAR(20)
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_exists_customers VALUES
    (1, 'Alice', 'NYC'),
    (2, 'Bob', 'LA'),
    (3, 'Charlie', 'Chicago'),
    (4, 'Diana', 'NYC'),
    (5, 'Eve', 'LA'),
    (6, 'Frank', 'Chicago');

INSERT INTO t_exists_orders VALUES
    (1, 1, 100.00, 'shipped'),
    (2, 1, 200.00, 'pending'),
    (3, 2, 150.00, 'shipped'),
    (4, 3, 300.00, 'cancelled'),
    (5, 5, 250.00, 'pending'),
    (6, 1, 50.00, 'shipped');

-- Test 18.1: Basic EXISTS subquery
SELECT * FROM t_exists_customers c WHERE EXISTS (SELECT 1 FROM t_exists_orders o WHERE o.customer_id = c.id) ORDER BY c.id;
-- Expected: customers who have at least one order (ids 1,2,3,5)

-- Test 18.2: NOT EXISTS subquery
SELECT * FROM t_exists_customers c WHERE NOT EXISTS (SELECT 1 FROM t_exists_orders o WHERE o.customer_id = c.id) ORDER BY c.id;
-- Expected: customers with no orders (ids 4,6)

-- Test 18.3: EXISTS with additional WHERE conditions
SELECT * FROM t_exists_customers c WHERE EXISTS (
    SELECT 1 FROM t_exists_orders o WHERE o.customer_id = c.id AND o.status = 'shipped'
) ORDER BY c.id;
-- Expected: customers with shipped orders (ids 1,2)

-- Test 18.4: EXISTS with multi-condition correlation
SELECT * FROM t_exists_customers c WHERE EXISTS (
    SELECT 1 FROM t_exists_orders o WHERE o.customer_id = c.id AND o.amount > 100
) ORDER BY c.id;
-- Expected: customers with orders > 100 (ids 1,2,3,5)

-- Test 18.5: EXISTS with NOT and additional filter
SELECT * FROM t_exists_customers c WHERE
    c.city = 'NYC'
    AND EXISTS (SELECT 1 FROM t_exists_orders o WHERE o.customer_id = c.id AND o.status = 'shipped')
ORDER BY c.id;
-- Expected: NYC customers with shipped orders (id 1)

-- Test 18.6: Correlated subquery with EXISTS and two levels of correlation
SELECT * FROM t_exists_orders o1 WHERE EXISTS (
    SELECT 1 FROM t_exists_orders o2 WHERE o2.customer_id = o1.customer_id AND o2.amount > o1.amount
) ORDER BY o1.id;
-- Expected: orders where the customer has another order with higher amount

-- Test 18.7: EXISTS with aggregate in subquery
SELECT * FROM t_exists_customers c WHERE EXISTS (
    SELECT 1 FROM t_exists_orders o WHERE o.customer_id = c.id GROUP BY o.customer_id HAVING COUNT(*) > 1
) ORDER BY c.id;
-- Expected: customers with more than 1 order (id 1)

-- Test 18.8: Double NOT EXISTS (all customers with orders, etc.)
-- Already covered by Test 18.2

-- Test 18.9: EXISTS in SELECT list (boolean result)
SELECT c.id, c.name,
    EXISTS (SELECT 1 FROM t_exists_orders o WHERE o.customer_id = c.id) AS has_orders
FROM t_exists_customers c ORDER BY c.id;
-- Expected: ids 1,2,3,5 have true; 4,6 have false

-- Test 18.10: EXISTS with correlated subquery comparing multiple columns
SELECT * FROM t_exists_customers c WHERE EXISTS (
    SELECT 1 FROM t_exists_orders o
    WHERE o.customer_id = c.id
    AND o.status IN ('shipped', 'pending')
    AND o.amount >= 100
) ORDER BY c.id;
-- Expected: ids 1,2,3,5

DROP TABLE t_exists_customers;
DROP TABLE t_exists_orders;

-- ============================================================================
-- Section 19: Subqueries - Correlated Subqueries
-- ============================================================================

CREATE TABLE t_corr_emp (
    id INT,
    name VARCHAR(50),
    salary DECIMAL(10, 2),
    dept_id INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_corr_emp VALUES
    (1, 'Alice', 50000, 1),
    (2, 'Bob', 60000, 1),
    (3, 'Charlie', 70000, 1),
    (4, 'Diana', 45000, 2),
    (5, 'Eve', 55000, 2),
    (6, 'Frank', 80000, 3),
    (7, 'Grace', 65000, 3),
    (8, 'Henry', 48000, 2);

-- Test 19.1: Correlated scalar subquery - employee vs dept average
SELECT e1.id, e1.name, e1.salary, e1.dept_id,
    (SELECT AVG(e2.salary) FROM t_corr_emp e2 WHERE e2.dept_id = e1.dept_id) AS dept_avg
FROM t_corr_emp e1 ORDER BY e1.id;
-- Expected: each row shows the avg salary of their department

-- Test 19.2: Correlated subquery - employees above dept average
SELECT * FROM t_corr_emp e1
WHERE e1.salary > (SELECT AVG(e2.salary) FROM t_corr_emp e2 WHERE e2.dept_id = e1.dept_id)
ORDER BY e1.id;
-- Expected: employees earning above their department average

-- Test 19.3: Correlated subquery with EXISTS
SELECT * FROM t_corr_emp e1
WHERE EXISTS (
    SELECT 1 FROM t_corr_emp e2 WHERE e2.dept_id = e1.dept_id AND e2.salary > e1.salary * 1.2
) ORDER BY e1.id;
-- Expected: employees who have a coworker earning > 1.2x their salary

-- Test 19.4: Correlated subquery using NOT EXISTS
SELECT * FROM t_corr_emp e1
WHERE NOT EXISTS (
    SELECT 1 FROM t_corr_emp e2 WHERE e2.dept_id = e1.dept_id AND e2.salary > e1.salary
) ORDER BY e1.id;
-- Expected: highest paid employee in each dept

-- Test 19.5: Correlated subquery with multiple correlations
SELECT * FROM t_corr_emp e1
WHERE e1.salary > (
    SELECT AVG(e2.salary) FROM t_corr_emp e2 WHERE e2.dept_id = e1.dept_id AND e2.id != e1.id
) ORDER BY e1.id;
-- Expected: employees above dept average excluding themselves

-- Test 19.6: Correlated subquery in SELECT with multiple levels
SELECT e1.id, e1.name, e1.salary,
    (SELECT COUNT(*) FROM t_corr_emp e2 WHERE e2.dept_id = e1.dept_id) AS dept_size,
    (SELECT MAX(e3.salary) FROM t_corr_emp e3 WHERE e3.dept_id = e1.dept_id) AS dept_max
FROM t_corr_emp e1 ORDER BY e1.id;
-- Expected: each row has dept_size and dept_max

-- Test 19.7: Deeply correlated - self-join style
SELECT * FROM t_corr_emp e1
WHERE e1.salary = (
    SELECT MAX(e2.salary) FROM t_corr_emp e2 WHERE e2.dept_id = e1.dept_id
) ORDER BY e1.id;
-- Expected: highest paid employee(s) per department

-- Test 19.8: Correlated subquery with inequality
SELECT * FROM t_corr_emp e1
WHERE e1.salary < (
    SELECT MAX(e2.salary) FROM t_corr_emp e2 WHERE e2.dept_id = e1.dept_id
) ORDER BY e1.id;
-- Expected: all employees who are NOT the highest paid in their dept

-- Test 19.9: Correlated subquery with ORDER BY
SELECT e1.id, e1.name, e1.salary, e1.dept_id
FROM t_corr_emp e1
ORDER BY (SELECT AVG(e2.salary) FROM t_corr_emp e2 WHERE e2.dept_id = e1.dept_id) DESC, e1.salary DESC;
-- Expected: ordered by dept avg salary (desc), then individual salary (desc)

-- Test 19.10: Correlated subquery with multiple table references
SELECT e1.id, e1.name, e1.salary
FROM t_corr_emp e1
WHERE e1.salary > (
    SELECT AVG(e2.salary) FROM t_corr_emp e2
    WHERE e2.dept_id = e1.dept_id
    AND e2.id IN (SELECT e3.id FROM t_corr_emp e3 WHERE e3.salary > 50000)
) ORDER BY e1.id;

DROP TABLE t_corr_emp;

-- ============================================================================
-- Section 20: Derived Tables (FROM subquery)
-- ============================================================================

-- Test 20.1: Simple derived table
SELECT * FROM (SELECT 1 AS a, 2 AS b) AS dt;
-- Expected: (1, 2)

-- Test 20.2: Derived table from actual table
CREATE TABLE t_derived (id INT, val INT, cat VARCHAR(10)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_derived VALUES
    (1, 100, 'X'), (2, 200, 'Y'), (3, 300, 'X'),
    (4, 400, 'Y'), (5, 500, 'Z');

SELECT dt.* FROM (SELECT * FROM t_derived) AS dt ORDER BY dt.id;
-- Expected: all rows

-- Test 20.3: Derived table with aggregation
SELECT cat, total FROM (
    SELECT cat, SUM(val) AS total FROM t_derived GROUP BY cat
) AS agg ORDER BY cat;
-- Expected: X:600, Y:600, Z:500

-- Test 20.4: Derived table with filter
SELECT * FROM (
    SELECT * FROM t_derived WHERE val > 200
) AS filtered ORDER BY id;
-- Expected: ids 3,4,5

-- Test 20.5: Nested derived tables
SELECT * FROM (
    SELECT cat, total FROM (
        SELECT cat, SUM(val) AS total FROM t_derived GROUP BY cat
    ) AS agg WHERE total > 500
) AS high_cats ORDER BY cat;
-- Expected: X:600, Y:600

-- Test 20.6: Derived table with column alias
SELECT a, b FROM (
    SELECT id AS a, val * 2 AS b FROM t_derived WHERE val > 200
) AS dt ORDER BY a;
-- Expected: a=3,b=600; a=4,b=800; a=5,b=1000

-- Test 20.7: Derived table with JOIN
-- Skipping for now (focus on SELECT features)

-- Test 20.8: Derived table with TOP/LIMIT
SELECT * FROM (SELECT * FROM t_derived ORDER BY val DESC LIMIT 3) AS top3 ORDER BY id;
-- Expected: top 3 by val: id=5 (500), id=4 (400), id=3 (300) -- but ORDER BY id in outer

-- Actually the inner order may not be preserved; let's keep it simpler
SELECT * FROM (SELECT * FROM t_derived LIMIT 3) AS dt ORDER BY dt.id;
-- Expected: ids 1,2,3

-- Test 20.9: Derived table with expression columns
SELECT dt.id, dt.doubled, dt.cat FROM (
    SELECT id, val * 2 AS doubled, cat FROM t_derived
) AS dt WHERE dt.doubled > 500 ORDER BY dt.id;
-- Expected: ids 3 (600, X), 4 (800, Y), 5 (1000, Z)

-- Test 20.10: Multiple derived tables
SELECT a.cat, a.cnt, b.max_val FROM (
    SELECT cat, COUNT(*) AS cnt FROM t_derived GROUP BY cat
) AS a JOIN (
    SELECT cat, MAX(val) AS max_val FROM t_derived GROUP BY cat
) AS b ON a.cat = b.cat ORDER BY a.cat;
-- Expected: X:2,300; Y:2,400; Z:1,500

DROP TABLE t_derived;

-- ============================================================================
-- Section 21: Combined Feature Tests
-- ============================================================================

-- Test 21.1: SELECT with WHERE, ORDER BY, LIMIT
CREATE TABLE t_combined (
    id INT,
    name VARCHAR(30),
    score INT,
    dept VARCHAR(20)
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_combined VALUES
    (1, 'Alice', 85, 'Eng'),
    (2, 'Bob', 92, 'Eng'),
    (3, 'Charlie', 78, 'Sales'),
    (4, 'Diana', 95, 'Eng'),
    (5, 'Eve', 88, 'Sales'),
    (6, 'Frank', 72, 'Sales'),
    (7, 'Grace', 91, 'Eng'),
    (8, 'Henry', 83, 'Sales');

SELECT * FROM t_combined WHERE dept = 'Eng' ORDER BY score DESC LIMIT 2;
-- Expected: top 2 Eng scores (Diana 95, Bob 92)

-- Test 21.2: DISTINCT + WHERE + ORDER BY
SELECT DISTINCT dept FROM t_combined WHERE score >= 80 ORDER BY dept;
-- Expected: Eng, Sales

-- Test 21.3: CASE WHEN + WHERE + ORDER BY
SELECT id, name, score,
    CASE WHEN score >= 90 THEN 'A'
         WHEN score >= 80 THEN 'B'
         ELSE 'C' END AS grade
FROM t_combined WHERE dept = 'Sales' ORDER BY score DESC;
-- Expected: (5, Eve, 88, B), (3, Charlie, 78, C), (8, Henry, 83, B), (6, Frank, 72, C)
-- Actually correct order DESC: 88, 83, 78, 72

-- Test 21.4: CAST + WHERE + ORDER BY
SELECT * FROM t_combined ORDER BY CAST(score AS VARCHAR) LIMIT 3;
-- Expected: ordered lexicographically by string score

-- Test 21.5: COALESCE + CASE WHEN
-- (Use t_combined - no nulls, so create nulls scenario)
CREATE TABLE t_comb_null (id INT, val1 INT, val2 INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_comb_null VALUES (1, 10, NULL), (2, NULL, 20), (3, NULL, NULL), (4, 30, 40);
SELECT id,
    CASE
        WHEN COALESCE(val1, val2, 0) > 15 THEN 'Large'
        WHEN COALESCE(val1, val2, 0) > 5 THEN 'Medium'
        ELSE 'Small'
    END AS size
FROM t_comb_null ORDER BY id;
-- Expected: 1:Medium (10), 2:Large (20), 3:Small (0), 4:Large (30)

-- Test 21.6: Arithmetic + CASE WHEN + WHERE
SELECT id, name, score,
    CASE
        WHEN score >= 90 THEN score * 1.1
        WHEN score >= 80 THEN score * 1.05
        ELSE score
    END AS adjusted
FROM t_combined WHERE dept = 'Eng' ORDER BY id;
-- Expected: Alice 85*1.05=89.25, Bob 92*1.1=101.2, Diana 95*1.1=104.5, Grace 91*1.1=100.1

-- Test 21.7: Subquery + JOIN
SELECT * FROM t_combined WHERE id IN (
    SELECT id FROM t_combined WHERE score > 80
) AND dept = 'Eng' ORDER BY id;
-- Expected: Eng employees with score > 80 (Alice, Bob, Diana, Grace)

-- Test 21.8: DISTINCT + ORDER BY alias + LIMIT
SELECT DISTINCT score AS s FROM t_combined ORDER BY s DESC LIMIT 4;
-- Expected: 95, 92, 91, 88

-- Test 21.9: CASE WHEN with BETWEEN + WHERE
SELECT id, name, score,
    CASE
        WHEN score BETWEEN 90 AND 100 THEN 'Excellent'
        WHEN score BETWEEN 80 AND 89 THEN 'Good'
        WHEN score BETWEEN 70 AND 79 THEN 'Average'
        ELSE 'Below'
    END AS rating
FROM t_combined WHERE dept = 'Sales' ORDER BY id;
-- Expected: (3, 78, Average), (5, 88, Good), (6, 72, Average), (8, 83, Good)

-- Test 21.10: Expression + Alias + WHERE + ORDER BY
SELECT id, name, score, score / 10 AS tens_digit FROM t_combined WHERE dept = 'Eng' ORDER BY tens_digit DESC;
-- Expected: Eng employees ordered by score/10 descending

-- Test 21.11: CAST + COALESCE + CASE WHEN
-- Create table with diverse types
CREATE TABLE t_mixed (id INT, val VARCHAR(20), flag BOOLEAN) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_mixed VALUES (1, '50', TRUE), (2, NULL, FALSE), (3, '100', TRUE);
SELECT id,
    COALESCE(CAST(val AS INT), 0) AS num_val,
    CASE WHEN flag THEN 'Yes' ELSE 'No' END AS flag_label
FROM t_mixed ORDER BY id;
-- Expected: (1, 50, Yes), (2, 0, No), (3, 100, Yes)

-- Test 21.12: Multiple subqueries in single query
SELECT * FROM t_combined e WHERE e.score > (
    SELECT AVG(score) FROM t_combined WHERE dept = e.dept
) ORDER BY e.id;
-- Expected: employees above their department average

-- Test 21.13: WHERE + ORDER BY + LIMIT + OFFSET
SELECT * FROM t_combined ORDER BY score DESC LIMIT 3 OFFSET 1;
-- Expected: 2nd-4th highest scores (92, 91, 88)

-- Test 21.14: DISTINCT + CASE WHEN
SELECT DISTINCT
    CASE WHEN score >= 85 THEN 'High' ELSE 'Low' END AS bracket
FROM t_combined ORDER BY bracket;
-- Expected: High, Low

-- Test 21.15: Multiple aggregations with CASE
SELECT dept,
    COUNT(*) AS total,
    SUM(CASE WHEN score >= 90 THEN 1 ELSE 0 END) AS excellent,
    SUM(CASE WHEN score >= 80 AND score < 90 THEN 1 ELSE 0 END) AS good,
    SUM(CASE WHEN score < 80 THEN 1 ELSE 0 END) AS needs_improvement
FROM t_combined GROUP BY dept ORDER BY dept;
-- Expected: Eng: 4 total, excellent=2 (Bob, Diana, Grace), good=1 (Alice), needs=0
-- Wait: Alice 85 -> not >= 90, but >= 80 so 'good'
-- Eng: Bob 92 (excellent), Diana 95 (excellent), Grace 91 (excellent) -> 3 excellent, Alice 85 -> 1 good
-- Sales: Eve 88 (good), Henry 83 (good) -> 2 good; Charlie 78, Frank 72 -> 2 needs

-- Test 21.16: Subquery in SELECT list with CASE
SELECT id, name, score,
    CASE
        WHEN score > (SELECT AVG(score) FROM t_combined) THEN 'Above Avg'
        WHEN score = (SELECT AVG(score) FROM t_combined) THEN 'Average'
        ELSE 'Below Avg'
    END AS comparison
FROM t_combined ORDER BY id;

-- Test 21.17: Nested CASE + arithmetic
SELECT id, name, score,
    CASE
        WHEN score >= 80 THEN
            CASE WHEN score >= 95 THEN 'Star'
                 ELSE 'Solid'
            END
        ELSE CASE WHEN score >= 75 THEN 'Close'
                  ELSE 'Needs Work'
             END
    END AS category
FROM t_combined ORDER BY id;

-- Test 21.18: ORDER BY with multiple expressions
SELECT * FROM t_combined ORDER BY dept ASC, CASE WHEN score >= 90 THEN 0 ELSE 1 END, score DESC;
-- Expected: Eng first, highest scores first, with 90+ sorted first within each dept

-- Test 21.19: LIMIT 0 with WHERE
SELECT * FROM t_combined WHERE dept = 'Eng' LIMIT 0;
-- Expected: empty result set

-- Test 21.20: OFFSET large value
SELECT * FROM t_combined ORDER BY id OFFSET 100;
-- Expected: empty (beyond table rows)

-- Test 21.21: WHERE with IN and subquery combined
SELECT * FROM t_combined WHERE id IN (
    SELECT id FROM t_combined WHERE score > 80
) AND dept IN ('Eng') ORDER BY id;
-- Expected: Eng employees with score > 80

-- Test 21.22: Scalar subquery with COALESCE
SELECT id, name,
    COALESCE((SELECT MAX(score) FROM t_combined WHERE dept = 'Eng'), 0) AS eng_max
FROM t_combined ORDER BY id;
-- Expected: each row shows max Eng score (95)

-- Test 21.23: EXISTS with ORDER BY
SELECT * FROM t_combined e WHERE EXISTS (
    SELECT 1 FROM t_combined e2 WHERE e2.dept = e.dept AND e2.score > 90
) ORDER BY e.score DESC;
-- Expected: only employees in depts with someone scoring > 90 (both Eng and... check)
-- Eng has Diana 95, Bob 92, Grace 91 -> all Eng qualify
-- Sales has no one > 90 (max 88) -> no Sales qualify
-- So only Eng employees, ordered by score desc

-- Test 21.24: Combination of IN, BETWEEN, ORDER BY
SELECT * FROM t_combined WHERE id IN (1, 3, 5, 7, 9) AND score BETWEEN 80 AND 100 ORDER BY name;
-- Expected: ids in {1,3,5,7} with score >= 80 (1: 85, 5: 88, 7: 91)

-- Test 21.25: All features combined
SELECT id, name, score, dept,
    CASE
        WHEN score >= 90 THEN 'A'
        WHEN score >= 80 THEN 'B'
        ELSE 'C'
    END AS grade,
    score - (SELECT AVG(s) FROM (SELECT score AS s FROM t_combined WHERE dept = e.dept) AS sub) AS diff_from_dept_avg
FROM t_combined e
WHERE dept IN (SELECT DISTINCT dept FROM t_combined WHERE score >= 90)
ORDER BY grade, diff_from_dept_avg DESC
LIMIT 5;
-- Expected: students in departments with at least one 90+ scorer, ordered by grade then diff from dept avg

DROP TABLE t_combined;
DROP TABLE t_comb_null;
DROP TABLE t_mixed;

-- ============================================================================
-- Section 22: Edge Cases - Single Row Tables
-- ============================================================================

-- Test 22.1: SELECT from single row table
CREATE TABLE t_one_row (id INT, val VARCHAR(20)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_one_row VALUES (1, 'only');
SELECT * FROM t_one_row;
-- Expected: (1, 'only')

-- Test 22.2: DISTINCT on single row
SELECT DISTINCT val FROM t_one_row;
-- Expected: 'only'

-- Test 22.3: WHERE on single row (match)
SELECT * FROM t_one_row WHERE id = 1;
-- Expected: (1, 'only')

-- Test 22.4: WHERE on single row (no match)
SELECT * FROM t_one_row WHERE id = 2;
-- Expected: empty

-- Test 22.5: ORDER BY on single row
SELECT * FROM t_one_row ORDER BY val DESC;
-- Expected: (1, 'only')

-- Test 22.6: LIMIT on single row
SELECT * FROM t_one_row ORDER BY id LIMIT 1;
-- Expected: (1, 'only')

-- Test 22.7: CASE WHEN on single row
SELECT id, CASE WHEN id = 1 THEN 'yes' ELSE 'no' END AS result FROM t_one_row;
-- Expected: (1, 'yes')

-- Test 22.8: Subquery with single row source
SELECT * FROM t_one_row WHERE id = (SELECT id FROM t_one_row);
-- Expected: (1, 'only')

-- Test 22.9: EXISTS with single row tables
CREATE TABLE t_one_row2 (id INT, label VARCHAR(20)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_one_row2 VALUES (1, 'other');
SELECT * FROM t_one_row WHERE EXISTS (SELECT 1 FROM t_one_row2 WHERE t_one_row2.id = t_one_row.id);
-- Expected: (1, 'only')

-- Test 22.10: Aggregation on single row
SELECT COUNT(*), MAX(val), MIN(val) FROM t_one_row;
-- Expected: 1, 'only', 'only'

DROP TABLE t_one_row;
DROP TABLE t_one_row2;

-- ============================================================================
-- Section 23: Edge Cases - Many Rows (Bulk)
-- ============================================================================

-- Test 23.1: SELECT from table with many rows
CREATE TABLE t_many (id INT, grp INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_many VALUES
    (1, 1, 10), (2, 1, 20), (3, 1, 30), (4, 1, 40), (5, 1, 50),
    (6, 2, 60), (7, 2, 70), (8, 2, 80), (9, 2, 90), (10, 2, 100),
    (11, 3, 110), (12, 3, 120), (13, 3, 130), (14, 3, 140), (15, 3, 150),
    (16, 4, 160), (17, 4, 170), (18, 4, 180), (19, 4, 190), (20, 4, 200);

SELECT COUNT(*) FROM t_many;
-- Expected: 20

-- Test 23.2: ORDER BY on many rows
SELECT * FROM t_many ORDER BY val DESC;
-- Expected: 200 down to 10

-- Test 23.3: LIMIT on many rows
SELECT * FROM t_many ORDER BY id LIMIT 5;
-- Expected: ids 1-5

-- Test 23.4: OFFSET on many rows
SELECT * FROM t_many ORDER BY id OFFSET 15;
-- Expected: ids 16-20

-- Test 23.5: WHERE on many rows
SELECT * FROM t_many WHERE val BETWEEN 50 AND 150 ORDER BY id;
-- Expected: ids 5-13 (9 rows)

-- Test 23.6: DISTINCT on many rows
SELECT DISTINCT grp FROM t_many ORDER BY grp;
-- Expected: 1, 2, 3, 4

-- Test 23.7: Group aggregation on many rows
SELECT grp, COUNT(*) AS cnt, SUM(val) AS total, AVG(val) AS avg_val
FROM t_many GROUP BY grp ORDER BY grp;
-- Expected: grp1: 5, 150, 30; grp2: 5, 400, 80; grp3: 5, 650, 130; grp4: 5, 900, 180

-- Test 23.8: Subquery on many rows
SELECT * FROM t_many WHERE val > (SELECT AVG(val) FROM t_many) ORDER BY id;
-- Expected: rows with val > 105 (ids 7-20)

-- Test 23.9: IN subquery on many rows
SELECT * FROM t_many WHERE grp IN (SELECT DISTINCT grp FROM t_many WHERE val > 100) ORDER BY id;
-- Expected: all rows from grp 3 and 4 (ids 11-20)

-- Test 23.10: LIMIT 0 with many rows
SELECT * FROM t_many ORDER BY id LIMIT 0;
-- Expected: empty

DROP TABLE t_many;

-- ============================================================================
-- Section 24: Edge Cases - NULL Handling in WHERE and ORDER BY
-- ============================================================================

CREATE TABLE t_null_advanced (
    id INT,
    name VARCHAR(30),
    score INT,
    category VARCHAR(20)
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_null_advanced VALUES
    (1, 'Alice', 100, 'A'),
    (2, 'Bob', NULL, 'A'),
    (3, 'Charlie', 80, NULL),
    (4, 'Diana', NULL, NULL),
    (5, 'Eve', 90, 'B'),
    (6, 'Frank', 70, 'B'),
    (7, NULL, 85, 'A'),
    (8, NULL, NULL, 'B');

-- Test 24.1: WHERE with equality on nullable column (should not match NULLs)
SELECT * FROM t_null_advanced WHERE name = NULL;
-- Expected: empty (NULL != anything)

-- Test 24.2: WHERE with IS NULL for condition
SELECT * FROM t_null_advanced WHERE name IS NULL ORDER BY id;
-- Expected: ids 7,8

-- Test 24.3: WHERE with NOT NULL on all nullable columns
SELECT * FROM t_null_advanced WHERE name IS NOT NULL AND score IS NOT NULL AND category IS NOT NULL ORDER BY id;
-- Expected: ids 1,5,6 (completely non-null rows)

-- Test 24.4: WHERE with IS NULL on multiple columns (OR)
SELECT * FROM t_null_advanced WHERE name IS NULL OR category IS NULL ORDER BY id;
-- Expected: id 3 (cat NULL), id 4 (cat NULL, score NULL), id 7 (name NULL), id 8 (name NULL, score NULL, cat B)

-- Test 24.5: ORDER BY with NULL values (ASC default)
SELECT * FROM t_null_advanced ORDER BY name;
-- Expected: NULL names typically last in ASC order

-- Test 24.6: ORDER BY with NULL values (DESC)
SELECT * FROM t_null_advanced ORDER BY name DESC;
-- Expected: NULL names typically first in DESC order

-- Test 24.7: ORDER BY multiple columns with NULLs
SELECT * FROM t_null_advanced ORDER BY category, score;

-- Test 24.8: WHERE with comparison (NULLs excluded)
SELECT * FROM t_null_advanced WHERE score > 80 ORDER BY id;
-- Expected: ids 1 (100), 5 (90), 7 (85) -- NULL scores excluded

-- Test 24.9: WHERE with BETWEEN (NULLs excluded)
SELECT * FROM t_null_advanced WHERE score BETWEEN 80 AND 100 ORDER BY id;
-- Expected: ids 1 (100), 3 (80), 5 (90), 7 (85) -- NULL scores excluded

-- Test 24.10: IN with NULL values in subquery
-- NULL handling in IN subqueries
SELECT * FROM t_null_advanced WHERE id IN (SELECT id FROM t_null_advanced WHERE score IS NULL) ORDER BY id;
-- Expected: ids 2,4,8 (rows where score IS NULL)

-- Test 24.11: NOT IN with NULL values
-- Note: NOT IN with NULL values in subquery can be tricky
SELECT * FROM t_null_advanced WHERE id NOT IN (SELECT id FROM t_null_advanced WHERE score > 80) ORDER BY id;
-- Expected: ids not in {1,5,7}: ids 2,3,4,6,8

-- Test 24.12: COALESCE with NULLs
SELECT id, COALESCE(name, 'No Name') AS display_name,
       COALESCE(CAST(score AS VARCHAR), 'No Score') AS display_score,
       COALESCE(category, 'No Cat') AS display_cat
FROM t_null_advanced ORDER BY id;
-- Expected: all NULLs replaced with default values

-- Test 24.13: CASE WHEN with NULL comparisons
SELECT id, name,
    CASE
        WHEN name IS NULL THEN 'Missing'
        WHEN name = 'Alice' THEN 'Found Alice'
        ELSE 'Other'
    END AS name_status
FROM t_null_advanced ORDER BY id;
-- Expected: (1, Alice, Found Alice), (2, Bob, Other), ..., (7, null, Missing), (8, null, Missing)

-- Test 24.14: Arithmetic with NULLs (NULL propagates)
SELECT id, score, score + 10 AS plus_ten FROM t_null_advanced ORDER BY id;
-- Expected: NULL scores produce NULL results

-- Test 24.15: String concat with NULLs
SELECT id, name || ' (' || category || ')' AS full FROM t_null_advanced ORDER BY id;
-- Expected: NULL components produce NULL result (depending on concat behavior)

DROP TABLE t_null_advanced;

-- ============================================================================
-- Section 25: Complex Nested Subqueries
-- ============================================================================

CREATE TABLE t_nested_a (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
CREATE TABLE t_nested_b (id INT, a_id INT, label VARCHAR(20)) DISTRIBUTED BY HASH(id) BUCKETS 3;
CREATE TABLE t_nested_c (id INT, b_id INT, amount DECIMAL(10,2)) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_nested_a VALUES (1, 10), (2, 20), (3, 30), (4, 40), (5, 50);
INSERT INTO t_nested_b VALUES (1, 1, 'X'), (2, 1, 'Y'), (3, 2, 'X'), (4, 3, 'Z'), (5, 5, 'X');
INSERT INTO t_nested_c VALUES
    (1, 1, 100.00), (2, 1, 200.00), (3, 2, 150.00),
    (4, 3, 300.00), (5, 4, 250.00), (6, 5, 180.00);

-- Test 25.1: Three-level nested subquery with IN
SELECT * FROM t_nested_a WHERE id IN (
    SELECT a_id FROM t_nested_b WHERE id IN (
        SELECT b_id FROM t_nested_c WHERE amount > 150
    )
) ORDER BY id;
-- Expected: a_ids from b entries that have c entries with amount > 150
-- c with amount > 150: (2, 150), (3, 300), (4, 250), (5, 180), (6, 180) -> b_ids 2,3,4,5,6
-- b with id in {2,3,4,5,6}: a_ids = {1, 2, 3, 5}
-- a with id in {1,2,3,5}: rows 1,2,3,5

-- Test 25.2: Three-level nested subquery with EXISTS
SELECT * FROM t_nested_a a WHERE EXISTS (
    SELECT 1 FROM t_nested_b b WHERE b.a_id = a.id AND EXISTS (
        SELECT 1 FROM t_nested_c c WHERE c.b_id = b.id AND c.amount > 100
    )
) ORDER BY a.id;
-- Expected: a rows that have b entries with c entries > 100
-- Each a_id with path: a2 has no b entries -> excluded
-- a1 has b1 (c1=100 -> not > 100, c2=200 -> ok), b2 (c3=150 -> ok) -> included
-- a3 has b4 (c5=250 -> ok) -> included
-- a5 has b5 (c6=180 -> ok) -> included
-- a4 has no b entries -> excluded
-- Result: a ids 1,3,5

-- Test 25.3: Nested subquery with aggregate in middle
SELECT * FROM t_nested_a WHERE id IN (
    SELECT a_id FROM t_nested_b WHERE a_id IN (
        SELECT a_id FROM t_nested_b GROUP BY a_id HAVING COUNT(*) > 1
    )
) ORDER BY id;
-- Expected: a_ids with multiple b entries (a_id 1 has 2 entries) -> id=1

-- Test 25.4: Nested subquery in SELECT
SELECT a.id,
    (SELECT COUNT(*) FROM t_nested_b b WHERE b.a_id = a.id) AS b_count,
    (SELECT SUM(c.amount) FROM t_nested_c c
     WHERE c.b_id IN (SELECT b2.id FROM t_nested_b b2 WHERE b2.a_id = a.id)
    ) AS c_total
FROM t_nested_a a ORDER BY a.id;
-- Expected: id1: b_count=2, c_total=450; id2: b_count=1, c_total=300; id3: b_count=1, c_total=250;
--           id4: b_count=0, c_total=NULL; id5: b_count=1, c_total=180

-- Test 25.5: Nested subquery with NOT EXISTS
SELECT * FROM t_nested_a a WHERE NOT EXISTS (
    SELECT 1 FROM t_nested_b b WHERE b.a_id = a.id AND NOT EXISTS (
        SELECT 1 FROM t_nested_c c WHERE c.b_id = b.id
    )
) ORDER BY a.id;
-- Expected: a rows where ALL their b entries have at least one c entry
-- a1 has b1 (has c1,c2) and b2 (has c3) -> all b have c -> included
-- a2 no b entries -> vacuously true -> included
-- a3 has b4 (has c5) -> included
-- a4 no b entries -> included
-- a5 has b5 (has c6) -> included
-- All 5 rows (no b entry lacks a c entry)

-- Re-check: a_id=1, b1 has c1,c2 -> ok; b2 has c3 -> ok. a_id=2, b3 is for a_id=2? NO, b3 is a_id=2
-- Wait: t_nested_b: (1,1,X), (2,1,Y), (3,2,X), (4,3,Z), (5,5,X)
-- a_id=1 -> b_ids 1,2 both have c entries -> ok
-- a_id=2 -> b_id 3, c_id 4 -> ok
-- a_id=3 -> b_id 4, c_id 5 -> ok
-- a_id=4 -> no b entries -> vacuously true
-- a_id=5 -> b_id 5, c_id 6 -> ok
-- All 5 rows indeed

-- Test 25.6: Deeply nested correlated subquery
SELECT a.id, a.val,
    (SELECT COUNT(*) FROM t_nested_b b
     WHERE b.a_id = a.id AND b.label IN (
         SELECT c.label FROM (SELECT 'X' AS label) AS c
     )
    ) AS x_count
FROM t_nested_a a ORDER BY a.id;
-- Expected: count of b entries with label='X' for each a
-- a1: b1(X), b2(Y) -> 1; a2: b3(X) -> 1; a3: 0; a4: 0; a5: b5(X) -> 1

-- Test 25.7: Three-way join via subqueries
SELECT a.id, a.val,
    COALESCE((SELECT SUM(c.amount) FROM t_nested_c c WHERE c.b_id IN (
        SELECT b.id FROM t_nested_b b WHERE b.a_id = a.id
    )), 0) AS total_amount
FROM t_nested_a a ORDER BY a.id;
-- Expected: id1: 450, id2: 300, id3: 250, id4: 0, id5: 180

-- Test 25.8: Subquery with DISTINCT in WHERE
SELECT * FROM t_nested_b WHERE a_id IN (
    SELECT DISTINCT a_id FROM t_nested_b WHERE label = 'X'
) ORDER BY id;
-- Expected: all b rows whose a_id has at least one b with label='X'
-- a_ids with label='X': 1,2,5 -> b rows: 1,2,3,5 (and a_id=4 is not included)
-- Wait: b1 (a_id=1, X), b3 (a_id=2, X), b5 (a_id=5, X) -> a_ids {1,2,5}
-- b rows with a_id in {1,2,5}: b1, b2 (a_id=1), b3 (a_id=2), b5 (a_id=5) -> 4 rows

-- Test 25.9: Subquery with aggregate comparison
SELECT * FROM t_nested_a WHERE val > (
    SELECT AVG(val) FROM t_nested_a WHERE id IN (
        SELECT a_id FROM t_nested_b
    )
) ORDER BY id;
-- Expected: a rows with val > avg val of a_ids referenced in b
-- a_ids in b: {1,2,3,5}, avg = (10+20+30+50)/4 = 27.5
-- a rows with val > 27.5: ids 3 (30), 4(40), 5(50) -> but a4 not in b, so check inner subquery only
-- Actually inner subquery returns avg over a_ids {1,2,3,5} = 27.5
-- Outer: val > 27.5 from ALL a: ids 3(30), 4(40), 5(50)

-- Test 25.10: Nested subquery with HAVING
SELECT a_id, COUNT(*) AS cnt FROM t_nested_b
GROUP BY a_id
HAVING COUNT(*) > (
    SELECT AVG(cnt) FROM (
        SELECT COUNT(*) AS cnt FROM t_nested_b GROUP BY a_id
    ) AS subq
) ORDER BY a_id;
-- Expected: a_ids with b_count > avg b_count
-- counts: a1=2, a2=1, a3=1, a4=0, a5=1 -> avg = (2+1+1+0+1)/5 = 1
-- a_ids with count > 1: a_id=1 only

DROP TABLE t_nested_a;
DROP TABLE t_nested_b;
DROP TABLE t_nested_c;

-- ============================================================================
-- Section 26: ORDER BY with Expressions and Edge Cases
-- ============================================================================

-- Test 26.1: ORDER BY with arithmetic expression
CREATE TABLE t_ord_expr (id INT, a INT, b INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_ord_expr VALUES (1, 10, 3), (2, 20, 5), (3, 15, 2), (4, 5, 10);
SELECT * FROM t_ord_expr ORDER BY a + b DESC;
-- Expected: ordered by sum descending: (2:25), (3:17), (1:13), (4:15)? Actually (4:15), (1:13) -> 2,4,3,1

-- Test 26.2: ORDER BY with modulo
SELECT * FROM t_ord_expr ORDER BY a % b, a;
-- Expected: ordered by a % b then a

-- Test 26.3: ORDER BY with string length
SELECT id, a, b, a * b AS product FROM t_ord_expr ORDER BY LENGTH(CAST(product AS VARCHAR)) DESC;

-- Test 26.4: ORDER BY with CASE expression
SELECT id, a, b,
    CASE WHEN a > b THEN a ELSE b END AS greater
FROM t_ord_expr ORDER BY greater DESC;
-- Expected: ordered by greater of a,b descending: (2:20), (3:15), (4:10), (1:10)

-- Test 26.5: ORDER BY with COALESCE
CREATE TABLE t_ord_null (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_ord_null VALUES (1, NULL), (2, 10), (3, NULL), (4, 5);
SELECT * FROM t_ord_null ORDER BY COALESCE(val, 999);
-- Expected: (4,5), (2,10), (1,null->999), (3,null->999)

-- Test 26.6: ORDER BY with multiple expressions
SELECT * FROM t_ord_expr ORDER BY a + b DESC, a - b ASC;

-- Test 26.7: ORDER BY with computed column alias
SELECT id, a * b AS product FROM t_ord_expr ORDER BY product DESC;

-- Test 26.8: ORDER BY DESC on expression
SELECT id, a + b AS sum FROM t_ord_expr ORDER BY sum DESC;

-- Test 26.9: ORDER BY ASC on expression
SELECT id, a + b AS sum FROM t_ord_expr ORDER BY sum ASC;

-- Test 26.10: ORDER BY with CAST
SELECT * FROM t_ord_expr ORDER BY CAST(a AS VARCHAR);
-- Expected: ordered lexicographically: 10, 15, 20, 5 (wait, '10','15','20','5' -> '10','15','20','5')

-- Test 26.11: ORDER BY with CASE in one sort key
SELECT id, a, b FROM t_ord_expr
ORDER BY CASE WHEN a > b THEN a ELSE b END DESC, CASE WHEN a < b THEN a ELSE b END DESC;

-- Test 26.12: ORDER BY with subquery
SELECT * FROM t_ord_expr ORDER BY (SELECT MAX(a) FROM t_ord_expr) - a;

-- Test 26.13: ORDER BY with ternary-like expression via CASE
SELECT id, a, b,
    CASE WHEN a > 15 THEN 'large' WHEN a > 10 THEN 'medium' ELSE 'small' END AS size
FROM t_ord_expr
ORDER BY CASE size WHEN 'large' THEN 1 WHEN 'medium' THEN 2 WHEN 'small' THEN 3 END;

-- Test 26.14: ORDER BY with NULL handling via COALESCE
SELECT * FROM t_ord_null ORDER BY COALESCE(val, 0);
-- Expected: (4,5), (2,10), (1,0), (3,0) -- but actual: NULL COALESCE to 999 in 26.5

-- Test 26.15: ORDER BY DESC with NULLs first
SELECT * FROM t_ord_null ORDER BY val DESC;

DROP TABLE t_ord_expr;
DROP TABLE t_ord_null;

-- ============================================================================
-- Section 27: LIMIT/OFFSET Edge Cases
-- ============================================================================

-- Test 27.1: LIMIT with exact row count
CREATE TABLE t_limit_edge (id INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_limit_edge VALUES (1), (2), (3), (4), (5);
SELECT * FROM t_limit_edge ORDER BY id LIMIT 5;
-- Expected: all 5 rows

-- Test 27.2: LIMIT with more rows than table
SELECT * FROM t_limit_edge ORDER BY id LIMIT 100;
-- Expected: all 5 rows

-- Test 27.3: LIMIT 1 with OFFSET 0
SELECT * FROM t_limit_edge ORDER BY id LIMIT 1 OFFSET 0;
-- Expected: (1)

-- Test 27.4: LIMIT 1 with OFFSET last
SELECT * FROM t_limit_edge ORDER BY id LIMIT 1 OFFSET 4;
-- Expected: (5)

-- Test 27.5: LIMIT with OFFSET at exact boundary
SELECT * FROM t_limit_edge ORDER BY id LIMIT 2 OFFSET 3;
-- Expected: (4), (5)

-- Test 27.6: OFFSET 0
SELECT * FROM t_limit_edge ORDER BY id OFFSET 0;
-- Expected: all rows

-- Test 27.7: LIMIT 0 with OFFSET
SELECT * FROM t_limit_edge ORDER BY id LIMIT 0 OFFSET 2;
-- Expected: empty

-- Test 27.8: LIMIT with WHERE returning fewer rows than LIMIT
SELECT * FROM t_limit_edge WHERE id > 3 ORDER BY id LIMIT 10;
-- Expected: (4), (5)

-- Test 27.9: LIMIT after OFFSET exhausting result
SELECT * FROM t_limit_edge ORDER BY id LIMIT 2 OFFSET 100;
-- Expected: empty

-- Test 27.10: Chained operations: WHERE -> ORDER BY -> LIMIT -> OFFSET
SELECT * FROM t_limit_edge WHERE id > 1 ORDER BY id DESC LIMIT 2 OFFSET 1;
-- Expected: sorted desc: 5,4,3,2; offset 1: 4,3; limit 2: (4), (3)
-- Wait: ORDER BY id DESC -> 5,4,3,2; OFFSET 1 -> 4,3,2; LIMIT 2 -> (4),(3)

DROP TABLE t_limit_edge;

-- ============================================================================
-- Section 28: Misc Edge Cases
-- ============================================================================

-- Test 28.1: SELECT with all supported types
CREATE TABLE t_all_types (
    id INT,
    b BOOLEAN,
    ti TINYINT,
    si SMALLINT,
    i INT,
    bi BIGINT,
    f FLOAT,
    d DOUBLE,
    dec_col DECIMAL(10,2),
    vc VARCHAR(20),
    txt TEXT,
    dt DATE,
    dttm DATETIME
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_all_types VALUES (
    1, TRUE, 127, 32767, 2147483647, 9223372036854775807,
    3.14, 2.718281828, 123.45, 'hello', 'long text here',
    '2024-01-15', '2024-01-15 10:30:00'
);
SELECT * FROM t_all_types;
-- Expected: single row with all type columns

-- Test 28.2: SELECT with multiple NULLs across types
INSERT INTO t_all_types VALUES (
    2, NULL, NULL, NULL, NULL, NULL,
    NULL, NULL, NULL, NULL, NULL,
    NULL, NULL
);
SELECT * FROM t_all_types WHERE id = 2;
-- Expected: row with all NULLs

-- Test 28.3: SELECT with very long string in VARCHAR
CREATE TABLE t_long_str (id INT, val VARCHAR(1000)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_long_str VALUES (1, REPEAT('A', 500));
SELECT id, LENGTH(val) AS len FROM t_long_str;
-- Expected: (1, 500)

-- Test 28.4: SELECT with negative numbers
CREATE TABLE t_neg (id INT, a INT, b DECIMAL(10,2)) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_neg VALUES (1, -100, -99.99), (2, -1, -0.01), (3, 0, 0.00);
SELECT * FROM t_neg ORDER BY id;

-- Test 28.5: SELECT with zero values
SELECT * FROM t_neg WHERE a = 0;
-- Expected: (3, 0, 0.00)

-- Test 28.6: SELECT with boundary values
CREATE TABLE t_bounds (id INT, i INT, bi BIGINT, f FLOAT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_bounds VALUES
    (1, 0, 0, 0.0),
    (2, 2147483647, 9223372036854775807, 3.40282347e+38),
    (3, -2147483648, -9223372036854775808, -3.40282347e+38);
SELECT * FROM t_bounds ORDER BY id;

-- Test 28.7: ORDER BY with all-boundary values
SELECT * FROM t_bounds ORDER BY bi DESC;
-- Expected: (2, max), (1, 0), (3, min)

-- Test 28.8: WHERE with boolean expression
SELECT * FROM t_bounds WHERE i > 0 OR i < 0 ORDER BY id;
-- Expected: ids 2,3 (all non-zero)

-- Test 28.9: Scalar subquery returning no rows (NULL)
CREATE TABLE t_sub_empty (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_sub_empty VALUES (1, 10);
SELECT id, (SELECT val FROM t_sub_empty WHERE id = 999) AS missing FROM t_sub_empty;
-- Expected: (1, NULL)

-- Test 28.10: IN subquery returning no rows
SELECT * FROM t_sub_empty WHERE id IN (SELECT id FROM t_sub_empty WHERE val = 999);
-- Expected: empty

-- Test 28.11: EXISTS with subquery returning no rows
SELECT * FROM t_sub_empty WHERE EXISTS (SELECT 1 FROM t_sub_empty WHERE val = 999);
-- Expected: empty

-- Test 28.12: NOT EXISTS with subquery returning no rows (should return all)
SELECT * FROM t_sub_empty WHERE NOT EXISTS (SELECT 1 FROM t_sub_empty WHERE val = 999);
-- Expected: (1, 10)

-- Test 28.13: WHERE with LIKE on empty column
CREATE TABLE t_like_empty (id INT, name VARCHAR(50)) DISTRIBUTED BY HASH(id) BUCKETS 3;
SELECT * FROM t_like_empty WHERE name LIKE '%test%';
-- Expected: empty

-- Test 28.14: ORDER BY with all rows having same value
CREATE TABLE t_identical (id INT, val INT) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t_identical VALUES (1, 5), (2, 5), (3, 5);
SELECT * FROM t_identical ORDER BY val, id;
-- Expected: all 3 rows, ordered by id (since val is same)

-- Test 28.15: DISTINCT with all same values
SELECT DISTINCT val FROM t_identical;
-- Expected: (5)

-- Test 28.16: LIMIT on single-distinct-value result
SELECT DISTINCT val FROM t_identical LIMIT 1;
-- Expected: (5)

-- Test 28.17: Multiple ORDER BY keys with one DESC
CREATE TABLE t_multi_sort (a INT, b INT, c INT) DISTRIBUTED BY HASH(a) BUCKETS 3;
INSERT INTO t_multi_sort VALUES (1, 3, 10), (1, 2, 20), (1, 1, 30), (2, 1, 40), (2, 2, 50);
SELECT * FROM t_multi_sort ORDER BY a ASC, b DESC, c ASC;
-- Expected: a:1, then b desc: (1,3,10), (1,2,20), (1,1,30); then a:2, b desc: (2,2,50), (2,1,40)

-- Test 28.18: ORDER BY on all columns
SELECT * FROM t_multi_sort ORDER BY a, b, c;

-- Test 28.19: ORDER BY on columns not in SELECT
SELECT a, b FROM t_multi_sort ORDER BY c DESC;
-- Expected: ordered by c descending (30,20,10,50,40)

-- Test 28.20: SELECT with duplicate column names (via alias)
SELECT a AS x, b AS x, c AS x FROM t_multi_sort ORDER BY a;
-- Expected: works with duplicate alias names

DROP TABLE t_all_types;
DROP TABLE t_long_str;
DROP TABLE t_neg;
DROP TABLE t_bounds;
DROP TABLE t_sub_empty;
DROP TABLE t_like_empty;
DROP TABLE t_identical;
DROP TABLE t_multi_sort;

-- ============================================================================
-- Cleanup
-- ============================================================================

DROP DATABASE e2e_select_test;

-- ============================================================================
-- Summary
-- ============================================================================
SELECT 'E2E SELECT Queries Test Completed Successfully' AS status;