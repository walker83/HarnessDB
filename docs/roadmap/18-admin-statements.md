# 管理语句增强

## 概述
当前许多管理语句 (SHOW VARIABLE 等) 支持不完整。

## 现状分析
测试结果:
- `admin/02_system_administration.sql`: 80 errors (大量 SHOW VARIABLE)
- `admin/05_monitoring_diagnostic.sql`: 52 errors
- `monitoring/01_statistics_collection.sql`: 38 errors
- `monitoring/02_audit_monitoring.sql`: 110 errors

主要缺失:
- SHOW TABLETS 语句
- SHOW DATA 语句
- SHOW COMPACTION 语句
- SHOW PROC 语句
- 统计信息收集

## 子任务

### Task 1: SHOW 变量语句
- 支持 SHOW VARIABLES 完整语法
- 支持 SHOW GLOBAL VARIABLES
- 支持 LIKE 过滤
- 支持 WHERE 过滤
- 验证: `admin/02_system_administration.sql` SHOW VARIABLES 部分通过

### Task 2: SHOW 集群语句
- 实现 SHOW TABLETS 语句
- 实现 SHOW BACKENDS 语句
- 实现 SHOW FRONTENDS 语句
- 实现 SHOW BROKER 语句
- 验证: `admin/05_monitoring_diagnostic.sql` 集群语句部分通过

### Task 3: SHOW 表/分区语句
- 实现 SHOW TABLETS FROM table 语句
- 实现 SHOW DATA 语句
- 实现 SHOW PARTITIONS FROM table 语句
- 验证: `admin/02_system_administration.sql` 表/分区语句部分通过

### Task 4: 统计信息收集
- 实现 ANALYZE TABLE 语句
- 实现 SHOW ANALYZE 语句
- 实现统计信息存储
- 支持查询计划统计
- 验证: `monitoring/01_statistics_collection.sql` 通过率 > 80%

### Task 5: 审计日志
- 支持 AUDIT 语句
- 实现审计日志记录
- 实现 SHOW AUDIT 语句
- 验证: `monitoring/02_audit_monitoring.sql` 部分通过

## 验收标准
- [ ] SHOW VARIABLES 正常工作
- [ ] SHOW TABLETS/BACKENDS/FRONTENDS 返回正确信息
- [ ] ANALYZE TABLE 正常工作
- [ ] 管理语句测试通过率 > 80%

## 影响范围
- `fe-sql-parser`: 管理语句语法解析
- `fe-catalog`: 统计信息存储
- `fe-scheduler`: 集群状态查询
- `be-storage`: 统计信息收集
