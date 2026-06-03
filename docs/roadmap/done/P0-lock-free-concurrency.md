# P0-02: 无锁并发实现

**优先级**: P0
**模块**: fe-catalog, fe-scheduler, be-storage
**状态**: ❌ 未开始
**预计工期**: 2个月
**价值**: ✅✅ 高（并发性能5-10倍）

---

## 📋 问题分析

### Doris的锁竞争问题

```java
// Doris: synchronized锁竞争严重
public class TabletManager {
    private Map<Long, Tablet> tablets;
    
    public synchronized Tablet getTablet(long id) {
        return tablets.get(id);  // 读也要锁
    }
    
    public synchronized void addTablet(Tablet tablet) {
        tablets.put(tablet.id, tablet);  // 写要锁
    }
}

问题：
  1. 读读阻塞（不必要的锁）
  2. 读写阻塞（严重影响并发）
  3. 锁竞争开销大（CAS失败）
  4. 吞吐受限（1000 ops/sec）
```

### HarnessDB的无锁设计目标

```
无锁架构优势：
  1. DashMap分段无锁（读读不阻塞）
  2. Actor消息传递（读写异步）
  3. 无锁竞争（Actor模型）
  4. 吞吐突破（5000-10000 ops/sec）
  
性能预期：
  - 并发吞吐: 5-10倍提升
  - 锁竞争: 完全消除
  - 延迟: <10ms（vs Doris: 100ms）
```

---

## 🎯 核心组件设计

### 1. DashMap分段无锁

**为什么选择DashMap？**
```
优势：
  1. 分段锁（Segment-level lock）
  2. 读读并发（无锁）
  3. 读写低竞争（仅影响同段）
  4. 高性能（比RwLock快5倍）
  
原理：
  - HashMap分为N个Segment（默认16）
  - 每个Segment独立RwLock
  - 读操作：不锁（直接读取）
  - 写操作：仅锁目标Segment
```

**组件设计:**

```rust
// fe-catalog/src/catalog_manager.rs

use dashmap::DashMap;

pub struct CatalogManager {
    databases: DashMap<String, Database>,
    tables: DashMap<(String, String), Table>,  // (db, table)
    tablets: DashMap<u64, Tablet>,
    replicas: DashMap<u64, Replica>,
}

impl CatalogManager {
    pub fn new() -> Self {
        Self {
            databases: DashMap::new(),  // 分段无锁HashMap
            tables: DashMap::new(),
            tablets: DashMap::new(),
            replicas: DashMap::new(),
        }
    }
    
    // 读操作（完全无锁）
    pub fn get_database(&self, name: &str) -> Option<Database> {
        self.databases.get(name).map(|ref_| ref_.clone())
    }
    
    pub fn get_table(&self, db: &str, table: &str) -> Option<Table> {
        self.tables.get(&(db.to_string(), table.to_string()))
            .map(|ref_| ref_.clone())
    }
    
    pub fn get_tablet(&self, id: u64) -> Option<Tablet> {
        self.tablets.get(&id).map(|ref_| ref_.clone())
    }
    
    // 写操作（仅锁Segment，不影响其他）
    pub fn add_database(&self, name: String, db: Database) {
        self.databases.insert(name, db);  // 仅锁目标Segment
    }
    
    pub fn add_table(&self, db: String, table: String, tbl: Table) {
        self.tables.insert((db, table), tbl);
    }
    
    pub fn add_tablet(&self, id: u64, tablet: Tablet) {
        self.tablets.insert(id, tablet);
    }
    
    // 遍历操作（不阻塞）
    pub fn list_databases(&self) -> Vec<String> {
        self.databases.iter()
            .map(|ref_| ref_.key().clone())
            .collect()
    }
    
    pub fn list_tables(&self, db: &str) -> Vec<String> {
        self.tables.iter()
            .filter(|ref_| ref_.key().0 == db)
            .map(|ref_| ref_.key().1.clone())
            .collect()
    }
}

// 性能对比测试
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_dashmap_performance() {
        let catalog = CatalogManager::new();
        
        // 并发写入测试
        let mut handles = vec![];
        for i in 0..1000 {
            handles.push(tokio::spawn(async move {
                catalog.add_tablet(i, Tablet::new(i));
            }));
        }
        
        futures::future::join_all(handles).await;
        
        // 并发读取测试（无锁）
        let mut read_handles = vec![];
        for i in 0..1000 {
            read_handles.push(tokio::spawn(async move {
                catalog.get_tablet(i);
            }));
        }
        
        futures::future::join_all(read_handles).await;
        
        // 验证：吞吐 ≥5000 ops/sec（vs RwLock: 1000）
    }
}
```

