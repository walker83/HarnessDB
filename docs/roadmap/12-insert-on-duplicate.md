# INSERT ON DUPLICATE KEY 语法

## 概述
当前 `INSERT ... ON DUPLICATE KEY UPDATE` 语法无法解析，这是最常见的解析错误之一。

## 现状分析
测试结果:
- `admin/01_session_variables.sql`: 4 errors
- `admin/02_system_administration.sql`: 80 errors (大量 DUPLICATE 相关)
- `admin/03_compaction_backup.sql`: 120 errors
- `admin/04_resource_management.sql`: 100 errors
- `basic/04_dml_insert_update_delete.sql`: 32 errors
- 几乎所有涉及 INSERT 的测试文件都有 DUPLICATE 语法错误

主要错误:
```
PARSE ERROR: syntax error at position 0: sql parser error: Expected: end of statement, found: DUPLICATE
```

## 子任务

### Task 1: ON DUPLICATE KEY 语法解析
- 修改 INSERT 语句语法支持 ON DUPLICATE KEY UPDATE
- 支持 ON DUPLICATE KEY UPDATE col=value [, col=value]...
- 支持 VALUES() 函数引用原始值
- 支持 UPDATE/INSERT 混合语法
- 验证: `basic/04_dml_insert_update_delete.sql` INSERT 部分通过

### Task 2: ON DUPLICATE KEY 执行
- 实现 ON DUPLICATE KEY 执行逻辑
- 实现行级别的 UPDATE 或 INSERT 选择
- 处理多行 INSERT + ON DUPLICATE KEY
- 验证: `dml/01_insert_operations.sql` ON DUPLICATE KEY 部分通过

### Task 3: 扩展到 UPSERT
- 为未来 UPSERT (INSERT IGNORE, REPLACE) 预留语法支持
- 验证: `dml/03_upsert_merge_operations.sql` 相关部分通过

## 验收标准
- [ ] `INSERT ... ON DUPLICATE KEY UPDATE` 可以正常解析
- [ ] 执行时正确处理重复键更新
- [ ] 支持多行 VALUES 的 ON DUPLICATE KEY
- [ ] 相关测试通过率 > 80%

## 影响范围
- `fe-sql-parser`: INSERT 语句语法扩展
- `fe-sql-planner`: ON DUPLICATE KEY 计划生成
- `be-execution`: ON DUPLICATE KEY 执行算子
