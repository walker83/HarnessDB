# RorisDB SQL Reference Manual

This manual provides detailed information about the SQL syntax and functions supported by RorisDB.

## SQL Language Basics

RorisDB supports standard SQL syntax and is compatible with the MySQL protocol.

### Statement Termination

SQL statements end with a semicolon `;`:

```sql
SELECT * FROM user;
```

## Data Definition Language (DDL)

### Database Operations

#### Create Database

```sql
CREATE DATABASE [IF NOT EXISTS] database_name;
```

Examples:
```sql
CREATE DATABASE test;
CREATE DATABASE IF NOT EXISTS test;
```

#### Drop Database

```sql
DROP DATABASE [IF EXISTS] database_name;
```

Examples:
```sql
DROP DATABASE test;
DROP DATABASE IF EXISTS test;
```

#### Switch Database

```sql
USE database_name;
```

Example:
```sql
USE test;
```

#### Show All Databases

```sql
SHOW DATABASES;
```

### Table Operations

#### Create Table

```sql
CREATE TABLE [IF NOT EXISTS] table_name (
    column1_name column1_type [PRIMARY KEY],
    column2_name column2_type,
    ...
) [table_model];
```

**Table Models**:
- `DUPLICATE KEY`: Allows duplicates, suitable for detailed data (currently the primary supported model)

Example:
```sql
CREATE TABLE user (
    id BIGINT PRIMARY KEY,
    name VARCHAR(64),
    age INT,
    city VARCHAR(64)
) DUPLICATE KEY;
```

#### Show Create Table

```sql
SHOW CREATE TABLE table_name;
```

Example:
```sql
SHOW CREATE TABLE user;
```

#### Describe Table

```sql
DESCRIBE table_name;
-- or
DESC table_name;
```

#### Drop Table

```sql
DROP TABLE [IF EXISTS] table_name;
```

Examples:
```sql
DROP TABLE user;
DROP TABLE IF EXISTS user;
```

#### Truncate Table

```sql
TRUNCATE TABLE table_name;
```

Example:
```sql
TRUNCATE TABLE user;
```

### View Operations

#### Create View

```sql
CREATE VIEW view_name AS
SELECT ...;
```

Example:
```sql
CREATE VIEW view_user_beijing AS
SELECT * FROM user WHERE city = 'Beijing';
```

#### Drop View

```sql
DROP VIEW [IF EXISTS] view_name;
```

Example:
```sql
DROP VIEW view_user_beijing;
```

## Data Manipulation Language (DML)

### Insert Data

#### Insert Single Row

```sql
INSERT INTO table_name VALUES (value1, value2, ...);
```

Example:
```sql
INSERT INTO user VALUES (1, 'Alice', 30, 'Beijing');
```

#### Insert Multiple Rows

```sql
INSERT INTO table_name VALUES 
    (value1, value2, ...),
    (value3, value4, ...),
    ...;
```

Example:
```sql
INSERT INTO user VALUES 
    (2, 'Bob', 25, 'Shanghai'),
    (3, 'Charlie', 35, 'Beijing'),
    (4, 'David', 28, 'Shenzhen');
```

### Query Data

#### Basic Query

```sql
SELECT column1, column2, ...
FROM table_name
[WHERE condition]
[GROUP BY column1, column2, ...]
[HAVING condition]
[ORDER BY column1 [ASC|DESC], ...]
[LIMIT n];
```

Examples:
```sql
-- Query all columns
SELECT * FROM user;

-- Query specific columns
SELECT name, age FROM user;

-- Conditional query
SELECT * FROM user WHERE age > 30;

-- Sorting
SELECT * FROM user ORDER BY age DESC;

-- Limit result count
SELECT * FROM user LIMIT 10;

-- Group aggregation
SELECT city, COUNT(*), AVG(age) 
FROM user 
GROUP BY city
HAVING COUNT(*) >= 2;
```

#### WHERE Clause

Supported conditional operators:

| Operator | Description | Example |
|----------|-------------|---------|
| `=` | Equal | `age = 30` |
| `<>` or `!=` | Not equal | `age != 30` |
| `>` | Greater than | `age > 25` |
| `<` | Less than | `age < 40` |
| `>=` | Greater than or equal | `age >= 18` |
| `<=` | Less than or equal | `age <= 65` |
| `BETWEEN ... AND ...` | Range | `age BETWEEN 20 AND 30` |
| `IN (...)` | In list | `city IN ('Beijing', 'Shanghai')` |
| `LIKE` | Pattern matching | `name LIKE 'A%'` |
| `IS NULL` | Is null | `name IS NULL` |
| `IS NOT NULL` | Is not null | `name IS NOT NULL` |
| `AND` | Logical AND | `age > 20 AND city = 'Beijing'` |
| `OR` | Logical OR | `age < 20 OR age > 60` |
| `NOT` | Logical NOT | `NOT (age > 30)` |

