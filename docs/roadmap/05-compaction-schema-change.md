# Compaction 和 Schema Change

## 概述
Compaction 和 Schema Change 是存储引擎的核心功能，当前不支持。

## 现状分析
测试结果:
- `storage_engine/05_compaction_schema_change.sql`: 298 errors
- `ddl/03_table_alter_operations.sql`: 146 errors (部分 Schema Change)
- `admin/03_compaction_backup.sql`: 120 errors (部分 Compaction)

主要错误:
```
PARSE ERROR: Expected: ADD, RENAME, PARTITION, SWAP, DROP, or SET TBLPROPERTIES after ALTER TABLE, found: COMPACT
PARSE ERROR: Expected: end of statement, found: SWAP
```

## 子任务

### Task 1: ALTER TABLE COMPACT
- 支持 ALTER TABLE xxx COMPACT
- 支持 ALTER TABLE xxx COMPACT PARTITION xxx
- 实现 Compaction 执行逻辑
- 验证: `storage_engine/05_compaction_schema_change.sql` COMPACT 部分通过

### Task 2: Schema Change 基本操作
- 支持 ALTER TABLE ADD COLUMN
- 支持 ALTER TABLE DROP COLUMN
- 支持 ALTER TABLE MODIFY COLUMN
- 支持 ALTER TABLE RENAME COLUMN
- 验证: `ddl/03_table_alter_operations.sql` 基本操作通过

### Task 3: Schema Change 高级操作
- 支持 ALTER TABLE SWAP (表交换)
- 支持 ALTER TABLE REPLACE (表替换)
- 实现 Schema Change 调度
- 验证: `storage_engine/05_compaction_schema_change.sql` SWAP/REPLACE 部分通过

### Task 4: 分区操作增强
- 支持 ALTER TABLE ADD PARTITION
- 支持 ALTER TABLE DROP PARTITION
- 支持 ALTER TABLE MODIFY PARTITION
- 验证: `ddl/01_partition_operations.sql` ALTER PARTITION 部分通过

## 验收标准
- [ ] ALTER TABLE COMPACT 正常工作
- [ ] ADD/DROP/MODIFY COLUMN 正常工作
- [ ] ALTER TABLE SWAP/REPLACE 正常工作
- [ ] Compaction 和 Schema Change 测试通过率 > 80%

## 影响范围
- `fe-sql-parser`: ALTER TABLE 语法扩展
- `fe-sql-planner`: Compaction/Schema Change 计划
- `be-storage`: Compaction 执行和数据合并
- `fe-scheduler`: Schema Change 任务调度
