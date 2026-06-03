# P1-03: FE高可用（Raft共识）

**优先级**: P1
**模块**: fe-common, fe-catalog
**状态**: ❌ 未开始
**预计工期**: 2个月
**价值**: ✅ 中（高可用基础）

---

## 📋 问题分析

### Doris的FE高可用（BDBJE）

```java
// Doris: 基于BDBJE的FE高可用
public class EditLog {
    private BDBJEJournal journal;  // 分布式日志
    
    public void logEdit(Edit edit) {
        // 写入分布式日志（Quorum确认）
        journal.logEdit(edit);
        // 等待majority确认
        waitForQuorum();
    }
}

// Doris高可用：
// 1. BDBJE分布式日志（Berkeley DB Java Edition）
// 2. Master/Follower/Observer角色
// 3. Leader自动选举（BDBJE内置）
// 4. Quorum写确认（majority）
// 5. Fencing epoch（防脑裂）
```

### HarnessDB的创新选择（Raft）

```
为什么选择Raft而不是BDBJE？
  1. ✅ Raft更简单（易于理解实现）
  2. ✅ Raft生态成熟（raft-rs/openraft）
  3. ✅ Raft性能更好（现代实现）
  4. ✅ Raft社区活跃（持续维护）
  5. ✅ Raft无Java依赖（纯Rust）
  
Raft优势：
  - Leader选举明确（算法简单）
  - 日志复制清晰（易于调试）
  - 性能优化（批量提交）
  - Rust原生（无缝集成）
```

---

## 🎯 核心组件设计

### 1. Raft集成（openraft）

**为什么选择openraft？**
```
openraft优势：
  1. ✅ 纯Rust实现（无需外部依赖）
  2. ✅ 灵活存储接口（适配HarnessDB）
  3. ✅ 异步友好（tokio集成）
  4. ✅ 性能优化（批量提交、心跳优化）
  5. ✅ 社区活跃（持续维护）
```

**组件设计:**

