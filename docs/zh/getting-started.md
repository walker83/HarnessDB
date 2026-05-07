# RorisDB 快速开始

本指南将帮助你快速上手 RorisDB，完成从安装到执行第一个查询的全过程。

## 前置条件

- 已完成 RorisDB 的编译安装（参见[安装部署指南](installation.md)）
- 已启动 FE 和 BE 服务
- 可选：安装 MySQL 客户端（用于连接数据库）

## 连接数据库

### 方式一：使用 MySQL 客户端

RorisDB 支持 MySQL 协议，可以直接使用 MySQL 客户端连接：

```bash
mysql -h 127.0.0.1 -P 9030 -uroot
```

连接成功后，你会看到类似如下的提示：

```
Welcome to the MySQL monitor.  Commands end with ; or \g.
Your MySQL connection id is 1
Server version: RorisDB 0.2.0

Copyright (c) 2000, 2024, Oracle and/or its affiliates.

Type 'help;' or '\h' for help. Type '\c' to clear the current input statement.

mysql>
```

### 方式二：使用 roris-cli

RorisDB 自带命令行客户端：

```bash
./target/release/roris-cli
```

## 基础操作

### 1. 创建数据库

```sql
CREATE DATABASE IF NOT EXISTS test;
USE test;
```

### 2. 创建表

RorisDB 支持多种表模型，当前主要支持 `DUPLICATE KEY` 模型：

```sql
CREATE TABLE user (
    id BIGINT PRIMARY KEY,
    name VARCHAR(64),
    age INT,
    city VARCHAR(64)
) DUPLICATE KEY;
```

**表模型说明**：
- `DUPLICATE KEY`：允许重复数据，适合明细数据存储
- 其他模型（AGGREGATE、UNIQUE）正在规划中

### 3. 插入数据

```sql
-- 插入单条数据
INSERT INTO user VALUES (1, 'Alice', 30, 'Beijing');

-- 插入多条数据
INSERT INTO user VALUES 
    (2, 'Bob', 25, 'Shanghai'),
    (3, 'Charlie', 35, 'Beijing'),
    (4, 'David', 28, 'Shenzhen'),
    (5, 'Eve', 32, 'Shanghai');
```

### 4. 查询数据

```sql
-- 查询所有数据
SELECT * FROM user;

-- 条件查询
SELECT * FROM user WHERE age > 30;

-- 聚合查询
SELECT city, COUNT(*), AVG(age) 
FROM user 
GROUP BY city;

-- 排序
SELECT * FROM user ORDER BY age DESC;

-- 限制结果数量
SELECT * FROM user LIMIT 3;
```

### 5. 更新和删除

RorisDB 现已完整支持 UPDATE 和 DELETE 操作：

```sql
-- 更新数据
UPDATE user SET age = 31 WHERE name = 'Alice';

-- 删除数据
DELETE FROM user WHERE age < 25;

-- DELETE with ORDER BY and LIMIT
DELETE FROM user ORDER BY age DESC LIMIT 2;
```

### 6. 事务支持

RorisDB 支持基本的事务操作：

```sql
-- 开始事务
BEGIN;

-- 或使用 START TRANSACTION
START TRANSACTION;

-- 执行多个操作
INSERT INTO user VALUES (6, 'Frank', 40, 'Guangzhou');
UPDATE user SET city = 'Hangzhou' WHERE id = 2;

-- 提交事务
COMMIT;

-- 回滚事务
ROLLBACK;

-- 使用保存点
BEGIN;
INSERT INTO user VALUES (7, 'Grace', 45, 'Nanjing');
SAVEPOINT sp1;
UPDATE user SET age = 46 WHERE id = 7;
ROLLBACK TO sp1;  -- 回滚到保存点
COMMIT;
```

### 7. INSERT ON DUPLICATE KEY

支持 MySQL 兼容的 Upsert 语法：

```sql
CREATE TABLE unique_user (
    id BIGINT PRIMARY KEY,
    name VARCHAR(64),
    age INT
) UNIQUE KEY;

-- 如果 id 存在则更新，不存在则插入
INSERT INTO unique_user VALUES (1, 'Alice', 30)
ON DUPLICATE KEY UPDATE name = 'Alice', age = 31;

-- INSERT SET 语法
INSERT INTO unique_user SET id = 2, name = 'Bob', age = 25;
```

## 数据类型

RorisDB 支持以下数据类型：

| 类型 | 说明 | 示例 |
|------|------|------|
| `BIGINT` | 64 位整数 | `1234567890` |
| `INT` | 32 位整数 | `12345` |
| `FLOAT` | 单精度浮点数 | `3.14` |
| `DOUBLE` | 双精度浮点数 | `3.1415926` |
| `VARCHAR(n)` | 变长字符串 | `'Hello'` |
| `DATE` | 日期 | `'2024-01-01'` |
| `DATETIME` | 日期时间 | `'2024-01-01 12:00:00'` |
| `BOOLEAN` | 布尔值 | `true` / `false` |

### 示例：创建包含多种类型的表

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

## 常用 SQL 操作

### 聚合函数

RorisDB 支持丰富的聚合函数：

