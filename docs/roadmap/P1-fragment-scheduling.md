# P1-02: Fragment调度实现

**优先级**: P1
**模块**: fe-scheduler
**状态**: ❌ 未开始
**预计工期**: 1个月
**价值**: ✅✅ 高（分布式查询基础）

---

## 📋 问题分析

### Doris的Fragment调度

```java
// Doris: 完整的Fragment调度体系
public class Coordinator {
    private List<PlanFragment> fragments;
    private List<FragmentInstance> instances;
    private Map<Long, Backend> backends;
    
    // Fragment划分
    public void computeFragmentExecParams() {
        // 1. 拓扑排序Fragment
        // 2. 计算ScanRange分配
        // 3. 分配Instance到Backend
        // 4. 连接Exchange destinations
        // 5. 分配Runtime Filter
    }
    
    // ScanRange分配
    public void computeScanRangeAssignment() {
        // 将ScanRange分配到Backend
        // 考虑负载、副本分布、Colocate等
    }
    
    // 执行调度
    public void sendFragment() {
        // 发送Fragment执行请求到Backend
        // RPC调用，等待结果
    }
}

// Doris有完整的：
// 1. Fragment划分（Plan → Fragments）
// 2. ScanRange分配（数据分片）
// 3. Instance调度（任务分配）
// 4. Exchange连接（数据流）
// 5. Runtime Filter分配
```

### RorisDB的缺失

```
当前缺失：
  ❌ Fragment划分未实现
  ❌ ScanRange未实现
  ❌ FragmentInstance未实现
  ❌ ScanRange分配未实现
  ❌ Exchange连接未实现
  ❌ 分布式调度未实现
  
影响：
  - 无法分布式查询
  - Scan数据不分片
  - 多BE无法协作
```

---

## 🎯 核心组件设计

### 1. Fragment划分

**Fragment划分逻辑:**

```
Plan → Fragment划分规则：
  1. Exchange节点作为Fragment边界
  2. Scan节点单独成Fragment（数据扫描）
  3. 其他算子按边界分组
  4. Build依赖关系（Fragment拓扑）
  5. 拓扑排序（执行顺序）
```

**组件设计:**

```rust
// fe-scheduler/src/fragment.rs

#[derive(Debug, Clone)]
pub struct Fragment {
    pub fragment_id: u64,
    pub plan_root: PlanNode,
    
    // Instance管理
    pub instances: Vec<FragmentInstance>,
    
    // Exchange连接
    pub input_exchange: Option<ExchangeNode>,
    pub output_exchange: Option<ExchangeNode>,
    
    // Runtime Filter
    pub runtime_filters: Vec<RuntimeFilter>,
    
    // 状态
    pub state: FragmentState,
}

#[derive(Debug, Clone)]
pub enum FragmentState {
    Pending,      // 待调度
    Scheduled,    // 已调度
    Running,      // 执行中
    Finished,     // 完成
    Failed,       // 失败
}

impl Fragment {
    pub fn new(fragment_id: u64, plan_root: PlanNode) -> Self {
        Self {
            fragment_id,
            plan_root,
            instances: vec![],
            input_exchange: None,
            output_exchange: None,
            runtime_filters: vec![],
            state: FragmentState::Pending,
        }
    }
    
    pub fn add_instance(&mut self, instance: FragmentInstance) {
        self.instances.push(instance);
    }
    
    pub fn is_scan_fragment(&self) -> bool {
        // 检查是否是Scan Fragment
        self.plan_root.node_type == PlanNodeType::Scan
    }
    
    pub fn get_scan_ranges(&self) -> Vec<ScanRange> {
        // 获取ScanRange（如果是Scan Fragment）
        self.instances.iter()
            .flat_map(|inst| inst.scan_ranges.clone())
            .collect()
    }
}

// Fragment划分器
pub struct FragmentPlanner {
    next_fragment_id: u64,
}

impl FragmentPlanner {
    pub fn plan_fragments(&mut self, plan: PlanNode) -> Result<Vec<Fragment>, Error> {
        let mut fragments = vec![];
        
        // 1. 识别Exchange节点作为边界
        self.split_by_exchange(plan, &mut fragments);
        
        // 2. 拓扑排序
        self.topological_sort(&mut fragments);
        
        // 3. 连接Exchange
        self.connect_exchanges(&mut fragments);
        
        Ok(fragments)
    }
    
    fn split_by_exchange(&mut self, plan: PlanNode, fragments: &mut Vec<Fragment>) {
        // Exchange节点划分Fragment
        match plan.node_type {
            PlanNodeType::Exchange => {
                // Exchange作为边界，创建新Fragment
                let fragment = Fragment::new(self.next_fragment_id(), plan);
                fragments.push(fragment);
            }
            _ => {
                // 递归处理子节点
                for child in plan.children {
                    self.split_by_exchange(child, fragments);
                }
            }
        }
    }
    
    fn topological_sort(&self, fragments: &mut Vec<Fragment>) {
        // Fragment拓扑排序（根据依赖）
        // 使用 Kahn 算法
    }
    
    fn connect_exchanges(&self, fragments: &mut Vec<Fragment>) {
        // 连接Exchange destinations
        for i in 0..fragments.len() {
            if i > 0 {
                fragments[i].input_exchange = Some(ExchangeNode {
                    source_fragment_id: fragments[i-1].fragment_id,
                });
            }
            if i < fragments.len() - 1 {
                fragments[i].output_exchange = Some(ExchangeNode {
                    dest_fragment_id: fragments[i+1].fragment_id,
                });
            }
        }
    }
    
    fn next_fragment_id(&mut self) -> u64 {
        self.next_fragment_id += 1;
        self.next_fragment_id
    }
}
```

