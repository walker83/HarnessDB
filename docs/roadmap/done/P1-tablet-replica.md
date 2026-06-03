# P1-01: Tablet/Replica管理

**优先级**: P1
**模块**: fe-catalog
**状态**: ❌ 未开始
**预计工期**: 1个月
**价值**: ✅✅ 高（分布式存储基础）

---

## 📋 问题分析

### Doris的Tablet/Replica管理

```java
// Doris: 完整的Tablet/Replica管理体系
public class OlapTable {
    private Map<Long, Partition> idToPartition;
    private Map<String, Partition> nameToPartition;
    private Map<Long, Index> indexes;  // Rollup indexes
    
    public synchronized Tablet getTablet(long tabletId) {
        for (Partition partition : idToPartition.values()) {
            for (Index index : partition.getIndexes()) {
                Tablet tablet = index.getTablet(tabletId);
                if (tablet != null) {
                    return tablet;
                }
            }
        }
        return null;
    }
}

// Doris有完整的：
// 1. Tablet管理（创建/删除/状态）
// 2. Replica管理（分布/健康/均衡）
// 3. TabletScheduler（修复/均衡）
// 4. TabletChecker（健康检查）
// 5. Rebalancer（负载均衡）
```

### HarnessDB的缺失

```
当前缺失：
  ❌ Tablet管理未实现
  ❌ Replica管理未实现
  ❌ TabletScheduler未实现
  ❌ TabletChecker未实现
  ❌ Rebalancer未实现
  ❌ 副本配置未实现
  ❌ 副本分布未实现
  
影响：
  - 分区查询错误（无Tablet）
  - 副本分布错误（无Replica）
  - 副本修复缺失（无健康检查）
```

---

## 🎯 核心组件设计

### 1. Tablet管理

**组件设计:**

```rust
// fe-catalog/src/tablet.rs

use std::sync::Arc;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tablet {
    pub tablet_id: u64,
    pub table_id: u64,
    pub partition_id: u64,
    pub index_id: u64,
    
    // 版本管理
    pub version: u64,
    pub min_version: u64,
    pub max_version: u64,
    
    // 副本管理
    pub replicas: Vec<Replica>,
    pub replication_num: usize,  // 副本数配置
    
    // 状态管理
    pub state: TabletState,
    pub enabled: bool,
    
    // 统计信息
    pub data_size: u64,
    pub row_count: u64,
    
    // 元数据
    pub create_time: u64,
    pub last_update_time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TabletState {
    Normal,      // 正常
    Altering,    // Schema Change中
    Cloning,     // 克隆中
    Dropping,    // 删除中
}

impl Tablet {
    pub fn new(
        tablet_id: u64,
        table_id: u64,
        partition_id: u64,
        index_id: u64,
        replication_num: usize,
    ) -> Self {
        Self {
            tablet_id,
            table_id,
            partition_id,
            index_id,
            version: 0,
            min_version: 0,
            max_version: 0,
            replicas: vec![],
            replication_num,
            state: TabletState::Normal,
            enabled: true,
            data_size: 0,
            row_count: 0,
            create_time: chrono::Utc::now().timestamp(),
            last_update_time: chrono::Utc::now().timestamp(),
        }
    }
    
    pub fn get_healthy_replicas(&self) -> Vec<&Replica> {
        self.replicas.iter()
            .filter(|r| r.is_healthy())
            .collect()
    }
    
    pub fn check_health(&self) -> TabletHealth {
        let healthy_count = self.get_healthy_replicas().len();
        
        if healthy_count >= self.replication_num {
            TabletHealth::Healthy
        } else if healthy_count > 0 {
            TabletHealth::UnderReplicated
        } else {
            TabletHealth::Unhealthy
        }
    }
    
    pub fn add_replica(&mut self, replica: Replica) {
        self.replicas.push(replica);
        self.last_update_time = chrono::Utc::now().timestamp();
    }
    
    pub fn remove_replica(&mut self, replica_id: u64) {
        self.replicas.retain(|r| r.replica_id != replica_id);
        self.last_update_time = chrono::Utc::now().timestamp();
    }
    
    pub fn update_version(&mut self, version: u64) {
        self.version = version;
        self.max_version = version.max(self.max_version);
        self.last_update_time = chrono::Utc::now().timestamp();
    }
}

#[derive(Debug, Clone)]
pub enum TabletHealth {
    Healthy,           // 健康（副本数充足）
    UnderReplicated,   // 副本不足
    Unhealthy,         // 不健康（副本缺失）
}
```