Example:
```sql
SELECT * FROM user 
WHERE age > 25 
  AND city IN ('Beijing', 'Shanghai')
  AND name LIKE 'A%';
```

#### GROUP BY Clause

```sql
SELECT city, COUNT(*), AVG(age)
FROM user
GROUP BY city;
```

#### HAVING Clause

```sql
SELECT city, COUNT(*) as cnt
FROM user
GROUP BY city
HAVING cnt >= 2;
```

#### ORDER BY Clause

```sql
-- Ascending (default)
SELECT * FROM user ORDER BY age;

-- Descending
SELECT * FROM user ORDER BY age DESC;

-- Multi-column sorting
SELECT * FROM user ORDER BY city ASC, age DESC;
```

#### LIMIT Clause

```sql
-- Limit number of rows returned
SELECT * FROM user LIMIT 10;

-- Pagination (skip first 20 rows, return 10 rows)
SELECT * FROM user LIMIT 10 OFFSET 20;
```

### Subqueries

#### IN Subquery

```sql
SELECT * FROM user 
WHERE city IN (SELECT city FROM orders);
```

#### EXISTS Subquery

```sql
SELECT * FROM user u
WHERE EXISTS (SELECT 1 FROM orders o WHERE o.user_id = u.id);
```

#### NOT IN / NOT EXISTS

```sql
SELECT * FROM user 
WHERE city NOT IN (SELECT city FROM orders);

SELECT * FROM user u
WHERE NOT EXISTS (SELECT 1 FROM orders o WHERE o.user_id = u.id);
```

### Set Operations

#### UNION (Deduplicated Union)

```sql
SELECT name FROM user WHERE age < 30
UNION
SELECT name FROM user WHERE age > 30;
```

#### UNION ALL (Preserve Duplicates)

```sql
SELECT name FROM user WHERE age < 30
UNION ALL
SELECT name FROM user WHERE age > 25;
```

#### INTERSECT (Intersection)

```sql
SELECT city FROM user
INTERSECT
SELECT city FROM orders;
```

#### EXCEPT (Difference)

```sql
SELECT city FROM user
EXCEPT
SELECT city FROM orders;
```

### CTE (Common Table Expressions)

```sql
WITH cte_name AS (
    SELECT * FROM user WHERE age > 25
)
SELECT * FROM cte_name;

-- Multiple CTEs
WITH 
young_users AS (SELECT * FROM user WHERE age < 30),
beijing_users AS (SELECT * FROM user WHERE city = 'Beijing')
SELECT * FROM young_users
INTERSECT
SELECT * FROM beijing_users;
```

## Aggregate Functions

### Basic Aggregate Functions

| Function | Description | Example |
|----------|-------------|---------|
| `COUNT(*)` | Count (including NULL) | `SELECT COUNT(*) FROM user` |
| `COUNT(expr)` | Count (excluding NULL) | `SELECT COUNT(name) FROM user` |
| `COUNT(DISTINCT expr)` | Count distinct | `SELECT COUNT(DISTINCT city) FROM user` |
| `SUM(expr)` | Sum | `SELECT SUM(age) FROM user` |
| `AVG(expr)` | Average | `SELECT AVG(age) FROM user` |
| `MIN(expr)` | Minimum | `SELECT MIN(age) FROM user` |
| `MAX(expr)` | Maximum | `SELECT MAX(age) FROM user` |
| `GROUP_CONCAT(expr)` | String concatenation | `SELECT GROUP_CONCAT(name) FROM user` |

Example:
```sql
SELECT 
    COUNT(*) as total,
    COUNT(DISTINCT city) as city_count,
    SUM(age) as sum_age,
    AVG(age) as avg_age,
    MIN(age) as min_age,
    MAX(age) as max_age
FROM user;
```

## Window Functions

### Ranking Functions

| Function | Description |
|----------|-------------|
| `ROW_NUMBER()` | Row number (no ties) |
| `RANK()` | Rank (with ties, gaps in numbering) |
| `DENSE_RANK()` | Dense rank (with ties, no gaps) |