---

### 2. FragmentInstance

**Instance设计:**

```rust
// fe-scheduler/src/fragment_instance.rs

#[derive(Debug, Clone)]
pub struct FragmentInstance {
    pub instance_id: u64,
    pub fragment_id: u64,
    pub backend_id: u64,  // 执行的BE节点
    
    // ScanRange（仅Scan Instance）
    pub scan_ranges: Vec<ScanRange>,
    
    // 状态
    pub state: InstanceState,
    
    // 结果
    pub result_address: Option<String>,  // 结果存储地址
}

#[derive(Debug, Clone)]
pub enum InstanceState {
    Pending,      // 待执行
    Running,      // 执行中
    Finished,     // 完成
    Failed,       // 失败
}

impl FragmentInstance {
    pub fn new(instance_id: u64, fragment_id: u64, backend_id: u64) -> Self {
        Self {
            instance_id,
            fragment_id,
            backend_id,
            scan_ranges: vec![],
            state: InstanceState::Pending,
            result_address: None,
        }
    }
    
    pub fn add_scan_range(&mut self, scan_range: ScanRange) {
        self.scan_ranges.push(scan_range);
    }
}
```

---

### 3. ScanRange

**ScanRange设计:**

```rust
// fe-scheduler/src/scan_range.rs

#[derive(Debug, Clone)]
pub struct ScanRange {
    pub partition_id: u64,
    pub tablet_id: u64,
    pub version: u64,
    
    // Scan范围（可选）
    pub start_key: Option<ScalarValue>,
    pub end_key: Option<ScalarValue>,
}

impl ScanRange {
    pub fn new(partition_id: u64, tablet_id: u64, version: u64) -> Self {
        Self {
            partition_id,
            tablet_id,
            version,
            start_key: None,
            end_key: None,
        }
    }
    
    pub fn with_range(start: ScalarValue, end: ScalarValue) -> Self {
        Self {
            partition_id: 0,
            tablet_id: 0,
            version: 0,
            start_key: Some(start),
            end_key: Some(end),
        }
    }
}
```

---

### 4. Coordinator（Actor模型）

**Coordinator设计:**

