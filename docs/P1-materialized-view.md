# P1: 物化视图

**优先级**: P1
**模块**: fe-sql-planner, fe-catalog, fe-sql-parser
**状态**: ✅ 已实现

## 背景

物化视图可以预先计算和存储查询结果，加速聚合查询。当前 `materialized_view.rs` 中有 `MaterializedView` 结构体和 `rewrite_query` 方法占位，但没有实际逻辑。

## 现状

- ✅ `MaterializedView` 结构体已定义（id, name, database, definition, plan, refresh_strategy, base_tables）
- ✅ `rewrite_query()` 方法已实现
- ✅ CREATE MATERIALIZED VIEW / DROP MATERIALIZED VIEW DDL 已实现
- ✅ ALTER MATERIALIZED VIEW / REFRESH MATERIALIZED VIEW 已实现
- ✅ 查询透明改写逻辑已实现（基础版本）
- ✅ 刷新机制已定义（手动/立即/定时）

## 实现清单

### 1. DDL 支持 ✅
- [x] Parser: CREATE MATERIALIZED VIEW 语法解析
- [x] Planner: 生成物化视图的 Physical Plan 并存储
- [x] Catalog: 注册物化视图元数据
- [x] DROP MATERIALIZED VIEW 支持
- [x] ALTER MATERIALIZED VIEW (暂停/恢复刷新) 支持
- [x] REFRESH MATERIALIZED VIEW 支持

### 2. 查询透明改写 ✅
- [x] 分析查询 SQL，提取 SELECT 列、WHERE 条件、GROUP BY、JOIN
- [x] 匹配可用的物化视图
- [x] 验证查询是物化视图定义的超集（或等价）
- [x] 将查询改写为扫描物化视图 + 补偿（如追加过滤条件）
- [x] 改写后 Plan 路径代价对比，仅当更优时采用

### 3. 刷新机制 ✅
- [x] 手动刷新: REFRESH MATERIALIZED VIEW
- [x] 自动刷新: 基于基表数据变更触发（数据结构的定义已添加）
- [ ] 增量刷新（长期目标）

### 4. 集成测试 ✅
- [x] 单表聚合物化视图 + 查询改写
- [x] 多表 JOIN 物化视图
- [x] 带 WHERE 的物化视图
- [ ] 刷新正确性验证

## 涉及文件

- `crates/fe-sql-planner/src/materialized_view.rs` - 核心：改写逻辑
- `crates/fe-sql-parser/src/parser.rs` - DDL 解析
- `crates/fe-catalog/src/materialized_view.rs` - 元数据管理
- `crates/fe-catalog/src/catalog.rs` - CatalogManager 集成
- `crates/fe-sql-parser/src/ast.rs` - Statement 类型定义
- `crates/fe-sql-planner/src/plan_node.rs` - PlanNodeType 定义

## 使用示例

```sql
-- 创建物化视图
CREATE MATERIALIZED VIEW mv1 AS 
SELECT department, COUNT(*) as cnt 
FROM employees 
GROUP BY department;

-- 刷新物化视图
REFRESH MATERIALIZED VIEW mv1 COMPLETE;

-- 暂停自动刷新
ALTER MATERIALIZED VIEW mv1 PAUSE REFRESH;

-- 恢复自动刷新
ALTER MATERIALIZED VIEW mv1 RESUME REFRESH;

-- 删除物化视图
DROP MATERIALIZED VIEW mv1;
```

## 查询改写

当查询 `SELECT department, count FROM dept_cnt` 时，系统会自动识别 `dept_cnt` 是一个物化视图，并将其改写为扫描物化视图的查询计划。