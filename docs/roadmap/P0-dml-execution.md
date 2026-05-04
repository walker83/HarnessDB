# P0: DML 执行层实现

**优先级**: P0 (核心缺失)
**模块**: be-execution
**状态**: ❌ 未开始

## 背景

UPDATE 和 DELETE 的 AST 解析、Logical Plan、Physical Plan 均已实现，但 BE 执行层只是打日志，没有实际的数据修改逻辑。这导致这两个关键 DML 操作无法真正生效。

## 现状

- ✅ AST: `fe-sql-parser/src/ast.rs` 中 `UpdateStmt`、`DeleteStmt` 已定义
- ✅ Planner: `fe-sql-planner/src/planner.rs` 中 `plan_update`、`plan_delete` 已实现
- ✅ ExecNode: `be-execution/src/exec_node.rs` 中 `UpdateExecNode`、`DeleteExecNode` 已定义
- ❌ 执行层: 仅 `log` 操作，无实际数据读写

## 任务清单

### 1. UPDATE 执行实现
- [ ] 在 BE 端实现 UpdateOperator，读取目标 Tablet 数据
- [ ] 根据 WHERE 条件过滤行
- [ ] 对匹配行执行 SET 子句的列值更新
- [ ] 将修改后的数据写入新的 Rowset
- [ ] 处理 Primary Key 场景下的 upsert 逻辑
- [ ] 返回受影响行数

### 2. DELETE 执行实现
- [ ] 在 BE 端实现 DeleteOperator，读取目标 Tablet 数据
- [ ] 根据 WHERE 条件过滤要删除的行
- [ ] 生成删除标记（Delete Predicate 或实际删除数据）
- [ ] 将结果写入新的 Rowset
- [ ] 返回受影响行数

### 3. 事务保障
- [ ] 确保 UPDATE/DELETE 操作在事务内完成
- [ ] 支持回滚机制
- [ ] 与 Compaction 协调（避免读写冲突）

### 4. 集成测试
- [ ] INSERT → UPDATE → 验证数据正确性
- [ ] INSERT → DELETE → 验证数据已删除
- [ ] 带条件的大批量 UPDATE/DELETE 性能测试
- [ ] 并发 UPDATE 场景测试

## 涉及文件

- `crates/be-execution/src/exec_node.rs` - 修改 UpdateExecNode/DeleteExecNode 实现
- `crates/be-execution/src/operator.rs` - 新增 Update/Delete operator
- `crates/be-storage/src/tablet.rs` - Tablet 数据读写接口
- `crates/be-storage/src/rowset.rs` - Rowset 写入