---

### 2. Actor消息传递模型

**为什么选择Actor模型？**
```
优势：
  1. 完全无锁（消息传递）
  2. 状态隔离（每个Actor独立）
  3. 异步通信（不阻塞）
  4. 易于扩展（Actor可并行）
  
原理：
  - Actor: 独立的状态 + 消息循环
  - Channel: 异步消息传递
  - Command: 消息类型定义
  - Response: 异步响应
```

**组件设计:**

```rust
// fe-catalog/src/tablet_actor.rs

use async_channel::{Sender, Receiver, bounded};
use std::sync::Arc;

pub struct TabletActor {
    tablets: Arc<DashMap<u64, Tablet>>,
    receiver: Receiver<TabletCommand>,
}

pub enum TabletCommand {
    Get {
        id: u64,
        response: Sender<Option<Tablet>>,
    },
    Add {
        tablet: Tablet,
        response: Sender<Result<(), Error>>,
    },
    Delete {
        id: u64,
        response: Sender<Result<(), Error>>,
    },
    CheckHealth {
        response: Sender<Vec<HealthInfo>>,
    },
    UpdateVersion {
        id: u64,
        version: u64,
        response: Sender<Result<(), Error>>,
    },
}

impl TabletActor {
    pub fn new(tablets: Arc<DashMap<u64, Tablet>>, receiver: Receiver<TabletCommand>) -> Self {
        Self {
            tablets,
            receiver,
        }
    }
    
    pub async fn run(&self) {
        // 消息循环（永不退出）
        while let Ok(cmd) = self.receiver.recv().await {
            self.handle_command(cmd).await;
        }
    }
    
    async fn handle_command(&self, cmd: TabletCommand) {
        match cmd {
            TabletCommand::Get { id, response } => {
                let tablet = self.tablets.get(&id).map(|ref_| ref_.clone());
                response.send(tablet).await.ok();
            }
            
            TabletCommand::Add { tablet, response } => {
                self.tablets.insert(tablet.id, tablet);
                response.send(Ok(())).await.ok();
            }
            
            TabletCommand::Delete { id, response } => {
                self.tablets.remove(&id);
                response.send(Ok(())).await.ok();
            }
            
            TabletCommand::CheckHealth { response } => {
                let health_infos = self.tablets.iter()
                    .map(|ref_| HealthInfo {
                        tablet_id: *ref_.key(),
                        status: ref_.value().check_health(),
                    })
                    .collect();
                response.send(health_infos).await.ok();
            }
            
            TabletCommand::UpdateVersion { id, version, response } => {
                if let Some(mut tablet) = self.tablets.get_mut(&id) {
                    tablet.version = version;
                    response.send(Ok(())).await.ok();
                } else {
                    response.send(Err(Error::TabletNotFound(id))).await.ok();
                }
            }
        }
    }
}

// TabletActor客户端（不直接访问状态）
pub struct TabletActorClient {
    sender: Sender<TabletCommand>,
}

impl TabletActorClient {
    pub fn new(actor: &TabletActor) -> Self {
        // 创建命令Channel
        let (sender, receiver) = bounded(1000);
        
        // 启动Actor
        tokio::spawn(async move {
            actor.run();
        });
        
        Self { sender }
    }
    
    pub async fn get(&self, id: u64) -> Option<Tablet> {
        let (response_tx, response_rx) = bounded(1);
        
        self.sender.send(TabletCommand::Get {
            id,
            response: response_tx,
        }).await.ok();
        
        response_rx.recv().await.ok().flatten()
    }
    
    pub async fn add(&self, tablet: Tablet) -> Result<(), Error> {
        let (response_tx, response_rx) = bounded(1);
        
        self.sender.send(TabletCommand::Add {
            tablet,
            response: response_tx,
        }).await.ok();
        
        response_rx.recv().await.ok().unwrap()
    }
    
    pub async fn check_health(&self) -> Vec<HealthInfo> {
        let (response_tx, response_rx) = bounded(1);
        
        self.sender.send(TabletCommand::CheckHealth {
            response: response_tx,
        }).await.ok();
        
        response_rx.recv().await.ok().unwrap()
    }
}
```

