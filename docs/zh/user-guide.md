# RorisDB SQL用户使用指南

本指南面向使用SQL查询和分析数据的用户，帮助您快速掌握RorisDB的使用方法。RorisDB高度兼容Apache Doris的SQL语法，您可以沿用熟悉的Doris SQL技能。

## 关于RorisDB

### 与Apache Doris的兼容性

RorisDB采用与Apache Doris相同的架构设计理念（MPP架构、列式存储、物化视图），在SQL层面高度兼容：

- **SQL语法**：MySQL兼容，支持Doris风格SQL
- **数据类型**：支持常用OLAP数据类型
- **表模型**：DUPLICATE KEY、UNIQUE KEY、AGGREGATE KEY
- **分区策略**：Range、List、Hash分区
- **查询优化**：谓词下推、列裁剪、Runtime Filter

### 主要优势

- **内存安全**：Rust语言实现，无内存泄漏风险
- **高性能**：向量化执行，零拷贝优化
- **云原生**：支持容器化部署，易于K8s集成
- **实时分析**：低延迟查询响应

## 快速连接

### 使用MySQL客户端

```bash
mysql -h 127.0.0.1 -P 9030 -uroot
```

连接参数：
- `-h`：FE主机地址（默认127.0.0.1）
- `-P`：MySQL协议端口（默认9030）
- `-u`：用户名（默认root）

### 使用RorisDB CLI

```bash
./target/release/roris-cli
```

## 数据库管理

### 创建数据库

```sql
CREATE DATABASE IF NOT EXISTS analytics_db;
```

### 查看数据库列表

```sql
SHOW DATABASES;
```

### 切换数据库

```sql
USE analytics_db;
```

### 删除数据库

```sql
DROP DATABASE IF EXISTS analytics_db;
```

## 表管理

### 表模型

RorisDB支持三种表模型，与Doris一致：

#### 1. DUPLICATE KEY模型（明细表）

适合保留完整明细数据，无聚合：

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

#### 2. UNIQUE KEY模型（唯一主键表）

适合需要唯一键约束的场景：

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

特点：
- 相同主键的数据会更新而非追加
- 支持`INSERT ON DUPLICATE KEY UPDATE`
- 适合维度表、用户画像等场景

#### 3. AGGREGATE KEY模型（聚合表）

适合预聚合场景（规划中）：

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

### 分区表

#### Range分区

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

动态添加分区：

```sql
ALTER TABLE sales_range ADD PARTITION p202304 
VALUES LESS THAN ('2023-05-01');
```

#### List分区

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

#### Hash分区

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

### 表操作

#### 查看表结构

```sql
DESCRIBE table_name;

SHOW CREATE TABLE table_name;
```

#### 修改表

```sql
-- 重命名表
ALTER TABLE table_name RENAME new_table_name;

-- 添加列
ALTER TABLE table_name ADD COLUMN new_col VARCHAR(64);

-- 删除列
ALTER TABLE table_name DROP COLUMN old_col;

-- 修改列类型
ALTER TABLE table_name MODIFY COLUMN col_name BIGINT;
```

#### 删除表

```sql
DROP TABLE IF EXISTS table_name;
```

#### 清空表

```sql
TRUNCATE TABLE table_name;
```

## 数据操作（DML）

### INSERT

#### 基本INSERT

```sql
-- VALUES方式
INSERT INTO table_name VALUES 
    (1, 'Alice', 30),
    (2, 'Bob', 25),
    (3, 'Charlie', 35);

-- 指定列
INSERT INTO table_name (id, name) VALUES 
    (4, 'David'),
    (5, 'Eve');
```

#### INSERT ... SELECT

```sql
INSERT INTO target_table 
SELECT * FROM source_table WHERE condition;
```

#### INSERT SET语法

```sql
INSERT INTO table_name SET 
    id = 6, 
    name = 'Frank', 
    age = 40;
```

#### INSERT ON DUPLICATE KEY UPDATE

用于UNIQUE KEY表的Upsert操作：

```sql
INSERT INTO user_profile VALUES (1, 'Alice', 'alice@example.com', '123456', NOW())
ON DUPLICATE KEY UPDATE 
    user_name = 'Alice',
    email = 'alice@example.com',
    last_update = NOW();
```

### UPDATE

```sql
-- 基本UPDATE
UPDATE table_name SET age = 31 WHERE name = 'Alice';

-- 多列UPDATE
UPDATE table_name SET 
    age = 32, 
    city = 'Shanghai' 
WHERE id = 1;

-- 使用表达式
UPDATE table_name SET price = price * 1.1 WHERE category = 'electronics';
```

### DELETE

