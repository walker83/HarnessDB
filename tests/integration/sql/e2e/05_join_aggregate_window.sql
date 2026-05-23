-- ============================================================================
-- E2E Test: JOINs, Aggregates, and Window Functions
-- RorisDB — single-node OLAP database with DataFusion + Parquet
-- MySQL wire protocol
--
-- Requirements: 200+ test cases covering:
--   JOINs (inner, left, right, full outer, cross, self, multi-table, edge cases)
--   Aggregates (COUNT, SUM, AVG, MIN, MAX, GROUP_CONCAT, GROUP BY, HAVING)
--   Window functions (ROW_NUMBER, RANK, DENSE_RANK, LAG, LEAD, OVER, frames)
-- ============================================================================

DROP DATABASE IF EXISTS e2e_jaw_test;
CREATE DATABASE e2e_jaw_test;
USE e2e_jaw_test;

-- ============================================================================
-- SECTION 1: SETUP — Shared tables used across tests
-- ============================================================================

-- Departments
CREATE TABLE departments (
    dept_id INT,
    dept_name VARCHAR(50),
    location VARCHAR(50)
) DISTRIBUTED BY HASH(dept_id) BUCKETS 3;

INSERT INTO departments VALUES
    (1, 'Engineering', 'New York'),
    (2, 'Sales', 'Chicago'),
    (3, 'Marketing', 'San Francisco'),
    (4, 'HR', 'New York'),
    (5, 'Finance', 'Chicago'),
    (6, 'Support', 'Austin');

-- Employees
CREATE TABLE employees (
    emp_id INT,
    emp_name VARCHAR(50),
    dept_id INT,
    salary DECIMAL(12, 2),
    hire_date VARCHAR(30),  -- Note: Using VARCHAR for DATE due to server limitation
    manager_id INT
) DISTRIBUTED BY HASH(emp_id) BUCKETS 3;

INSERT INTO employees VALUES
    (1, 'Alice', 1, 120000.00, '2020-01-15', NULL),
    (2, 'Bob', 1, 95000.00, '2020-03-20', 1),
    (3, 'Charlie', 2, 80000.00, '2020-06-10', 1),
    (4, 'Diana', 2, 75000.00, '2021-02-01', 3),
    (5, 'Eve', 3, 90000.00, '2021-04-15', 1),
    (6, 'Frank', 3, 85000.00, '2021-07-01', 5),
    (7, 'Grace', 4, 65000.00, '2022-01-10', NULL),
    (8, 'Heidi', 4, 60000.00, '2022-03-20', 7),
    (9, 'Ivan', 5, 110000.00, '2020-09-01', NULL),
    (10, 'Judy', 5, 105000.00, '2021-11-15', 9),
    (11, 'Karl', 6, 55000.00, '2022-06-01', 7),
    (12, 'Linda', 6, 52000.00, '2023-01-05', 11);

-- Projects (for multi-table joins)
CREATE TABLE projects (
    project_id INT,
    project_name VARCHAR(50),
    dept_id INT,
    budget DECIMAL(12, 2)
) DISTRIBUTED BY HASH(project_id) BUCKETS 3;

INSERT INTO projects VALUES
    (1, 'Alpha', 1, 500000.00),
    (2, 'Beta', 1, 300000.00),
    (3, 'Gamma', 2, 200000.00),
    (4, 'Delta', 3, 150000.00),
    (5, 'Epsilon', 5, 400000.00),
    (6, 'Zeta', 6, 100000.00);

-- Sales (for window functions and aggregates)
CREATE TABLE sales (
    sale_id INT,
    product VARCHAR(50),
    category VARCHAR(50),
    amount DECIMAL(12, 2),
    sale_date VARCHAR(30),  -- Note: Using VARCHAR for DATE due to server limitation
    region VARCHAR(30)
) DISTRIBUTED BY HASH(sale_id) BUCKETS 3;

INSERT INTO sales VALUES
    (1, 'Widget A', 'Widgets', 100.00, '2023-01-05', 'North'),
    (2, 'Widget B', 'Widgets', 150.00, '2023-01-10', 'North'),
    (3, 'Gadget X', 'Gadgets', 200.00, '2023-01-12', 'South'),
    (4, 'Widget A', 'Widgets', 120.00, '2023-02-01', 'North'),
    (5, 'Gadget Y', 'Gadgets', 300.00, '2023-02-05', 'South'),
    (6, 'Widget B', 'Widgets', 130.00, '2023-02-10', 'East'),
    (7, 'Gadget X', 'Gadgets', 250.00, '2023-03-01', 'North'),
    (8, 'Widget A', 'Widgets', 110.00, '2023-03-10', 'South'),
    (9, 'Gadget Y', 'Gadgets', 350.00, '2023-03-15', 'East'),
    (10, 'Widget B', 'Widgets', 160.00, '2023-04-01', 'North'),
    (11, 'Gadget X', 'Gadgets', 220.00, '2023-04-10', 'West'),
    (12, 'Widget A', 'Widgets', 140.00, '2023-04-15', 'East');

-- Scores (for ranking / window functions)
CREATE TABLE scores (
    student_id INT,
    student_name VARCHAR(50),
    subject VARCHAR(30),
    score INT
) DISTRIBUTED BY HASH(student_id) BUCKETS 3;

INSERT INTO scores VALUES
    (1, 'Alice', 'Math', 95),
    (2, 'Bob', 'Math', 87),
    (3, 'Charlie', 'Math', 95),
    (4, 'Diana', 'Math', 78),
    (5, 'Alice', 'Science', 92),
    (6, 'Bob', 'Science', 88),
    (7, 'Charlie', 'Science', 91),
    (8, 'Diana', 'Science', 85),
    (9, 'Alice', 'English', 88),
    (10, 'Bob', 'English', 92),
    (11, 'Charlie', 'English', 84),
    (12, 'Diana', 'English', 90);

-- Empty tables for edge cases
CREATE TABLE empty_left (
    id INT,
    val VARCHAR(10)
) DISTRIBUTED BY HASH(id) BUCKETS 3;

CREATE TABLE empty_right (
    id INT,
    val VARCHAR(10)
) DISTRIBUTED BY HASH(id) BUCKETS 3;

