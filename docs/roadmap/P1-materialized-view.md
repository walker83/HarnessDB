# P1: 物化视图

**优先级**: P1
**模块**: fe-sql-planner, fe-catalog
**状态**: 🚧 部分实现（数据结构已定义，逻辑未实现）

## 背景

物化视图可以预先计算和存储查询结果，加速聚合查询。当前 `materialized_view.rs` 中有 `MaterializedView` 结构体和 `rewrite_query` 方法占位，但没有实际逻辑。

## 现状

- ✅ `MaterializedView` 结构体已定义（id, name, database, definition, plan, refresh_strategy, base_tables）
- ❌ `rewrite_query()` 方法返回 `None`（未实现）
- ❌ 无 CREATE MATERIALIZED VIEW / DROP MATERIALIZED VIEW DDL
- ❌ 无查询透明改写逻辑
- ❌ 无刷新机制

## 任务清单

### 1. DDL 支持
- [ ] Parser: CREATE MATERIALIZED VIEW 语法解析
- [ ] Planner: 生成物化视图的 Physical Plan 并存储
- [ ] Catalog: 注册物化视图元数据
- [ ] DROP MATERIALIZED VIEW 支持
- [ ] ALTER MATERIALIZED VIEW (暂停/恢复刷新) 支持

### 2. 查询透明改写
- [ ] 分析查询 SQL，提取 SELECT 列、WHERE 条件、GROUP BY、JOIN
- [ ] 匹配可用的物化视图
- [ ] 验证查询是物化视图定义的超集（或等价）
- [ ] 将查询改写为扫描物化视图 + 补偿（如追加过滤条件）
- [ ] 改写后 Plan 路径代价对比，仅当更优时采用

### 3. 刷新机制
- [ ] 手动刷新: REFRESH MATERIALIZED VIEW
- [ ] 自动刷新: 基于基表数据变更触发
- [ ] 增量刷新（长期目标）

### 4. 集成测试
- [ ] 单表聚合物化视图 + 查询改写
- [ ] 多表 JOIN 物化视图
- [ ] 带 WHERE 的物化视图
- [ ] 刷新正确性验证

## 涉及文件

- `crates/fe-sql-planner/src/materialized_view.rs` - 核心：改写逻辑
- `crates/fe-sql-parser/src/parser.rs` - DDL 解析
- `crates/fe-catalog/src/materialized_view.rs` - 元数据管理