Example:
```sql
SELECT name, age,
    ROW_NUMBER() OVER (ORDER BY age) as rn,
    RANK() OVER (ORDER BY age DESC) as rank,
    DENSE_RANK() OVER (ORDER BY age DESC) as dense_rank
FROM user;
```

### Lead/Lag Functions

| Function | Description |
|----------|-------------|
| `LAG(expr, n, default)` | Access previous n rows |
| `LEAD(expr, n, default)` | Access next n rows |

Example:
```sql
SELECT name, age,
    LAG(age, 1, 0) OVER (ORDER BY age) as prev_age,
    LEAD(age, 1, 0) OVER (ORDER BY age) as next_age
FROM user;
```

### Window Function Syntax

```sql
window_function() OVER (
    [PARTITION BY column1, column2, ...]
    [ORDER BY column1 [ASC|DESC], ...]
    [ROWS BETWEEN ... AND ...]
)
```

Example:
```sql
-- Partition by city, order by age
SELECT name, city, age,
    ROW_NUMBER() OVER (PARTITION BY city ORDER BY age) as city_rank
FROM user;
```

## Mathematical Functions

| Function | Description | Example |
|----------|-------------|---------|
| `ABS(x)` | Absolute value | `SELECT ABS(-10)` → `10` |
| `CEIL(x)` or `CEILING(x)` | Ceiling (round up) | `SELECT CEIL(10.1)` → `11` |
| `FLOOR(x)` | Floor (round down) | `SELECT FLOOR(10.9)` → `10` |
| `ROUND(x)` | Round to nearest | `SELECT ROUND(10.5)` → `11` |
| `POWER(x, y)` or `POW(x, y)` | Power | `SELECT POWER(2, 3)` → `8` |
| `SQRT(x)` | Square root | `SELECT SQRT(16)` → `4` |
| `EXP(x)` | Exponential | `SELECT EXP(1)` → `2.718...` |
| `LOG(x)` | Natural logarithm | `SELECT LOG(2.718...)` → `1` |
| `LOG10(x)` | Base-10 logarithm | `SELECT LOG10(100)` → `2` |
| `MOD(x, y)` | Modulo | `SELECT MOD(10, 3)` → `1` |
| `PI()` | Pi constant | `SELECT PI()` → `3.141593` |
| `RAND()` | Random number | `SELECT RAND()` → `0.123...` |
| `SIGN(x)` | Sign function | `SELECT SIGN(-10)` → `-1` |

### Trigonometric Functions

| Function | Description |
|----------|-------------|
| `SIN(x)` | Sine |
| `COS(x)` | Cosine |
| `TAN(x)` | Tangent |
| `ASIN(x)` | Arcsine |
| `ACOS(x)` | Arccosine |
| `ATAN(x)` | Arctangent |

Example:
```sql
SELECT SIN(PI()/2), COS(0), TAN(PI()/4);
```

## String Functions

| Function | Description | Example |
|----------|-------------|---------|
| `CONCAT(str1, str2, ...)` | String concatenation | `SELECT CONCAT('Hello', ' ', 'World')` |
| `LENGTH(str)` | String length | `SELECT LENGTH('Hello')` → `5` |
| `UPPER(str)` | To uppercase | `SELECT UPPER('hello')` → `'HELLO'` |
| `LOWER(str)` | To lowercase | `SELECT LOWER('HELLO')` → `'hello'` |
| `TRIM(str)` | Trim spaces from both ends | `SELECT TRIM('  hello  ')` → `'hello'` |
| `LTRIM(str)` | Trim spaces from left | `SELECT LTRIM('  hello')` → `'hello'` |
| `RTRIM(str)` | Trim spaces from right | `SELECT RTRIM('hello  ')` → `'hello'` |
| `SUBSTRING(str, pos, len)` | Substring | `SELECT SUBSTRING('Hello', 2, 3)` → `'ell'` |
| `REPLACE(str, from, to)` | Replace | `SELECT REPLACE('Hello', 'H', 'J')` → `'Jello'` |

## Date and Time Functions

| Function | Description | Example |
|----------|-------------|---------|
| `NOW()` | Current date and time | `SELECT NOW()` |
| `CURDATE()` | Current date | `SELECT CURDATE()` |
| `CURTIME()` | Current time | `SELECT CURTIME()` |
| `YEAR(date)` | Extract year | `SELECT YEAR('2024-01-15')` → `2024` |
| `MONTH(date)` | Extract month | `SELECT MONTH('2024-01-15')` → `1` |
| `DAY(date)` | Extract day | `SELECT DAY('2024-01-15')` → `15` |
| `DATE_ADD(date, interval)` | Add to date | `SELECT DATE_ADD('2024-01-01', INTERVAL 1 DAY)` |
| `DATE_SUB(date, interval)` | Subtract from date | `SELECT DATE_SUB('2024-01-01', INTERVAL 1 DAY)` |
| `DATEDIFF(date1, date2)` | Date difference | `SELECT DATEDIFF('2024-01-15', '2024-01-01')` → `14` |