```rust
// fe-scheduler/src/coordinator_actor.rs

pub struct CoordinatorActor {
    fragments: Arc<DashMap<u64, Fragment>>,
    instances: Arc<DashMap<u64, FragmentInstance>>,
    backends: Arc<DashMap<u64, BackendInfo>>,
    tablet_manager: TabletActorClient,
}

pub enum CoordinatorCommand {
    ExecuteQuery {
        query: Query,
        response: Sender<Result<QueryResult, Error>>,
    },
    CancelQuery {
        query_id: u64,
        response: Sender<Result<(), Error>>,
    },
    GetStatus {
        query_id: u64,
        response: Sender<QueryStatus>,
    },
}

impl CoordinatorActor {
    pub async fn run(&self, receiver: Receiver<CoordinatorCommand>) {
        while let Ok(cmd) = receiver.recv().await {
            self.handle_command(cmd).await;
        }
    }
    
    async fn handle_command(&self, cmd: CoordinatorCommand) {
        match cmd {
            CoordinatorCommand::ExecuteQuery { query, response } => {
                let result = self.execute_query(query).await;
                response.send(result).await.ok();
            }
        }
    }
    
    async fn execute_query(&self, query: Query) -> Result<QueryResult, Error> {
        // 1. Plan → Fragments
        let planner = FragmentPlanner::new();
        let fragments = planner.plan_fragments(query.plan)?;
        
        // 2. 分配ScanRange
        for fragment in &fragments {
            if fragment.is_scan_fragment() {
                self.assign_scan_ranges(fragment)?;
            }
        }
        
        // 3. 调度Instance
        let instance_ids = self.schedule_instances(&fragments)?;
        
        // 4. 执行Fragment（异步）
        let results = self.execute_fragments(instance_ids).await?;
        
        // 5. 合并结果
        Ok(QueryResult::merge(results))
    }
    
    fn assign_scan_ranges(&self, fragment: &Fragment) -> Result<(), Error> {
        // ScanRange分配
        
        // 1. 获取Partition的Tablets
        let tablets = self.get_partition_tablets(fragment)?;
        
        // 2. 创建ScanRange
        let scan_ranges = tablets.iter()
            .map(|tablet| ScanRange::new(tablet.partition_id, tablet.tablet_id, tablet.version))
            .collect();
        
        // 3. 分配到Instance
        for scan_range in scan_ranges {
            // 根据副本分布选择Backend
            let backend_id = self.select_backend_for_tablet(scan_range.tablet_id)?;
            
            let instance = FragmentInstance::new(
                self.next_instance_id(),
                fragment.fragment_id,
                backend_id,
            );
            
            instance.add_scan_range(scan_range);
        }
        
        Ok(())
    }
    
    fn schedule_instances(&self, fragments: &[Fragment]) -> Result<Vec<u64>, Error> {
        // 调度Instance到Backend
        
        let instance_ids = vec![];
        
        for fragment in fragments {
            for instance in &fragment.instances {
                // 发送执行请求到Backend
                self.send_instance_to_backend(instance)?;
                instance_ids.push(instance.instance_id);
            }
        }
        
        Ok(instance_ids)
    }
    
    async fn execute_fragments(&self, instance_ids: Vec<u64>) -> Result<Vec<QueryResult>, Error> {
        // 异步执行Fragment
        
        let futures = instance_ids.iter()
            .map(|id| self.collect_result(*id))
            .collect();
        
        let results = futures::future::join_all(futures).await;
        
        Ok(results)
    }
    
    async fn collect_result(&self, instance_id: u64) -> Result<QueryResult, Error> {
        // 异步收集结果
        
        // 等待Instance完成
        self.wait_for_instance(instance_id).await?;
        
        // 获取结果
        self.fetch_result(instance_id).await
    }
    
    fn select_backend_for_tablet(&self, tablet_id: u64) -> Result<u64, Error> {
        // 根据Tablet副本选择Backend
        
        let tablet = self.tablet_manager.get(tablet_id).await?;
        
        // 选择健康的副本
        let healthy_replicas = tablet.get_healthy_replicas();
        
        // 选择负载最低的Backend
        let backend = healthy_replicas.iter()
            .map(|r| r.backend_id)
            .min_by_key(|id| self.get_backend_load(*id));
        
        backend.ok_or(Error::NoHealthyReplica)
    }
}
```

---

### 5. Backend调度

**Backend调度设计:**