```sql
-- 条件删除
DELETE FROM table_name WHERE age < 25;

-- DELETE with ORDER BY and LIMIT
DELETE FROM table_name ORDER BY create_time DESC LIMIT 100;

-- 全表删除（慎用）
DELETE FROM table_name;
```

## 事务支持

RorisDB支持基本的事务操作：

### 开始事务

```sql
BEGIN;

-- 或
START TRANSACTION;
```

### 提交事务

```sql
COMMIT;
```

### 回滚事务

```sql
ROLLBACK;
```

### 保存点

```sql
BEGIN;

INSERT INTO table_name VALUES (1, 'Alice', 30);
SAVEPOINT sp1;

UPDATE table_name SET age = 31 WHERE id = 1;
SAVEPOINT sp2;

-- 回滚到保存点
ROLLBACK TO sp1;

-- 释放保存点
RELEASE SAVEPOINT sp2;

COMMIT;
```

### 事务示例

```sql
BEGIN;

INSERT INTO orders VALUES (1001, 1, 'Laptop', 5999.99, NOW());
UPDATE user_profile SET last_order_time = NOW() WHERE user_id = 1;

COMMIT;
```

## 数据查询

### 基本查询

```sql
-- 全表查询
SELECT * FROM table_name;

-- 选择列
SELECT col1, col2, col3 FROM table_name;

-- 条件过滤
SELECT * FROM table_name WHERE age > 30 AND city = 'Beijing';

-- 排序
SELECT * FROM table_name ORDER BY age DESC, name ASC;

-- 限制结果
SELECT * FROM table_name LIMIT 10;
SELECT * FROM table_name LIMIT 10 OFFSET 20;
```

### 聚合查询

```sql
-- 聚合函数
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

### 窗口函数

```sql
-- ROW_NUMBER
SELECT 
    name,
    age,
    ROW_NUMBER() OVER (ORDER BY age DESC) as rank
FROM user_profile;

-- RANK（允许并列，跳号）
SELECT 
    name,
    age,
    RANK() OVER (ORDER BY age DESC) as rank
FROM user_profile;

-- DENSE_RANK（允许并列，不跳号）
SELECT 
    name,
    age,
    DENSE_RANK() OVER (ORDER BY age DESC) as rank
FROM user_profile;

-- 分组窗口函数
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

### 连接查询

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

#### 多表连接

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

### 子查询

#### IN子查询

```sql
SELECT * FROM orders 
WHERE user_id IN (
    SELECT user_id FROM user_profile WHERE city = 'Beijing'
);
```

#### EXISTS子查询

```sql
SELECT u.user_name 
FROM user_profile u
WHERE EXISTS (
    SELECT 1 FROM orders o 
    WHERE o.user_id = u.user_id 
    AND o.amount > 1000
);
```

### CTE（公用表表达式）

#### 非递归CTE

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

#### 递归CTE

```sql
WITH RECURSIVE user_hierarchy AS (
    -- 基础查询
    SELECT user_id, manager_id, 1 as level
    FROM employee
    WHERE manager_id IS NULL
    
    UNION ALL
    
    -- 递归查询
    SELECT e.user_id, e.manager_id, h.level + 1
    FROM employee e
    INNER JOIN user_hierarchy h ON e.manager_id = h.user_id
)
SELECT * FROM user_hierarchy ORDER BY level;
```

### 集合操作

```sql
-- UNION（去重）
SELECT user_id FROM orders_2023
UNION
SELECT user_id FROM orders_2024;

-- UNION ALL（保留重复）
SELECT user_id FROM orders_2023
UNION ALL
SELECT user_id FROM orders_2024;

-- INTERSECT（交集）
SELECT user_id FROM orders_2023
INTERSECT
SELECT user_id FROM orders_2024;

-- EXCEPT（差集）
SELECT user_id FROM orders_2023
EXCEPT
SELECT user_id FROM orders_2024;
```

## 视图

### 创建视图

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

### 查询视图

```sql
SELECT * FROM view_user_orders WHERE order_count > 10;
```

### 查看视图定义

```sql
SHOW CREATE VIEW view_user_orders;
```

### 删除视图

```sql
DROP VIEW IF EXISTS view_user_orders;
```

## 物化视图（Materialized View）

物化视图可以提升复杂查询性能：

### 创建物化视图

```sql
CREATE MATERIALIZED VIEW mv_daily_sales AS
SELECT 
    sale_date,
    SUM(amount) as daily_total,
    COUNT(*) as daily_count
FROM sales
GROUP BY sale_date;
```

### 查询自动重写

RorisDB会自动将符合条件的查询重写为使用物化视图：