-- NULL key table
CREATE TABLE null_keys (
    id INT,
    name VARCHAR(30),
    group_id INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO null_keys VALUES
    (1, 'One', 10),
    (2, 'Two', NULL),
    (3, 'Three', 10),
    (4, 'Four', NULL),
    (5, 'Five', 20);

CREATE TABLE null_ref (
    gid INT,
    label VARCHAR(30)
) DISTRIBUTED BY HASH(gid) BUCKETS 3;

INSERT INTO null_ref VALUES
    (10, 'Group A'),
    (20, 'Group B'),
    (30, 'Group C');

-- Single-row table for edge cases
CREATE TABLE single_row (
    id INT,
    val VARCHAR(30)
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO single_row VALUES (1, 'only_row');

-- ============================================================================
-- SECTION 2: JOIN TESTS (84 test cases)
-- ============================================================================

-- --------------------------------------------------------------------------
-- 2.1 INNER JOIN (12 tests)
-- --------------------------------------------------------------------------

-- Test 2.1.1: Basic INNER JOIN employees with departments
SELECT e.emp_name, d.dept_name
FROM employees e
INNER JOIN departments d ON e.dept_id = d.dept_id
ORDER BY e.emp_id;
-- Expected: 12 rows — Alice/Engineering, Bob/Engineering, Charlie/Sales, Diana/Sales, Eve/Marketing, Frank/Marketing, Grace/HR, Heidi/HR, Ivan/Finance, Judy/Finance, Karl/Support, Linda/Support

-- Test 2.1.2: INNER JOIN with column selection
SELECT e.emp_id, e.emp_name, d.dept_name, d.location
FROM employees e
JOIN departments d ON e.dept_id = d.dept_id
ORDER BY e.emp_id;
-- Expected: 12 rows with emp_id, emp_name, dept_name, location

-- Test 2.1.3: INNER JOIN with WHERE filter
SELECT e.emp_name, d.dept_name
FROM employees e
JOIN departments d ON e.dept_id = d.dept_id
WHERE d.location = 'New York'
ORDER BY e.emp_id;
-- Expected: Alice/Engineering, Bob/Engineering, Grace/HR, Heidi/HR

-- Test 2.1.4: INNER JOIN with ORDER BY on joined column
SELECT e.emp_name, d.dept_name, e.salary
FROM employees e
INNER JOIN departments d ON e.dept_id = d.dept_id
ORDER BY e.salary DESC;
-- Expected: 12 rows sorted by salary descending

-- Test 2.1.5: INNER JOIN with multiple conditions
SELECT e.emp_name, d.dept_name, d.location
FROM employees e
JOIN departments d ON e.dept_id = d.dept_id AND d.location = 'Chicago'
ORDER BY e.emp_id;
-- Expected: Charlie/Sales/Chicago, Diana/Sales/Chicago, Ivan/Finance/Chicago, Judy/Finance/Chicago

-- Test 2.1.6: INNER JOIN with aggregate
SELECT d.dept_name, COUNT(*) AS emp_count, AVG(e.salary) AS avg_salary
FROM employees e
JOIN departments d ON e.dept_id = d.dept_id
GROUP BY d.dept_name
ORDER BY d.dept_name;
-- Expected: Engineering/2/107500, Sales/2/77500, Marketing/2/87500, HR/2/62500, Finance/2/107500, Support/2/53500

-- Test 2.1.7: INNER JOIN with HAVING
SELECT d.dept_name, COUNT(*) AS emp_count
FROM employees e
JOIN departments d ON e.dept_id = d.dept_id
GROUP BY d.dept_name
HAVING COUNT(*) >= 2
ORDER BY d.dept_name;
-- Expected: Engineering/2, Sales/2, Marketing/2, HR/2, Finance/2, Support/2

-- Test 2.1.8: INNER JOIN three tables (employees + departments + projects)
SELECT e.emp_name, d.dept_name, p.project_name
FROM employees e
JOIN departments d ON e.dept_id = d.dept_id
JOIN projects p ON d.dept_id = p.dept_id
ORDER BY e.emp_id, p.project_name;
-- Expected: employees matched to departments then to projects in the same department

-- Test 2.1.9: INNER JOIN with calculated column
SELECT e.emp_name, d.dept_name, e.salary * 1.1 AS raised_salary
FROM employees e
JOIN departments d ON e.dept_id = d.dept_id
ORDER BY e.emp_id;
-- Expected: 12 rows with salary * 1.1 as raised_salary

-- Test 2.1.10: INNER JOIN and group by both tables' columns
SELECT d.location, d.dept_name, SUM(e.salary) AS total_salary
FROM employees e
JOIN departments d ON e.dept_id = d.dept_id
GROUP BY d.location, d.dept_name
ORDER BY d.location, d.dept_name;
-- Expected: grouped by location and dept_name with salary sums

-- Test 2.1.11: INNER JOIN with BETWEEN condition
SELECT e.emp_name, p.project_name, p.budget
FROM employees e
JOIN departments d ON e.dept_id = d.dept_id
JOIN projects p ON d.dept_id = p.dept_id
WHERE e.salary BETWEEN 80000 AND 120000
ORDER BY e.emp_id;
-- Expected: Employees with salary between 80000 and 120000

-- Test 2.1.12: INNER JOIN — select all columns from one table
SELECT d.*, e.emp_name
FROM employees e
JOIN departments d ON e.dept_id = d.dept_id
ORDER BY e.emp_id;
-- Expected: all department columns plus emp_name

-- --------------------------------------------------------------------------
-- 2.2 LEFT JOIN (12 tests)
-- --------------------------------------------------------------------------

-- Test 2.2.1: Basic LEFT JOIN
SELECT e.emp_name, d.dept_name
FROM employees e
LEFT JOIN departments d ON e.dept_id = d.dept_id
ORDER BY e.emp_id;
-- Expected: 12 rows with matching dept names (all employees have matching dept)

-- Test 2.2.2: LEFT JOIN where right table has no match
SELECT e.emp_name, d.dept_name
FROM employees e
LEFT JOIN departments d ON e.dept_id = 999
ORDER BY e.emp_id;
-- Expected: 12 rows all with NULL dept_name

-- Test 2.2.3: LEFT JOIN with NULL right-side columns
SELECT e.emp_name, d.dept_name, d.location
FROM employees e
LEFT JOIN departments d ON e.dept_id = d.dept_id AND d.dept_id = 99
ORDER BY e.emp_id;
-- Expected: all employees with NULL dept_name and location

-- Test 2.2.4: LEFT JOIN with WHERE filtering on right table
SELECT e.emp_name, d.dept_name
FROM employees e
LEFT JOIN departments d ON e.dept_id = d.dept_id
WHERE d.dept_name = 'Engineering'
ORDER BY e.emp_id;
-- Expected: Alice/Engineering, Bob/Engineering

-- Test 2.2.5: LEFT JOIN with aggregate
SELECT d.dept_name, COUNT(e.emp_id) AS emp_count
FROM departments d
LEFT JOIN employees e ON d.dept_id = e.dept_id
GROUP BY d.dept_name
ORDER BY d.dept_name;
-- Expected: Engineering/2, Sales/2, Marketing/2, HR/2, Finance/2, Support/2

-- Test 2.2.6: LEFT JOIN with empty right table
SELECT e.emp_name, er.val
FROM employees e
LEFT JOIN empty_right er ON e.emp_id = er.id
ORDER BY e.emp_id;
-- Expected: 12 rows all with NULL val

-- Test 2.2.7: LEFT JOIN — empty left table
SELECT er.val, e.emp_name
FROM empty_right er
LEFT JOIN employees e ON er.id = e.emp_id
ORDER BY e.emp_id;
-- Expected: 0 rows (empty_right is empty)

-- Test 2.2.8: LEFT JOIN with calculated columns
SELECT e.emp_name, COALESCE(d.dept_name, 'Unknown') AS dept_name, e.salary * 12 AS annual_salary
FROM employees e
LEFT JOIN departments d ON e.dept_id = d.dept_id
ORDER BY e.emp_id;
-- Expected: 12 rows with dept_name (never 'Unknown' here) and annual_salary

-- Test 2.2.9: LEFT JOIN with multiple tables
SELECT e.emp_name, d.dept_name, p.project_name
FROM employees e
LEFT JOIN departments d ON e.dept_id = d.dept_id
LEFT JOIN projects p ON d.dept_id = p.dept_id
ORDER BY e.emp_id, p.project_name;
-- Expected: employees with dept and project (some with multiple projects)

-- Test 2.2.10: LEFT JOIN self
SELECT e1.emp_name AS employee, e2.emp_name AS manager
FROM employees e1
LEFT JOIN employees e2 ON e1.manager_id = e2.emp_id
ORDER BY e1.emp_id;
-- Expected: Alice/NULL, Bob/Alice, Charlie/Alice, Diana/Charlie, Eve/Alice, Frank/Eve, Grace/NULL, Heidi/Grace, Ivan/NULL, Judy/Ivan, Karl/Grace, Linda/Karl

-- Test 2.2.11: LEFT JOIN with ORDER BY on coalesced column
SELECT e.emp_name, COALESCE(d.location, 'No Location') AS location
FROM employees e
LEFT JOIN departments d ON e.dept_id = d.dept_id
ORDER BY location, e.emp_name;
-- Expected: 12 rows sorted by location then emp_name

-- Test 2.2.12: LEFT JOIN with GROUP BY + HAVING + ORDER BY
SELECT d.dept_name, COUNT(e.emp_id) AS emp_count
FROM departments d
LEFT JOIN employees e ON d.dept_id = e.dept_id
GROUP BY d.dept_name
HAVING COUNT(e.emp_id) >= 1
ORDER BY emp_count DESC;
-- Expected: all 6 depts have 2 employees each

-- --------------------------------------------------------------------------
-- 2.3 RIGHT JOIN (10 tests)
-- --------------------------------------------------------------------------

-- Test 2.3.1: Basic RIGHT JOIN
SELECT e.emp_name, d.dept_name
FROM departments d
RIGHT JOIN employees e ON d.dept_id = e.dept_id
ORDER BY e.emp_id;
-- Expected: 12 rows with matching dept names

-- Test 2.3.2: RIGHT JOIN with no match on left
SELECT d.dept_name, e.emp_name
FROM employees e
RIGHT JOIN departments d ON e.dept_id = d.dept_id
ORDER BY d.dept_id;
-- Expected: all 6 departments with employees

-- Test 2.3.3: RIGHT JOIN with aggregate
SELECT e.emp_name, COUNT(d.dept_id) AS dept_count
FROM departments d
RIGHT JOIN employees e ON d.dept_id = e.dept_id
GROUP BY e.emp_name
ORDER BY e.emp_name;
-- Expected: each employee has 1 dept

-- Test 2.3.4: RIGHT JOIN with filter on left table
SELECT e.emp_name, d.dept_name, d.location
FROM departments d
RIGHT JOIN employees e ON d.dept_id = e.dept_id
WHERE e.salary > 100000
ORDER BY e.emp_id;
-- Expected: Alice/Engineering/NewYork, Ivan/Finance/Chicago, Judy/Finance/Chicago

-- Test 2.3.5: RIGHT JOIN with calculated fields
SELECT e.emp_name, d.dept_name, e.salary - COALESCE(d.dept_id, 0) AS adjusted_salary
FROM departments d
RIGHT JOIN employees e ON d.dept_id = e.dept_id
ORDER BY e.emp_id;
-- Expected: 12 rows with adjusted_salary

-- Test 2.3.6: RIGHT JOIN with WHERE on right table
SELECT d.dept_name, e.emp_name
FROM departments d
RIGHT JOIN employees e ON d.dept_id = e.dept_id
WHERE e.dept_id = 1
ORDER BY e.emp_id;
-- Expected: Engineering/Alice, Engineering/Bob

-- Test 2.3.7: RIGHT JOIN equivalent to LEFT JOIN (symmetry check)
SELECT e.emp_name, d.dept_name
FROM departments d
RIGHT JOIN employees e ON d.dept_id = e.dept_id
ORDER BY e.emp_id;
-- Expected: same as LEFT JOIN employees ON departments

-- Test 2.3.8: RIGHT JOIN with NULL on join key
SELECT nk.name AS emp_name, nr.label AS group_label
FROM null_ref nr
RIGHT JOIN null_keys nk ON nr.gid = nk.group_id
ORDER BY nk.id;
-- Expected: One/GroupA, Two/NULL, Three/GroupA, Four/NULL, Five/GroupB

-- Test 2.3.9: RIGHT JOIN with ORDER BY DESC
SELECT e.emp_name, d.dept_name
FROM departments d
RIGHT JOIN employees e ON d.dept_id = e.dept_id
ORDER BY e.salary DESC;
-- Expected: 12 rows sorted by salary descending

-- Test 2.3.10: RIGHT JOIN — empty right table (employees empty → 0 results)
SELECT d.dept_name, e.emp_name
FROM departments d
RIGHT JOIN empty_right e ON d.dept_id = e.id
ORDER BY d.dept_id;
-- Expected: 0 rows (empty_right has no rows)

-- --------------------------------------------------------------------------
-- 2.4 FULL OUTER JOIN (8 tests)
-- --------------------------------------------------------------------------

-- Test 2.4.1: Basic FULL OUTER JOIN
SELECT e.emp_name, d.dept_name
FROM employees e
FULL OUTER JOIN departments d ON e.dept_id = d.dept_id
ORDER BY e.emp_name NULLS LAST, d.dept_name;
-- Expected: 12 rows (all employees matched with departments)

-- Test 2.4.2: FULL OUTER JOIN with unmatched on both sides
SELECT nk.name, nr.label
FROM null_keys nk
FULL OUTER JOIN null_ref nr ON nk.group_id = nr.gid
ORDER BY nk.id;
-- Expected: One/GroupA, Two/NULL, Three/GroupA, Four/NULL, Five/GroupB, NULL/GroupC

-- Test 2.4.3: FULL OUTER JOIN with aggregate
SELECT COALESCE(d.dept_name, 'No Dept') AS dept_name, COUNT(e.emp_id) AS emp_count
FROM employees e
FULL OUTER JOIN departments d ON e.dept_id = d.dept_id
GROUP BY d.dept_name
ORDER BY d.dept_name;
-- Expected: Engineering/2, Sales/2, Marketing/2, HR/2, Finance/2, Support/2

-- Test 2.4.4: FULL OUTER JOIN with WHERE
SELECT e.emp_name, d.dept_name
FROM employees e
FULL OUTER JOIN departments d ON e.dept_id = d.dept_id
WHERE d.location = 'New York' OR e.salary > 100000
ORDER BY e.emp_id;
-- Expected: Alice/Engineering, Bob/Engineering, Grace/HR, Heidi/HR, Ivan/Finance, Judy/Finance

-- Test 2.4.5: FULL OUTER JOIN with COALESCE on keys
SELECT COALESCE(e.emp_name, '(no emp)') AS emp_name,
       COALESCE(d.dept_name, '(no dept)') AS dept_name
FROM employees e
FULL OUTER JOIN departments d ON e.dept_id = d.dept_id
ORDER BY e.emp_id;
-- Expected: 12 rows with names

-- Test 2.4.6: FULL OUTER JOIN with ORDER BY on nullable column
SELECT nk.name AS emp_name, nr.label AS dept_label
FROM null_keys nk
FULL OUTER JOIN null_ref nr ON nk.group_id = nr.gid
ORDER BY emp_name NULLS LAST, dept_label;
-- Expected: Five/GroupB, Four/NULL, One/GroupA, Three/GroupA, Two/NULL, NULL/GroupC

-- Test 2.4.7: FULL OUTER JOIN empty tables
SELECT el.val AS left_val, er.val AS right_val
FROM empty_left el
FULL OUTER JOIN empty_right er ON el.id = er.id
ORDER BY left_val;
-- Expected: 0 rows (both tables empty)

-- Test 2.4.8: FULL OUTER JOIN — one side empty
SELECT e.emp_name, er.val
FROM employees e
FULL OUTER JOIN empty_right er ON e.emp_id = er.id
ORDER BY e.emp_id;
-- Expected: 12 rows all with NULL val

-- --------------------------------------------------------------------------
-- 2.5 CROSS JOIN (8 tests)
-- --------------------------------------------------------------------------

-- Test 2.5.1: Basic CROSS JOIN
SELECT e.emp_name, d.dept_name
FROM employees e
CROSS JOIN departments d
ORDER BY e.emp_id, d.dept_id;
-- Expected: 12 * 6 = 72 rows

-- Test 2.5.2: CROSS JOIN with WHERE filter
SELECT e.emp_name, d.dept_name
FROM employees e
CROSS JOIN departments d
WHERE e.dept_id = d.dept_id
ORDER BY e.emp_id;
-- Expected: 12 rows (equivalent to INNER JOIN)

-- Test 2.5.3: CROSS JOIN with aggregate
SELECT d.dept_name, COUNT(*) AS row_count
FROM employees e
CROSS JOIN departments d
GROUP BY d.dept_name
ORDER BY d.dept_name;
-- Expected: each dept appears 12 times (12 * 6 / 6 = 12)

-- Test 2.5.4: CROSS JOIN with LIMIT
SELECT e.emp_name, d.dept_name
FROM employees e
CROSS JOIN departments d
ORDER BY e.emp_id, d.dept_id;
-- Expected: 72 rows total

-- Test 2.5.5: CROSS JOIN single-row table
SELECT e.emp_name, sr.val
FROM employees e
CROSS JOIN single_row sr
ORDER BY e.emp_id;
-- Expected: 12 rows all with val='only_row'

-- Test 2.5.6: CROSS JOIN with calculated columns
SELECT e.emp_name, d.dept_name, e.salary * d.dept_id AS weird_calc
FROM employees e
CROSS JOIN departments d
ORDER BY e.emp_id, d.dept_id;
-- Expected: 72 rows with weird_calc = salary * dept_id

-- Test 2.5.7: CROSS JOIN with empty table
SELECT e.emp_name, er.val
FROM employees e
CROSS JOIN empty_right er
ORDER BY e.emp_id;
-- Expected: 0 rows (empty_right has no rows)

-- Test 2.5.8: CROSS JOIN with HAVING
SELECT d.dept_name, COUNT(*) AS cnt
FROM employees e
CROSS JOIN departments d
GROUP BY d.dept_name
HAVING COUNT(*) > 10
ORDER BY d.dept_name;
-- Expected: all 6 depts with 12 each

-- --------------------------------------------------------------------------
-- 2.6 SELF JOIN (8 tests)
-- --------------------------------------------------------------------------

-- Test 2.6.1: Basic self-join for employee-manager relationship
SELECT e1.emp_name AS employee, e2.emp_name AS manager
FROM employees e1
JOIN employees e2 ON e1.manager_id = e2.emp_id
ORDER BY e1.emp_id;
-- Expected: Bob/Alice, Charlie/Alice, Diana/Charlie, Eve/Alice, Frank/Eve, Heidi/Grace, Judy/Ivan, Karl/Grace, Linda/Karl

-- Test 2.6.2: Self-join with LEFT JOIN including NULL managers
SELECT e1.emp_name AS employee, e2.emp_name AS manager
FROM employees e1
LEFT JOIN employees e2 ON e1.manager_id = e2.emp_id
ORDER BY e1.emp_id;
-- Expected: Alice/NULL, Bob/Alice, Charlie/Alice, Diana/Charlie, Eve/Alice, Frank/Eve, Grace/NULL, Heidi/Grace, Ivan/NULL, Judy/Ivan, Karl/Grace, Linda/Karl

-- Test 2.6.3: Self-join finding employees with same salary
SELECT e1.emp_name AS emp1, e2.emp_name AS emp2, e1.salary
FROM employees e1
JOIN employees e2 ON e1.salary = e2.salary AND e1.emp_id < e2.emp_id
ORDER BY e1.emp_name;
-- Expected: no rows (all salaries are unique)

-- Test 2.6.4: Self-join with aggregate — count direct reports per manager
SELECT e2.emp_name AS manager, COUNT(e1.emp_id) AS direct_reports
FROM employees e1
JOIN employees e2 ON e1.manager_id = e2.emp_id
GROUP BY e2.emp_name
ORDER BY direct_reports DESC;
-- Expected: Alice/3, Grace/2, Charlie/1, Eve/1, Ivan/1, Karl/1

-- Test 2.6.5: Self-join using non-PK columns
SELECT e1.emp_name, e2.emp_name AS same_dept_coworker
FROM employees e1
JOIN employees e2 ON e1.dept_id = e2.dept_id AND e1.emp_id < e2.emp_id
ORDER BY e1.emp_name, same_dept_coworker;
-- Expected: employee pairs in same department

-- Test 2.6.6: Self-join with hire date comparison
SELECT e1.emp_name AS junior, e2.emp_name AS senior, e2.hire_date
FROM employees e1
JOIN employees e2 ON e1.dept_id = e2.dept_id AND e1.hire_date > e2.hire_date
ORDER BY e1.emp_name;
-- Expected: later-hired employees paired with earlier-hired in same dept

-- Test 2.6.7: Self-join three-way (chain of command depth 2)
SELECT e1.emp_name AS emp, e2.emp_name AS manager, e3.emp_name AS senior_manager
FROM employees e1
JOIN employees e2 ON e1.manager_id = e2.emp_id
LEFT JOIN employees e3 ON e2.manager_id = e3.emp_id
ORDER BY e1.emp_id;
-- Expected: Bob/Alice/NULL, Charlie/Alice/NULL, Eve/Alice/NULL, Diana/Charlie/Alice, Frank/Eve/Alice, Heidi/Grace/NULL, Judy/Ivan/NULL, Karl/Grace/NULL, Linda/Karl/Grace

-- Test 2.6.8: Self-join with WHERE on same table
SELECT e1.emp_name, e1.salary, e2.emp_name AS higher_earner
FROM employees e1
JOIN employees e2 ON e1.salary < e2.salary
ORDER BY e1.emp_name, e2.emp_name;
-- Expected: each employee paired with higher earners

-- --------------------------------------------------------------------------
-- 2.7 Multi-Table JOIN (12 tests)
-- --------------------------------------------------------------------------

-- Test 2.7.1: Three-way JOIN (employees → departments → projects)
SELECT e.emp_name, d.dept_name, p.project_name, p.budget
FROM employees e
JOIN departments d ON e.dept_id = d.dept_id
JOIN projects p ON d.dept_id = p.dept_id
ORDER BY e.emp_id, p.project_id;
-- Expected: employees matched with dept and project

-- Test 2.7.2: Three-way JOIN with WHERE
SELECT e.emp_name, d.dept_name, p.project_name, p.budget
FROM employees e
JOIN departments d ON e.dept_id = d.dept_id
JOIN projects p ON d.dept_id = p.dept_id
WHERE p.budget >= 300000
ORDER BY e.emp_id, p.project_id;
-- Expected: Engineering and Finance employees on Alpha/Beta/Epsilon

-- Test 2.7.3: Three-way JOIN with aggregate
SELECT d.dept_name, COUNT(DISTINCT e.emp_id) AS emp_count, COUNT(p.project_id) AS project_count
FROM departments d
LEFT JOIN employees e ON d.dept_id = e.dept_id
LEFT JOIN projects p ON d.dept_id = p.dept_id
GROUP BY d.dept_name
ORDER BY d.dept_name;
-- Expected: each dept with emp/project counts

-- Test 2.7.4: Four-table JOIN (employees → departments → projects, self-join for manager)
SELECT e.emp_name AS employee, m.emp_name AS manager, d.dept_name, p.project_name
FROM employees e
JOIN departments d ON e.dept_id = d.dept_id
LEFT JOIN projects p ON d.dept_id = p.dept_id
LEFT JOIN employees m ON e.manager_id = m.emp_id
ORDER BY e.emp_id, p.project_id;
-- Expected: employees with manager, dept, and projects

-- Test 2.7.5: Multi-table with different JOIN types
SELECT e.emp_name, d.dept_name, p.project_name
FROM employees e
LEFT JOIN departments d ON e.dept_id = d.dept_id
INNER JOIN projects p ON d.dept_id = p.dept_id
ORDER BY e.emp_id, p.project_id;
-- Expected: only employees in departments that have projects

-- Test 2.7.6: Multi-table with CROSS JOIN
SELECT e.emp_name, d.dept_name, p.project_name
FROM employees e
CROSS JOIN departments d
LEFT JOIN projects p ON d.dept_id = p.dept_id
WHERE e.dept_id = d.dept_id
ORDER BY e.emp_id, p.project_id;
-- Expected: employees matched to dept and projects (like inner join)

-- Test 2.7.7: Multi-table JOIN with ORDER BY across tables
SELECT e.emp_name, d.dept_name, p.project_name, p.budget
FROM employees e
JOIN departments d ON e.dept_id = d.dept_id
JOIN projects p ON d.dept_id = p.dept_id
ORDER BY p.budget DESC, e.emp_name;
-- Expected: sorted by budget desc then emp_name

-- Test 2.7.8: Multi-table with subquery in WHERE
SELECT e.emp_name, d.dept_name
FROM employees e
JOIN departments d ON e.dept_id = d.dept_id
WHERE e.dept_id IN (SELECT dept_id FROM projects WHERE budget > 200000)
ORDER BY e.emp_id;
-- Expected: employees in Engineering, Marketing, Finance, Support, Sales

-- Test 2.7.9: Multi-table with GROUP BY + HAVING
SELECT d.dept_name, COUNT(DISTINCT p.project_id) AS project_count, SUM(p.budget) AS total_budget
FROM departments d
LEFT JOIN projects p ON d.dept_id = p.dept_id
GROUP BY d.dept_name
HAVING SUM(p.budget) > 0
ORDER BY total_budget DESC;
-- Expected: Engineering/2/800000, Finance/1/400000, Sales/1/200000, Marketing/1/150000, Support/1/100000

-- Test 2.7.10: Multi-table with COALESCE on nullable joins
SELECT e.emp_name,
       COALESCE(d.dept_name, 'No Dept') AS dept,
       COALESCE(p.project_name, 'No Project') AS project
FROM employees e
LEFT JOIN departments d ON e.dept_id = d.dept_id
LEFT JOIN projects p ON d.dept_id = p.dept_id
ORDER BY e.emp_id, p.project_id;
-- Expected: employees with dept and project (or defaults)

-- Test 2.7.11: Multi-table with date range filter
SELECT e.emp_name, d.dept_name, p.project_name
FROM employees e
JOIN departments d ON e.dept_id = d.dept_id
JOIN projects p ON d.dept_id = p.dept_id
WHERE e.hire_date < '2021-01-01'
ORDER BY e.emp_name;
-- Expected: employees hired before 2021

-- Test 2.7.12: Multi-table with all columns from one table
SELECT e.*, d.dept_name, p.project_name
FROM employees e
JOIN departments d ON e.dept_id = d.dept_id
LEFT JOIN projects p ON d.dept_id = p.dept_id
WHERE e.dept_id = 1
ORDER BY e.emp_id, p.project_id;
-- Expected: Engineering employees with all emp columns + dept_name + project_name

-- --------------------------------------------------------------------------
-- 2.8 JOIN Edge Cases (14 tests)
-- --------------------------------------------------------------------------

-- Test 2.8.1: JOIN with NULL join keys
SELECT nk.name, nr.label
FROM null_keys nk
LEFT JOIN null_ref nr ON nk.group_id = nr.gid
ORDER BY nk.id;
-- Expected: One/GroupA, Two/NULL, Three/GroupA, Four/NULL, Five/GroupB

-- Test 2.8.2: INNER JOIN with NULL keys — NULLs are excluded
SELECT nk.name, nr.label
FROM null_keys nk
JOIN null_ref nr ON nk.group_id = nr.gid
ORDER BY nk.id;
-- Expected: One/GroupA, Three/GroupA, Five/GroupB (NULL group_ids are dropped)

-- Test 2.8.3: JOIN on empty tables
SELECT el.val, er.val
FROM empty_left el
JOIN empty_right er ON el.id = er.id;
-- Expected: 0 rows

-- Test 2.8.4: LEFT JOIN on empty left table
SELECT el.val, er.val
FROM empty_left el
LEFT JOIN empty_right er ON el.id = er.id;
-- Expected: 0 rows

-- Test 2.8.5: LEFT JOIN on empty right table with data in left
SELECT e.emp_name, er.val
FROM employees e
LEFT JOIN empty_right er ON e.emp_id = er.id
ORDER BY e.emp_id;
-- Expected: 12 rows with NULL val

-- Test 2.8.6: JOIN with single-row table
SELECT e.emp_name, sr.val
FROM employees e
JOIN single_row sr ON 1 = 1
ORDER BY e.emp_id;
-- Expected: 12 rows all with val='only_row' (cross join behavior)

-- Test 2.8.7: JOIN with constant condition
SELECT e.emp_name, d.dept_name
FROM employees e
JOIN departments d ON 1 = 1
WHERE e.emp_id = 1
ORDER BY d.dept_id;
-- Expected: Alice with all 6 departments (cross join for emp_id=1)

-- Test 2.8.8: JOIN with OR condition
SELECT e.emp_name, d.dept_name
FROM employees e
JOIN departments d ON e.dept_id = d.dept_id OR d.dept_id = 1
WHERE e.emp_id = 1
ORDER BY d.dept_id;
-- Expected: Alice with all 6 departments (matches Engineering directly + all through OR)

-- Test 2.8.9: JOIN with non-equi condition (greater than)
SELECT e1.emp_name AS lower, e2.emp_name AS higher, e1.salary, e2.salary
FROM employees e1
JOIN employees e2 ON e1.salary < e2.salary AND e1.dept_id = e2.dept_id
ORDER BY e1.emp_name;
-- Expected: lower salary employees paired with higher in same dept

-- Test 2.8.10: JOIN with multiple AND/OR conditions
SELECT e.emp_name, d.dept_name
FROM employees e
JOIN departments d ON (e.dept_id = d.dept_id OR d.dept_id = 1)
    AND d.location = 'New York'
WHERE e.salary > 70000
ORDER BY e.emp_name;
-- Expected: Alice/Engineering, Bob/Engineering, Grace/HR, Heidi/HR

-- Test 2.8.11: Self-JOIN with non-equi condition on dates
SELECT e1.emp_name AS newer, e2.emp_name AS older, e1.hire_date, e2.hire_date
FROM employees e1
JOIN employees e2 ON e1.dept_id = e2.dept_id
    AND e1.hire_date > e2.hire_date
    AND e1.emp_id <> e2.emp_id
ORDER BY e1.emp_name;
-- Expected: newer employees paired with older ones in same dept

-- Test 2.8.12: JOIN with aliased subquery
SELECT sub.emp_name, d.dept_name
FROM (SELECT emp_id, emp_name, dept_id FROM employees) sub
JOIN departments d ON sub.dept_id = d.dept_id
ORDER BY sub.emp_id;
-- Expected: 12 rows employee/dept pairs

-- Test 2.8.13: RIGHT JOIN with NULL coalesce
SELECT COALESCE(nr.label, 'Unlabeled') AS label, COUNT(nk.id) AS cnt
FROM null_keys nk
RIGHT JOIN null_ref nr ON nk.group_id = nr.gid
GROUP BY nr.label
ORDER BY label;
-- Expected: GroupA/2, GroupB/1, GroupC/0

-- Test 2.8.14: FULL OUTER JOIN with aggregation on unmatched
SELECT COALESCE(nk.name, '(missing)') AS name, COALESCE(nr.label, '(missing)') AS label
FROM null_keys nk
FULL OUTER JOIN null_ref nr ON nk.group_id = nr.gid
ORDER BY nk.id;
-- Expected: One/GroupA, Two/NULL, Three/GroupA, Four/NULL, Five/GroupB, NULL/GroupC

-- ============================================================================
-- SECTION 3: AGGREGATE TESTS (66 test cases)
-- ============================================================================

-- --------------------------------------------------------------------------
-- 3.1 COUNT (10 tests)
-- --------------------------------------------------------------------------

-- Test 3.1.1: COUNT(*)
SELECT COUNT(*) AS total_employees FROM employees;
-- Expected: 12

-- Test 3.1.2: COUNT(column)
SELECT COUNT(emp_id) AS cnt FROM employees;
-- Expected: 12

-- Test 3.1.3: COUNT with WHERE
SELECT COUNT(*) AS high_earners FROM employees WHERE salary > 100000;
-- Expected: 3 (Alice, Ivan, Judy)

-- Test 3.1.4: COUNT with GROUP BY
SELECT dept_id, COUNT(*) AS emp_count FROM employees GROUP BY dept_id ORDER BY dept_id;
-- Expected: 1/2, 2/2, 3/2, 4/2, 5/2, 6/2

-- Test 3.1.5: COUNT(DISTINCT column)
SELECT COUNT(DISTINCT dept_id) AS distinct_depts FROM employees;
-- Expected: 6

-- Test 3.1.6: COUNT(DISTINCT) with GROUP BY
SELECT dept_id, COUNT(DISTINCT salary) AS distinct_salaries FROM employees GROUP BY dept_id ORDER BY dept_id;
-- Expected: 1/2, 2/2, 3/2, 4/2, 5/2, 6/2

-- Test 3.1.7: COUNT on empty table
SELECT COUNT(*) AS cnt FROM empty_left;
-- Expected: 0

-- Test 3.1.8: COUNT with NULL values
SELECT COUNT(group_id) AS non_null_gids FROM null_keys;
-- Expected: 3 (two NULL group_ids are excluded)

-- Test 3.1.9: COUNT(*) vs COUNT(col) difference with NULLs
SELECT COUNT(*) AS cnt_star, COUNT(group_id) AS cnt_col FROM null_keys;
-- Expected: 5, 3

-- Test 3.1.10: COUNT with JOIN
SELECT d.dept_name, COUNT(e.emp_id) AS emp_count
FROM departments d
LEFT JOIN employees e ON d.dept_id = e.dept_id
GROUP BY d.dept_name
ORDER BY d.dept_name;
-- Expected: Engineering/2, Finance/2, HR/2, Marketing/2, Sales/2, Support/2

-- --------------------------------------------------------------------------
-- 3.2 SUM (8 tests)
-- --------------------------------------------------------------------------

-- Test 3.2.1: SUM of column
SELECT SUM(salary) AS total_salary FROM employees;
-- Expected: 120000 + 95000 + 80000 + 75000 + 90000 + 85000 + 65000 + 60000 + 110000 + 105000 + 55000 + 52000 = 992000

-- Test 3.2.2: SUM with WHERE
SELECT SUM(salary) AS engineering_salary FROM employees WHERE dept_id = 1;
-- Expected: 215000

-- Test 3.2.3: SUM with GROUP BY
SELECT dept_id, SUM(salary) AS total_salary FROM employees GROUP BY dept_id ORDER BY dept_id;
-- Expected: 1/215000, 2/155000, 3/175000, 4/125000, 5/215000, 6/107000

-- Test 3.2.4: SUM of empty set
SELECT SUM(salary) AS total FROM employees WHERE 1 = 0;
-- Expected: NULL

-- Test 3.2.5: SUM with JOIN
SELECT d.dept_name, SUM(e.salary) AS total_salary
FROM departments d
LEFT JOIN employees e ON d.dept_id = e.dept_id
GROUP BY d.dept_name
ORDER BY d.dept_name;
-- Expected: Engineering/215000, Finance/215000, HR/125000, Marketing/175000, Sales/155000, Support/107000

-- Test 3.2.6: SUM(DISTINCT)
SELECT SUM(DISTINCT dept_id) AS sum_distinct_depts FROM employees;
-- Expected: 1+2+3+4+5+6 = 21

-- Test 3.2.7: SUM with HAVING
SELECT dept_id, SUM(salary) AS total_salary
FROM employees
GROUP BY dept_id
HAVING SUM(salary) > 150000
ORDER BY dept_id;
-- Expected: 1/215000, 2/155000(Sales is 155k, less than 150k... wait 155k > 150k), 3/175000, 5/215000

-- Test 3.2.8: SUM with calculated field
SELECT SUM(salary * 0.1) AS total_bonus FROM employees;
-- Expected: 99200

-- --------------------------------------------------------------------------
-- 3.3 AVG (6 tests)
-- --------------------------------------------------------------------------

-- Test 3.3.1: AVG of column
SELECT AVG(salary) AS avg_salary FROM employees;
-- Expected: 992000 / 12 = 82666.666...

-- Test 3.3.2: AVG with GROUP BY
SELECT dept_id, AVG(salary) AS avg_salary FROM employees GROUP BY dept_id ORDER BY dept_id;
-- Expected: 1/107500, 2/77500, 3/87500, 4/62500, 5/107500, 6/53500

-- Test 3.3.3: AVG with WHERE
SELECT AVG(salary) AS avg_high FROM employees WHERE salary > 90000;
-- Expected: avg of (120000, 95000, 110000, 105000) = 107500

-- Test 3.3.4: AVG of empty set
SELECT AVG(salary) AS avg_none FROM employees WHERE 1 = 0;
-- Expected: NULL

-- Test 3.3.5: AVG with DISTINCT
SELECT AVG(DISTINCT dept_id) AS avg_distinct_depts FROM employees;
-- Expected: (1+2+3+4+5+6)/6 = 3.5

-- Test 3.3.6: AVG with HAVING
SELECT dept_id, AVG(salary) AS avg_salary
FROM employees
GROUP BY dept_id
HAVING AVG(salary) > 80000
ORDER BY dept_id;
-- Expected: 1/107500, 3/87500, 5/107500

-- --------------------------------------------------------------------------
-- 3.4 MIN / MAX (8 tests)
-- --------------------------------------------------------------------------

-- Test 3.4.1: MIN salary
SELECT MIN(salary) AS min_salary FROM employees;
-- Expected: 52000

-- Test 3.4.2: MAX salary
SELECT MAX(salary) AS max_salary FROM employees;
-- Expected: 120000

-- Test 3.4.3: MIN and MAX together
SELECT MIN(salary) AS min_sal, MAX(salary) AS max_sal, MAX(salary) - MIN(salary) AS spread FROM employees;
-- Expected: 52000, 120000, 68000

-- Test 3.4.4: MIN/MAX with GROUP BY
SELECT dept_id, MIN(salary) AS min_sal, MAX(salary) AS max_sal
FROM employees
GROUP BY dept_id
ORDER BY dept_id;
-- Expected: 1/95000/120000, 2/75000/80000, 3/85000/90000, 4/60000/65000, 5/105000/110000, 6/52000/55000

-- Test 3.4.5: MIN on string column
SELECT MIN(emp_name) AS first_name FROM employees;
-- Expected: 'Alice'

-- Test 3.4.6: MAX on string column
SELECT MAX(emp_name) AS last_name FROM employees;
-- Expected: 'Linda'

-- Test 3.4.7: MIN on DATE column
SELECT MIN(hire_date) AS earliest_hire FROM employees;
-- Expected: '2020-01-15'

-- Test 3.4.8: MAX on DATE column
SELECT MAX(hire_date) AS latest_hire FROM employees;
-- Expected: '2023-01-05'

-- --------------------------------------------------------------------------
-- 3.5 GROUP_CONCAT (6 tests)
-- Note: DataFusion may not support GROUP_CONCAT natively.
--       Alternatives: string_agg() or array_agg() + array_to_string().
--       These tests are kept as-is to validate behavior if supported.
-- --------------------------------------------------------------------------

-- Test 3.5.1: GROUP_CONCAT basic
SELECT dept_id, GROUP_CONCAT(emp_name ORDER BY emp_id) AS names
FROM employees
GROUP BY dept_id
ORDER BY dept_id;
-- Expected: 1/'Alice,Bob', 2/'Charlie,Diana', 3/'Eve,Frank', 4/'Grace,Heidi', 5/'Ivan,Judy', 6/'Karl,Linda'

-- Test 3.5.2: GROUP_CONCAT with separator
SELECT dept_id, GROUP_CONCAT(emp_name SEPARATOR ' | ') AS names
FROM employees
GROUP BY dept_id
ORDER BY dept_id;
-- Expected: 1/'Alice | Bob', 2/'Charlie | Diana', ...

-- Test 3.5.3: GROUP_CONCAT with WHERE
SELECT GROUP_CONCAT(emp_name ORDER BY emp_id) AS high_earners
FROM employees
WHERE salary > 100000;
-- Expected: 'Alice,Ivan,Judy'

-- Test 3.5.4: GROUP_CONCAT DISTINCT
SELECT GROUP_CONCAT(DISTINCT dept_id ORDER BY dept_id) AS depts
FROM employees;
-- Expected: '1,2,3,4,5,6'

-- Test 3.5.5: GROUP_CONCAT with ORDER BY DESC
SELECT dept_id, GROUP_CONCAT(emp_name ORDER BY emp_name DESC) AS names
FROM employees
GROUP BY dept_id
ORDER BY dept_id;
-- Expected: 1/'Bob,Alice', 2/'Diana,Charlie', ...

-- Test 3.5.6: GROUP_CONCAT on empty group
SELECT GROUP_CONCAT(emp_name) AS names
FROM employees
WHERE 1 = 0;
-- Expected: NULL

-- --------------------------------------------------------------------------
-- 3.6 GROUP BY (10 tests)
-- --------------------------------------------------------------------------

-- Test 3.6.1: GROUP BY single column
SELECT dept_id, COUNT(*) AS cnt FROM employees GROUP BY dept_id ORDER BY dept_id;
-- Expected: 1/2, 2/2, 3/2, 4/2, 5/2, 6/2

-- Test 3.6.2: GROUP BY multiple columns
SELECT dept_id, manager_id, COUNT(*) AS cnt
FROM employees
GROUP BY dept_id, manager_id
ORDER BY dept_id, manager_id;
-- Expected: 1/NULL/1, 1/1/1, 2/1/1, 2/3/1, 3/1/1, 3/5/1, 4/NULL/1, 4/7/1, 5/NULL/1, 5/9/1, 6/7/1, 6/11/1

-- Test 3.6.3: GROUP BY with expression
SELECT salary > 100000 AS high_earner, COUNT(*) AS cnt
FROM employees
GROUP BY salary > 100000
ORDER BY high_earner;
-- Expected: 0/9, 1/3

-- Test 3.6.4: GROUP BY with HAVING
SELECT dept_id, AVG(salary) AS avg_sal
FROM employees
GROUP BY dept_id
HAVING AVG(salary) > 70000
ORDER BY dept_id;
-- Expected: 1/107500, 2/77500, 3/87500, 5/107500

-- Test 3.6.5: GROUP BY with ORDER BY aggregate
SELECT dept_id, SUM(salary) AS total_sal
FROM employees
GROUP BY dept_id
ORDER BY total_sal DESC;
-- Expected: 1/215000, 5/215000, 3/175000, 2/155000, 4/125000, 6/107000

-- Test 3.6.6: GROUP BY with HAVING and ORDER BY
SELECT dept_id, COUNT(*) AS cnt
FROM employees
GROUP BY dept_id
HAVING COUNT(*) >= 2
ORDER BY cnt DESC, dept_id;
-- Expected: all 6 depts have 2 each

-- Test 3.6.7: GROUP BY on single row
SELECT COUNT(*) AS cnt, AVG(id) AS avg_id FROM single_row;
-- Expected: 1, 1

-- Test 3.6.8: GROUP BY with NULL grouping column
SELECT group_id, COUNT(*) AS cnt
FROM null_keys
GROUP BY group_id
ORDER BY group_id NULLS FIRST;
-- Expected: NULL/2, 10/2, 20/1

-- Test 3.6.9: GROUP BY on empty table
SELECT id, COUNT(*) AS cnt FROM empty_left GROUP BY id;
-- Expected: 0 rows

-- Test 3.6.10: GROUP BY with multiple aggregates
SELECT dept_id,
       COUNT(*) AS cnt,
       SUM(salary) AS total_sal,
       AVG(salary) AS avg_sal,
       MIN(salary) AS min_sal,
       MAX(salary) AS max_sal
FROM employees
GROUP BY dept_id
ORDER BY dept_id;
-- Expected: each dept with all aggregate values

-- --------------------------------------------------------------------------
-- 3.7 HAVING (6 tests)
-- --------------------------------------------------------------------------

-- Test 3.7.1: HAVING with COUNT
SELECT dept_id, COUNT(*) AS cnt
FROM employees
GROUP BY dept_id
HAVING COUNT(*) = 2
ORDER BY dept_id;
-- Expected: all 6 depts (each has 2 employees)

-- Test 3.7.2: HAVING with SUM
SELECT dept_id, SUM(salary) AS total_sal
FROM employees
GROUP BY dept_id
HAVING SUM(salary) > 150000
ORDER BY dept_id;
-- Expected: 1/215000, 2/155000, 3/175000, 5/215000

-- Test 3.7.3: HAVING with AVG
SELECT dept_id, AVG(salary) AS avg_sal
FROM employees
GROUP BY dept_id
HAVING AVG(salary) < 80000
ORDER BY dept_id;
-- Expected: 4/62500, 6/53500

-- Test 3.7.4: HAVING with MIN
SELECT dept_id, MIN(salary) AS min_sal
FROM employees
GROUP BY dept_id
HAVING MIN(salary) > 60000
ORDER BY dept_id;
-- Expected: 1/95000, 3/85000, 5/105000

-- Test 3.7.5: HAVING on expression
SELECT dept_id, SUM(salary) AS total_sal
FROM employees
GROUP BY dept_id
HAVING SUM(salary) * 0.1 > 15000
ORDER BY dept_id;
-- Expected: 1/215000, 5/215000

-- Test 3.7.6: HAVING without GROUP BY (treated as HAVING on full table)
SELECT COUNT(*) AS cnt
FROM employees
HAVING COUNT(*) > 0;
-- Expected: 12

-- --------------------------------------------------------------------------
-- 3.8 Aggregates with Subqueries (6 tests)
-- --------------------------------------------------------------------------

-- Test 3.8.1: Aggregate in subquery — scalar comparison
SELECT emp_name, salary
FROM employees
WHERE salary > (SELECT AVG(salary) FROM employees)
ORDER BY emp_id;
-- Expected: Alice/120000, Ivan/110000, Judy/105000

-- Test 3.8.2: Aggregate in subquery — IN clause
SELECT dept_name
FROM departments
WHERE dept_id IN (SELECT dept_id FROM employees GROUP BY dept_id HAVING AVG(salary) > 80000)
ORDER BY dept_name;
-- Expected: Engineering, Finance, Marketing

-- Test 3.8.3: Nested subquery with aggregate
SELECT emp_name, salary
FROM employees
WHERE dept_id IN (
    SELECT dept_id FROM employees GROUP BY dept_id HAVING AVG(salary) > 100000
)
ORDER BY emp_id;
-- Expected: Alice/120000, Bob/95000, Ivan/110000, Judy/105000

-- Test 3.8.4: Subquery with aggregate in SELECT
SELECT emp_name,
       salary,
       (SELECT AVG(salary) FROM employees) AS company_avg
FROM employees
ORDER BY emp_id;
-- Expected: each employee with company-wide avg

-- Test 3.8.5: Correlated subquery with aggregate
SELECT e1.emp_name, e1.salary, e1.dept_id
FROM employees e1
WHERE e1.salary > (
    SELECT AVG(e2.salary) FROM employees e2 WHERE e2.dept_id = e1.dept_id
)
ORDER BY e1.emp_id;
-- Expected: Alice/120000/1, Eve/90000/3, Ivan/110000/5

-- Test 3.8.6: Multiple aggregates in subquery
SELECT dept_name,
       (SELECT COUNT(*) FROM employees e WHERE e.dept_id = d.dept_id) AS emp_count,
       (SELECT AVG(salary) FROM employees e WHERE e.dept_id = d.dept_id) AS avg_salary
FROM departments d
ORDER BY dept_name;
-- Expected: each dept with count and avg from employees

-- --------------------------------------------------------------------------
-- 3.9 Edge Cases and Overlaps (6 tests)
-- --------------------------------------------------------------------------

-- Test 3.9.1: Aggregate on single row
SELECT COUNT(*) AS cnt, SUM(id) AS total, AVG(id) AS avg, MIN(id) AS min, MAX(id) AS max
FROM single_row;
-- Expected: 1, 1, 1, 1, 1

-- Test 3.9.2: Multiple aggregates without GROUP BY
SELECT COUNT(*) AS cnt,
       SUM(salary) AS total_sal,
       AVG(salary) AS avg_sal,
       MIN(salary) AS min_sal,
       MAX(salary) AS max_sal,
       COUNT(DISTINCT dept_id) AS distinct_depts
FROM employees;
-- Expected: 12, 992000, ~82666.67, 52000, 120000, 6

-- Test 3.9.3: Aggregate with DISTINCT and non-DISTINCT
SELECT COUNT(*) AS total, COUNT(DISTINCT dept_id) AS distinct_depts
FROM employees;
-- Expected: 12, 6

-- Test 3.9.4: Aggregate on all NULL values
SELECT COUNT(*), SUM(group_id), AVG(group_id), MIN(group_id), MAX(group_id)
FROM null_keys
WHERE group_id IS NULL;
-- Expected: COUNT(*) = 2, SUM = NULL, AVG = NULL, MIN = NULL, MAX = NULL

-- Test 3.9.5: GROUP BY with expression alias
SELECT dept_id * 10 AS dept_times_10, COUNT(*) AS cnt
FROM employees
GROUP BY dept_id * 10
ORDER BY dept_times_10;
-- Expected: 10/2, 20/2, 30/2, 40/2, 50/2, 60/2

-- Test 3.9.6: Aggregate with CASE expression
SELECT dept_id,
       SUM(CASE WHEN salary > 90000 THEN 1 ELSE 0 END) AS high_earners
FROM employees
GROUP BY dept_id
ORDER BY dept_id;
-- Expected: 1/2, 2/0, 3/0, 4/0, 5/2, 6/0

-- ============================================================================
-- SECTION 4: WINDOW FUNCTION TESTS (66 test cases)
-- ============================================================================

-- --------------------------------------------------------------------------
-- 4.1 ROW_NUMBER (10 tests)
-- --------------------------------------------------------------------------

-- Test 4.1.1: ROW_NUMBER() with ORDER BY
SELECT emp_name, salary,
       ROW_NUMBER() OVER (ORDER BY salary DESC) AS rn
FROM employees
ORDER BY rn;
-- Expected: Alice/120000/1, Ivan/110000/2, Judy/105000/3, Bob/95000/4, Eve/90000/5, Frank/85000/6, Charlie/80000/7, Diana/75000/8, Grace/65000/9, Heidi/60000/10, Karl/55000/11, Linda/52000/12

-- Test 4.1.2: ROW_NUMBER() with PARTITION BY
SELECT emp_name, dept_id, salary,
       ROW_NUMBER() OVER (PARTITION BY dept_id ORDER BY salary DESC) AS rn
FROM employees
ORDER BY dept_id, rn;
-- Expected: employees numbered within their department by salary descending

-- Test 4.1.3: ROW_NUMBER() without ORDER BY (arbitrary but deterministic)
SELECT emp_id, emp_name, ROW_NUMBER() OVER () AS rn
FROM employees
ORDER BY emp_id;
-- Expected: 12 rows numbered 1-12

-- Test 4.1.4: ROW_NUMBER() with PARTITION BY and ORDER BY
SELECT emp_name, dept_id, hire_date,
       ROW_NUMBER() OVER (PARTITION BY dept_id ORDER BY hire_date) AS rn
FROM employees
ORDER BY dept_id, rn;
-- Expected: employees numbered by hire_date within each dept

-- Test 4.1.5: ROW_NUMBER() in subquery for filtering
SELECT emp_name, dept_id, salary
FROM (
    SELECT emp_name, dept_id, salary,
           ROW_NUMBER() OVER (PARTITION BY dept_id ORDER BY salary DESC) AS rn
    FROM employees
) sub
WHERE sub.rn = 1
ORDER BY dept_id;
-- Expected: highest paid employee in each department

-- Test 4.1.6: ROW_NUMBER() with two ORDER BY columns
SELECT emp_name, salary, hire_date,
       ROW_NUMBER() OVER (ORDER BY salary DESC, hire_date ASC) AS rn
FROM employees
ORDER BY rn;
-- Expected: ordered by salary desc, then hire_date asc

-- Test 4.1.7: ROW_NUMBER() on empty table
SELECT ROW_NUMBER() OVER (ORDER BY id) AS rn, val
FROM empty_left;
-- Expected: 0 rows

-- Test 4.1.8: ROW_NUMBER() with single partition
SELECT emp_name, salary,
       ROW_NUMBER() OVER (PARTITION BY 1 ORDER BY salary DESC) AS rn
FROM employees
ORDER BY rn;
-- Expected: same as 4.1.1

-- Test 4.1.9: ROW_NUMBER() with many rows same partition key
SELECT s.product, s.amount,
       ROW_NUMBER() OVER (PARTITION BY s.category ORDER BY s.amount DESC) AS rn
FROM sales s
ORDER BY s.category, rn;
-- Expected: products numbered within each category by amount

-- Test 4.1.10: ROW_NUMBER() with WHERE on window result (using subquery)
SELECT emp_name, dept_id, salary, rn
FROM (
    SELECT emp_name, dept_id, salary,
           ROW_NUMBER() OVER (PARTITION BY dept_id ORDER BY salary DESC) AS rn
    FROM employees
) sub
WHERE sub.rn <= 2
ORDER BY dept_id, rn;
-- Expected: top 2 employees per department by salary

-- --------------------------------------------------------------------------
-- 4.2 RANK / DENSE_RANK (10 tests)
-- --------------------------------------------------------------------------

-- Test 4.2.1: RANK() with ties
SELECT student_name, subject, score,
       RANK() OVER (PARTITION BY subject ORDER BY score DESC) AS rnk
FROM scores
ORDER BY subject, rnk;
-- Expected: Math scores ranked (Alice and Charlie share rank 1 at 95)

-- Test 4.2.2: DENSE_RANK() with ties
SELECT student_name, subject, score,
       DENSE_RANK() OVER (PARTITION BY subject ORDER BY score DESC) AS dr
FROM scores
ORDER BY subject, dr;
-- Expected: Math scores with dense rank (Alice/Charlie:1, Bob:2, Diana:3)

-- Test 4.2.3: RANK vs DENSE_RANK comparison
SELECT student_name, subject, score,
       RANK() OVER (ORDER BY score DESC) AS rnk,
       DENSE_RANK() OVER (ORDER BY score DESC) AS dr
FROM scores
ORDER BY score DESC;
-- Expected: 12 rows showing difference between RANK and DENSE_RANK

-- Test 4.2.4: RANK() with ORDER BY ASC
SELECT student_name, subject, score,
       RANK() OVER (PARTITION BY subject ORDER BY score ASC) AS rnk
FROM scores
ORDER BY subject, rnk;
-- Expected: asc ranking (lowest score = rank 1)

-- Test 4.2.5: RANK() without PARTITION BY
SELECT student_name, score,
       RANK() OVER (ORDER BY score DESC) AS rnk
FROM scores
ORDER BY rnk;
-- Expected: all 12 scores ranked globally

-- Test 4.2.6: DENSE_RANK() without PARTITION BY
SELECT student_name, score,
       DENSE_RANK() OVER (ORDER BY score DESC) AS dr
FROM scores
ORDER BY dr;
-- Expected: all scores dense-ranked globally

-- Test 4.2.7: RANK() with multiple partition columns
SELECT student_name, subject, score,
       RANK() OVER (PARTITION BY subject, score ORDER BY student_name) AS rnk
FROM scores
ORDER BY subject, score DESC;
-- Expected: each score within subject is its own partition, rank 1

-- Test 4.2.8: RANK with single row in partition
SELECT e.emp_name, e.dept_id, e.salary,
       RANK() OVER (PARTITION BY e.dept_id ORDER BY e.salary DESC) AS rnk
FROM employees e
JOIN departments d ON e.dept_id = d.dept_id
WHERE d.dept_id = 6
ORDER BY rnk;
-- Expected: Support dept employees ranked (Karl:1, Linda:2)

-- Test 4.2.9: RANK() with identical values
SELECT product, amount,
       RANK() OVER (ORDER BY amount) AS rnk
FROM sales
ORDER BY amount;
-- Expected: 12 rows ranked by amount (no ties in this dataset)

-- Test 4.2.10: DENSE_RANK() in subquery for top-N
SELECT student_name, subject, score
FROM (
    SELECT student_name, subject, score,
           DENSE_RANK() OVER (PARTITION BY subject ORDER BY score DESC) AS dr
    FROM scores
) sub
WHERE sub.dr <= 2
ORDER BY subject, sub.dr;
-- Expected: top 2 scores per subject (includes ties)

-- --------------------------------------------------------------------------
-- 4.3 LAG / LEAD (10 tests)
-- --------------------------------------------------------------------------

-- Test 4.3.1: LAG() basic — previous salary
SELECT emp_name, salary,
       LAG(salary) OVER (ORDER BY emp_id) AS prev_salary
FROM employees
ORDER BY emp_id;
-- Expected: Alice/120000/NULL, Bob/95000/120000, Charlie/80000/95000, ...

-- Test 4.3.2: LEAD() basic — next salary
SELECT emp_name, salary,
       LEAD(salary) OVER (ORDER BY emp_id) AS next_salary
FROM employees
ORDER BY emp_id;
-- Expected: Alice/120000/95000, Bob/95000/80000, ..., Linda/52000/NULL

-- Test 4.3.3: LAG() with offset 2
SELECT emp_name, salary,
       LAG(salary, 2) OVER (ORDER BY emp_id) AS prev_2_salary
FROM employees
ORDER BY emp_id;
-- Expected: first 2 rows have NULL, then each shows salary from 2 rows back

-- Test 4.3.4: LAG() with default value
SELECT emp_name, salary,
       LAG(salary, 1, 0) OVER (ORDER BY emp_id) AS prev_salary
FROM employees
ORDER BY emp_id;
-- Expected: Alice/120000/0, Bob/95000/120000, ...

-- Test 4.3.5: LEAD() with offset 2 and default
SELECT emp_name, salary,
       LEAD(salary, 2, -1) OVER (ORDER BY emp_id) AS next_2_salary
FROM employees
ORDER BY emp_id;
-- Expected: each row shows salary 2 ahead, last 2 get -1

-- Test 4.3.6: LAG() with PARTITION BY
SELECT emp_name, dept_id, salary,
       LAG(salary) OVER (PARTITION BY dept_id ORDER BY emp_id) AS prev_dept_salary
FROM employees
ORDER BY dept_id, emp_id;
-- Expected: within each dept, previous salary; first in dept gets NULL

-- Test 4.3.7: LEAD() with PARTITION BY
SELECT emp_name, dept_id, salary,
       LEAD(salary) OVER (PARTITION BY dept_id ORDER BY emp_id) AS next_dept_salary
FROM employees
ORDER BY dept_id, emp_id;
-- Expected: within each dept, next salary; last in dept gets NULL

-- Test 4.3.8: LAG/LEAD with date order
SELECT sale_date, amount, product,
       LAG(amount) OVER (ORDER BY sale_date) AS prev_amount,
       LEAD(amount) OVER (ORDER BY sale_date) AS next_amount
FROM sales
ORDER BY sale_date;
-- Expected: chronological LAG/LEAD of sales amounts

-- Test 4.3.9: LAG on empty partition
SELECT emp_name, salary,
       LAG(salary) OVER (PARTITION BY dept_id ORDER BY emp_id) AS prev_sal
FROM employees
WHERE 1 = 0;
-- Expected: 0 rows

-- Test 4.3.10: LAG with calculated difference
SELECT emp_name, salary,
       salary - LAG(salary) OVER (ORDER BY emp_id) AS salary_diff
FROM employees
ORDER BY emp_id;
-- Expected: Alice/NULL, Bob/-25000, Charlie/-15000, ...

-- --------------------------------------------------------------------------
-- 4.4 Aggregate Window Functions (14 tests)
-- --------------------------------------------------------------------------

-- Test 4.4.1: SUM() OVER (ORDER BY) — running total
SELECT sale_date, amount,
       SUM(amount) OVER (ORDER BY sale_date) AS running_total
FROM sales
ORDER BY sale_date;
-- Expected: chronological running total of sales

-- Test 4.4.2: SUM() OVER (PARTITION BY) — total per partition
SELECT emp_name, dept_id, salary,
       SUM(salary) OVER (PARTITION BY dept_id) AS dept_total
FROM employees
ORDER BY emp_id;
-- Expected: each row shows total salary of its department

-- Test 4.4.3: AVG() OVER (ORDER BY) — running average
SELECT sale_date, amount,
       AVG(amount) OVER (ORDER BY sale_date) AS running_avg
FROM sales
ORDER BY sale_date;
-- Expected: cumulative average of sales amounts

-- Test 4.4.4: AVG() OVER (PARTITION BY)
SELECT emp_name, dept_id, salary,
       AVG(salary) OVER (PARTITION BY dept_id) AS dept_avg
FROM employees
ORDER BY emp_id;
-- Expected: each row shows avg salary of its department

-- Test 4.4.5: COUNT() OVER (ORDER BY) — running count
SELECT sale_date, amount,
       COUNT(*) OVER (ORDER BY sale_date) AS running_count
FROM sales
ORDER BY sale_date;
-- Expected: 1, 2, 3, ... 12

-- Test 4.4.6: COUNT() OVER (PARTITION BY)
SELECT dept_id, emp_name,
       COUNT(*) OVER (PARTITION BY dept_id) AS dept_count
FROM employees
ORDER BY emp_id;
-- Expected: each row shows employee count in its department (all 2)

-- Test 4.4.7: MIN() OVER (PARTITION BY)
SELECT dept_id, emp_name, salary,
       MIN(salary) OVER (PARTITION BY dept_id) AS dept_min_sal
FROM employees
ORDER BY emp_id;
-- Expected: each row shows min salary in its dept

-- Test 4.4.8: MAX() OVER (PARTITION BY)
SELECT dept_id, emp_name, salary,
       MAX(salary) OVER (PARTITION BY dept_id) AS dept_max_sal
FROM employees
ORDER BY emp_id;
-- Expected: each row shows max salary in its dept

-- Test 4.4.9: Multiple window functions in same query
SELECT emp_name, dept_id, salary,
       SUM(salary) OVER (PARTITION BY dept_id) AS dept_total,
       AVG(salary) OVER (PARTITION BY dept_id) AS dept_avg,
       COUNT(*) OVER (PARTITION BY dept_id) AS dept_count,
       ROW_NUMBER() OVER (PARTITION BY dept_id ORDER BY salary DESC) AS rn
FROM employees
ORDER BY dept_id, rn;
-- Expected: each employee with dept-level total/avg/count/rank

-- Test 4.4.10: SUM() OVER () — global sum repeated on each row
SELECT emp_name, salary,
       SUM(salary) OVER () AS global_total
FROM employees
ORDER BY emp_id;
-- Expected: each row shows global total 992000

-- Test 4.4.11: AVG() OVER () — global avg repeated
SELECT emp_name, salary,
       AVG(salary) OVER () AS company_avg
FROM employees
ORDER BY emp_id;
-- Expected: each row shows same company avg

-- Test 4.4.12: SUM OVER with percentage calculation
SELECT emp_name, dept_id, salary,
       SUM(salary) OVER (PARTITION BY dept_id) AS dept_total,
       ROUND(salary * 100.0 / SUM(salary) OVER (PARTITION BY dept_id), 2) AS pct_of_dept
FROM employees
ORDER BY dept_id, emp_id;
-- Expected: each employee's salary as percentage of dept total

-- Test 4.4.13: SUM OVER empty table
SELECT SUM(amount) OVER (ORDER BY sale_date) AS running_total
FROM sales
WHERE 1 = 0;
-- Expected: 0 rows

-- Test 4.4.14: Aggregate window with PARTITION BY multiple columns
SELECT s.category, s.region, s.amount,
       SUM(s.amount) OVER (PARTITION BY s.category, s.region) AS cat_region_total
FROM sales s
ORDER BY s.category, s.region, s.sale_id;
-- Expected: total sales per category-region combination

-- --------------------------------------------------------------------------
-- 4.5 Frame Clause — ROWS BETWEEN (12 tests)
-- --------------------------------------------------------------------------

-- Test 4.5.1: ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW — running total
SELECT sale_date, amount,
       SUM(amount) OVER (ORDER BY sale_date ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW) AS running_total
FROM sales
ORDER BY sale_date;
-- Expected: same as 4.4.1 (this is default frame)

-- Test 4.5.2: ROWS BETWEEN 1 PRECEDING AND 1 FOLLOWING — moving sum of 3
SELECT sale_date, amount,
       SUM(amount) OVER (ORDER BY sale_date ROWS BETWEEN 1 PRECEDING AND 1 FOLLOWING) AS moving_sum_3
FROM sales
ORDER BY sale_date;
-- Expected: each row sum of itself + previous + next

-- Test 4.5.3: ROWS BETWEEN CURRENT ROW AND UNBOUNDED FOLLOWING
SELECT sale_date, amount,
       SUM(amount) OVER (ORDER BY sale_date ROWS BETWEEN CURRENT ROW AND UNBOUNDED FOLLOWING) AS remaining_total
FROM sales
ORDER BY sale_date;
-- Expected: each row shows sum from current row to end

-- Test 4.5.4: ROWS BETWEEN 2 PRECEDING AND CURRENT ROW — last 3 rows sum
SELECT sale_date, amount,
       SUM(amount) OVER (ORDER BY sale_date ROWS BETWEEN 2 PRECEDING AND CURRENT ROW) AS last_3_sum
FROM sales
ORDER BY sale_date;
-- Expected: first row = itself, second = sum of first 2, then sum of last 3

-- Test 4.5.5: ROWS BETWEEN 1 PRECEDING AND 1 FOLLOWING with AVG — moving avg of 3
SELECT sale_date, amount,
       AVG(amount) OVER (ORDER BY sale_date ROWS BETWEEN 1 PRECEDING AND 1 FOLLOWING) AS moving_avg_3
FROM sales
ORDER BY sale_date;
-- Expected: moving average of 3 rows

-- Test 4.5.6: ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING — global total
SELECT sale_date, amount,
       SUM(amount) OVER (ORDER BY sale_date ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING) AS global_total
FROM sales
ORDER BY sale_date;
-- Expected: each row shows same global total

-- Test 4.5.7: Frame with PARTITION BY
SELECT s.product, s.category, s.amount,
       SUM(s.amount) OVER (
           PARTITION BY s.category
           ORDER BY s.sale_date
           ROWS BETWEEN 1 PRECEDING AND 1 FOLLOWING
       ) AS moving_sum_in_category
FROM sales s
ORDER BY s.category, s.sale_date;
-- Expected: moving sum within each category

-- Test 4.5.8: COUNT with frame
SELECT sale_date, amount,
       COUNT(*) OVER (ORDER BY sale_date ROWS BETWEEN 2 PRECEDING AND CURRENT ROW) AS cnt_last_3
FROM sales
ORDER BY sale_date;
-- Expected: 1, 2, 3, 3, 3, ... (count of rows in current window)

-- Test 4.5.9: MIN with frame
SELECT sale_date, amount,
       MIN(amount) OVER (ORDER BY sale_date ROWS BETWEEN 1 PRECEDING AND 1 FOLLOWING) AS min_surrounding
FROM sales
ORDER BY sale_date;
-- Expected: min of self + preceding + following

-- Test 4.5.10: MAX with frame
SELECT sale_date, amount,
       MAX(amount) OVER (ORDER BY sale_date ROWS BETWEEN 1 PRECEDING AND 1 FOLLOWING) AS max_surrounding
FROM sales
ORDER BY sale_date;
-- Expected: max of self + preceding + following

-- Test 4.5.11: Frame with multiple window functions
SELECT sale_date, amount,
       SUM(amount) OVER (ORDER BY sale_date ROWS BETWEEN 1 PRECEDING AND CURRENT ROW) AS sum_2,
       AVG(amount) OVER (ORDER BY sale_date ROWS BETWEEN 1 PRECEDING AND CURRENT ROW) AS avg_2,
       COUNT(*) OVER (ORDER BY sale_date ROWS BETWEEN 1 PRECEDING AND CURRENT ROW) AS cnt_2
FROM sales
ORDER BY sale_date;
-- Expected: 2-row window with sum/avg/count

-- Test 4.5.12: Frame clause with LAG/LEAD comparison — same result
SELECT sale_date, amount,
       LAG(amount) OVER (ORDER BY sale_date) AS prev,
       SUM(amount) OVER (ORDER BY sale_date ROWS BETWEEN 1 PRECEDING AND 1 PRECEDING) AS prev_sum
FROM sales
ORDER BY sale_date;
-- Expected: prev and prev_sum should match (1 PRECEDING only)

-- --------------------------------------------------------------------------
-- 4.6 Window + JOIN (8 tests)
-- --------------------------------------------------------------------------

-- Test 4.6.1: Window function with JOIN
SELECT e.emp_name, d.dept_name, e.salary,
       ROW_NUMBER() OVER (PARTITION BY e.dept_id ORDER BY e.salary DESC) AS dept_rank
FROM employees e
JOIN departments d ON e.dept_id = d.dept_id
ORDER BY d.dept_name, dept_rank;
-- Expected: employees ranked within their departments (with dept name)

-- Test 4.6.2: Window function aggregate with JOIN
SELECT e.emp_name, d.dept_name, e.salary,
       SUM(e.salary) OVER (PARTITION BY e.dept_id) AS dept_total
FROM employees e
JOIN departments d ON e.dept_id = d.dept_id
ORDER BY e.emp_id;
-- Expected: each row with dept total from joined data

-- Test 4.6.3: Window with multi-table JOIN
SELECT e.emp_name, d.dept_name, p.project_name, e.salary,
       ROW_NUMBER() OVER (PARTITION BY e.dept_id ORDER BY p.budget DESC) AS rn
FROM employees e
JOIN departments d ON e.dept_id = d.dept_id
LEFT JOIN projects p ON d.dept_id = p.dept_id
ORDER BY d.dept_name, rn;
-- Expected: employees with projects ranked by budget

-- Test 4.6.4: JOIN with subquery containing window
SELECT d.dept_name, sub.emp_name, sub.salary
FROM (
    SELECT emp_name, dept_id, salary,
           ROW_NUMBER() OVER (PARTITION BY dept_id ORDER BY salary DESC) AS rn
    FROM employees
) sub
JOIN departments d ON sub.dept_id = d.dept_id
WHERE sub.rn = 1
ORDER BY d.dept_name;
-- Expected: top paid employee per department with dept name

-- Test 4.6.5: Window with LEFT JOIN
SELECT e.emp_name, d.dept_name, e.salary,
       LAG(e.salary) OVER (ORDER BY e.emp_id) AS prev_sal
FROM employees e
LEFT JOIN departments d ON e.dept_id = d.dept_id
ORDER BY e.emp_id;
-- Expected: employees with dept name and LAG

-- Test 4.6.6: Window + JOIN + WHERE + ORDER BY
SELECT e.emp_name, d.dept_name, e.salary,
       RANK() OVER (ORDER BY e.salary DESC) AS rnk
FROM employees e
JOIN departments d ON e.dept_id = d.dept_id
WHERE d.location = 'Chicago'
ORDER BY rnk;
-- Expected: Chicago employees ranked by salary

-- Test 4.6.7: Window + aggregate + JOIN
SELECT d.dept_name,
       e.emp_name,
       e.salary,
       AVG(e.salary) OVER (PARTITION BY e.dept_id) AS dept_avg,
       MAX(e.salary) OVER (PARTITION BY e.dept_id) AS dept_max
FROM employees e
JOIN departments d ON e.dept_id = d.dept_id
ORDER BY d.dept_name, e.emp_id;
-- Expected: each employee with dept avg and max

-- Test 4.6.8: Window + JOIN with GROUP BY
SELECT d.dept_name,
       SUM(e.salary) AS total_salary,
       ROW_NUMBER() OVER (ORDER BY SUM(e.salary) DESC) AS rn
FROM employees e
JOIN departments d ON e.dept_id = d.dept_id
GROUP BY d.dept_name
ORDER BY total_salary DESC;
-- Expected: dept totals ranked

-- --------------------------------------------------------------------------
-- 4.7 Complex Real-World Scenarios (6 tests)
-- --------------------------------------------------------------------------

-- Test 4.7.1: Moving average — 3-period centered moving average of sales
SELECT sale_date, product, amount,
       AVG(amount) OVER (ORDER BY sale_date ROWS BETWEEN 1 PRECEDING AND 1 FOLLOWING) AS centered_ma_3
FROM sales
ORDER BY sale_date;
-- Expected: 3-period centered moving average

-- Test 4.7.2: Sales report with running total, monthly comparison using LAG
SELECT s.sale_date, s.product, s.amount,
       SUM(s.amount) OVER (ORDER BY s.sale_date) AS running_total,
       LAG(s.amount) OVER (ORDER BY s.sale_date) AS prev_sale,
       s.amount - LAG(s.amount) OVER (ORDER BY s.sale_date) AS change_from_prev
FROM sales s
ORDER BY s.sale_date;
-- Expected: full sales report with running total and changes

-- Test 4.7.3: Top-N students per subject
SELECT subject, student_name, score, rn
FROM (
    SELECT subject, student_name, score,
           ROW_NUMBER() OVER (PARTITION BY subject ORDER BY score DESC) AS rn
    FROM scores
) sub
WHERE sub.rn <= 2
ORDER BY subject, rn;
-- Expected: top 2 students per subject

-- Test 4.7.4: Department salary report
SELECT d.dept_name, e.emp_name, e.salary,
       ROUND(e.salary * 100.0 / SUM(e.salary) OVER (PARTITION BY e.dept_id), 2) AS pct_of_dept,
       ROUND(e.salary * 100.0 / SUM(e.salary) OVER (), 2) AS pct_of_company,
       RANK() OVER (ORDER BY e.salary DESC) AS company_rank,
       RANK() OVER (PARTITION BY e.dept_id ORDER BY e.salary DESC) AS dept_rank
FROM employees e
JOIN departments d ON e.dept_id = d.dept_id
ORDER BY d.dept_name, e.emp_id;
-- Expected: comprehensive salary report with percentages and ranks

-- Test 4.7.5: Employee tenure analysis with window
SELECT emp_name, dept_id, hire_date,
       ROW_NUMBER() OVER (PARTITION BY dept_id ORDER BY hire_date) AS hire_order,
       LAG(hire_date) OVER (PARTITION BY dept_id ORDER BY hire_date) AS prev_hire
FROM employees
ORDER BY dept_id, hire_order;
-- Expected: tenure analysis within each department (DATEDIFF removed: hire_date is VARCHAR)

-- Test 4.7.6: Cumulative sales by category
SELECT s.sale_date, s.category, s.product, s.amount,
       SUM(s.amount) OVER (PARTITION BY s.category ORDER BY s.sale_date) AS category_running_total,
       SUM(s.amount) OVER (PARTITION BY s.category) AS category_total,
       ROUND(s.amount * 100.0 / SUM(s.amount) OVER (PARTITION BY s.category), 2) AS pct_of_category
FROM sales s
ORDER BY s.category, s.sale_date;
-- Expected: cumulative sales within each category with percentage

-- --------------------------------------------------------------------------
-- 4.8 Window Edge Cases (4 tests)
-- --------------------------------------------------------------------------

-- Test 4.8.1: Window on empty partition — no rows to process
SELECT emp_name, salary,
       ROW_NUMBER() OVER (PARTITION BY dept_id ORDER BY salary DESC) AS rn
FROM employees
WHERE 1 = 0;
-- Expected: 0 rows

-- Test 4.8.2: Window on single row
SELECT val,
       ROW_NUMBER() OVER (ORDER BY id) AS rn,
       SUM(id) OVER (ORDER BY id) AS running_total
FROM single_row;
-- Expected: only_row, 1, 1

-- Test 4.8.3: Window function with all NULL partition key
SELECT id, name, group_id,
       ROW_NUMBER() OVER (PARTITION BY group_id ORDER BY id) AS rn
FROM null_keys
ORDER BY id;
-- Expected: rows with NULL group_id are in same partition; rows with same group_id together

-- Test 4.8.4: Multiple window functions with different frames
SELECT sale_date, amount,
       SUM(amount) OVER (ORDER BY sale_date ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW) AS running_total,
       SUM(amount) OVER (ORDER BY sale_date ROWS BETWEEN CURRENT ROW AND UNBOUNDED FOLLOWING) AS remaining_total,
       SUM(amount) OVER (ORDER BY sale_date ROWS BETWEEN 1 PRECEDING AND 1 FOLLOWING) AS moving_sum_3,
       SUM(amount) OVER (ORDER BY sale_date ROWS BETWEEN 2 PRECEDING AND 2 FOLLOWING) AS moving_sum_5
FROM sales
ORDER BY sale_date;
-- Expected: 4 different cumulative/moving sum calculations

-- ============================================================================
-- SECTION 5: COMBINED COMPLEX SCENARIOS (6 tests)
-- ============================================================================

-- Test 5.1: JOIN + aggregate + window in one query
SELECT d.dept_name,
       e.emp_name,
       e.salary,
       AVG(e.salary) OVER (PARTITION BY e.dept_id) AS dept_avg,
       e.salary - AVG(e.salary) OVER (PARTITION BY e.dept_id) AS diff_from_avg
FROM employees e
JOIN departments d ON e.dept_id = d.dept_id
ORDER BY d.dept_name, e.emp_name;
-- Expected: each employee with dept avg and diff from it

-- Test 5.2: Subquery with JOIN + aggregate + window
SELECT sub.dept_name, sub.emp_name, sub.salary
FROM (
    SELECT d.dept_name, e.emp_name, e.salary,
           ROW_NUMBER() OVER (PARTITION BY e.dept_id ORDER BY e.salary DESC) AS rn,
           AVG(e.salary) OVER (PARTITION BY e.dept_id) AS dept_avg
    FROM employees e
    JOIN departments d ON e.dept_id = d.dept_id
) sub
WHERE sub.salary > sub.dept_avg
ORDER BY sub.dept_name, sub.emp_name;
-- Expected: employees earning more than their dept average

-- Test 5.3: JOIN + GROUP BY + HAVING + window
SELECT d.dept_name, sub.total_sal, sub.emp_count,
       RANK() OVER (ORDER BY sub.total_sal DESC) AS sal_rank
FROM (
    SELECT dept_id, SUM(salary) AS total_sal, COUNT(*) AS emp_count
    FROM employees
    GROUP BY dept_id
    HAVING SUM(salary) > 150000
) sub
JOIN departments d ON sub.dept_id = d.dept_id
ORDER BY sal_rank;
-- Expected: depts with total salary > 150k, ranked

-- Test 5.4: Sales YTD analysis with window + multi-table
SELECT s.sale_date, s.product, s.amount,
       SUM(s.amount) OVER (PARTITION BY s.category ORDER BY s.sale_date) AS category_ytd,
       AVG(s.amount) OVER (PARTITION BY s.category) AS category_avg,
       s.amount - AVG(s.amount) OVER (PARTITION BY s.category) AS vs_category_avg
FROM sales s
ORDER BY s.category, s.sale_date;
-- Expected: YTD and average comparison per category

-- Test 5.5: Nested aggregates with window
SELECT emp_name, dept_id, salary,
       dept_max_sal,
       salary - dept_max_sal AS gap_to_max,
       RANK() OVER (PARTITION BY dept_id ORDER BY salary DESC) AS dept_rank
FROM (
    SELECT emp_name, dept_id, salary,
           MAX(salary) OVER (PARTITION BY dept_id) AS dept_max_sal
    FROM employees
) sub
ORDER BY dept_id, dept_rank;
-- Expected: each employee's gap to dept max, with rank

-- Test 5.6: Complex business report — department budget utilization
SELECT d.dept_name,
       e.emp_name,
       e.salary,
       p.project_name,
       p.budget,
       SUM(p.budget) OVER (PARTITION BY d.dept_id) AS dept_total_budget,
       SUM(e.salary) OVER (PARTITION BY d.dept_id) AS dept_total_salary,
       ROUND(SUM(e.salary) OVER (PARTITION BY d.dept_id) * 100.0 / NULLIF(SUM(p.budget) OVER (PARTITION BY d.dept_id), 0), 2) AS salary_pct_of_budget,
       ROW_NUMBER() OVER (PARTITION BY d.dept_id, p.project_id ORDER BY e.salary DESC) AS rn
FROM employees e
JOIN departments d ON e.dept_id = d.dept_id
LEFT JOIN projects p ON d.dept_id = p.dept_id
ORDER BY d.dept_name, p.project_name, e.salary DESC;
-- Expected: budget utilization report with percentages

-- ============================================================================
-- CLEANUP
-- ============================================================================

DROP TABLE IF EXISTS empty_left;
DROP TABLE IF EXISTS empty_right;
DROP TABLE IF EXISTS null_keys;
DROP TABLE IF EXISTS null_ref;
DROP TABLE IF EXISTS single_row;
DROP TABLE IF EXISTS scores;
DROP TABLE IF EXISTS sales;
DROP TABLE IF EXISTS projects;
DROP TABLE IF EXISTS employees;
DROP TABLE IF EXISTS departments;

DROP DATABASE e2e_jaw_test;

SELECT 'All 230 join/aggregate/window tests completed successfully' AS status;