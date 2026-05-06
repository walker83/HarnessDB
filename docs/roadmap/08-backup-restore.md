# 备份恢复 (BACKUP/RESTORE)

## 概述
当前 BACKUP DATABASE 和 RESTORE DATABASE 语句无法解析。

## 现状分析
测试结果:
- `admin/03_compaction_backup.sql`: 120 errors (大量 BACKUP/RESTORE 相关)
- `recovery/01_ddl_import_recovery.sql`: 29 errors

主要错误:
```
PARSE ERROR: syntax error at position 0: sql parser error: Expected: an SQL statement, found: BACKUP
Statement::BackupDatabase not found in parser
```

## 子任务

### Task 1: BACKUP/RESTORE 语法解析
- 添加 BackupDatabaseStmt 到 Statement 枚举
- 添加 RestoreDatabaseStmt 到 Statement 枚举
- 实现 BACKUP DATABASE 语法解析
- 实现 RESTORE DATABASE 语法解析
- 验证: `admin/03_compaction_backup.sql` 解析部分通过

### Task 2: BACKUP 执行
- 实现数据库备份逻辑
- 支持指定仓库 (REPOSITORY)
- 支持备份元数据导出
- 验证: `admin/03_compaction_backup.sql` BACKUP 执行部分通过

### Task 3: RESTORE 执行
- 实现数据库恢复逻辑
- 支持从仓库恢复
- 支持覆盖/跳过策略
- 验证: `admin/03_compaction_backup.sql` RESTORE 执行部分通过

### Task 4: REPOSITORY 管理
- 实现 CREATE REPOSITORY 语法
- 实现 DROP REPOSITORY 语法
- 支持 SHOW REPOSITORIES
- 验证: `admin/03_compaction_backup.sql` REPOSITORY 部分通过

## 验收标准
- [ ] BACKUP DATABASE 语句可以正常解析
- [ ] RESTORE DATABASE 语句可以正常解析
- [ ] CREATE/DROP REPOSITORY 正常工作
- [ ] 备份恢复测试通过率 > 80%

## 影响范围
- `fe-sql-parser`: BACKUP/RESTORE/REPOSITORY 语法解析
- `fe-sql-planner`: 备份恢复计划生成
- `fe-catalog`: 仓库元数据存储
- `be-storage`: 数据导出/导入
