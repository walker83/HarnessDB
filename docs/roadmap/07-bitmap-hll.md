# BITMAP 和 HLL 函数

## 概述
BITMAP 和 HLL 是分析型工作负载的关键功能，用于高基数聚合操作。

## 现状分析
测试结果:
- `analytics/01_bitmap_operations.sql`: 64 errors
- `analytics/02_hll_operations.sql`: 84 errors
- `advanced/02_bitmap_hll_positive.sql`: 24 errors

主要缺失:
- bitmap_union 聚合函数
- bitmap_count 函数
- hll_union 聚合函数
- hll_raw_estimate 函数

## 子任务

### Task 1: BITMAP 函数支持
- 实现 bitmap_empty() 构造函数
- 实现 bitmap_hash() 函数
- 实现 bitmap_union() 聚合函数
- 实现 bitmap_count() 函数
- 实现 bitmap_to_string() 函数
- 验证: `analytics/01_bitmap_operations.sql` 基础函数通过

### Task 2: BITMAP 高级操作
- 实现 bitmap_and() 函数
- 实现 bitmap_or() 函数
- 实现 bitmap_andnot() 函数
- 实现 bitmap_xor() 函数
- 验证: `analytics/01_bitmap_operations.sql` 高级操作通过

### Task 3: HLL 函数支持
- 实现 hll_empty() 构造函数
- 实现 hll_hash() 函数
- 实现 hll_union() 聚合函数
- 实现 hll_raw_estimate() 函数
- 实现 hll_estimate_unicode() 函数
- 验证: `analytics/02_hll_operations.sql` 基础函数通过

### Task 4: 物化视图 BITMAP/HLL
- 支持创建包含 BITMAP/HLL 聚合的物化视图
- 支持增量刷新 BITMAP/HLL 物化视图
- 验证: `advanced/02_bitmap_hll_positive.sql` 物化视图部分通过

## 验收标准
- [ ] BITMAP 基本操作函数正常工作
- [ ] HLL 基本操作函数正常工作
- [ ] bitmap_union/hll_union 可以用于 GROUP BY
- [ ] BITMAP/HLL 相关测试通过率 > 80%

## 影响范围
- `fe-expression`: BITMAP/HLL 函数注册
- `be-execution`: BITMAP/HLL 算子实现
- `types`: BITMAP/HLL 数据类型
