# P1-05: Runtime Filter（运行时过滤）

**优先级**: P1
**模块**: fe-sql-planner, fe-scheduler
**状态**: ❌ 未开始
**预计工期**: 1个月
**价值**: ✅ 中（Join优化）

---

## 📋 问题分析

### Doris的Runtime Filter

```java
// Doris: Runtime Filter Join优化
public class RuntimeFilter {
    private RuntimeFilterId rfId;
    private PlanNodeId buildSideNodeId;   // Hash Join Build侧
    private PlanNodeId probeSideNodeId;   // Hash Join Probe侧
    
    // Build侧生成Runtime Filter
    public void buildRuntimeFilter(HashTable hashTable) {
        // 从Hash Table提取Min/Max、BloomFilter等
        BloomFilter bf = hashTable.extractBloomFilter();
        MinMax mm = hashTable.extractMinMax();
        
        // 发送到Probe侧
        sendToProbeSide(bf, mm);
    }
    
    // Probe侧应用Runtime Filter
    public void applyRuntimeFilter(Block inputBlock) {
        // 使用Runtime Filter过滤输入数据
        BloomFilter bf = this.getRuntimeFilter();
        Block filtered = inputBlock.filter(bf);
        
        return filtered;
    }
}

// Doris Runtime Filter类型：
// 1. IN Filter（值列表）
// 2. BloomFilter（概率过滤）
// 3. MinMax Filter（范围过滤）
```

### HarnessDB的缺失

```
当前缺失：
  ❌ Runtime Filter未实现
  ❌ Build侧生成RF未实现
  ❌ Probe侧应用RF未实现
  ❌ RF传递机制未实现
  ❌ RF类型未实现
  
影响：
  - Join性能差（Probe侧数据量大）
  - 无法提前过滤数据
  - Hash Join开销大
```

---

## 🎯 核心组件设计

### 1. Runtime Filter类型

**RF类型设计:**

```rust
// fe-sql-planner/src/runtime_filter.rs

#[derive(Debug, Clone)]
pub enum RuntimeFilterType {
    In { values: Vec<ScalarValue> },         // IN Filter（小集合）
    BloomFilter { bf: BloomFilterData },     // BloomFilter（概率过滤）
    MinMax { min: ScalarValue, max: ScalarValue },  // MinMax（范围过滤）
}

#[derive(Debug, Clone)]
pub struct RuntimeFilter {
    rf_id: u64,
    rf_type: RuntimeFilterType,
    
    // Build/Probe节点
    build_node_id: u64,  // Hash Join Build侧
    probe_node_id: u64,  // Hash Join Probe侧
    
    // Join条件
    join_column: String,  // Join列名
}

impl RuntimeFilter {
    pub fn new(rf_id: u64, build_node_id: u64, probe_node_id: u64) -> Self {
        Self {
            rf_id,
            rf_type: RuntimeFilterType::In { values: vec![] },  // 默认IN
            build_node_id,
            probe_node_id,
            join_column: "",
        }
    }
    
    pub fn filter(&self, value: ScalarValue) -> bool {
        // 应用Runtime Filter过滤值
        
        match &self.rf_type {
            RuntimeFilterType::In { values } => {
                // IN Filter：检查值是否在列表中
                values.contains(&value)
            }
            RuntimeFilterType::BloomFilter { bf } => {
                // BloomFilter：检查值是否可能存在
                bf.may_contain(&value)
            }
            RuntimeFilterType::MinMax { min, max } => {
                // MinMax：检查值是否在范围内
                value >= *min && value <= *max
            }
        }
    }
    
    pub fn filter_block(&self, block: Block) -> Result<Block, Error> {
        // 应用Runtime Filter过滤Block
        
        let column = block.get_column_by_name(&self.join_column)?;
        
        // 根据RF类型过滤
        let mask = match &self.rf_type {
            RuntimeFilterType::In { values } => {
                // IN Filter过滤
                column.filter_in(values)
            }
            RuntimeFilterType::BloomFilter { bf } => {
                // BloomFilter过滤
                column.filter_bloom(bf)
            }
            RuntimeFilterType::MinMax { min, max } => {
                // MinMax过滤
                column.filter_range(min, max)
            }
        };
        
        // 应用mask过滤Block
        block.apply_mask(mask)
    }
}
```

---

### 2. Build侧生成Runtime Filter

**Build侧生成逻辑:**