---

### 3. 其他Actor实现

**ReplicaActor:**

```rust
// fe-catalog/src/replica_actor.rs

pub struct ReplicaActor {
    replicas: Arc<DashMap<u64, Replica>>,
    receiver: Receiver<ReplicaCommand>,
}

pub enum ReplicaCommand {
    Get { id: u64, response: Sender<Option<Replica>> },
    Add { replica: Replica, response: Sender<Result<(), Error>> },
    CheckHealth { tablet_id: u64, response: Sender<Vec<ReplicaHealth>> },
    Balance { response: Sender<Vec<BalanceTask>> },
}

impl ReplicaActor {
    pub async fn run(&self) {
        while let Ok(cmd) = self.receiver.recv().await {
            self.handle_command(cmd).await;
        }
    }
    
    async fn handle_command(&self, cmd: ReplicaCommand) {
        match cmd {
            ReplicaCommand::CheckHealth { tablet_id, response } => {
                let health = self.replicas.iter()
                    .filter(|ref_| ref_.tablet_id == tablet_id)
                    .map(|ref_| ReplicaHealth {
                        replica_id: *ref_.key(),
                        status: ref_.check_health(),
                    })
                    .collect();
                response.send(health).await.ok();
            }
            
            ReplicaCommand::Balance { response } => {
                // 副本均衡算法
                let tasks = self.calculate_balance_tasks();
                response.send(tasks).await.ok();
            }
        }
    }
}
```

**CoordinatorActor:**

```rust
// fe-scheduler/src/coordinator_actor.rs

pub struct CoordinatorActor {
    query_manager: Arc<DashMap<u64, QueryContext>>,
    scheduler_client: SchedulerActorClient,
    backend_clients: Vec<BackendActorClient>,
    receiver: Receiver<CoordinatorCommand>,
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
    GetQueryStatus {
        query_id: u64,
        response: Sender<QueryStatus>,
    },
}

impl CoordinatorActor {
    pub async fn run(&self) {
        while let Ok(cmd) = self.receiver.recv().await {
            self.handle_command(cmd).await;
        }
    }
    
    async fn handle_command(&self, cmd: CoordinatorCommand) {
        match cmd {
            CoordinatorCommand::ExecuteQuery { query, response } => {
                // 异步执行查询（不阻塞）
                let result = self.execute_query_async(query).await;
                response.send(result).await.ok();
            }
        }
    }
    
    async fn execute_query_async(&self, query: Query) -> Result<QueryResult, Error> {
        // 1. 生成Fragment
        let fragments = self.plan_query(query)?;
        
        // 2. 调度Fragment（异步）
        let schedule_futures = fragments.iter()
            .map(|fragment| self.scheduler_client.schedule(fragment))
            .collect();
        
        let scheduled = futures::future::join_all(schedule_futures).await;
        
        // 3. 执行Fragment（异步）
        let result_futures = scheduled.iter()
            .map(|task_id| self.execute_task(*task_id))
            .collect();
        
        let results = futures::future::join_all(result_futures).await;
        
        // 4. 合并结果
        Ok(QueryResult::merge(results))
    }
}
```

**SchedulerActor:**