## Conditional Expressions

### CASE WHEN

```sql
-- Simple CASE
SELECT name, age,
    CASE city
        WHEN 'Beijing' THEN 'North'
        WHEN 'Shanghai' THEN 'East'
        ELSE 'Other'
    END as region
FROM user;

-- Searched CASE
SELECT name, age,
    CASE
        WHEN age < 20 THEN 'Young'
        WHEN age BETWEEN 20 AND 40 THEN 'Adult'
        ELSE 'Senior'
    END as age_group
FROM user;
```

### IF Function

```sql
SELECT name, IF(age >= 18, 'Adult', 'Minor') as status
FROM user;
```

### COALESCE Function

```sql
SELECT COALESCE(name, 'Unknown') FROM user;
```

## Query Plan Analysis

### EXPLAIN

View the query execution plan:

```sql
EXPLAIN SELECT city, COUNT(*) FROM user GROUP BY city;
```

Sample output:
```
Query Plan:
  HashAggregate {
    group_by: [city],
    aggr_exprs: [COUNT(*)],
    input: TableScan {
      table: "user",
      projections: [city]
    }
  }
```

## Data Types

### Numeric Types

| Type | Description | Range |
|------|-------------|-------|
| `TINYINT` | 8-bit signed integer | -128 to 127 |
| `SMALLINT` | 16-bit signed integer | -32768 to 32767 |
| `INT` / `INTEGER` | 32-bit signed integer | -2^31 to 2^31-1 |
| `BIGINT` | 64-bit signed integer | -2^63 to 2^63-1 |
| `FLOAT` | Single-precision float | ~7 significant digits |
| `DOUBLE` | Double-precision float | ~15 significant digits |

### String Types

| Type | Description |
|------|-------------|
| `VARCHAR(n)` | Variable-length string, maximum n characters |
| `CHAR(n)` | Fixed-length string (planned) |
| `TEXT` | Long text (planned) |

### Date and Time Types

| Type | Description | Format |
|------|-------------|--------|
| `DATE` | Date | `YYYY-MM-DD` |
| `DATETIME` | Date and time | `YYYY-MM-DD HH:MM:SS` |
| `TIMESTAMP` | Timestamp (planned) | `YYYY-MM-DD HH:MM:SS` |

### Other Types

| Type | Description |
|------|-------------|
| `BOOLEAN` / `BOOL` | Boolean (true/false) |
| `NULL` | Null value |

## SQL Mode and Compatibility

RorisDB's SQL syntax is compatible with MySQL, but has some limitations:

### Currently Supported
- Basic DDL (CREATE, DROP, TRUNCATE)
- Basic DML (INSERT, SELECT)
- Aggregate functions
- Window functions
- Subqueries (IN, EXISTS)
- Set operations (UNION, INTERSECT, EXCEPT)
- CTE (WITH clause)
- Views

### Not Yet Supported
- UPDATE, DELETE (planned)
- Foreign key constraints
- Stored procedures
- Triggers
- Transactions (multi-database transactions planned)

## Best Practices

### Query Optimization

1. **Use WHERE to filter data early**
   ```sql
   -- Good: filter early
   SELECT city, COUNT(*) FROM user WHERE age > 20 GROUP BY city;
   ```

2. **Query only the columns you need**
   ```sql
   -- Good: query only needed columns
   SELECT id, name FROM user;
   ```

3. **Use LIMIT to restrict result sets**
   ```sql
   SELECT * FROM user LIMIT 100;
   ```

4. **Use indexes appropriately**
   - ZoneMap indexes are created automatically for range filtering
   - BloomFilter indexes are suitable for high-cardinality columns

### Data Modeling

1. **Choose the appropriate table model**
   - Currently, the `DUPLICATE KEY` model is primarily used

2. **Design primary keys appropriately**
   ```sql
   CREATE TABLE orders (
       order_id BIGINT PRIMARY KEY,
       ...
   ) DUPLICATE KEY;
   ```

## Next Steps

- See [Configuration Guide](configuration.md) for system configuration
- Read [Features](features.md) for the complete feature list
- Refer to [Developer Guide](developer-guide.md) to contribute to the project
