# 物化视图 (Materialized View)

## 概述
物化视图是加速查询的核心功能，当前支持不完整，需要增强 ROLLUP 和 MV 支持。

## 现状分析
测试结果:
- `advanced/01_materialized_view_positive.sql`: 20 errors
- `storage_engine/03_materialized_view_deep.sql`: 114 errors
- `performance/06_materialized_view_perf.sql`: 58 errors
- `ddl/02_index_rollup_operations.sql`: 182 errors (部分 ROLLUP)

主要缺失:
- ALTER TABLE ... ADD ROLLUP
- ALTER TABLE ... DROP ROLLUP
- 物化视图查询改写
- 物化视图刷新策略

## 子任务

### Task 1: ROLLUP DDL
- 支持 ALTER TABLE ADD ROLLUP
- 支持 ALTER TABLE DROP ROLLUP
- 存储 ROLLUP 元数据
- 验证: `ddl/02_index_rollup_operations.sql` ROLLUP 部分通过

### Task 2: 物化视图创建
- 支持 CREATE MATERIALIZED VIEW 完整语法
- 支持 WITH SCHEMA
- 支持 DISTRIBUTED BY
- 验证: `advanced/01_materialized_view_positive.sql` CREATE 部分通过

### Task 3: 物化视图查询改写
- 实现基于物化视图的查询改写
- 支持自动选择最优物化视图
- 验证: `storage_engine/03_materialized_view_deep.sql` 查询改写部分通过

### Task 4: 物化视图刷新
- 实现 REFRESH MATERIALIZED VIEW
- 支持 ON COMMIT 刷新策略
- 支持 ON DEMAND 刷新策略
- 支持增量刷新
- 验证: `advanced/01_materialized_view_positive.sql` REFRESH 部分通过

## 验收标准
- [ ] 可以创建物化视图
- [ ] 可以添加/删除 ROLLUP
- [ ] 查询可以正确使用物化视图加速
- [ ] 物化视图刷新策略正常工作
- [ ] MV 相关测试通过率 > 80%

## 影响范围
- `fe-sql-parser`: MV/ROLLUP 语法解析
- `fe-sql-planner`: MV 计划生成和改写
- `fe-catalog`: MV/ROLLUP 元数据
- `be-storage`: MV 数据存储和刷新
- `be-execution`: MV 查询执行
