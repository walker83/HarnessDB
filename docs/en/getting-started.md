# RorisDB Quick Start

This guide will help you get started with RorisDB quickly, covering the entire process from installation to executing your first query.

## Prerequisites

- RorisDB has been compiled and installed (see [Installation Guide](installation.md))
- FE and BE services are running
- Optional: Install MySQL client (for connecting to the database)

## Connecting to the Database

### Method 1: Using MySQL Client

RorisDB supports the MySQL protocol and can be connected directly using a MySQL client:

```bash
mysql -h 127.0.0.1 -P 9030 -uroot
```

After successful connection, you will see a prompt similar to the following:

```
Welcome to the MySQL monitor.  Commands end with ; or \g.
Your MySQL connection id is 1
Server version: RorisDB 0.1.3

Copyright (c) 2000, 2024, Oracle and/or its affiliates.

Type 'help;' or '\h' for help. Type '\c' to clear the current input statement.

mysql>
```

### Method 2: Using roris-cli

RorisDB comes with a command-line client:

```bash
./target/release/roris-cli
```

## Basic Operations

### 1. Create Database

```sql
CREATE DATABASE IF NOT EXISTS test;
USE test;
```

### 2. Create Table

RorisDB supports multiple table models, currently primarily supporting the `DUPLICATE KEY` model:

```sql
CREATE TABLE user (
    id BIGINT PRIMARY KEY,
    name VARCHAR(64),
    age INT,
    city VARCHAR(64)
) DUPLICATE KEY;
```

**Table Model Description**:
- `DUPLICATE KEY`: Allows duplicate data, suitable for detailed data storage
- Other models (AGGREGATE, UNIQUE) are under planning

### 3. Insert Data

```sql
-- Insert single row
INSERT INTO user VALUES (1, 'Alice', 30, 'Beijing');

-- Insert multiple rows
INSERT INTO user VALUES 
    (2, 'Bob', 25, 'Shanghai'),
    (3, 'Charlie', 35, 'Beijing'),
    (4, 'David', 28, 'Shenzhen'),
    (5, 'Eve', 32, 'Shanghai');
```

### 4. Query Data

```sql
-- Query all data
SELECT * FROM user;

-- Conditional query
SELECT * FROM user WHERE age > 30;

-- Aggregation query
SELECT city, COUNT(*), AVG(age) 
FROM user 
GROUP BY city;

-- Sorting
SELECT * FROM user ORDER BY age DESC;

-- Limit results
SELECT * FROM user LIMIT 3;
```

### 5. Update and Delete

The current version primarily supports append writes; update and delete functions are being improved.

## Data Types

RorisDB supports the following data types:

| Type | Description | Example |
|------|-------------|---------|
| `BIGINT` | 64-bit integer | `1234567890` |
| `INT` | 32-bit integer | `12345` |
| `FLOAT` | Single-precision floating point | `3.14` |
| `DOUBLE` | Double-precision floating point | `3.1415926` |
| `VARCHAR(n)` | Variable-length string | `'Hello'` |
| `DATE` | Date | `'2024-01-01'` |
| `DATETIME` | Date and time | `'2024-01-01 12:00:00'` |
| `BOOLEAN` | Boolean value | `true` / `false` |

### Example: Creating a Table with Multiple Types

```sql
CREATE TABLE orders (
    order_id BIGINT PRIMARY KEY,
    user_id BIGINT,
    product_name VARCHAR(128),
    price DOUBLE,
    quantity INT,
    order_date DATE,
    is_paid BOOLEAN
) DUPLICATE KEY;

INSERT INTO orders VALUES
    (1, 1, 'Laptop', 5999.99, 1, '2024-01-15', true),
    (2, 2, 'Phone', 3999.99, 2, '2024-01-16', true),
    (3, 3, 'Tablet', 2999.99, 1, '2024-01-17', false);
```

## Common SQL Operations

### Aggregate Functions

RorisDB supports a rich set of aggregate functions:

```sql
-- Count
SELECT COUNT(*) FROM user;

-- Sum
SELECT SUM(age) FROM user;

-- Average
SELECT AVG(age) FROM user;

-- Maximum and minimum
SELECT MAX(age), MIN(age) FROM user;

-- Count distinct
SELECT COUNT(DISTINCT city) FROM user;

-- Concatenate strings
SELECT GROUP_CONCAT(name) FROM user WHERE age > 25;
```

### Window Functions

Supports commonly used window functions:

```sql
-- Row number
SELECT name, age, ROW_NUMBER() OVER (ORDER BY age) as rn
FROM user;

-- Rank
SELECT name, age, RANK() OVER (ORDER BY age DESC) as rank
FROM user;

-- Dense rank
SELECT name, age, DENSE_RANK() OVER (ORDER BY age DESC) as dense_rank
FROM user;

-- Lag and lead
SELECT name, age, 
       LAG(age, 1) OVER (ORDER BY age) as prev_age,
       LEAD(age, 1) OVER (ORDER BY age) as next_age
FROM user;
```