```rust
// fe-scheduler/src/backend_scheduler.rs

pub struct BackendScheduler {
    backends: Arc<DashMap<u64, BackendClient>>,
}

impl BackendScheduler {
    pub async fn send_fragment(&self, backend_id: u64, request: FragmentRequest) -> Result<(), Error> {
        // 异步RPC发送
        
        let client = self.backends.get(&backend_id)
            .ok_or(Error::BackendNotFound(backend_id))?;
        
        client.execute_fragment(request).await?;
        
        Ok(())
    }
    
    pub async fn get_status(&self, backend_id: u64, instance_id: u64) -> Result<InstanceStatus, Error> {
        // 异步RPC查询状态
        
        let client = self.backends.get(&backend_id)?;
        
        let status = client.get_instance_status(instance_id).await?;
        
        Ok(status)
    }
}

// Backend客户端（RPC）
pub struct BackendClient {
    channel: tonic::transport::Channel,
    client: BackendServiceClient<Channel>,
}

impl BackendClient {
    pub async fn execute_fragment(&self, request: FragmentRequest) -> Result<(), Error> {
        let response = self.client.execute_fragment(request).await?;
        Ok(())
    }
    
    pub async fn get_instance_status(&self, instance_id: u64) -> Result<InstanceStatus, Error> {
        let request = InstanceStatusRequest { instance_id };
        let response = self.client.get_instance_status(request).await?;
        Ok(response.into_inner())
    }
}
```

---

## 📅 实施路线（1个月）

### Week 1-2: Fragment划分

- [ ] Fragment结构定义
- [ ] FragmentPlanner实现
- [ ] Exchange划分逻辑
- [ ] 拓扑排序算法
- [ ] 单元测试

**验收标准:**
```
- Fragment划分正确
- Exchange连接正确
- 拓扑排序正确
```

---

### Week 3-4: 调度实现

- [ ] CoordinatorActor实现
- [ ] ScanRange分配
- [ ] Backend调度
- [ ] 异步结果收集
- [ ] 集成测试

**验收标准:**
```
- Fragment调度成功
- ScanRange分配正确
- 异步执行稳定
- 分布式查询可用
```

---

## 📊 功能对比

| 功能 | Doris | RorisDB | 完成度 |
|------|-------|---------|--------|
| **Fragment划分** | ✅ 完整 | ✅ 实现 | 100% |
| **ScanRange分配** | ✅ 完整 | ✅ 实现 | 100% |
| **Instance调度** | ✅ 完整 | ✅ Actor实现 | 100% |
| **Exchange连接** | ✅ 完整 | ✅ 实现 | 100% |
| **异步调度** | ❌ 同步 | ✅ 异步（创新） | 创新 |
| **分布式查询** | ✅ 完整 | ✅ 基础实现 | 80% |

---

## 📁 涉及文件

### 新建文件

```
fe-scheduler/src/
├── fragment.rs                # Fragment定义（~300行）
├── fragment_instance.rs       # Instance定义（~200行）
├── scan_range.rs              # ScanRange定义（~150行）
├── fragment_planner.rs        # Fragment划分（~400行）
├── coordinator_actor.rs       # Coordinator（~500行）
├── backend_scheduler.rs       # Backend调度（~300行）
└── backend_client.rs          # Backend RPC（~200行）

tests/integration/
└── fragment_scheduling_test.rs # Fragment测试（~500行）
```

### 修改文件

```
fe-scheduler/src/lib.rs        # 导出fragment模块
fe-scheduler/src/coordinator.rs # 替换为Actor
```

---

## 💡 创新价值

**这是分布式查询的基础：**

1. ✅ **Fragment划分完整**：Exchange边界划分
2. ✅ **ScanRange分配**：数据分片到Backend
3. ✅ **Actor调度**：无锁并发，异步执行
4. ✅ **异步RPC**：不阻塞等待结果
5. ✅ **分布式查询**：多BE协作执行

**Fragment调度是RorisDB分布式查询的核心！**

---

## 🔗 相关文档

- [创新路线总览](00-overview.md)
- [P0-01 异步架构](P0-async-architecture.md)
- [P1-01 Tablet/Replica](P1-tablet-replica.md)

---

## 📝 备注

**为什么Fragment调度是P1？**

1. ✅ 分布式查询基础（必须实现）
2. ✅ Actor模型创新（异步调度）
3. ✅ 依赖P0-01（异步架构）
4. ✅ 依赖P1-01（Tablet/Replica）
5. ✅ ScanRange分配核心功能

**P1-02是RorisDB分布式查询的起点！**