---

### 2. Replica管理

**组件设计:**

```rust
// fe-catalog/src/replica.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Replica {
    pub replica_id: u64,
    pub tablet_id: u64,
    pub backend_id: u64,  // BE节点ID
    
    // 版本管理
    pub version: u64,
    pub version_hash: u64,
    
    // 数据统计
    pub data_size: u64,
    pub row_count: u64,
    
    // 状态管理
    pub state: ReplicaState,
    pub last_failed_version: Option<u64>,
    
    // 心跳信息
    pub last_report_time: u64,
    
    // 压缩状态
    pub compaction: CompactionInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReplicaState {
    Normal,          // 正常
    CloneSource,     // 克隆源
    CloneTarget,     // 克隆目标
    Altering,        // Schema Change中
    Decommission,    // 下线中
    Recover,         // 恢复中
    Bad,             // 损坏
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionInfo {
    pub cumulative_compaction: u64,
    pub base_compaction: u64,
}

impl Replica {
    pub fn new(replica_id: u64, tablet_id: u64, backend_id: u64) -> Self {
        Self {
            replica_id,
            tablet_id,
            backend_id,
            version: 0,
            version_hash: 0,
            data_size: 0,
            row_count: 0,
            state: ReplicaState::Normal,
            last_failed_version: None,
            last_report_time: chrono::Utc::now().timestamp(),
            compaction: CompactionInfo {
                cumulative_compaction: 0,
                base_compaction: 0,
            },
        }
    }
    
    pub fn is_healthy(&self) -> bool {
        // 检查Replica健康状态
        self.state == ReplicaState::Normal
            && self.last_report_time > chrono::Utc::now().timestamp() - 300  // 5分钟内心跳
    }
    
    pub fn update_heartbeat(&mut self, report: ReplicaReport) {
        self.version = report.version;
        self.data_size = report.data_size;
        self.row_count = report.row_count;
        self.last_report_time = chrono::Utc::now().timestamp();
    }
    
    pub fn check_version_consistency(&self, tablet_version: u64) -> bool {
        self.version == tablet_version
    }
}

#[derive(Debug, Clone)]
pub struct ReplicaReport {
    pub replica_id: u64,
    pub version: u64,
    pub data_size: u64,
    pub row_count: u64,
    pub state: ReplicaState,
}
```

---

### 3. TabletManager（Actor模型）

**组件设计:**

