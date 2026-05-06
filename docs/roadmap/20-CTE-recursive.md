# CTE 和递归查询

## 概述
当前 CTE (Common Table Expression) 和递归查询支持不完整。

## 现状分析
测试结果:
- `query_advanced/01_cte_recursive_queries.sql`: 32 errors
- `query_advanced/02_set_theory_queries.sql`: 27 errors

主要缺失:
- WITH RECURSIVE 递归语法
- 递归终止条件
- 递归深度控制

## 子任务

### Task 1: 递归 CTE 语法
- 支持 WITH RECURSIVE 语法
- 支持递归终止条件检测
- 支持 MAX_RECURSIVE_DEPTH 设置
- 验证: `query_advanced/01_cte_recursive_queries.sql` 基础递归部分通过

### Task 2: 递归查询执行
- 实现递归查询执行
- 支持层级遍历
- 支持递归结果排序
- 验证: `query_advanced/01_cte_recursive_queries.sql` 执行部分通过

### Task 3: 集合运算增强
- 支持 UNION/INTERSECT/EXCEPT
- 支持 UNION ALL
- 支持集合运算优先级
- 验证: `query_advanced/02_set_theory_queries.sql` 通过率 > 90%

## 验收标准
- [ ] WITH RECURSIVE 语法正常解析
- [ ] 递归查询返回正确结果
- [ ] 集合运算正常工作
- [ ] CTE 测试通过率 > 80%

## 影响范围
- `fe-sql-parser`: WITH RECURSIVE 语法解析
- `fe-sql-planner`: 递归查询计划
- `be-execution`: 递归执行算子