### Subqueries

Supports subqueries such as IN, EXISTS, etc.:

```sql
-- IN subquery
SELECT * FROM user 
WHERE city IN (SELECT city FROM user WHERE age > 30);

-- EXISTS subquery
SELECT * FROM user u
WHERE EXISTS (SELECT 1 FROM orders o WHERE o.user_id = u.id);
```

### Set Operations

Supports UNION, INTERSECT, EXCEPT:

```sql
-- UNION: Combine results (deduplicated)
SELECT name FROM user WHERE age < 30
UNION
SELECT name FROM user WHERE age > 30;

-- UNION ALL: Combine results (keep duplicates)
SELECT name FROM user WHERE age < 30
UNION ALL
SELECT name FROM user WHERE age > 25;

-- INTERSECT: Intersection
SELECT city FROM user
INTERSECT
SELECT city FROM orders;

-- EXCEPT: Difference
SELECT city FROM user
EXCEPT
SELECT city FROM orders;
```

### CTE (Common Table Expressions)

Supports WITH clauses:

```sql
WITH young_users AS (
    SELECT * FROM user WHERE age < 30
)
SELECT * FROM young_users WHERE city = 'Shanghai';
```

### Views

Supports creating views:

```sql
CREATE VIEW view_user_beijing AS
SELECT * FROM user WHERE city = 'Beijing';

-- Query view
SELECT * FROM view_user_beijing;

-- Drop view
DROP VIEW view_user_beijing;
```

## Data Import

### CSV Import (Planned)

```sql
-- Will be supported in the future
LOAD DATA INFILE '/path/to/data.csv'
INTO TABLE user
FIELDS TERMINATED BY ','
LINES TERMINATED BY '\n';
```

### Stream Load (Planned)

Import data via HTTP interface:

```bash
curl -X PUT \
  -H "format: csv" \
  -T data.csv \
  http://127.0.0.1:8030/api/test/user/_stream_load
```

## Query Analysis

### Using EXPLAIN to View Execution Plans

```sql
EXPLAIN SELECT city, COUNT(*) FROM user GROUP BY city;
```

Output example:
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

### Performance Analysis

```sql
-- View query execution time (in MySQL client)
SELECT /*+ SET_VAR(profile=true) */ city, COUNT(*) FROM user GROUP BY city;
```

## Practical Examples

### Example 1: User Age Distribution Statistics

```sql
SELECT 
    CASE 
        WHEN age < 20 THEN 'Under 20'
        WHEN age BETWEEN 20 AND 30 THEN '20-30'
        WHEN age BETWEEN 30 AND 40 THEN '30-40'
        ELSE 'Over 40'
    END as age_group,
    COUNT(*) as user_count
FROM user
GROUP BY age_group
ORDER BY user_count DESC;
```

### Example 2: Average Age and Count per City

```sql
SELECT 
    city,
    COUNT(*) as user_count,
    AVG(age) as avg_age,
    MIN(age) as min_age,
    MAX(age) as max_age
FROM user
GROUP BY city
HAVING COUNT(*) >= 2
ORDER BY user_count DESC;
```

### Example 3: Using Window Functions for Ranking

```sql
SELECT 
    name,
    age,
    city,
    RANK() OVER (PARTITION BY city ORDER BY age DESC) as city_rank
FROM user
WHERE city IN ('Beijing', 'Shanghai');
```

## Management Operations

### View Table Structure

```sql
-- View table creation statement
SHOW CREATE TABLE user;

-- View table information (planned)
DESCRIBE user;
```

### Truncate Table

```sql
TRUNCATE TABLE user;
```

### Drop Table

```sql
DROP TABLE IF EXISTS user;
```

### Drop Database

```sql
DROP DATABASE IF EXISTS test;
```

## Exiting the Client

```sql
-- MySQL client
EXIT;

-- or
QUIT;
```

## Next Steps

- Deep dive into the [SQL Reference Manual](sql-reference.md)
- Learn about [Configuration](configuration.md) for advanced configuration
- Check [Features](features.md) for the complete feature list
- Read the [Developer Guide](developer-guide.md) to contribute to the project

## FAQ

### Connection Failed

**Problem**: Unable to connect to the database

**Solution**:
1. Check if FE is running normally
2. Check if the MySQL port is correct (default 9030)
3. View FE logs: `tail -f ./fe_data/logs/roris-fe.log`

### Table Creation Failed

**Problem**: Error when creating a table

**Solution**:
1. Ensure the correct table model is used (currently primarily supports `DUPLICATE KEY`)
2. Check if the SQL syntax is correct
3. Ensure a database is selected (`USE database_name;`)

### Empty Query Results

**Problem**: Query returns no data

**Solution**:
1. Confirm data has been successfully inserted
2. Check if the WHERE condition is correct
3. Use `SELECT * FROM table_name;` to view all data