```rust
// fe-catalog/src/tablet_actor.rs

use dashmap::DashMap;
use async_channel::{Sender, Receiver, bounded};

pub struct TabletManager {
    tablets: Arc<DashMap<u64, Tablet>>,
}

pub enum TabletCommand {
    CreateTablet {
        tablet: Tablet,
        response: Sender<Result<(), Error>>,
    },
    DropTablet {
        tablet_id: u64,
        response: Sender<Result<(), Error>>,
    },
    GetTablet {
        tablet_id: u64,
        response: Sender<Option<Tablet>>,
    },
    AddReplica {
        tablet_id: u64,
        replica: Replica,
        response: Sender<Result<(), Error>>,
    },
    RemoveReplica {
        tablet_id: u64,
        replica_id: u64,
        response: Sender<Result<(), Error>>,
    },
    CheckHealth {
        tablet_id: u64,
        response: Sender<TabletHealth>,
    },
    UpdateVersion {
        tablet_id: u64,
        version: u64,
        response: Sender<Result<(), Error>>,
    },
}

impl TabletManager {
    pub async fn run(&self, receiver: Receiver<TabletCommand>) {
        while let Ok(cmd) = receiver.recv().await {
            self.handle_command(cmd).await;
        }
    }
    
    async fn handle_command(&self, cmd: TabletCommand) {
        match cmd {
            TabletCommand::CreateTablet { tablet, response } => {
                self.tablets.insert(tablet.tablet_id, tablet);
                response.send(Ok(())).await.ok();
            }
            
            TabletCommand::DropTablet { tablet_id, response } => {
                self.tablets.remove(&tablet_id);
                response.send(Ok(())).await.ok();
            }
            
            TabletCommand::GetTablet { tablet_id, response } => {
                let tablet = self.tablets.get(&tablet_id).map(|r| r.clone());
                response.send(tablet).await.ok();
            }
            
            TabletCommand::AddReplica { tablet_id, replica, response } => {
                if let Some(mut tablet) = self.tablets.get_mut(&tablet_id) {
                    tablet.add_replica(replica);
                    response.send(Ok(())).await.ok();
                } else {
                    response.send(Err(Error::TabletNotFound(tablet_id))).await.ok();
                }
            }
            
            TabletCommand::CheckHealth { tablet_id, response } => {
                let health = self.tablets.get(&tablet_id)
                    .map(|t| t.check_health())
                    .unwrap_or(TabletHealth::Unhealthy);
                response.send(health).await.ok();
            }
        }
    }
}
```

---

### 4. ReplicaManager（Actor模型）

**组件设计:**

```rust
// fe-catalog/src/replica_actor.rs

pub struct ReplicaManager {
    replicas: Arc<DashMap<u64, Replica>>,
}

pub enum ReplicaCommand {
    CreateReplica {
        replica: Replica,
        response: Sender<Result<(), Error>>,
    },
    DropReplica {
        replica_id: u64,
        response: Sender<Result<(), Error>>,
    },
    GetReplica {
        replica_id: u64,
        response: Sender<Option<Replica>>,
    },
    UpdateHeartbeat {
        report: ReplicaReport,
        response: Sender<Result<(), Error>>,
    },
    CheckHealth {
        tablet_id: u64,
        response: Sender<Vec<ReplicaHealth>>,
    },
}

impl ReplicaManager {
    pub async fn run(&self, receiver: Receiver<ReplicaCommand>) {
        while let Ok(cmd) = receiver.recv().await {
            self.handle_command(cmd).await;
        }
    }
    
    async fn handle_command(&self, cmd: ReplicaCommand) {
        match cmd {
            ReplicaCommand::UpdateHeartbeat { report, response } => {
                if let Some(mut replica) = self.replicas.get_mut(&report.replica_id) {
                    replica.update_heartbeat(report);
                    response.send(Ok(())).await.ok();
                } else {
                    response.send(Err(Error::ReplicaNotFound(report.replica_id))).await.ok();
                }
            }
            
            ReplicaCommand::CheckHealth { tablet_id, response } => {
                let health = self.replicas.iter()
                    .filter(|r| r.tablet_id == tablet_id)
                    .map(|r| ReplicaHealth {
                        replica_id: r.replica_id,
                        is_healthy: r.is_healthy(),
                    })
                    .collect();
                response.send(health).await.ok();
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReplicaHealth {
    pub replica_id: u64,
    pub is_healthy: bool,
}
```

---

### 5. 副本分布策略

**副本分配策略:**

