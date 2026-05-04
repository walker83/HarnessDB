# P1: Runtime Filter

**优先级**: P1
**模块**: fe-sql-planner, fe-scheduler, be-execution
**状态**: ❌ 未开始

## 背景

Runtime Filter 是 OLAP 数据库中重要的 Join 优化技术。在运行时将 Build 侧的数据生成 Bloom Filter / Min-Max Filter，下推到 Probe 侧的 Scan 节点，提前过滤不满足条件的数据，减少网络传输和计算量。

## 任务清单

### 1. Runtime Filter 框架
- [ ] 定义 RuntimeFilter 数据结构（类型、列、大小限制）
- [ ] 支持 Bloom Filter 类型（适合高基数列）
- [ ] 支持 Min-Max Filter 类型（适合低基数列）
- [ ] 支持 IN Filter 类型（适合极低基数列）

### 2. Planner 集成
- [ ] 在 Join 节点分析中识别可生成 Runtime Filter 的场景
- [ ] 标记 Build 侧需要生成的 Filter
- [ ] 标记 Probe 侧需要应用的 Filter
- [ ] 考虑 Filter 下推到 Scan 节点

### 3. Scheduler 集成
- [ ] 在 Fragment 规划中传递 Runtime Filter 描述
- [ ] Broadcast Join: Filter 从 Build Fragment 广播到 Probe Fragment
- [ ] Shuffle Join: Filter 聚合后下发

### 4. BE 执行集成
- [ ] Build 侧: Hash Join 构建 Hash Table 同时生成 Filter
- [ ] Filter 序列化/反序列化
- [ ] Probe 侧: Scan 节点接收并应用 Filter 过滤行
- [ ] Probe 侧: Hash Join Probe 也应用 Filter

### 5. 集成测试
- [ ] 大表 JOIN 小表场景验证过滤效果
- [ ] 多个 Runtime Filter 同时生效
- [ ] 与 Broadcast/Shuffle Join 配合正确性

## 涉及文件

- `crates/fe-sql-planner/src/` - 新建 runtime_filter 规则
- `crates/fe-scheduler/src/` - Fragment 传递 Filter
- `crates/be-execution/src/` - BE 端 Filter 生成和应用
- `crates/types/src/` - Bloom Filter 数据结构（可能复用已有的）
