# 分区表支持

## 概述
当前分区表 DDL 和操作尚未完整实现，需要增强分区支持。

## 现状分析
测试结果:
- `ddl/01_partition_operations.sql`: 136 errors
- `performance/04_partition_performance.sql`: 81 errors
- `basic/02_table_ddl_positive.sql`: 36 errors (部分分区相关)

主要缺失:
- CREATE TABLE with PARTITION BY
- ALTER TABLE ADD/DROP/RENAME PARTITION
- PARTITION PRUNING in query optimization
- SHOW PARTITIONS 语句

## 子任务

### Task 1: 分区表创建
- 支持 RANGE PARTITION
- 支持 LIST PARTITION
- 支持 HASH PARTITION
- 存储分区元信息
- 验证: `ddl/01_partition_operations.sql` CREATE 部分通过

### Task 2: 分区表 DDL
- 实现 ALTER TABLE ADD PARTITION
- 实现 ALTER TABLE DROP PARTITION
- 实现 ALTER TABLE RENAME PARTITION
- 实现 TRUNCATE PARTITION
- 验证: `ddl/01_partition_operations.sql` ALTER 部分通过

### Task 3: 分区裁剪优化
- 实现查询时分区裁剪 (Partition Pruning)
- 支持 WHERE 条件分区过滤
- 验证: `performance/04_partition_performance.sql` 裁剪相关通过

### Task 4: 分区信息查询
- 实现 SHOW PARTITIONS 语句
- 实现 SHOW CREATE TABLE 显示分区信息
- 实现 INFORMATION_SCHEMA.PARTITIONS
- 验证: `catalog_view` 相关分区查询通过

## 验收标准
- [ ] 可以创建 RANGE/LIST/HASH 分区表
- [ ] 可以 ALTER TABLE 添加/删除分区
- [ ] 查询可以正确裁剪不需要的分区
- [ ] 分区相关 DDL 测试通过率 > 80%

## 影响范围
- `fe-sql-parser`: 分区语法解析
- `fe-sql-planner`: 分区计划生成
- `fe-catalog`: 分区元数据管理
- `be-storage`: 分区数据存储