```rust
// fe-common/src/raft.rs

use openraft::{Raft, Config, Store, LogStore, StateMachine};

pub struct HarnessRaft {
    raft: Raft<RaftConfig, HarnessStore>,
}

pub struct RaftConfig {
    cluster_id: String,
    node_id: u64,
    peers: Vec<u64>,
}

impl HarnessRaft {
    pub async fn new(config: RaftConfig) -> Result<Self, Error> {
        // 创建Raft节点
        let store = HarnessStore::new();
        
        let raft_config = Config {
            heartbeat_interval: 500,  // 500ms心跳
            election_timeout_min: 1500,  // 1.5s选举超时
            election_timeout_max: 3000,  // 3s选举超时
            snapshot_threshold: 10000,  // 1万条日志触发快照
        };
        
        let raft = Raft::new(
            config.node_id,
            raft_config,
            store,
            store,
        );
        
        Ok(Self { raft })
    }
    
    pub async fn start(&self) -> Result<(), Error> {
        // 启动Raft节点
        
        // 1. 初始化集群
        if self.is_first_node() {
            self.initialize_cluster().await?;
        } else {
            self.join_cluster().await?;
        }
        
        // 2. 启动Raft主循环
        self.raft.run().await?;
        
        Ok(())
    }
    
    async fn initialize_cluster(&self) -> Result<(), Error> {
        // 初始化集群（第一个节点）
        
        let mut members = HashMap::new();
        members.insert(self.config.node_id, RaftNode {
            node_id: self.config.node_id,
            addr: self.config.addr.clone(),
        });
        
        self.raft.initialize(members).await?;
        
        Ok(())
    }
    
    async fn join_cluster(&self) -> Result<(), Error> {
        // 加入现有集群
        
        // 1. 连接到Leader
        let leader_client = self.connect_to_leader()?;
        
        // 2. 发送加入请求
        leader_client.add_node(self.config.node_id, self.config.addr.clone()).await?;
        
        Ok(())
    }
    
    pub async fn propose(&self, edit: Edit) -> Result<(), Error> {
        // 提交Edit到Raft（等待Quorum确认）
        
        let proposal = RaftProposal {
            edit_type: edit.edit_type,
            data: edit.serialize(),
        };
        
        // 提交到Raft（等待majority确认）
        self.raft.client_write(proposal).await?;
        
        Ok(())
    }
    
    pub async fn is_leader(&self) -> bool {
        // 检查是否是Leader
        self.raft.is_leader().await
    }
    
    pub async fn get_leader(&self) -> Option<u64> {
        // 获取Leader节点ID
        self.raft.current_leader().await
    }
}

// Raft存储实现
pub struct HarnessStore {
    log_store: LogStore,
    state_machine: StateMachine,
}

impl Store for HarnessStore {
    async fn save_log(&self, log: RaftLog) -> Result<(), Error> {
        // 保存日志到存储
        self.log_store.save(log).await?;
        Ok(())
    }
    
    async fn load_log(&self, index: u64) -> Result<RaftLog, Error> {
        // 加载日志
        self.log_store.load(index).await
    }
    
    async fn apply(&self, log: RaftLog) -> Result<(), Error> {
        // 应用日志到状态机（执行Edit）
        
        let edit = Edit::deserialize(log.data);
        
        self.state_machine.apply(edit)?;
        
        Ok(())
    }
}

impl LogStore for HarnessStore {
    async fn save(&self, log: RaftLog) -> Result<(), Error> {
        // 持久化日志
        let path = format!("logs/{}.log", log.index);
        let data = log.serialize();
        
        tokio::fs::write(path, data).await?;
        
        Ok(())
    }
    
    async fn load(&self, index: u64) -> Result<RaftLog, Error> {
        // 加载日志
        let path = format!("logs/{}.log", index);
        let data = tokio::fs::read(path).await?;
        
        RaftLog::deserialize(data)
    }
}

impl StateMachine for HarnessStore {
    async fn apply(&self, edit: Edit) -> Result<(), Error> {
        // 应用Edit到Catalog
        
        match edit.edit_type {
            EditType::CreateDatabase => {
                self.catalog.create_database(edit.data)?;
            }
            EditType::CreateTable => {
                self.catalog.create_table(edit.data)?;
            }
            EditType::DropDatabase => {
                self.catalog.drop_database(edit.data)?;
            }
            EditType::DropTable => {
                self.catalog.drop_table(edit.data)?;
            }
        }
        
        Ok(())
    }
    
    async fn snapshot(&self) -> Result<Vec<u8>, Error> {
        // 创建快照（定期）
        
        let catalog_snapshot = self.catalog.snapshot()?;
        
        Ok(catalog_snapshot.serialize())
    }
    
    async fn restore(&self, snapshot: Vec<u8>) -> Result<(), Error> {
        // 恢复快照（新节点同步）
        
        let catalog_snapshot = CatalogSnapshot::deserialize(snapshot);
        
        self.catalog.restore(catalog_snapshot)?;
        
        Ok(())
    }
}
```

---

### 2. EditLog集成Raft

**EditLog设计:**

```rust
// fe-common/src/edit_log.rs

pub struct EditLog {
    raft: Arc<HarnessRaft>,
    pending_edits: Arc<Mutex<Vec<Edit>>>,
    batch_size: usize,
}

impl EditLog {
    pub async fn append(&self, edit: Edit) -> Result<(), Error> {
        // 批量提交Edit（性能优化）
        
        let mut pending = self.pending_edits.lock();
        pending.push(edit);
        
        if pending.len() >= self.batch_size {
            // 批量提交
            self.flush_batch(&mut pending).await?;
        }
        
        Ok(())
    }
    
    async fn flush_batch(&self, pending: &mut Vec<Edit>) -> Result<(), Error> {
        // 批量提交到Raft
        
        if pending.is_empty() {
            return Ok(());
        }
        
        // 合并多个Edit为一个Proposal
        let batch_edit = Edit::Batch {
            edits: pending.clone(),
        };
        
        // 提交到Raft（等待Quorum）
        self.raft.propose(batch_edit).await?;
        
        // 清空pending
        pending.clear();
        
        Ok(())
    }
    
    pub async fn replay(&self) -> Result<(), Error> {
        // 回放日志（启动恢复）
        
        // 1. 加载快照
        let snapshot = self.raft.load_snapshot()?;
        self.catalog.restore(snapshot)?;
        
        // 2. 回放日志
        let logs = self.raft.load_logs()?;
        for log in logs {
            self.raft.apply(log)?;
        }
        
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum Edit {
    CreateDatabase { db: Database },
    CreateTable { table: Table },
    DropDatabase { name: String },
    DropTable { db: String, table: String },
    Batch { edits: Vec<Edit> },
}

impl Edit {
    fn serialize(&self) -> Vec<u8> {
        // 序列化Edit
        serde_json::to_vec(self).unwrap()
    }
    
    fn deserialize(data: Vec<u8>) -> Self {
        // 反序列化Edit
        serde_json::from_slice(&data).unwrap()
    }
}
```