```sql
-- 计数
SELECT COUNT(*) FROM user;

-- 求和
SELECT SUM(age) FROM user;

-- 平均值
SELECT AVG(age) FROM user;

-- 最大值和最小值
SELECT MAX(age), MIN(age) FROM user;

-- 去重计数
SELECT COUNT(DISTINCT city) FROM user;

-- 拼接字符串
SELECT GROUP_CONCAT(name) FROM user WHERE age > 25;
```

### 窗口函数

支持常用的窗口函数：

```sql
-- 行号
SELECT name, age, ROW_NUMBER() OVER (ORDER BY age) as rn
FROM user;

-- 排名
SELECT name, age, RANK() OVER (ORDER BY age DESC) as rank
FROM user;

-- 密集排名
SELECT name, age, DENSE_RANK() OVER (ORDER BY age DESC) as dense_rank
FROM user;

-- 前后行访问
SELECT name, age, 
       LAG(age, 1) OVER (ORDER BY age) as prev_age,
       LEAD(age, 1) OVER (ORDER BY age) as next_age
FROM user;
```

### 子查询

支持 IN、EXISTS 等子查询：

```sql
-- IN 子查询
SELECT * FROM user 
WHERE city IN (SELECT city FROM user WHERE age > 30);

-- EXISTS 子查询
SELECT * FROM user u
WHERE EXISTS (SELECT 1 FROM orders o WHERE o.user_id = u.id);
```

### 集合操作

支持 UNION、INTERSECT、EXCEPT：

```sql
-- UNION：合并结果（去重）
SELECT name FROM user WHERE age < 30
UNION
SELECT name FROM user WHERE age > 30;

-- UNION ALL：合并结果（保留重复）
SELECT name FROM user WHERE age < 30
UNION ALL
SELECT name FROM user WHERE age > 25;

-- INTERSECT：交集
SELECT city FROM user
INTERSECT
SELECT city FROM orders;

-- EXCEPT：差集
SELECT city FROM user
EXCEPT
SELECT city FROM orders;
```

### CTE（公用表表达式）

支持 WITH 子句：

```sql
WITH young_users AS (
    SELECT * FROM user WHERE age < 30
)
SELECT * FROM young_users WHERE city = 'Shanghai';
```

### 视图

支持创建视图：

```sql
CREATE VIEW view_user_beijing AS
SELECT * FROM user WHERE city = 'Beijing';

-- 查询视图
SELECT * FROM view_user_beijing;

-- 删除视图
DROP VIEW view_user_beijing;
```

## 数据导入

### CSV 导入（规划中）

```sql
-- 未来将支持
LOAD DATA INFILE '/path/to/data.csv'
INTO TABLE user
FIELDS TERMINATED BY ','
LINES TERMINATED BY '\n';
```

### Stream Load（规划中）

通过 HTTP 接口导入数据：

```bash
curl -X PUT \
  -H "format: csv" \
  -T data.csv \
  http://127.0.0.1:8030/api/test/user/_stream_load
```

## 查询分析

### 使用 EXPLAIN 查看执行计划

```sql
EXPLAIN SELECT city, COUNT(*) FROM user GROUP BY city;
```

输出示例：
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

### 性能分析

```sql
-- 查看查询执行时间（在 mysql 客户端中）
SELECT /*+ SET_VAR(profile=true) */ city, COUNT(*) FROM user GROUP BY city;
```

## 实用示例

### 示例 1：用户年龄分布统计

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

### 示例 2：每个城市的平均年龄和人数

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

### 示例 3：使用窗口函数排名

```sql
SELECT 
    name,
    age,
    city,
    RANK() OVER (PARTITION BY city ORDER BY age DESC) as city_rank
FROM user
WHERE city IN ('Beijing', 'Shanghai');
```

## 管理操作

### 查看表结构

```sql
-- 查看建表语句
SHOW CREATE TABLE user;

-- 查看表信息（规划中）
DESCRIBE user;
```

### 清空表

```sql
TRUNCATE TABLE user;
```

### 删除表

```sql
DROP TABLE IF EXISTS user;
```

### 删除数据库

```sql
DROP DATABASE IF EXISTS test;
```

## 退出客户端

```sql
-- MySQL 客户端
EXIT;

-- 或
QUIT;
```

## 下一步

- 深入学习 [SQL 参考手册](sql-reference.md)
- 了解[配置说明](configuration.md)进行高级配置
- 查看[功能特性](features.md)了解完整功能列表
- 阅读[开发者指南](developer-guide.md)参与项目开发

## 常见问题

### 连接失败

**问题**：无法连接到数据库

**解决**：
1. 检查 FE 是否正常运行
2. 检查 MySQL 端口是否正确（默认 9030）
3. 查看 FE 日志：`tail -f ./fe_data/logs/roris-fe.log`

### 表创建失败

**问题**：创建表时报错

**解决**：
1. 确保使用了正确的表模型（当前主要支持 `DUPLICATE KEY`）
2. 检查 SQL 语法是否正确
3. 确保已选择数据库（`USE database_name;`）

### 查询结果为空

**问题**：查询没有返回数据

**解决**：
1. 确认数据已成功插入
2. 检查 WHERE 条件是否正确
3. 使用 `SELECT * FROM table_name;` 查看所有数据