```sql
-- 原始查询
SELECT sale_date, SUM(amount) 
FROM sales 
WHERE sale_date >= '2024-01-01'
GROUP BY sale_date;

-- 自动重写为使用物化视图
SELECT sale_date, daily_total 
FROM mv_daily_sales 
WHERE sale_date >= '2024-01-01';
```

## 数据类型

### 数值类型

| 类型 | 说明 | 范围 |
|------|------|------|
| TINYINT | 1字节整数 | -128 ~ 127 |
| SMALLINT | 2字节整数 | -32768 ~ 32767 |
| INT | 4字节整数 | -2147483648 ~ 2147483647 |
| BIGINT | 8字节整数 | -9223372036854775808 ~ 9223372036854775807 |
| FLOAT | 单精度浮点 | IEEE 754 |
| DOUBLE | 双精度浮点 | IEEE 754 |
| DECIMAL | 定点数 | 精确计算（规划中） |

### 字符串类型

| 类型 | 说明 | 最大长度 |
|------|------|---------|
| CHAR | 定长字符串 | 255 |
| VARCHAR | 变长字符串 | 65533 |
| STRING | 大字符串 | 2147483643 |

### 时间类型

| 类型 | 说明 | 格式 |
|------|------|------|
| DATE | 日期 | YYYY-MM-DD |
| DATETIME | 日期时间 | YYYY-MM-DD HH:MM:SS |

### 其他类型

| 类型 | 说明 |
|------|------|
| BOOLEAN | 布尔值（true/false） |
| ARRAY | 数组类型（规划中） |
| MAP | 映射类型（规划中） |
| JSON | JSON类型（规划中） |

## 常用函数

### 数学函数

```sql
SELECT 
    ABS(-5),           -- 5
    CEIL(3.2),         -- 4
    FLOOR(3.8),        -- 3
    ROUND(3.14159, 2), -- 3.14
    POW(2, 3),         -- 8
    SQRT(16),          -- 4
    LOG(100),          -- 自然对数
    LOG10(100),        -- 10为底的对数
    SIN(PI()/4),       -- 正弦
    COS(PI()/4),       -- 余弦
    TAN(PI()/4);       -- 正切
```

### 字符串函数

```sql
SELECT 
    LENGTH('Hello'),           -- 5
    CONCAT('Hello', 'World'),  -- HelloWorld
    CONCAT_WS('-', 'a', 'b'),  -- a-b
    UPPER('hello'),            -- HELLO
    LOWER('HELLO'),            -- hello
    TRIM('  hello  '),         -- hello
    LTRIM('  hello'),          -- hello
    RTRIM('hello  '),          -- hello
    SUBSTRING('hello', 2, 3),  -- ell
    REPLACE('hello', 'l', 'x'),-- hexxo
    LEFT('hello', 3),          -- hel
    RIGHT('hello', 3);         -- llo
```

### 日期函数

```sql
SELECT 
    NOW(),                      -- 当前时间
    CURDATE(),                  -- 当前日期
    CURTIME(),                  -- 当前时间
    DATE('2024-01-15 12:00:00'),-- 提取日期
    YEAR('2024-01-15'),         -- 2024
    MONTH('2024-01-15'),        -- 1
    DAY('2024-01-15'),          -- 15
    HOUR('12:30:45'),           -- 12
    DATE_ADD(NOW(), INTERVAL 7 DAY), -- 加7天
    DATE_SUB(NOW(), INTERVAL 7 DAY), -- 减7天
    DATEDIFF('2024-01-20', '2024-01-10'); -- 10天
```

### 聚合函数

```sql
SELECT 
    COUNT(*),               -- 计数
    COUNT(DISTINCT col),    -- 去重计数
    SUM(col),               -- 求和
    AVG(col),               -- 平均值
    MIN(col),               -- 最小值
    MAX(col),               -- 最大值
    GROUP_CONCAT(col);      -- 字符串拼接
```

### 条件函数

```sql
SELECT 
    CASE 
        WHEN age < 20 THEN 'Young'
        WHEN age BETWEEN 20 AND 40 THEN 'Adult'
        ELSE 'Senior'
    END as age_group,
    IF(age > 30, 'Old', 'Young'),
    COALESCE(col1, col2, 'default'),
    NULLIF(col1, col2);
```

## 用户和权限管理

### 用户管理

```sql
-- 创建用户
CREATE USER 'analyst'@'%' IDENTIFIED BY 'password123';

-- 修改密码
ALTER USER 'analyst'@'%' IDENTIFIED BY 'newpassword';

-- 设置密码
SET PASSWORD FOR 'analyst'@'%' = 'newpassword';

-- 删除用户
DROP USER 'analyst'@'%';
```

