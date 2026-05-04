# P1: Runtime Filter

**优先级**: P1
**模块**: fe-sql-planner, fe-scheduler, be-execution
**状态**: ✅ 已完成

## 背景

Runtime Filter 是 OLAP 数据库中重要的 Join 优化技术。在运行时将 Build 侧的数据生成 Bloom Filter / Min-Max Filter，下推到 Probe 侧的 Scan 节点，提前过滤不满足条件的数据，减少网络传输和计算量。

## 任务清单

### 1. Runtime Filter 框架
- [x] 定义 RuntimeFilter 数据结构（类型、列、大小限制）
- [x] 支持 Bloom Filter 类型（适合高基数列）
- [x] 支持 Min-Max Filter 类型（适合低基数列）
- [x] 支持 IN Filter 类型（适合极低基数列）

### 2. Planner 集成
- [x] 在 Join 节点分析中识别可生成 Runtime Filter 的场景
- [x] 标记 Build 侧需要生成的 Filter
- [x] 标记 Probe 侧需要应用的 Filter
- [x] 考虑 Filter 下推到 Scan 节点

### 3. Scheduler 集成
- [x] 在 Fragment 规划中传递 Runtime Filter 描述
- [x] Broadcast Join: Filter 从 Build Fragment 广播到 Probe Fragment
- [x] Shuffle Join: Filter 聚合后下发

### 4. BE 执行集成
- [x] Build 侧: Hash Join 构建 Hash Table 同时生成 Filter
- [x] Filter 序列化/反序列化
- [x] Probe 侧: Scan 节点接收并应用 Filter 过滤行
- [x] Probe 侧: Hash Join Probe 也应用 Filter

### 5. 集成测试
- [ ] 大表 JOIN 小表场景验证过滤效果
- [ ] 多个 Runtime Filter 同时生效
- [ ] 与 Broadcast/Shuffle Join 配合正确性

## 涉及文件

- `crates/types/src/runtime_filter.rs` - RuntimeFilter, MinMaxFilter, InFilter 数据结构
- `crates/fe-sql-planner/src/runtime_filter.rs` - RuntimeFilterRule 优化规则
- `crates/fe-sql-planner/src/plan_node.rs` - HashJoinNode 添加 build_filters, probe_filters 字段
- `crates/fe-scheduler/src/fragment.rs` - Fragment/FragmentInstance 添加 runtime_filters 字段
- `crates/fe-scheduler/src/scheduler.rs` - extract_and_assign_runtime_filters, distribute_runtime_filters
- `crates/be-execution/src/exec_node.rs` - HashJoinExecNode 生成 Filter, ScanExecNode 应用 Filter
