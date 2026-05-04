# P2: 多租户 + 资源隔离

**优先级**: P2
**模块**: fe-scheduler, be-execution, fe-common
**状态**: ❌ 未开始

## 背景

多租户支持允许不同用户/工作组共享同一集群，同时实现资源（CPU、内存、IO）隔离，避免一个租户影响其他租户。

## 任务清单

### 1. Workload Group
- [ ] `CREATE WORKLOAD GROUP` 语法
- [ ] 定义资源限制: CPU、内存、查询并发数
- [ ] 将用户/角色绑定到 Workload Group
- [ ] Workload Group 元数据持久化

### 2. 资源隔离
- [ ] BE 端: 查询执行时绑定 Workload Group
- [ ] 内存隔离: Workload Group 级别的内存配额
- [ ] CPU 隔离: 通过 Task Slot 或 CGroup 限制
- [ ] 查询排队: 超过配额的查询排队等待

### 3. 查询管理
- [ ] `KILL QUERY` 支持
- [ ] 查询超时设置
- [ ] 查询优先级调度
- [ ] Resource Group 级别的查询统计

### 4. 多租户隔离
- [ ] Database 级别的数据隔离
- [ ] 存储配额限制
- [ ] 租户级别的监控指标

### 5. 测试
- [ ] 并发查询资源隔离验证
- [ ] 超限查询排队/拒绝
- [ ] 多 Workload Group 并行运行

## 涉及文件

- `crates/fe-common/src/workload.rs` - 新建，Workload Group 管理
- `crates/fe-scheduler/src/` - 查询调度适配
- `crates/be-execution/src/` - 资源隔离执行
- `crates/be-common/src/` - 资源追踪