---

### 3. FE节点管理

**节点管理设计:**

```rust
// fe-common/src/fe_node.rs

pub struct FeNode {
    node_id: u64,
    role: FeRole,
    addr: String,
    raft: Arc<HarnessRaft>,
}

pub enum FeRole {
    Leader,      // 主节点（可写）
    Follower,    // 从节点（只读）
    Observer,    // 观察者（只读，不参与投票）
}

impl FeNode {
    pub async fn start(&self) -> Result<(), Error> {
        // 启动FE节点
        
        // 1. 启动Raft
        self.raft.start().await?;
        
        // 2. 等待角色确定
        self.wait_for_role().await?;
        
        // 3. 启动服务
        self.start_services().await?;
        
        Ok(())
    }
    
    async fn wait_for_role(&self) -> Result<(), Error> {
        // 等待角色确定（Leader选举）
        
        while !self.raft.is_leader().await {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        
        // 如果是Leader，设置为Leader角色
        self.role = FeRole::Leader;
        
        Ok(())
    }
    
    pub async fn handle_write(&self, edit: Edit) -> Result<(), Error> {
        // 处理写请求（仅Leader）
        
        if self.role != FeRole::Leader {
            return Err(Error::NotLeader);
        }
        
        // 提交到EditLog
        self.edit_log.append(edit).await?;
        
        Ok(())
    }
    
    pub async fn handle_read(&self, query: Query) -> Result<QueryResult, Error> {
        // 处理读请求（所有节点）
        
        // 直接读取Catalog（线性一致性读）
        let catalog = self.catalog.read();
        
        // 执行查询
        self.execute_query(query, catalog)?;
        
        Ok(result)
    }
}

// FE集群管理
pub struct FeCluster {
    nodes: Arc<DashMap<u64, FeNode>>,
    leader_id: Arc<Mutex<Option<u64>>>,
}

impl FeCluster {
    pub async fn add_node(&self, node_id: u64, addr: String, role: FeRole) -> Result<(), Error> {
        // 添加FE节点
        
        let node = FeNode::new(node_id, role, addr);
        self.nodes.insert(node_id, node);
        
        // 如果是Follower，通知Leader
        if role == FeRole::Follower {
            self.notify_leader_add_node(node_id, addr).await?;
        }
        
        Ok(())
    }
    
    pub async fn remove_node(&self, node_id: u64) -> Result<(), Error> {
        // 移除FE节点
        
        self.nodes.remove(&node_id);
        
        // 通知Leader移除
        self.notify_leader_remove_node(node_id).await?;
        
        Ok(())
    }
    
    pub async fn get_leader(&self) -> Option<FeNode> {
        // 获取Leader节点
        
        let leader_id = self.leader_id.lock().clone();
        leader_id.map(|id| self.nodes.get(&id).unwrap().clone())
    }
    
    pub async fn switch_leader(&self) -> Result<(), Error> {
        // 切换Leader（手动切换）
        
        // 触发新的选举
        self.raft.trigger_election().await?;
        
        Ok(())
    }
}
```

---

### 4. Fencing防脑裂

**Fencing设计:**

