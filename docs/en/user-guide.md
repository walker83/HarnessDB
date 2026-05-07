# RorisDB SQL User Guide

This guide is for users who query and analyze data using SQL. RorisDB is highly compatible with Apache Doris SQL syntax, allowing you to leverage your existing Doris SQL skills.

## About RorisDB

### Compatibility with Apache Doris

RorisDB adopts the same architectural philosophy as Apache Doris (MPP architecture, columnar storage, materialized views) with high SQL-level compatibility:

- **SQL Syntax**: MySQL-compatible, supports Doris-style SQL
- **Data Types**: Common OLAP data types
- **Table Models**: DUPLICATE KEY, UNIQUE KEY, AGGREGATE KEY
- **Partitioning**: Range, List, Hash partitions
- **Query Optimization**: Predicate pushdown, column pruning, runtime filter

### Key Advantages

- **Memory Safety**: Implemented in Rust, no memory leak risks
- **High Performance**: Vectorized execution, zero-copy optimization
- **Cloud Native**: Container deployment ready, easy K8s integration
- **Real-time Analytics**: Low latency query response

## Quick Connection

### Using MySQL Client

```bash
mysql -h 127.0.0.1 -P 9030 -uroot
```

Connection parameters:
- `-h`: FE host address (default 127.0.0.1)
- `-P`: MySQL protocol port (default 9030)
- `-u`: Username (default root)

### Using RorisDB CLI

```bash
./target/release/roris-cli
```

## Database Management

### Create Database

```sql
CREATE DATABASE IF NOT EXISTS analytics_db;
```

### Show Databases

```sql
SHOW DATABASES;
```

### Switch Database

```sql
USE analytics_db;
```

### Drop Database

```sql
DROP DATABASE IF EXISTS analytics_db;
```

## Table Management

### Table Models

RorisDB supports three table models, consistent with Doris:

#### 1. DUPLICATE KEY Model (Detail Table)

Suitable for retaining complete detail data without aggregation:

```sql
CREATE TABLE order_detail (
    order_id BIGINT,
    user_id BIGINT,
    product_name VARCHAR(128),
    quantity INT,
    price DOUBLE,
    order_time DATETIME
)
DUPLICATE KEY(order_id)
DISTRIBUTED BY HASH(user_id) BUCKETS 10;
```

#### 2. UNIQUE KEY Model (Unique Primary Key Table)

Suitable for scenarios requiring unique key constraints:

```sql
CREATE TABLE user_profile (
    user_id BIGINT,
    user_name VARCHAR(64),
    email VARCHAR(128),
    phone VARCHAR(32),
    last_update DATETIME
)
UNIQUE KEY(user_id)
DISTRIBUTED BY HASH(user_id) BUCKETS 10;
```

Features:
- Same primary key data updates rather than appends
- Supports `INSERT ON DUPLICATE KEY UPDATE`
- Suitable for dimension tables, user profiles

#### 3. AGGREGATE KEY Model (Aggregation Table)

Suitable for pre-aggregation scenarios (planned):

```sql
CREATE TABLE user_visit_stats (
    user_id BIGINT,
    visit_date DATE,
    visit_count INT SUM,
    total_duration BIGINT SUM
)
AGGREGATE KEY(user_id, visit_date)
DISTRIBUTED BY HASH(user_id) BUCKETS 10;
```

### Partitioned Tables

#### Range Partitioning

```sql
CREATE TABLE sales_range (
    sale_id BIGINT,
    sale_date DATE,
    amount DOUBLE
)
DUPLICATE KEY(sale_id)
PARTITION BY RANGE(sale_date) (
    PARTITION p202301 VALUES LESS THAN ('2023-02-01'),
    PARTITION p202302 VALUES LESS THAN ('2023-03-01'),
    PARTITION p202303 VALUES LESS THAN ('2023-04-01')
)
DISTRIBUTED BY HASH(sale_id) BUCKETS 10;
```

Add partition dynamically:

```sql
ALTER TABLE sales_range ADD PARTITION p202304 
VALUES LESS THAN ('2023-05-01');
```

#### List Partitioning

```sql
CREATE TABLE sales_list (
    sale_id BIGINT,
    region VARCHAR(32),
    amount DOUBLE
)
DUPLICATE KEY(sale_id)
PARTITION BY LIST(region) (
    PARTITION p_north VALUES IN ('Beijing', 'Tianjin'),
    PARTITION p_south VALUES IN ('Shanghai', 'Guangzhou')
)
DISTRIBUTED BY HASH(sale_id) BUCKETS 10;
```

