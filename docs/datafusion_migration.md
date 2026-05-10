# DataFusion 查询引擎迁移文档

## 背景

RorisDB 之前的查询路径存在两套系统：
1. **DataFusion 路径** - SELECT 查询通过 `SessionContext.sql()` 执行
2. **旧查询引擎路径** - 包含 Planner、Scheduler、Execution 等组件，但实际未被使用

迁移目标是完全移除旧查询引擎代码，简化架构，降低维护成本。

## 删除的组件

删除了约 **18,500 行** 代码：

| Crate | 行数 | 说明 |
|-------|------|------|
| `fe-sql-planner` | ~5,000 | Planner, PlanNode, Optimizer, CBO, MaterializedView |
| `fe-scheduler` | ~2,400 | ClusterManager, Coordinator, Scheduler, Fragment |
| `be-execution` | ~5,000 | ExecutionContext, Pipeline, Planner, PredicateParser |
| `fe-expression` | ~3,700 | ExprEvaluator, FunctionRegistry, Accumulator |
| `tests/integration/tests/optimizer_test.rs` | ~350 | 测试旧 planner 的优化能力 |
| `tools/roris-cli` | ~100 | CLI 工具使用旧 planner |

## 修改详情

### 1. fe_main.rs 修改

**删除的导入：**
```rust
// 已删除
use fe_scheduler::ClusterManager;
use be_execution::exec_node::TransactionContext;
```

**替换 TransactionContext：**
创建了最小化的 `SimpleTransactionState` 结构替代 `be-execution` 的 `TransactionContext`：
```rust
struct SimpleTransactionState {
    in_transaction: bool,
    isolation_level: String,
    savepoints: Vec<String>,
}
```

**删除未使用代码：**
- ClusterManager 初始化（创建后立即丢弃）

### 2. Cargo.toml 修改

**roris-server/Cargo.toml：**
- 删除：`fe-sql-planner`, `fe-scheduler`, `be-execution`, `fe-expression`

**workspace Cargo.toml：**
- 删除成员：`crates/fe-sql-planner`, `crates/fe-scheduler`, `crates/be-execution`, `crates/fe-expression`, `tools/roris-cli`
- 删除依赖：相应 workspace.dependencies

### 3. Integration Tests 修改

**删除文件：**
- `tests/integration/tests/optimizer_test.rs` - 测试旧 planner 优化功能

**更新文件：**
- `common.rs` - 删除 `plan_sql`, `collect_node_types`, `format_node_type` 函数
- `lib.rs` - 删除 `pub use fe_sql_planner;`
- `Cargo.toml` - 删除旧依赖
- `sql_test.rs`, `sql_query_test.rs`, `ddl_lifecycle_test.rs`, `data_import_test.rs` - 重写为使用 parser/block 操作测试，移除 planner 相关测试

### 4. TPC-H Benchmarks 修改

重写为纯 DataFusion 执行路径：
```rust
pub fn run_sql(&self, name: &'static str, sql: &str) -> QueryResult {
    let ctx = self.datafusion_ctx();
    match ctx.sql(sql).await {
        Ok(df) => {
            match df.collect().await {
                Ok(batches) => { ... }
                Err(e) => { ... }
            }
        }
        Err(e) => { ... }
    }
}
```

### 5. mysql_server 工具修改

更新为使用 DataFusion `run_query` 方法，返回查询统计信息。

## 保留的组件

- **`fe-sql-parser`** - DDL 和 SQL 解析仍在使用
- **`fe-datafusion`** - DataFusion 集成层
- **`fe-catalog`** - Catalog 管理
- **`be-storage`** - 存储引擎

## 架构简化效果

迁移后的查询路径：
```
MySQL Protocol → SessionContext.sql() → DataFusion Plan → Execution → Results
```

之前的复杂路径（已删除）：
```
MySQL Protocol → Parser → Planner → Optimizer → Scheduler → Fragment → Execution → Results
```

## 测试策略

保留的测试分为两类：
1. **Parser 测试** - 验证 SQL 解析能力
2. **Block 操作测试** - 验证数据操作逻辑

移除的测试：
- Planner 优化测试（predicate pushdown, column pruning 等）
- 执行引擎测试

## 未来工作

1. 为 DataFusion 路径添加更完整的测试覆盖
2. 考虑扩展 DataFusion 的自定义优化规则
3. 完善 DDL 与 DataFusion catalog 的同步