```rust
// be-execution/src/hash_join_build.rs

pub struct HashJoinBuildOperator {
    hash_table: Arc<HashTable>,
    runtime_filter_manager: Arc<RuntimeFilterManager>,
}

impl HashJoinBuildOperator {
    pub async fn build(&self, input: Block) -> Result<(), Error> {
        // 构建Hash Table
        
        // 1. 插入数据到Hash Table
        self.hash_table.insert_block(input)?;
        
        // 2. 生成Runtime Filter（Build完成后）
        self.generate_runtime_filter()?;
        
        Ok(())
    }
    
    fn generate_runtime_filter(&self) -> Result<(), Error> {
        // 从Hash Table生成Runtime Filter
        
        let rf = RuntimeFilter::new(self.rf_id, self.build_node_id, self.probe_node_id);
        
        // 选择最佳RF类型
        let rf_type = self.select_rf_type()?;
        
        match rf_type {
            RuntimeFilterType::In => {
                // 生成IN Filter（值列表）
                let values = self.extract_in_values();
                rf.rf_type = RuntimeFilterType::In { values };
            }
            RuntimeFilterType::BloomFilter => {
                // 生成BloomFilter
                let bf = self.build_bloom_filter();
                rf.rf_type = RuntimeFilterType::BloomFilter { bf };
            }
            RuntimeFilterType::MinMax => {
                // 提取Min/Max
                let (min, max) = self.extract_min_max();
                rf.rf_type = RuntimeFilterType::MinMax { min, max };
            }
        }
        
        // 发送Runtime Filter到Probe侧
        self.send_runtime_filter(rf)?;
        
        Ok(())
    }
    
    fn select_rf_type(&self) -> Result<RuntimeFilterType, Error> {
        // 选择最佳RF类型
        
        let ndv = self.hash_table.ndv();  // Unique值数量
        
        if ndv < 100 {
            // 小集合：使用IN Filter
            RuntimeFilterType::In
        } else if ndv < 10000 {
            // 中集合：使用BloomFilter
            RuntimeFilterType::BloomFilter
        } else {
            // 大集合：使用MinMax
            RuntimeFilterType::MinMax
        }
    }
    
    fn extract_in_values(&self) -> Vec<ScalarValue> {
        // 提取IN值列表（小集合）
        
        self.hash_table.keys().iter()
            .take(100)  // 最多100个值
            .cloned()
            .collect()
    }
    
    fn build_bloom_filter(&self) -> BloomFilterData {
        // 构建BloomFilter
        
        let bf = BloomFilter::new(10000, 0.01);  // 1万容量，1%误判率
        
        for key in self.hash_table.keys() {
            bf.insert(&key);
        }
        
        BloomFilterData::from(bf)
    }
    
    fn extract_min_max(&self) -> (ScalarValue, ScalarValue) {
        // 提取Min/Max
        
        let min = self.hash_table.min_value();
        let max = self.hash_table.max_value();
        
        (min, max)
    }
    
    fn send_runtime_filter(&self, rf: RuntimeFilter) -> Result<(), Error> {
        // 发送Runtime Filter到Probe侧
        
        self.runtime_filter_manager.send(rf).await?;
        
        Ok(())
    }
}
```

---

### 3. Probe侧应用Runtime Filter

**Probe侧应用逻辑:**

```rust
// be-execution/src/hash_join_probe.rs

pub struct HashJoinProbeOperator {
    hash_table: Arc<HashTable>,
    runtime_filter: Option<Arc<RuntimeFilter>>,
}

impl HashJoinProbeOperator {
    pub async fn probe(&self, input: Block) -> Result<Block, Error> {
        // Probe Hash Join
        
        // 1. 应用Runtime Filter（提前过滤）
        let filtered_input = if let Some(rf) = &self.runtime_filter {
            rf.filter_block(input.clone())?
        } else {
            input.clone()
        };
        
        // 2. Probe Hash Table（仅处理过滤后的数据）
        let joined = self.hash_table.probe_block(filtered_input)?;
        
        Ok(joined)
    }
    
    pub async fn receive_runtime_filter(&mut self, rf: RuntimeFilter) -> Result<(), Error> {
        // 接收Runtime Filter
        
        self.runtime_filter = Some(Arc::new(rf));
        
        Ok(())
    }
}
```

---

### 4. Runtime Filter传递机制

**RF传递设计:**