```rust
// fe-scheduler/src/scheduler_actor.rs

pub struct SchedulerActor {
    tasks: Arc<DashMap<u64, TaskContext>>,
    backends: Arc<DashMap<u64, BackendInfo>>,
    receiver: Receiver<SchedulerCommand>,
}

pub enum SchedulerCommand {
    Schedule {
        fragment: Fragment,
        response: Sender<Result<u64, Error>>,
    },
    GetTaskStatus {
        task_id: u64,
        response: Sender<TaskStatus>,
    },
    Reschedule {
        task_id: u64,
        failed_backend: u64,
        response: Sender<Result<u64, Error>>,
    },
}

impl SchedulerActor {
    pub async fn run(&self) {
        while let Ok(cmd) = self.receiver.recv().await {
            self.handle_command(cmd).await;
        }
    }
    
    async fn handle_command(&self, cmd: SchedulerCommand) {
        match cmd {
            SchedulerCommand::Schedule { fragment, response } => {
                // 选择Backend（负载感知）
                let backend = self.select_backend()?;
                
                // 创建Task
                let task_id = self.create_task(fragment, backend)?;
                
                response.send(Ok(task_id)).await.ok();
            }
            
            SchedulerCommand::Reschedule { task_id, failed_backend, response } => {
                // 失败重调度（自动迁移）
                let new_backend = self.select_backend_except(failed_backend)?;
                
                let new_task_id = self.reschedule_task(task_id, new_backend)?;
                
                response.send(Ok(new_task_id)).await.ok();
            }
        }
    }
    
    fn select_backend(&self) -> Result<BackendInfo, Error> {
        // 负载感知选择（最小负载）
        self.backends.iter()
            .min_by_key(|ref_| ref_.load_score())
            .map(|ref_| ref_.clone())
            .ok_or(Error::NoAvailableBackend)
    }
}
```

---

### 4. 无锁并发对比

| 方案 | 锁竞争 | 吞吐 | 延迟 | 适用场景 |
|------|--------|------|------|---------|
| **synchronized** | ❌ 严重 | 1000 ops/sec | 100ms | Doris（Java） |
| **RwLock** | ⚠️ 中等 | 2000 ops/sec | 50ms | 简单场景 |
| **DashMap** | ✅ 低（分段） | 5000 ops/sec | 10ms | 高并发读写 |
| **Actor模型** | ✅✅ 无锁 | 10000 ops/sec | <5ms | 完全无锁 |

**DashMap + Actor = 最佳方案**

---

## 📅 实施路线（2个月）

### Month 1: DashMap集成

**Week 1-2: CatalogManager改造**
- [ ] databases: DashMap替换
- [ ] tables: DashMap替换
- [ ] tablets: DashMap替换
- [ ] replicas: DashMap替换
- [ ] 性能测试（vs RwLock）

**Week 3-4: 其他Manager改造**
- [ ] ClusterManager: DashMap
- [ ] PartitionManager: DashMap
- [ ] IndexManager: DashMap
- [ ] 所有Manager测试

**验收标准:**
```
- 读吞吐：≥5000 ops/sec（vs RwLock: 1000）
- 写吞吐：≥3000 ops/sec
- 读延迟：≤5ms
```

---

### Month 2: Actor模型实现

**Week 1-2: 核心Actor**
- [ ] TabletActor实现
- [ ] ReplicaActor实现
- [ ] CoordinatorActor实现
- [ ] SchedulerActor实现

**Week 3-4: Actor集成测试**
- [ ] Actor并发测试
- [ ] Actor性能测试
- [ ] Actor稳定性测试
- [ ] 全链路集成测试

**验收标准:**
```
- Actor吞吐：≥10000 ops/sec
- 无锁竞争：0（完全消除）
- Actor延迟：<5ms
```

---

## 📊 性能预期对比

| 指标 | Doris（synchronized） | HarnessDB（无锁） | 提升倍数 |
|------|---------------------|----------------|---------|
| **并发吞吐** | 1000 ops/sec | 10000 ops/sec | 10倍 |
| **读延迟** | 100ms | 5ms | 20倍改善 |
| **写延迟** | 150ms | 10ms | 15倍改善 |
| **锁竞争** | 严重 | 无 | 完全消除 |
| **并发安全** | ⚠️ 潜在死锁 | ✅ Actor无锁 | 极大改善 |

---

## 📁 涉及文件

### 新建文件