```rust
// fe-common/src/fencing.rs

pub struct FencingEpoch {
    epoch: Arc<Mutex<u64>>,
}

impl FencingEpoch {
    pub fn increment(&self) -> u64 {
        // Epoch增加（Leader切换时）
        let mut epoch = self.epoch.lock();
        *epoch += 1;
        *epoch
    }
    
    pub fn validate(&self, epoch: u64) -> bool {
        // 验证Epoch（防止旧Leader）
        let current_epoch = self.epoch.lock();
        epoch >= *current_epoch
    }
    
    pub async fn fencing(&self, old_leader_id: u64) -> Result<(), Error> {
        // Fencing旧Leader（防止脑裂）
        
        // 1. 增加Epoch
        let new_epoch = self.increment();
        
        // 2. 通知旧Leader停止写入
        self.notify_old_leader(old_leader_id, new_epoch).await?;
        
        // 3. 通知所有Follower新Epoch
        self.broadcast_epoch(new_epoch).await?;
        
        Ok(())
    }
}
```

---

## 📅 实施路线（2个月）

### Month 1: Raft集成

**Week 1-2: Raft基础**
- [ ] openraft集成
- [ ] HarnessStore实现
- [ ] Raft启动流程
- [ ] 单元测试

**Week 3-4: EditLog集成**
- [ ] EditLog改造
- [ ] 批量提交优化
- [ ] 日志回放
- [ ] 测试验证

**验收标准:**
```
- Raft启动成功
- EditLog集成成功
- Quorum写确认正确
```

---

### Month 2: FE节点管理 + Fencing

**Week 1-2: FE节点管理**
- [ ] FeNode实现
- [ ] FeCluster管理
- [ ] Leader选举
- [ ] 节点动态添加/删除

**Week 3-4: Fencing + 测试**
- [ ] FencingEpoch实现
- [ ] 脑裂防护
- [ ] 集群测试
- [ ] 故障恢复测试

**验收标准:**
```
- FE集群可用
- Leader选举正常
- Fencing防脑裂
- 故障恢复正常
```

---

## 📊 功能对比

| 功能 | Doris（BDBJE） | HarnessDB（Raft） | 优势 |
|------|---------------|-----------------|------|
| **分布式日志** | ✅ BDBJE | ✅ Raft | Raft更简单 |
| **Leader选举** | ✅ BDBJE内置 | ✅ Raft算法 | Raft更清晰 |
| **Quorum写** | ✅ majority | ✅ majority | 相同 |
| **Fencing** | ✅ epoch | ✅ epoch | 相同 |
| **实现复杂度** | ❌ 高（BDBJE） | ✅ 低（Raft） | Raft优势 |
| **性能** | ⚠️ 中等 | ✅ 高（优化） | Raft优势 |
| **维护成本** | ❌ 高（Java依赖） | ✅ 低（纯Rust） | Raft优势 |

---

## 📁 涉及文件

### 新建文件

```
fe-common/src/
├── raft.rs                    # Raft集成（~400行）
├── raft_store.rs              # Raft存储（~300行）
├── edit_log.rs                # EditLog改造（~200行）
├── fe_node.rs                 # FE节点（~250行）
├── fe_cluster.rs              # FE集群（~200行）
└── fencing.rs                 # Fencing防脑裂（~150行）

tests/integration/
└── raft_cluster_test.rs       # Raft集群测试（~400行）
```

### 修改文件

```
fe-common/src/lib.rs           # 导出raft模块
Cargo.toml                      # 添加openraft依赖
```

---

## 💡 创新价值

**这是高可用的基础：**

1. ✅ **Raft简化实现**：算法清晰，易于调试
2. ✅ **纯Rust实现**：无Java依赖，无缝集成
3. ✅ **性能优化**：批量提交，心跳优化
4. ✅ **社区活跃**：持续维护，技术领先
5. ✅ **异步友好**：tokio集成，异步执行

**FE高可用是HarnessDB稳定性的保障！**

---

## 🔗 相关文档

- [创新路线总览](00-overview.md)
- [openraft官方文档](https://github.com/datafuselabs/openraft)

---

## 📝 备注

**为什么选择Raft而不是BDBJE？**

1. ✅ Raft算法简单（易于理解和实现）
2. ✅ Raft生态成熟（raft-rs/openraft）
3. ✅ Raft性能更好（现代实现）
4. ✅ Raft社区活跃（持续维护）
5. ✅ Raft无Java依赖（纯Rust）

**P1-03是HarnessDB高可用的基石！**