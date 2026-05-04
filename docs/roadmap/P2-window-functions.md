# P2: 窗口函数补全

**优先级**: P2
**模块**: fe-expression, fe-sql-planner
**状态**: ❌ 未开始

## 背景

RorisDB 已实现 ROW_NUMBER、RANK、DENSE_RANK、LAG、LEAD 窗口函数。但缺少常用的聚合窗口函数和 FIRST_VALUE/LAST_VALUE。

## 已实现

- ✅ ROW_NUMBER
- ✅ RANK
- ✅ DENSE_RANK
- ✅ LAG
- ✅ LEAD

## 缺失

### 1. 聚合窗口函数
- [ ] SUM() OVER(...)
- [ ] AVG() OVER(...)
- [ ] COUNT() OVER(...)
- [ ] MIN() OVER(...)
- [ ] MAX() OVER(...)

### 2. 取值窗口函数
- [ ] FIRST_VALUE() OVER(...)
- [ ] LAST_VALUE() OVER(...)
- [ ] NTH_VALUE() OVER(...)

### 3. OVER 子句增强
- [ ] 支持窗口帧定义: ROWS BETWEEN ... AND ...
- [ ] 支持窗口帧定义: RANGE BETWEEN ... AND ...
- [ ] 默认帧语义（ROWS UNBOUNDED PRECEDING vs RANGE UNBOUNDED PRECEDING）

### 4. 测试
- [ ] 聚合窗口函数正确性（含 NULL、空分区）
- [ ] 窗口帧边界测试
- [ ] 多窗口函数在同一查询中
- [ ] 性能: 大数据量窗口函数执行

## 涉及文件

- `crates/fe-expression/src/functions.rs` - 新增窗口函数实现
- `crates/fe-sql-planner/src/planner.rs` - OVER 子句规划