```rust
// fe-scheduler/src/runtime_filter_manager.rs

pub struct RuntimeFilterManager {
    rf_registry: Arc<DashMap<u64, RuntimeFilter>>,
    rf_sender: async_channel::Sender<RuntimeFilter>,
    rf_receiver: async_channel::Receiver<RuntimeFilter>,
}

impl RuntimeFilterManager {
    pub async fn register(&self, rf: RuntimeFilter) -> Result<(), Error> {
        // 注册Runtime Filter
        
        self.rf_registry.insert(rf.rf_id, rf);
        
        Ok(())
    }
    
    pub async fn send(&self, rf: RuntimeFilter) -> Result<(), Error> {
        // 发送Runtime Filter（Build → Probe）
        
        self.rf_sender.send(rf).await?;
        
        Ok(())
    }
    
    pub async fn receive(&self) -> Result<RuntimeFilter, Error> {
        // 接收Runtime Filter
        
        let rf = self.rf_receiver.recv().await?;
        
        Ok(rf)
    }
    
    pub async fn assign_to_fragments(&self, fragments: Vec<Fragment>) -> Result<(), Error> {
        // 分配Runtime Filter到Fragment
        
        for fragment in fragments {
            // 识别Hash Join节点
            let hash_join_nodes = self.find_hash_join_nodes(&fragment);
            
            for join_node in hash_join_nodes {
                // 创建Runtime Filter
                let rf = RuntimeFilter::new(
                    self.next_rf_id(),
                    join_node.build_node_id,
                    join_node.probe_node_id,
                );
                
                // 注册RF
                self.register(rf)?;
                
                // 分配到Build侧
                self.assign_to_build_fragment(rf)?;
                
                // 分配到Probe侧
                self.assign_to_probe_fragment(rf)?;
            }
        }
        
        Ok(())
    }
    
    fn assign_to_build_fragment(&self, rf: RuntimeFilter) -> Result<(), Error> {
        // 分配RF到Build Fragment
        
        let build_fragment = self.find_fragment_by_node(rf.build_node_id)?;
        
        build_fragment.runtime_filters.push(rf.clone());
        
        Ok(())
    }
    
    fn assign_to_probe_fragment(&self, rf: RuntimeFilter) -> Result<(), Error> {
        // 分配RF到Probe Fragment
        
        let probe_fragment = self.find_fragment_by_node(rf.probe_node_id)?;
        
        probe_fragment.runtime_filters.push(rf.clone());
        
        Ok(())
    }
}
```

---

## 📅 实施路线（1个月）

### Week 1-2: Runtime Filter类型

- [ ] RuntimeFilter定义
- [ ] IN Filter实现
- [ ] BloomFilter实现
- [ ] MinMax Filter实现
- [ ] 单元测试

**验收标准:**
```
- RF类型正确
- Filter功能正确
- 性能合理
```

---

### Week 3-4: Build/Probe集成

- [ ] Build侧生成RF
- [ ] Probe侧应用RF
- [ ] RF传递机制
- [ ] Join性能测试
- [ ] 集成测试

**验收标准:**
```
- RF生成正确
- RF应用正确
- Join性能提升明显
```

---

## 📊 性能预期

| 场景 | 无RF | 有RF | 性能提升 |
|------|------|------|---------|
| **Hash Join** | 扫描全表 | 提前过滤 | 5-10倍 |
| **大表Join** | Probe大量数据 | Probe少量数据 | 明显改善 |
| **网络传输** | 传输全表 | 传输过滤表 | 减少50% |

---

## 📁 涉及文件

### 新建文件

```
fe-sql-planner/src/
├── runtime_filter.rs          # Runtime Filter定义（~250行）
└── runtime_filter_planner.rs  # RF规划（~200行）

fe-scheduler/src/
├── runtime_filter_manager.rs  # RF管理（~300行）

be-execution/src/
├── hash_join_build.rs         # Build侧生成RF（~250行）
├── hash_join_probe.rs         # Probe侧应用RF（~200行）
└── bloom_filter.rs            # BloomFilter实现（~150行）

tests/integration/
└── runtime_filter_test.rs     # RF测试（~300行）
```

### 修改文件

```
fe-sql-planner/src/lib.rs      # 导出RF模块
fe-scheduler/src/coordinator.rs # 集成RF管理
```

---

## 💡 创新价值

**这是Join优化的关键：**

1. ✅ **IN/BloomFilter/MinMax**：多种RF类型
2. ✅ **Build侧生成**：从Hash Table提取
3. ✅ **Probe侧应用**：提前过滤数据
4. ✅ **性能提升**：5-10倍（Hash Join）
5. ✅ **网络优化**：减少数据传输

**Runtime Filter是HarnessDB Join优化的重要组成部分！**

---

## 🔗 相关文档

- [创新路线总览](00-overview.md)
- [P1-02 Fragment调度](P1-fragment-scheduling.md)

---

## 📝 备注

**为什么Runtime Filter是P1？**

1. ✅ Join优化关键（Hash Join性能）
2. ✅ 依赖P1-02（Fragment调度）
3. ✅ 性能提升明显（5-10倍）
4. ✅ 标准优化功能（数据库必备）

**P1-05是HarnessDB Join优化的关键！**