#### Hash Partitioning

```sql
CREATE TABLE sales_hash (
    sale_id BIGINT,
    user_id BIGINT,
    amount DOUBLE
)
DUPLICATE KEY(sale_id)
PARTITION BY HASH(user_id) BUCKETS 32
DISTRIBUTED BY HASH(sale_id) BUCKETS 10;
```

### Table Operations

#### View Table Structure

```sql
DESCRIBE table_name;

SHOW CREATE TABLE table_name;
```

#### Alter Table

```sql
-- Rename table
ALTER TABLE table_name RENAME new_table_name;

-- Add column
ALTER TABLE table_name ADD COLUMN new_col VARCHAR(64);

-- Drop column
ALTER TABLE table_name DROP COLUMN old_col;

-- Modify column type
ALTER TABLE table_name MODIFY COLUMN col_name BIGINT;
```

#### Drop Table

```sql
DROP TABLE IF EXISTS table_name;
```

#### Truncate Table

```sql
TRUNCATE TABLE table_name;
```

## Data Operations (DML)

### INSERT

#### Basic INSERT

```sql
-- VALUES syntax
INSERT INTO table_name VALUES 
    (1, 'Alice', 30),
    (2, 'Bob', 25),
    (3, 'Charlie', 35);

-- Specify columns
INSERT INTO table_name (id, name) VALUES 
    (4, 'David'),
    (5, 'Eve');
```

#### INSERT ... SELECT

```sql
INSERT INTO target_table 
SELECT * FROM source_table WHERE condition;
```

#### INSERT SET Syntax

```sql
INSERT INTO table_name SET 
    id = 6, 
    name = 'Frank', 
    age = 40;
```

#### INSERT ON DUPLICATE KEY UPDATE

Upsert operation for UNIQUE KEY tables:

```sql
INSERT INTO user_profile VALUES (1, 'Alice', 'alice@example.com', '123456', NOW())
ON DUPLICATE KEY UPDATE 
    user_name = 'Alice',
    email = 'alice@example.com',
    last_update = NOW();
```

### UPDATE

```sql
-- Basic UPDATE
UPDATE table_name SET age = 31 WHERE name = 'Alice';

-- Multiple columns
UPDATE table_name SET 
    age = 32, 
    city = 'Shanghai' 
WHERE id = 1;

-- Using expression
UPDATE table_name SET price = price * 1.1 WHERE category = 'electronics';
```

### DELETE

```sql
-- Conditional delete
DELETE FROM table_name WHERE age < 25;

-- DELETE with ORDER BY and LIMIT
DELETE FROM table_name ORDER BY create_time DESC LIMIT 100;

-- Full table delete (use with caution)
DELETE FROM table_name;
```

## Transaction Support

RorisDB supports basic transaction operations:

### Begin Transaction

```sql
BEGIN;

-- Or
START TRANSACTION;
```

### Commit Transaction

```sql
COMMIT;
```

### Rollback Transaction

```sql
ROLLBACK;
```

### Savepoints

```sql
BEGIN;

INSERT INTO table_name VALUES (1, 'Alice', 30);
SAVEPOINT sp1;

UPDATE table_name SET age = 31 WHERE id = 1;
SAVEPOINT sp2;

-- Rollback to savepoint
ROLLBACK TO sp1;

-- Release savepoint
RELEASE SAVEPOINT sp2;

COMMIT;
```

### Transaction Example

```sql
BEGIN;

INSERT INTO orders VALUES (1001, 1, 'Laptop', 5999.99, NOW());
UPDATE user_profile SET last_order_time = NOW() WHERE user_id = 1;

COMMIT;
```

## Data Query

### Basic Query

```sql
-- Full table query
SELECT * FROM table_name;

-- Select columns
SELECT col1, col2, col3 FROM table_name;

-- Conditional filter
SELECT * FROM table_name WHERE age > 30 AND city = 'Beijing';

-- Sort
SELECT * FROM table_name ORDER BY age DESC, name ASC;

-- Limit results
SELECT * FROM table_name LIMIT 10;
SELECT * FROM table_name LIMIT 10 OFFSET 20;
```

### Aggregation Query