```rust
// fe-catalog/src/replica_allocator.rs

pub struct ReplicaAllocator {
    backends: Arc<DashMap<u64, BackendInfo>>,
}

impl ReplicaAllocator {
    pub fn allocate_replicas(&self, tablet_id: u64, replication_num: usize) -> Result<Vec<u64>, Error> {
        // 副本分布策略：
        // 1. 不同BE节点（容错）
        // 2. 不同机架（可选）
        // 3. 负载均衡
        
        let available_backends = self.get_available_backends();
        
        if available_backends.len() < replication_num {
            return Err(Error::NotEnoughBackends);
        }
        
        // 选择负载最低的Backend
        let selected_backends = available_backends.iter()
            .sorted_by_key(|b| b.load_score())
            .take(replication_num)
            .map(|b| b.backend_id)
            .collect();
        
        Ok(selected_backends)
    }
    
    fn get_available_backends(&self) -> Vec<BackendInfo> {
        self.backends.iter()
            .filter(|b| b.is_alive())
            .map(|r| r.clone())
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct BackendInfo {
    pub backend_id: u64,
    pub host: String,
    pub port: u16,
    pub alive: bool,
    pub load_score: u64,
}

impl BackendInfo {
    pub fn is_alive(&self) -> bool {
        self.alive
    }
    
    pub fn load_score(&self) -> u64 {
        self.load_score
    }
}
```

---

## 📅 实施路线（1个月）

### Week 1-2: Tablet/Replica定义

- [ ] Tablet结构定义
- [ ] Replica结构定义
- [ ] TabletState/ReplicaState定义
- [ ] 序列化/反序列化
- [ ] 单元测试

**验收标准:**
```
- Tablet/Replica定义完整
- 序列化正确
- 单元测试通过
```

---

### Week 3-4: Actor实现 + 副本分布

- [ ] TabletManager Actor实现
- [ ] ReplicaManager Actor实现
- [ ] ReplicaAllocator实现
- [ ] 副本分布策略
- [ ] 集成测试

**验收标准:**
```
- Actor运行稳定
- 副本分布正确
- 副本配置生效
- 集成测试通过
```

---

## 📊 功能对比

| 功能 | Doris | HarnessDB | 完成度 |
|------|-------|---------|--------|
| **Tablet管理** | ✅ 完整 | ✅ Actor实现 | 100% |
| **Replica管理** | ✅ 完整 | ✅ Actor实现 | 100% |
| **副本配置** | ✅ 支持 | ✅ 支持 | 100% |
| **副本分布** | ✅ 自动 | ✅ 自动分配 | 100% |
| **健康检查** | ✅ TabletChecker | ⚠️ 需补充（P1-02） | 50% |
| **副本均衡** | ✅ Rebalancer | ⚠️ 需补充（P1-02） | 50% |

---

## 📁 涉及文件

### 新建文件

```
fe-catalog/src/
├── tablet.rs                  # Tablet定义（~250行）
├── replica.rs                 # Replica定义（~250行）
├── tablet_actor.rs            # Tablet Actor（~300行）
├── replica_actor.rs           # Replica Actor（~300行）
├── replica_allocator.rs       # 副本分配（~200行）
└── backend_info.rs            # Backend信息（~150行）

tests/integration/
└── tablet_replica_test.rs     # Tablet/Replica测试（~400行）
```

### 修改文件

```
fe-catalog/src/lib.rs          # 导出tablet模块
fe-catalog/src/table.rs        # OlapTable集成Tablet
fe-catalog/src/catalog.rs      # Catalog集成TabletManager
```

---

## 💡 创新价值

**这是分布式存储的基础：**

1. ✅ **Tablet管理完整**：创建/删除/状态/健康检查
2. ✅ **Replica管理完整**：创建/删除/心跳/健康检查
3. ✅ **Actor模型**：无锁并发，高吞吐
4. ✅ **副本自动分配**：负载均衡，容错分布
5. ✅ **DashMap存储**：分段无锁，高并发

**Tablet/Replica是HarnessDB分布式存储的核心！**

---

## 🔗 相关文档

- [创新路线总览](00-overview.md)
- [P0-02 无锁并发](P0-lock-free-concurrency.md)
- [P1-02 Fragment调度](P1-fragment-scheduling.md)

---

## 📝 备注

**为什么Tablet/Replica是P1？**

1. ✅ 分布式存储基础（必须实现）
2. ✅ Actor模型创新（无锁并发）
3. ✅ 副本管理完整（容错和均衡）
4. ✅ 依赖P0-02（无锁并发）
5. ✅ 为P1-02打基础（Fragment调度）

**P1-01是HarnessDB分布式存储的起点！**