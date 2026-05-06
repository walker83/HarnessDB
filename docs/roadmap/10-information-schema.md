# Information Schema 增强

## 概述
当前 INFORMATION_SCHEMA 实现不完整，部分查询无法执行。

## 现状分析
测试结果:
- `mysql_compat/05_mysql_information_functions.sql`: 34 errors
- `catalog_view/01_view_operations.sql`: 112 errors (部分 INFORMATION_SCHEMA)
- `admin/01_session_variables.sql`: 4 errors (部分 INFORMATION_SCHEMA)

主要缺失:
- INFORMATION_SCHEMA 完整表列表
- COLUMNS 表完整信息
- TABLE_CONSTRAINTS 信息
- REFERENTIAL_CONSTRAINTS 信息

## 子任务

### Task 1: INFORMATION_SCHEMA 基础表
- 实现 SCHEMATA 表
- 实现 TABLES 表
- 实现 COLUMNS 表
- 实现 STATISTICS 表
- 验证: `mysql_compat/05_mysql_information_functions.sql` 基础表查询通过

### Task 2: INFORMATION_SCHEMA 约束表
- 实现 TABLE_CONSTRAINTS 表
- 实现 REFERENTIAL_CONSTRAINTS 表
- 实现 KEY_COLUMN_USAGE 表
- 验证: `catalog_view/01_view_operations.sql` 约束查询部分通过

### Task 3: INFORMATION_SCHEMA 权限视图
- 实现 USER_PRIVILEGES 表
-实现 SCHEMA_PRIVILEGES 表
- 实现 TABLE_PRIVILEGES 表
- 实现 COLUMN_PRIVILEGES 表
- 验证: `security/05_security_features.sql` 权限视图部分通过

### Task 4: INFORMATION_SCHEMA 高级表
- 实现 TRIGGERS 表
- 实现 ROUTINES 表
- 实现 VIEWS 表
- 实现 PARTITIONS 表
- 验证: `catalog_view/01_view_operations.sql` 高级表部分通过

## 验收标准
- [ ] INFORMATION_SCHEMA 基础表返回正确数据
- [ ] SHOW COLUMNS/TABLES 等效查询正常工作
- [ ] INFORMATION_SCHEMA 测试通过率 > 80%

## 影响范围
- `fe-catalog`: INFORMATION_SCHEMA 视图实现
- `fe-sql-planner`: Information Schema 查询计划
- `mysql-protocol`: SHOW 命令实现