```sql
-- Aggregate functions
SELECT 
    COUNT(*) as total_count,
    COUNT(DISTINCT user_id) as unique_users,
    SUM(amount) as total_amount,
    AVG(price) as avg_price,
    MIN(create_time) as earliest,
    MAX(create_time) as latest
FROM orders;

-- GROUP BY
SELECT 
    city,
    COUNT(*) as user_count,
    AVG(age) as avg_age
FROM user_profile
GROUP BY city;

-- HAVING
SELECT 
    city,
    COUNT(*) as user_count
FROM user_profile
GROUP BY city
HAVING COUNT(*) >= 100
ORDER BY user_count DESC;
```

### Window Functions

```sql
-- ROW_NUMBER
SELECT 
    name,
    age,
    ROW_NUMBER() OVER (ORDER BY age DESC) as rank
FROM user_profile;

-- RANK (allows ties, skips ranks)
SELECT 
    name,
    age,
    RANK() OVER (ORDER BY age DESC) as rank
FROM user_profile;

-- DENSE_RANK (allows ties, no skips)
SELECT 
    name,
    age,
    DENSE_RANK() OVER (ORDER BY age DESC) as rank
FROM user_profile;

-- Partitioned window function
SELECT 
    name,
    city,
    age,
    ROW_NUMBER() OVER (PARTITION BY city ORDER BY age DESC) as city_rank
FROM user_profile;

-- LAG/LEAD
SELECT 
    order_date,
    amount,
    LAG(amount, 1) OVER (ORDER BY order_date) as prev_amount,
    LEAD(amount, 1) OVER (ORDER BY order_date) as next_amount
FROM daily_sales;
```

### Join Query

#### INNER JOIN

```sql
SELECT 
    o.order_id,
    u.user_name,
    o.amount
FROM orders o
INNER JOIN user_profile u ON o.user_id = u.user_id;
```

#### LEFT JOIN

```sql
SELECT 
    u.user_name,
    COUNT(o.order_id) as order_count
FROM user_profile u
LEFT JOIN orders o ON u.user_id = o.user_id
GROUP BY u.user_name;
```

#### Multi-table Join

```sql
SELECT 
    o.order_id,
    u.user_name,
    p.product_name,
    o.quantity
FROM orders o
INNER JOIN user_profile u ON o.user_id = u.user_id
INNER JOIN products p ON o.product_id = p.product_id
WHERE o.order_date >= '2024-01-01';
```

### Subquery

#### IN Subquery

```sql
SELECT * FROM orders 
WHERE user_id IN (
    SELECT user_id FROM user_profile WHERE city = 'Beijing'
);
```

#### EXISTS Subquery

```sql
SELECT u.user_name 
FROM user_profile u
WHERE EXISTS (
    SELECT 1 FROM orders o 
    WHERE o.user_id = u.user_id 
    AND o.amount > 1000
);
```

### CTE (Common Table Expression)

#### Non-recursive CTE

```sql
WITH high_value_orders AS (
    SELECT * FROM orders WHERE amount > 5000
),
beijing_users AS (
    SELECT user_id FROM user_profile WHERE city = 'Beijing'
)
SELECT 
    h.order_id,
    h.amount,
    u.user_id
FROM high_value_orders h
INNER JOIN beijing_users u ON h.user_id = u.user_id;
```

#### Recursive CTE

```sql
WITH RECURSIVE user_hierarchy AS (
    -- Base query
    SELECT user_id, manager_id, 1 as level
    FROM employee
    WHERE manager_id IS NULL
    
    UNION ALL
    
    -- Recursive query
    SELECT e.user_id, e.manager_id, h.level + 1
    FROM employee e
    INNER JOIN user_hierarchy h ON e.manager_id = h.user_id
)
SELECT * FROM user_hierarchy ORDER BY level;
```

### Set Operations

```sql
-- UNION (distinct)
SELECT user_id FROM orders_2023
UNION
SELECT user_id FROM orders_2024;

-- UNION ALL (keep duplicates)
SELECT user_id FROM orders_2023
UNION ALL
SELECT user_id FROM orders_2024;

-- INTERSECT
SELECT user_id FROM orders_2023
INTERSECT
SELECT user_id FROM orders_2024;

-- EXCEPT
SELECT user_id FROM orders_2023
EXCEPT
SELECT user_id FROM orders_2024;
```

## View

### Create View