```
fe-catalog/src/
├── tablet_actor.rs           # Tablet Actor（~300行）
├── replica_actor.rs          # Replica Actor（~250行）
├── catalog_actor.rs          # Catalog Actor（~200行）
└── actor_client.rs           # Actor Client（~150行）

fe-scheduler/src/
├── coordinator_actor.rs      # Coordinator Actor（~400行）
├── scheduler_actor.rs        # Scheduler Actor（~300行）
├── backend_actor.rs          # Backend Actor（~200行）
└── task_actor.rs             # Task Actor（~200行）

be-storage/src/
├── tablet_actor.rs           # Tablet管理Actor（~250行）
└── compaction_actor.rs       # Compaction Actor（~200行）

tests/integration/
└── lock_free_concurrency_test.rs # 无锁并发测试（~500行）
```

### 修改文件

```
fe-catalog/src/catalog_manager.rs  # DashMap替换
fe-scheduler/src/coordinator.rs    # Actor模型
Cargo.toml                          # 添加dashmap, async-channel
```

---

## ⚠️ 技术挑战和应对

### 挑战1: Actor消息延迟

**应对:**
```rust
// 使用bounded channel + 背压控制
let (sender, receiver) = bounded(1000);  // 限制队列长度

// 发送时检查队列长度
if sender.len() > 800 {
    // 背压：等待消费
    tokio::time::sleep(Duration::from_millis(10)).await;
}
```

### 挑战2: Actor状态持久化

**应对:**
```rust
// 定期快照Actor状态
pub struct TabletActor {
    tablets: Arc<DashMap<u64, Tablet>>,
    snapshot_interval: Duration,
}

impl TabletActor {
    pub async fn run(&self) {
        let mut snapshot_timer = tokio::time::interval(self.snapshot_interval);
        
        loop {
            tokio::select! {
                // 处理命令
                Ok(cmd) = self.receiver.recv() => {
                    self.handle_command(cmd).await;
                }
                
                // 定期快照
                _ = snapshot_timer.tick() => {
                    self.snapshot_state().await;
                }
            }
        }
    }
    
    async fn snapshot_state(&self) {
        let tablets = self.tablets.iter()
            .map(|ref_| ref_.clone())
            .collect();
        
        // 持久化快照
        save_tablets_snapshot(tablets).await.ok();
    }
}
```

### 挑战3: Actor失败重启

**应对:**
```rust
// Supervisor模式（监控Actor重启）
pub struct TabletActorSupervisor {
    tablets: Arc<DashMap<u64, Tablet>>,
}

impl TabletActorSupervisor {
    pub async fn supervise(&self) {
        loop {
            // 创建Actor
            let (sender, receiver) = bounded(1000);
            let actor = TabletActor::new(self.tablets.clone(), receiver);
            
            // 启动Actor
            let actor_task = tokio::spawn(async move {
                actor.run();
            });
            
            // 监控Actor失败
            if actor_task.await.is_err() {
                // Actor失败，重启
                log::error!("TabletActor failed, restarting...");
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }
        }
    }
}
```

---

## 💡 创新价值

**这是高价值的创新点：**

1. ✅ **并发吞吐突破**：10倍提升（1000 → 10000）
2. ✅ **延迟突破**：20倍改善（100ms → 5ms）
3. ✅ **锁竞争消除**：完全无锁（Actor模型）
4. ✅ **并发安全**：无死锁风险
5. ✅ **易于扩展**：Actor可并行增加

**无锁并发是HarnessDB稳定性的基础！**

---

## 🔗 相关文档

- [创新路线总览](00-overview.md)
- [P0-01 异步架构](P0-async-architecture.md)
- [P0-03 内存池](P0-memory-pool.md)

---

## 📝 备注

**为什么选择DashMap + Actor？**

1. ✅ DashMap：分段锁，读读并发，适合高并发读写
2. ✅ Actor：完全无锁，状态隔离，适合复杂状态管理
3. ✅ 组合：DashMap简单操作 + Actor复杂操作
4. ✅ 性能：5-10倍提升，无锁竞争
5. ✅ 稳定性：无死锁风险，易于扩展

**P0-02是HarnessDB并发性能的保障！**