### 权限管理

```sql
-- 授权
GRANT SELECT, INSERT ON database_name.* TO 'analyst'@'%';

-- 授权所有权限
GRANT ALL PRIVILEGES ON database_name.* TO 'analyst'@'%';

-- 撤销权限
REVOKE INSERT ON database_name.* FROM 'analyst'@'%';

-- 查看权限
SHOW GRANTS FOR 'analyst'@'%';
```

### 权限类型

- SELECT：查询权限
- INSERT：插入权限
- UPDATE：更新权限
- DELETE：删除权限
- CREATE：创建表/库权限
- DROP：删除表/库权限
- ALTER：修改表权限
- ALL PRIVILEGES：所有权限

## 性能优化建议

### 1. 选择合适的表模型

- **明细查询**：使用DUPLICATE KEY
- **唯一键更新**：使用UNIQUE KEY
- **预聚合分析**：使用AGGREGATE KEY

### 2. 合理分区

- 按时间分区：便于时间范围查询和数据生命周期管理
- 按地区分区：便于地域分析
- 动态分区：自动管理历史数据

### 3. 使用索引

- ZoneMap索引：自动创建，适合范围查询
- BloomFilter索引：高基数列，等值查询优化

### 4. 查询优化

- **谓词下推**：尽早过滤数据
- **列裁剪**：只查询需要的列
- **使用EXPLAIN**：查看执行计划

```sql
EXPLAIN SELECT city, COUNT(*) FROM user_profile GROUP BY city;
```

### 5. 物化视图

为高频查询创建物化视图：

```sql
CREATE MATERIALIZED VIEW mv_summary AS
SELECT ... GROUP BY ...
```

### 6. 统计信息

定期收集统计信息以优化CBO：

```sql
ANALYZE TABLE table_name;
```

## 与Doris的差异

### 已支持功能

- ✅ DML完整支持（INSERT/UPDATE/DELETE）
- ✅ 事务（BEGIN/COMMIT/ROLLBACK）
- ✅ 分区表（Range/List/Hash）
- ✅ 物化视图框架
- ✅ Runtime Filter
- ✅ CBO优化器
- ✅ 用户权限管理

### 规划中功能

- 🚧 UDF/UDAF
- 🚧 联邦查询（Hive/Iceberg）
- 🚧 DECIMAL精确计算
- 🚧 行级安全
- 🚧 工作负载管理

## 常见问题

### Q: 如何选择DUPLICATE KEY还是UNIQUE KEY？

**A**: 
- DUPLICATE KEY：适合明细数据，保留所有历史记录
- UNIQUE KEY：适合维度表，需要唯一键约束和更新

### Q: 分区和分桶有什么区别？

**A**: 
- **分区（Partition）**：逻辑划分，便于管理和查询裁剪
- **分桶（Bucket）**：数据分布，便于并行执行和数据均衡

### Q: 如何查看查询执行计划？

**A**: 使用EXPLAIN命令：

```sql
EXPLAIN SELECT * FROM table_name WHERE condition;
```

### Q: 事务支持什么隔离级别？

**A**: 当前支持基本的BEGIN/COMMIT/ROLLBACK，完整隔离级别规划中。

### Q: 如何导入大量数据？

**A**: 
- INSERT批量插入：适合中小规模数据
- Stream Load（规划中）：适合大规模数据导入

## 下一步

- 查看[SQL参考手册](sql-reference.md)了解完整语法
- 阅读[性能报告](performance.md)了解性能特性
- 参考[配置说明](configuration.md)进行高级配置

## 附录：SQL快速参考

### DDL

```sql
CREATE DATABASE db_name;
CREATE TABLE table_name (...);
ALTER TABLE table_name ...;
DROP TABLE table_name;
TRUNCATE TABLE table_name;
```

### DML

```sql
INSERT INTO table_name VALUES ...;
INSERT INTO table_name SELECT ...;
INSERT INTO table_name ON DUPLICATE KEY UPDATE ...;
UPDATE table_name SET ... WHERE ...;
DELETE FROM table_name WHERE ...;
```

### DQL

```sql
SELECT ... FROM ... WHERE ... GROUP BY ... HAVING ... ORDER BY ... LIMIT ...;
```

### 事务

```sql
BEGIN;
COMMIT;
ROLLBACK;
SAVEPOINT sp_name;
ROLLBACK TO sp_name;
```

### 权限

```sql
CREATE USER ...;
GRANT ... ON ... TO ...;
REVOKE ... ON ... FROM ...;
SHOW GRANTS FOR ...;
```

---

**RorisDB** - Apache Doris兼容的Rust OLAP数据库