```sql
CREATE VIEW view_user_orders AS
SELECT 
    u.user_name,
    COUNT(o.order_id) as order_count,
    SUM(o.amount) as total_amount
FROM user_profile u
LEFT JOIN orders o ON u.user_id = o.user_id
GROUP BY u.user_name;
```

### Query View

```sql
SELECT * FROM view_user_orders WHERE order_count > 10;
```

### Show View Definition

```sql
SHOW CREATE VIEW view_user_orders;
```

### Drop View

```sql
DROP VIEW IF EXISTS view_user_orders;
```

## Materialized View

Materialized views can improve complex query performance:

### Create Materialized View

```sql
CREATE MATERIALIZED VIEW mv_daily_sales AS
SELECT 
    sale_date,
    SUM(amount) as daily_total,
    COUNT(*) as daily_count
FROM sales
GROUP BY sale_date;
```

### Query Rewrite

RorisDB automatically rewrites qualifying queries to use materialized views:

```sql
-- Original query
SELECT sale_date, SUM(amount) 
FROM sales 
WHERE sale_date >= '2024-01-01'
GROUP BY sale_date;

-- Automatically rewritten to use materialized view
SELECT sale_date, daily_total 
FROM mv_daily_sales 
WHERE sale_date >= '2024-01-01';
```

## User and Permission Management

### User Management

```sql
-- Create user
CREATE USER 'analyst'@'%' IDENTIFIED BY 'password123';

-- Change password
ALTER USER 'analyst'@'%' IDENTIFIED BY 'newpassword';

-- Set password
SET PASSWORD FOR 'analyst'@'%' = 'newpassword';

-- Drop user
DROP USER 'analyst'@'%';
```

### Permission Management

```sql
-- Grant permissions
GRANT SELECT, INSERT ON database_name.* TO 'analyst'@'%';

-- Grant all privileges
GRANT ALL PRIVILEGES ON database_name.* TO 'analyst'@'%';

-- Revoke permissions
REVOKE INSERT ON database_name.* FROM 'analyst'@'%';

-- Show grants
SHOW GRANTS FOR 'analyst'@'%';
```

### Permission Types

- SELECT: Query permission
- INSERT: Insert permission
- UPDATE: Update permission
- DELETE: Delete permission
- CREATE: Create table/database permission
- DROP: Drop table/database permission
- ALTER: Alter table permission
- ALL PRIVILEGES: All permissions

## Performance Optimization Tips

### 1. Choose Appropriate Table Model

- **Detail queries**: Use DUPLICATE KEY
- **Unique key updates**: Use UNIQUE KEY
- **Pre-aggregation**: Use AGGREGATE KEY

### 2. Proper Partitioning

- Time-based partitioning: Easy for time-range queries and data lifecycle management
- Region-based partitioning: Easy for regional analysis
- Dynamic partitioning: Automatic historical data management

### 3. Use Indexes

- ZoneMap index: Automatically created, suitable for range queries
- BloomFilter index: High cardinality columns, equality query optimization

### 4. Query Optimization

- **Predicate pushdown**: Filter data early
- **Column pruning**: Query only needed columns
- **Use EXPLAIN**: View execution plan

```sql
EXPLAIN SELECT city, COUNT(*) FROM user_profile GROUP BY city;
```

### 5. Materialized View

Create materialized views for high-frequency queries:

```sql
CREATE MATERIALIZED VIEW mv_summary AS
SELECT ... GROUP BY ...
```

### 6. Statistics

Collect statistics regularly for CBO optimization:

```sql
ANALYZE TABLE table_name;
```

## Differences from Doris

### Supported Features

- ✅ Complete DML support (INSERT/UPDATE/DELETE)
- ✅ Transactions (BEGIN/COMMIT/ROLLBACK)
- ✅ Partitioned tables (Range/List/Hash)
- ✅ Materialized view framework
- ✅ Runtime Filter
- ✅ CBO optimizer
- ✅ User permission management

### Planned Features

- 🚧 UDF/UDAF
- 🚧 Federation queries (Hive/Iceberg)
- 🚧 DECIMAL precise calculation
- 🚧 Row-level security
- 🚧 Workload management

## Next Steps

- View [SQL Reference Manual](sql-reference.md) for complete syntax
- Read [Performance Report](performance.md) for performance features
- Refer to [Configuration Guide](configuration.md) for advanced settings

---

**RorisDB** - Apache Doris-compatible Rust OLAP Database