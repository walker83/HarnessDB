# DML 执行层实现

## 概述
当前 INSERT/UPDATE/DELETE 语句无法真正执行，需要在执行层实现完整的 DML 功能。

## 现状分析
测试结果:
- `basic/04_dml_insert_update_delete.sql`: 32 errors
- `dml/01_insert_operations.sql`: 96 errors
- `dml/02_update_delete_operations.sql`: 90 errors
- `dml/03_upsert_merge_operations.sql`: 82 errors
- `dml/04_transaction_isolation.sql`: 262 errors

主要错误:
```
ERROR: INSERT execution not yet implemented - table: xxx
ERROR: UPDATE execution not yet implemented
ERROR: DELETE execution not yet implemented
```

## 子任务

### Task 1: INSERT 执行实现
- 实现单行 INSERT
- 实现批量 INSERT (VALUES multiple rows)
- 实现 INSERT ... SELECT
- 处理 NULL 值和默认值
- 验证: `basic/04_dml_insert_update_delete.sql` 通过率 > 80%

### Task 2: UPDATE 执行实现
- 实现单表 UPDATE
- 实现 WHERE 条件过滤
- 实现 ORDER BY + LIMIT
- 验证: `dml/02_update_delete_operations.sql` UPDATE 部分通过

### Task 3: DELETE 执行实现
- 实现单表 DELETE
- 实现 WHERE 条件过滤
- 实现 ORDER BY + LIMIT
- 验证: `dml/02_update_delete_operations.sql` DELETE 部分通过

### Task 4: 事务支持
- 实现 BEGIN/COMMIT/ROLLBACK
- 实现自动提交模式
- 支持 Savepoint
- 验证: `dml/04_transaction_isolation.sql` 通过率 > 80%

## 验收标准
- [ ] INSERT 语句可以成功插入数据到表
- [ ] UPDATE 语句可以更新已有数据
- [ ] DELETE 语句可以删除数据
- [ ] 事务 BEGIN/COMMIT/ROLLBACK 正常工作
- [ ] 相关测试用例通过率 > 80%

## 影响范围
- `be-execution`: 实现 InsertExecNode, UpdateExecNode, DeleteExecNode
- `fe-planner`: 添加对应 PlanNode 的执行逻辑
- `fe-scheduler`: 支持 DML 语句的调度
