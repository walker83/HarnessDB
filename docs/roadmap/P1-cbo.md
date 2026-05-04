# P1: CBO 代价模型 + 统计信息

**优先级**: P1
**模块**: fe-sql-planner
**状态**: 🚧 框架存在，核心逻辑未实现

## 背景

RorisDB 已有 RBO 优化器，统计信息的数据结构（TableStats、ColumnStats、NDV）已定义，但没有实际的统计信息收集机制和代价模型。

## 现状

- ✅ `fe-sql-planner/src/statistics.rs` 中 `TableStats`、`ColumnStats` 已定义
- ✅ `StatisticsProvider` trait 已定义
- ✅ Optimizer 接受 StatisticsProvider 参数
- ❌ 无实际代价模型（Cost Model）
- ❌ 无统计信息收集机制（ANALYZE TABLE）
- ❌ 无直方图
- ❌ 只有测试用的内存 Provider

## 任务清单

### 1. 统计信息收集
- [ ] 实现 `ANALYZE TABLE` 语法解析
- [ ] 采样统计: 对大表进行采样而非全量扫描
- [ ] 全量统计: 对小表全量计算
- [ ] 列统计: 行数、NULL 数、Min/Max、NDV、平均长度
- [ ] 统计信息持久化到 Catalog

### 2. 直方图
- [ ] 实现等深直方图（Equi-depth Histogram）
- [ ] 存储列值分布信息
- [ ] 用于范围查询选择性估算

### 3. 代价模型
- [ ] 定义 Cost 单位（CPU、IO、Network、Memory）
- [ ] Scan 代价估算: 基于行数、列数、压缩比
- [ ] Join 代价估算: 基于两侧行数、Join 类型
- [ ] Aggregate 代价估算: 基于分组数、输入行数
- [ ] Sort 代价估算
- [ ] 网络传输代价: Shuffle/Broadcast 开销

### 4. CBO 优化规则
- [ ] 基于 Join 代价自动选择 Broadcast vs Shuffle
- [ ] 基于 Join 代价重排序多表 Join
- [ ] 基于代价选择聚合策略（两阶段 vs 一阶段）
- [ ] 考虑数据倾斜的 Join 策略

### 5. 统计信息管理
- [ ] 自动统计信息收集（数据变更超过阈值触发）
- [ ] 统计信息过期机制
- [ ] 手动更新统计信息
- [ ] SHOW STATS 查看统计信息

## 涉及文件

- `crates/fe-sql-planner/src/statistics.rs` - 扩展统计信息结构
- `crates/fe-sql-planner/src/cost.rs` - 新建，代价模型
- `crates/fe-sql-planner/src/optimizer.rs` - 集成 CBO
- `crates/fe-sql-parser/src/parser.rs` - ANALYZE TABLE 语法
- `crates/fe-catalog/src/` - 统计信息持久化
