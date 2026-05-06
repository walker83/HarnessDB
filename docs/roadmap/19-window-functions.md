# 高级窗口函数

## 概述
当前窗口函数支持基本正常，但仍有一些高级模式不支持。

## 现状分析
测试结果:
- `analytics/05_advanced_window_patterns.sql`: 20 errors
- `query/04_window_function_positive.sql`: 0 errors
- `functions/04_aggregate_window_functions.sql`: 8 errors

## 子任务

### Task 1: 窗口帧增强
- 支持 ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW
- 支持 RANGE BETWEEN INTERVAL '1' DAY PRECEDING AND CURRENT ROW
- 支持 EXCLUDE CURRENT ROW
- 支持 EXCLUDE TIES
- 验证: `analytics/05_advanced_window_patterns.sql` 帧部分通过

### Task 2: 高级窗口函数
- 实现 CUME_DIST() 函数
- 实现 PERCENT_RANK() 函数
- 实现 NTH_VALUE() 函数
- 实现 LAG()/LEAD() 变体
- 验证: `analytics/05_advanced_window_patterns.sql` 函数部分通过

### Task 3: 窗口函数优化
- 实现窗口函数结果缓存
- 支持窗口函数下推优化
- 支持窗口函数并行执行
- 验证: `performance/01_query_optimization.sql` 窗口优化部分通过

## 验收标准
- [ ] 窗口帧语法完整支持
- [ ] 高级窗口函数正常工作
- [ ] 窗口函数性能优化
- [ ] 窗口函数测试通过率 > 90%

## 影响范围
- `fe-sql-planner`: 窗口函数计划优化
- `fe-expression`: 高级窗口函数注册
- `be-execution`: 窗口函数算子
