# Java → Rust 1:1复刻指南

## 核心思想

**不是重设计，是逐模块逐函数复刻，但要适配Rust特性**

---

## 🔄 Java → Rust 转换规则表

### 1. 数据结构转换

| Java类型 | Rust类型 | 转换要点 |
|---------|---------|---------|
| `class` | `struct` | 字段相同，移除`private/public`，改用`pub` |
| `interface` | `trait` | 方法签名相同，添加`&self`参数 |
| `abstract class` | `trait` + `struct` | 用trait定义抽象方法，struct实现具体字段 |
| `enum` | `enum` | 构造体改struct，方法改impl |
| `List<T>` | `Vec<T>` | ArrayList → Vec, LinkedList → VecDeque |
| `Map<K,V>` | `HashMap<K,V>` 或 `DashMap<K,V>` | HashMap单线程，DashMap多线程 |
| `Set<T>` | `HashSet<T>` | HashSet无序，BTreeSet有序 |
| `Optional<T>` | `Option<T>` | of() → Some, empty() → None |
| `Object` | `Any` 或具体类型 | 尽量避免Any，用enum或trait object |

### 2. 并发模型转换 ⚠️ **最容易出bug**

| Java并发 | Rust并发 | 转换要点 |
|---------|---------|---------|
| `synchronized` | `Mutex<T>` 或 `RwLock<T>` | lock()返回MutexGuard，自动释放 |
| `volatile` | `AtomicXxx` | AtomicBool/Isize/Usize等 |
| `Thread` | `tokio::spawn` | 异步任务，非线程 |
| `ExecutorService` | `tokio::runtime` | 异步运行时，非线程池 |
| `CompletableFuture` | `async/await` | async fn返回Future，await等待 |
| `Future` | `Future trait` | poll()方法，异步执行 |
| `shared state` | `Arc<Mutex<T>>` | Arc共享所有权，Mutex保护状态 |
| `lock.wait()` | `tokio::sync::Notify` | 异步通知机制 |
| `lock.notify()` | `tokio::sync::Notify` | notify()唤醒等待 |

**⚠️ 陷阱**: Java线程池 → Rust async
- Java: `ExecutorService.submit(task)` 多线程执行
- Rust: `tokio::spawn(async_task)` 异步任务（单线程或多线程取决于runtime配置）

### 3. 错误处理转换

| Java异常 | Rust错误 | 转换要点 |
|---------|---------|---------|
| `throws Exception` | `Result<T, Error>` | 返回Result，不抛出 |
| `throw new XxxException` | `return Err(XxxError)` | 返回错误，不抛出 |
| `try-catch-finally` | `match result { Ok/Err }` | pattern matching |
| `RuntimeException` | `panic!` | 尽量避免，用Result |
| `NullPointerException` | `Option<T>` | 用Option表示可能为null |
| `IOException` | `std::io::Error` | 标准IO错误 |
| 自定义Exception | 自定义Error enum | thiserror crate |
| `catch(Exception e)` | `map_err(|e| ...)` | 错误转换 |

**⚠️ 陷阱**: Java异常 → Rust Result
- Java: 异常会自动传播到调用栈
- Rust: Result必须显式处理（`?`运算符传播）

### 4. 集合操作转换

| Java操作 | Rust操作 | 示例 |
|---------|---------|---------|
| `list.add(e)` | `vec.push(e)` | 添加元素 |
| `list.get(i)` | `vec[i]` 或 `vec.get(i)` | 索引访问（get返回Option） |
| `list.size()` | `vec.len()` | 长度 |
| `list.isEmpty()` | `vec.is_empty()` | 是否空 |
| `list.remove(i)` | `vec.remove(i)` | 移除索引i |
| `map.put(k,v)` | `map.insert(k,v)` | 插入 |
| `map.get(k)` | `map.get(k)` | 获取（返回Option） |
| `map.containsKey(k)` | `map.contains_key(k)` | 是否包含 |
| `map.remove(k)` | `map.remove(k)` | 移除（返回Option） |
| `map.keySet()` | `map.keys()` | 键集合 |
| `map.values()` | `map.values()` | 值集合 |
| `stream().filter()` | `iter().filter()` | 过滤 |
| `stream().map()` | `iter().map()` | 映射 |
| `stream().collect()` | `collect()` | 收集 |
| `Collections.sort()` | `vec.sort()` | 排序 |
| `Collections.emptyList()` | `Vec::new()` | 空列表 |

### 5. 其他常见转换

| Java | Rust | 转换要点 |
|------|------|---------|
| `System.out.println` | `println!` | format!宏格式化 |
| `String.format` | `format!` | 宏格式化 |
| `StringBuilder` | `String::push_str` | 字符串拼接 |
| `new Object()` | `Object::new()` | 构造函数 |
| `this.field` | `self.field` | self引用 |
| `super.method()` | `BaseTrait::method(self)` | trait方法调用 |
| `static method` | `fn method()` 或关联函数 | 无self参数 |
| `final field` | `field: T` | Rust默认不可变 |
| `lazy initialization` | `lazy_static!` 或 `OnceLock` | 懒加载 |
| `singleton` | `lazy_static!` 或 `OnceLock` | 单例模式 |
| `reflection` | 避免 | 用trait object或enum |

---

## 📋 模块复刻顺序（按P0/P1优先级）

### 第一阶段：P0核心（必须完成）

#### 1. Catalog核心 - Tablet/Replica管理 ⚠️ **最高优先级**

**Java类 → Rust struct映射:**

```
Tablet.java → tablet.rs
  ├── TabletId tablet_id → tablet_id: u64
  ├── TableId table_id → table_id: u64  
  ├── PartitionId partition_id → partition_id: u64
  ├── long version → version: u64
  ├── long min_version → min_version: u64
  ├── Replica replicas → replicas: Vec<Replica>
  ├── TabletMeta tablet_meta → tablet_meta: TabletMeta
  ├── Object TabletInvertedIndex → inverted_index: Option<InvertedIndex>
  ├── Object delete_bitmap → delete_bitmap: Option<DeleteBitmap>
  ├── boolean enabled → enabled: bool
  ├── 列方法 → 列方法（一行一行复刻）
  ├── getReplica() → get_replica()
  ├── addReplica() → add_replica()
  ├── deleteReplica() → delete_replica()
  ├── getHealthyReplicas() → get_healthy_replicas()
  └── checkReplicaHealth() → check_replica_health()

Replica.java → replica.rs  
  ├── ReplicaId replica_id → replica_id: u64
  ├── TabletId tablet_id → tablet_id: u64
  ├── BackendId backend_id → backend_id: u64
  ├── long version → version: u64
  ├── long data_size → data_size: u64
  ├── long row_count → row_count: u64
  ├── ReplicaStatus status → status: ReplicaStatus
  ├── 列方法 → 列方法
  ├── getBackendId() → get_backend_id()
  ├── setVersion() → set_version()
  ├── checkHealth() → check_health()
  └── ～

OlapTable.java → table.rs（扩展现有）
  ├── 新增字段：
  ├── Index indexes → indexes: Vec<Index>（Rollup索引）
  ├── Partition partitions → partitions: HashMap<u64, Partition>
  ├── Map<Long, Tablet> tablets → tablets: HashMap<u64, Tablet>
  ├── Map<String, Column> nameToColumn → name_to_column: HashMap<String, Column>
  ├── DistributionInfo distribution_info → distribution_info: DistributionInfo
  ├── 列方法（Doris 4011行，逐个复刻）
  ├── getPartition() → get_partition()
  ├── addPartition() → add_partition()
  ├── dropPartition() → drop_partition()
  ├── getTablet() → get_tablet()
  ├── addTablet() → add_tablet()
  ├── dropTablet() → drop_tablet()
  ├── getIndex() → get_index()
  ├── getBaseIndex() → get_base_index()
  ├── checkPartitionHealth() → check_partition_health()
  ├── tablets的get/add/drop/check系列方法
  └── ～
```

**复刻检查清单（每个函数）:**
```
✅ 字段定义完整（1:1对应）
✅ 方法签名对应（参数类型、返回类型）
✅ 方法逻辑复刻（逐行对比）
✅ 并发安全（synchronized → Mutex）
✅ 错误处理（Exception → Result）
✅ 测试验证（相同输入，相同输出）
```

#### 2. Planner核心 - Fragment划分和调度

**Java类 → Rust struct映射:**

```
PlanFragment.java → fragment.rs（新建）
  ├── PlanFragmentId fragment_id → fragment_id: u64
  ├── PlanNode root → root: PlanNode
  ├── List<PlanFragment> children → children: Vec<PlanFragment>
  ├── List<FragmentInstance> instances → instances: Vec<FragmentInstance>
  ├── RuntimeFilter runtime_filters → runtime_filters: HashMap<u64, RuntimeFilter>
  ├── ExchangeNode input_exchange → input_exchange: Option<ExchangeNode>
  ├── ExchangeNode output_exchange → output_exchange: Option<ExchangeNode>
  ├── 列方法：
  ├── toThrift() → to_thrift()（1:1复刻序列化）
  ├── finalize() → finalize()
  ├── computeInstanceExecParams() → compute_instance_exec_params()
  ├── getOutputExchange() → get_output_exchange()
  └── ～

FragmentInstance.java → fragment_instance.rs（新建）
  ├── InstanceId instance_id → instance_id: u64
  ├── PlanFragment fragment → fragment: PlanFragment
  ├── BackendId backend_id → backend_id: u64
  ├── List<ScanRange> scan_ranges → scan_ranges: Vec<ScanRange>
  ├── 列方法：
  ├── toThrift() → to_thrift()
  ├── getBackendId() → get_backend_id()
  ├── getScanRanges() → get_scan_ranges()
  └ ～

Coordinator.java → coordinator.rs（扩展）
  ├── 新增字段：
  ├── Map<Long, PlanFragment> fragments → fragments: HashMap<u64, PlanFragment>
  ├── Map<Long, FragmentInstance> instances → instances: HashMap<u64, FragmentInstance>
  ├── Map<Long, Backend> backends → backends: HashMap<u64, Backend>
  ├── Map<Long, ScanRange> scan_ranges → scan_ranges: HashMap<u64, ScanRange>
  ├── Object profile → profile: QueryProfile
  ├── List<RuntimeFilter> runtime_filters → runtime_filters: Vec<RuntimeFilter>
  ├── CountDownLatch latch → latch: Arc<Notify>（异步等待）
  ├── 新增方法（Doris 3545行，逐个复刻）：
  ├── computeFragmentExecParams() → compute_fragment_exec_params()
  ├── computeScanRangeAssignment() → compute_scan_range_assignment()
  ├── sendFragment() → send_fragment()（RPC调用）
  ├── assignRuntimeFilters() → assign_runtime_filters()
  ├── collectResults() → collect_results()
  ├── waitForCompletion() → wait_for_completion()（异步等待）
  ├── 失败处理 → 失败处理
  └── ～

ScanRange.java → scan_range.rs（新建）
  ├── long partition_id → partition_id: u64
  ├── long tablet_id → tablet_id: u64
  ├── long version → version: u64
  ├── 列方法：
  ├── toThrift() → to_thrift()
  └ ～

ExchangeNode.java → plan_node.rs（扩展）
  ├── 新增ExchangeNode类型
  ├── ExchangeType exchange_type → exchange_type: ExchangeType（Broadcast/HashPartition/Gather）
  ├── PlanFragmentId dest_fragment_id → dest_fragment_id: u64
  ├── List<FragmentInstance> dest_instances → dest_instances: Vec<FragmentInstance>
  ├── 列方法：
  ├── toThrift() → to_thrift()
  ├── getDestInstances() → get_dest_instances()
  └ ～

RuntimeFilter.java → runtime_filter.rs（新建）
  ├── RuntimeFilterId rf_id → rf_id: u64
  ├── PlanNodeId src_node_id → src_node_id: u64
  ├── PlanNodeId target_node_id → target_node_id: u64
  ├── RuntimeFilterType rf_type → rf_type: RuntimeFilterType（IN/BloomFilter/MinMax）
  ├── 列方法：
  ├── toThrift() → to_thrift()
  ├── registerTargetFragment() → register_target_fragment()
  └ ～

```

#### 3. FE高可用 - Raft共识

**Java类 → Rust struct映射:**

```
EditLog.java → edit_log.rs（扩展）
  ├── 新增Raft相关：
  ├── RaftNode raft_node → raft_node: Option<RaftNode>（raft-rs集成）
  ├── long term → term: u64
  ├── long index → index: u64
  ├── String voted_for → voted_for: Option<String>
  ├── boolean is_leader → is_leader: bool
  ├── 新增方法：
  ├── appendRaftLog() → append_raft_log()（Raft写入）
  ├── waitForQuorum() → wait_for_quorum()（等待多数派确认）
  ├── becomeLeader() → become_leader()（Leader选举）
  ├── stepDown() → step_down()（Leader下台）
  └ ～

HAProtocol.java → ha.rs（新建）
  ├── trait HAProtocol（接口）
  ├── fencing() → fencing()
  ├── getLeader() → get_leader()
  ├── getElectableNodes() → get_electable_nodes()
  ├── getObserverNodes() → get_observer_nodes()
  └── removeElectableNode() → remove_electable_node()

BDBHA.java → bdb_ha.rs（新建，改用raft-rs）
  ├── RaftNode raft_node → raft_node: RaftNode
  ├── Map<Long, Backend> nodes → nodes: HashMap<u64, Backend>
  ├── Epoch epoch → epoch: u64
  ├── impl HAProtocol trait
  ├── fencing() → fencing()
  ├── getLeader() → get_leader()
  ├── getElectableNodes() → get_electable_nodes()
  └ ～

Backend.java → backend.rs（新建）
  ├── BackendId backend_id → backend_id: u64
  ├── String host → host: String
  ├── int heartbeat_port → heartbeat_port: u16
  ├── int be_port → be_port: u16
  ├── BackendState state → state: BackendState（ Alive/Dead）
  ├── long epoch → epoch: u64
  ├── 列方法：
  ├── updateHeartbeat() → update_heartbeat()
  ├── checkAlive() → check_alive()
  └ ～

```

---

### 第二阶段：P1重要功能

#### 4. Parser - 深度语义分析

**Java类 → Rust struct映射:**

```
Analyzer.java → analyzer.rs（新建）
  ├── AnalyzerContext context → context: AnalyzerContext
  ├── List<ErrorMsg> errors → errors: Vec<ErrorMsg>
  ├── Catalog catalog → catalog: CatalogManager
  ├── 列方法（1:1复刻）：
  ├── analyze() → analyze()
  ├── analyzeQueryStmt() → analyze_query_stmt()
  ├── analyzeSelectStmt() → analyze_select_stmt()
  ├── analyzeExpr() → analyze_expr()
  ├── analyzeTableRef() → analyze_table_ref()
  ├── analyzeOrderBy() → analyze_order_by()
  ├── analyzeAggInfo() → analyze_agg_info()
  ├── analyzeWhereClause() → analyze_where_clause()
  ├── 类型检查逻辑 → 类型检查逻辑
  ├── 列引用解析 → 列引用解析
  ├── 表别名作用域 → 表别名作用域
  └ ～

Expr.java → expression.rs（扩展）
  ├── 新增字段：
  ├── Type type → type: DataType
  ├── boolean is_constant → is_constant: bool
  ├── boolean is_nullable → is_nullable: bool
  ├── SlotRef slot_ref → slot_ref: Option<SlotRef>
  ├── 新增方法：
  ├── analyze() → analyze()
  ├── getType() → get_type()
  ├── isConstant() → is_constant()
  ├── checkType() → check_type()
  └ ～

```

#### 5. Planner - Partition Pruning

**Java类 → Rust struct映射:**

```
PartitionPruner.java → partition_pruner.rs（新建）
  ├── OlapTable table → table: OlapTable
  ├── List<Expr> partition_exprs → partition_exprs: Vec<Expr>
  ├── Map<Long, Partition> partitions → partitions: HashMap<u64, Partition>
  ├── 列方法：
  ├── prune() → prune()
  ├── pruneRangePartition() → prune_range_partition()
  ├── pruneListPartition() → prune_list_partition()
  ├── getPartitionRange() → get_partition_range()
  └ ～

RangePartitionPruner.java → range_partition_pruner.rs
  ├── RangePartitionInfo partition_info → partition_info: RangePartitionInfo
  ├── List<Expr> conjuncts → conjuncts: Vec<Expr>
  ├── 列方法（1:1复刻算法）：
  ├── prune() → prune()
  ├── getRangePartitionIds() → get_range_partition_ids()
  ├── getRangeFromConjuncts() → get_range_from_conjuncts()
  └ ～

```

#### 6. Storage - Version Graph管理 ⚠️ **BE核心**

**Java类 → Rust struct映射:**

```
version_graph.cpp → version_graph.rs（新建）
  ├── struct VersionNode
  ├── long version → version: u64
  ├── long version_hash → version_hash: u64
  ├── List<VersionNode> next → next: Vec<VersionNode>
  ├── List<VersionNode> prev → prev: Vec<VersionNode>
  ├── 列方法（C++ 26325行，核心算法1:1复刻）：
  ├── addVersion() → add_version()
  ├── deleteVersion() → delete_version()
  ├── findVersionPath() → find_version_path()
  ├── checkVersionConsistency() → check_version_consistency()
  ├── ～（这是BE最重要的算法）

Rowset.java → rowset.rs（扩展）
  ├── 新增字段：
  ├── long version → version: u64
  ├── long version_hash → version_hash: u64
  ├── long rowset_id → rowset_id: u64
  ├── long data_size → data_size: u64
  ├── long num_rows → num_rows: u64
  ├── 列方法：
  ├── getVersion() → get_version()
  ├── getVersionPath() → get_version_path()（调用version_graph）
  └ ～

```

---

## 📝 Catalog模块详细复刻示例

### Table.java → table.rs 详细映射

**Java源码分析:**
```java
// Table.java (593行)
public class Table {
    private long id;
    private String name;
    private long dbId;
    private TableType type;
    private List<Column> baseSchema;
    private volatile TableState state;
    private long createTime;
    private Map<String, Column> nameToColumn;
    
    // synchronized方法
    public synchronized void addColumn(Column column) {
        nameToColumn.put(column.getName(), column);
        baseSchema.add(column);
    }
    
    public Column getColumn(String name) {
        return nameToColumn.get(name);
    }
    
    public synchronized void dropColumn(String name) {
        nameToColumn.remove(name);
        baseSchema.removeIf(c -> c.getName().equals(name));
    }
}
```

**Rust复刻方案:**
```rust
// table.rs
use std::sync::{Arc, RwLock};
use std::collections::HashMap;

pub struct Table {
    pub id: u64,
    pub name: String,
    pub db_id: u64,
    pub table_type: TableType,
    pub base_schema: Vec<Column>,
    pub state: RwLock<TableState>,  // volatile → RwLock
    pub create_time: u64,
    pub name_to_column: RwLock<HashMap<String, Column>>,  // synchronized → RwLock
}

impl Table {
    // synchronized方法 → RwLock::write()
    pub fn add_column(&self, column: Column) {
        let mut name_map = self.name_to_column.write().unwrap();
        let mut schema = self.base_schema.write().unwrap();
        name_map.insert(column.name.clone(), column.clone());
        schema.push(column);
    }
    
    pub fn get_column(&self, name: &str) -> Option<Column> {
        let name_map = self.name_to_column.read().unwrap();
        name_map.get(name).cloned()  // get返回Option
    }
    
    pub fn drop_column(&self, name: &str) -> Result<(), Error> {
        let mut name_map = self.name_to_column.write().unwrap();
        let mut schema = self.base_schema.write().unwrap();
        name_map.remove(name)
            .ok_or_else(|| Error::ColumnNotFound(name.to_string()))?;
        schema.retain(|c| c.name != name);
        Ok(())
    }
}
```

**复刻要点:**
```
✅ 字段完全对应（id/name/dbId → id/name/db_id）
✅ 类型转换（long → u64, String → String, List → Vec）
✅ synchronized → RwLock（读写锁）
✅ volatile → RwLock（内存可见性）
✅ Map.get() → HashMap.get()返回Option
✅ 方法签名对应（参数、返回）
✅ 错误处理（remove失败 → Result::Err）
✅ 集合操作（removeIf → retain）
```

### OlapTable.java → table.rs 扩展

**Java源码分析（4011行）:**
```java
// OlapTable.java
public class OlapTable extends Table {
    private Map<Long, Partition> idToPartition;
    private Map<String, Partition> nameToPartition;
    private Map<Long, Index> indexes;  // Rollup indexes
    private DistributionInfo distributionInfo;
    private KeysType keysType;
    private PartitionInfo partitionInfo;
    private List<Tablet> tablets;  // ⚠️ 核心字段
    
    // ⚠️ synchronized关键方法
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
    
    public synchronized void addTablet(Tablet tablet) {
        Partition partition = idToPartition.get(tablet.getPartitionId());
        if (partition != null) {
            Index index = partition.getIndex(tablet.getIndexId());
            if (index != null) {
                index.addTablet(tablet);
            }
        }
    }
    
    public synchronized void checkPartitionHealth() {
        for (Partition partition : idToPartition.values()) {
            partition.checkHealth();
        }
    }
}
```

**Rust复刻方案:**
```rust
// table.rs扩展
pub struct OlapTable {
    pub base: Table,  // 继承 → 组合
    
    // 新增字段（1:1对应）
    pub id_to_partition: RwLock<HashMap<u64, Partition>>,
    pub name_to_partition: RwLock<HashMap<String, Partition>>,
    pub indexes: RwLock<HashMap<u64, Index>>,  // Rollup indexes
    pub distribution_info: DistributionInfo,
    pub keys_type: KeysType,
    pub partition_info: PartitionInfo,
    pub tablets: RwLock<HashMap<u64, Tablet>>,  // ⚠️ 核心字段
}

impl OlapTable {
    // ⚠️ synchronized关键方法 → RwLock::read()
    pub fn get_tablet(&self, tablet_id: u64) -> Option<Tablet> {
        let partitions = self.id_to_partition.read().unwrap();
        for partition in partitions.values() {
            let indexes = partition.get_indexes();
            for index in indexes.values() {
                if let Some(tablet) = index.get_tablet(tablet_id) {
                    return Some(tablet);
                }
            }
        }
        None  // Java null → Rust None
    }
    
    // ⚠️ add方法 → RwLock::write()
    pub fn add_tablet(&self, tablet: Tablet) -> Result<(), Error> {
        let mut tablets = self.tablets.write().unwrap();
        let partitions = self.id_to_partition.read().unwrap();
        
        let partition = partitions.get(&tablet.partition_id)
            .ok_or_else(|| Error::PartitionNotFound(tablet.partition_id))?;
        
        let indexes = partition.get_indexes();
        let index = indexes.get(&tablet.index_id)
            .ok_or_else(|| Error::IndexNotFound(tablet.index_id))?;
        
        index.add_tablet(tablet.clone())?;
        tablets.insert(tablet.tablet_id, tablet);
        Ok(())
    }
    
    // ⚠️ health检查 → RwLock::read()
    pub fn check_partition_health(&self) -> Vec<PartitionHealthInfo> {
        let partitions = self.id_to_partition.read().unwrap();
        partitions.values()
            .map(|p| p.check_health())
            .collect()
    }
}
```

**复刻要点:**
```
✅ 继承 → 组合（pub base: Table）
✅ 字段完全对应（4011行所有字段）
✅ synchronized → RwLock
✅ 嵌套遍历 → 嵌套for循环（逻辑1:1）
✅ null → Option
✅ 错误处理（get失败 → Result::Err）
✅ 集合操作（values/iter → values().iter()）
```

---

## 🧪 复刻验证流程

### 每个函数的验证步骤:

```bash
# 1. 字段定义检查
✅ 所有Java字段 → Rust字段对应
✅ 类型转换正确（List → Vec, Map → HashMap, synchronized → RwLock）

# 2. 方法签名检查
✅ 参数类型对应
✅ 返回类型对应（null → Option, Exception → Result）
✅ self参数正确（&self或&mut self）

# 3. 方法逻辑检查（逐行对比）
✅ Java代码 → Rust代码一行一行对比
✅ 循环逻辑相同（for/while → for/while）
✅ 条件逻辑相同（if/else → if/else）
✅ 算法逻辑相同（排序/查找/计算）

# 4. 并发安全检查
✅ synchronized方法 → RwLock/Mutex保护
✅ volatile字段 → RwLock/Atomic保护
✅ lock.wait/notify → Notify异步等待

# 5. 错误处理检查
✅ throws Exception → Result<T, Error>
✅ throw Exception → return Err(Error)
✅ try-catch → match Ok/Err
✅ NullPointerException → Option处理

# 6. 测试验证
✅ 相同输入 → 相同输出
✅ 边界case测试
✅ 并发安全测试（多线程调用）
✅ 错误case测试（异常 → Result::Err）
```

---

## 🛠️ 复刻工具和技巧

### 1. Java代码阅读辅助

```bash
# 使用grep快速定位Java类和方法
grep -n "public.*getTablet" ~/code/doris/fe/fe-core/src/main/java/org/apache/doris/catalog/OlapTable.java

# 使用find查找相关类
find ~/code/doris -name "Tablet.java" | head -5

# 查看方法实现细节
grep -A 20 "public synchronized Tablet getTablet" ~/code/doris/fe/fe-core/src/main/java/org/apache/doris/catalog/OlapTable.java
```

### 2. Rust代码生成模板

```rust
// 标准结构体模板
pub struct XxxStruct {
    pub field1: Type1,
    pub field2: Type2,
    pub concurrent_field: RwLock<Type3>,  // synchronized字段
}

impl XxxStruct {
    // getter方法（&self）
    pub fn get_field(&self) -> Type {
        self.field.clone()
    }
    
    // setter方法（&mut self）
    pub fn set_field(&mut self, value: Type) {
        self.field = value;
    }
    
    // synchronized方法（&self + RwLock）
    pub fn concurrent_method(&self) -> Result<Type, Error> {
        let guard = self.concurrent_field.read().unwrap();
        // 业务逻辑
        Ok(result)
    }
    
    // synchronized写方法（&self + RwLock::write）
    pub fn concurrent_write_method(&self, value: Type) -> Result<(), Error> {
        let mut guard = self.concurrent_field.write().unwrap();
        // 写逻辑
        Ok(())
    }
}
```

### 3. 错误处理模板

```rust
// 自定义Error枚举（thiserror crate）
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CatalogError {
    #[error("Table not found: {0}")]
    TableNotFound(String),
    
    #[error("Partition not found: {0}")]
    PartitionNotFound(u64),
    
    #[error("Tablet not found: {0}")]
    TabletNotFound(u64),
    
    #[error("Column not found: {0}")]
    ColumnNotFound(String),
    
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
}

// Result类型别名
pub type CatalogResult<T> = Result<T, CatalogError>;
```

### 4. 并发安全模板

```rust
// RwLock使用模板
use std::sync::RwLock;

pub struct ConcurrentManager {
    pub data: RwLock<HashMap<u64, Data>>,
}

impl ConcurrentManager {
    // 读操作
    pub fn get_data(&self, id: u64) -> Option<Data> {
        let guard = self.data.read().unwrap();
        guard.get(&id).cloned()
    }
    
    // 写操作
    pub fn add_data(&self, id: u64, data: Data) -> Result<(), Error> {
        let mut guard = self.data.write().unwrap();
        guard.insert(id, data);
        Ok(())
    }
    
    // 遍历操作
    pub fn list_all(&self) -> Vec<Data> {
        let guard = self.data.read().unwrap();
        guard.values().cloned().collect()
    }
}

// Mutex使用模板（需要mut操作）
use std::sync::Mutex;

pub struct MutableManager {
    pub data: Mutex<Vec<Data>>,
}

impl MutableManager {
    pub fn push_data(&self, data: Data) {
        let mut guard = self.data.lock().unwrap();
        guard.push(data);
    }
}

// Arc共享所有权模板
use std::sync::Arc;

pub fn shared_manager() -> Arc<ConcurrentManager> {
    Arc::new(ConcurrentManager {
        data: RwLock::new(HashMap::new()),
    })
}
```

---

## ⚠️ 复刻常见陷阱和解决方案

### 陷阱1: synchronized → RwLock死锁

**问题:**
```rust
// ❌ 错误：嵌套lock导致死锁
pub fn nested_lock(&self) {
    let guard1 = self.field1.read().unwrap();
    let guard2 = self.field2.read().unwrap();  // ❌ 可能死锁
    // ...
}
```

**解决:**
```rust
// ✅ 正确：先获取所有guard，再使用
pub fn nested_lock(&self) {
    let guard1 = self.field1.read().unwrap();
    let value1 = guard1.get_data();  // 先取值
    drop(guard1);  // 显式释放
    
    let guard2 = self.field2.read().unwrap();
    // 使用value1和guard2
}
```

### 陷阱2: NullPointerException → Option unwrap panic

**问题:**
```rust
// ❌ 错误：unwrap导致panic
pub fn get_table(&self, name: &str) -> Table {
    self.tables.get(name).unwrap()  // ❌ 可能panic
}
```

**解决:**
```rust
// ✅ 正确：返回Option或Result
pub fn get_table(&self, name: &str) -> Option<Table> {
    self.tables.get(name).cloned()
}

// 或使用Result
pub fn get_table(&self, name: &str) -> Result<Table, Error> {
    self.tables.get(name)
        .cloned()
        .ok_or_else(|| Error::TableNotFound(name.to_string()))
}
```

### 陷阱3: Java线程池 → Rust async误用

**问题:**
```rust
// ❌ 错误：在async中使用std::sync::Mutex
pub async fn bad_async(&self) {
    let guard = self.std_mutex.lock().unwrap();  // ❌ std Mutex会阻塞async
    // ...
}
```

**解决:**
```rust
// ✅ 正确：使用tokio::sync::Mutex
use tokio::sync::Mutex;

pub struct AsyncManager {
    pub data: Mutex<Vec<Data>>,  // tokio Mutex
}

impl AsyncManager {
    pub async fn async_method(&self) {
        let guard = self.data.lock().await;  // ✅ async Mutex
        // ...
    }
}

// 或使用RwLock（异步）
use tokio::sync::RwLock;

pub async fn async_read(&self) {
    let guard = self.data.read().await;
    // ...
}
```

### 陷阱4: 集合removeIf → Vec.remove错误

**问题:**
```java
// Java
baseSchema.removeIf(c -> c.getName().equals(name));  // 删除所有匹配项
```

```rust
// ❌ 错误：remove只删除一个
self.base_schema.remove(name);  // ❌ Vec.remove需要索引
```

**解决:**
```rust
// ✅ 正确：使用retain
self.base_schema.retain(|c| c.name != name);  // 保留不匹配的，删除匹配的
```

### 陷阱5: Java继承 → Rust组合误用

**问题:**
```rust
// ❌ 错误：Rust没有继承
pub struct OlapTable extends Table {  // ❌ Rust不支持extends
    // ...
}
```

**解决:**
```rust
// ✅ 正确：使用组合（composition）
pub struct OlapTable {
    pub base: Table,  // 组合
    pub olap_fields: OlapFields,
}

impl OlapTable {
    // 通过base访问父类字段
    pub fn get_id(&self) -> u64 {
        self.base.id
    }
    
    // 通过base调用父类方法
    pub fn get_column(&self, name: &str) -> Option<Column> {
        self.base.get_column(name)
    }
}
```

---

## 📊 复刻进度跟踪表

### Catalog模块复刻进度

| Java类 | 行数 | Rust文件 | 状态 | 完成度 |
|--------|------|----------|------|--------|
| Table.java | 593 | table.rs | 已有基础 | 20% |
| OlapTable.java | 4011 | table.rs | 需扩展 | 0% |
| Database.java | 1015 | database.rs | 已有基础 | 10% |
| Tablet.java | ~800 | tablet.rs | 未创建 | 0% ⚠️ |
| Replica.java | ~300 | replica.rs | 未创建 | 0% ⚠️ |
| Partition.java | ~500 | partition.rs | 已有基础 | 30% |
| Index.java | ~400 | 未创建 | 0% ⚠️ |
| Column.java | ~200 | 已有 | 80% |

### Planner模块复刻进度

| Java类 | 行数 | Rust文件 | 状态 | 完成度 |
|--------|------|----------|------|--------|
| Planner.java | ~500 | planner.rs | 已有基础 | 40% |
| Coordinator.java | 3545 | coordinator.rs | 已有基础 | 10% ⚠️ |
| PlanFragment.java | ~300 | fragment.rs | 未创建 | 0% ⚠️ |
| FragmentInstance.java | ~150 | 未创建 | 0% ⚠️ |
| ScanRange.java | ~100 | 未创建 | 0% ⚠️ |
| RuntimeFilter.java | ~200 | runtime_filter.rs | 已有基础 | 20% |
| PartitionPruner.java | ~800 | 未创建 | 0% |

### BE模块复刻进度

| C++文件 | 行数 | Rust文件 | 状态 | 完成度 |
|---------|------|----------|------|--------|
| version_graph.cpp | 26325 | version_graph.rs | 未创建 | 0% ⚠️ |
| tablet.cpp | ~1000 | tablet.rs | 已有基础 | 20% |
| rowset.cpp | ~500 | rowset.rs | 已有基础 | 30% |
| compaction.cpp | ~2000 | compaction.rs | 已有基础 | 20% |

---

## 🎯 下一步行动建议

### 立即开始：P0核心复刻

1. **Tablet/Replica管理**（最高优先级）
   ```
   任务：复刻 Tablet.java + Replica.java → tablet.rs + replica.rs
   
   步骤：
   ✅ 创建tablet.rs和replica.rs文件
   ✅ 1:1映射所有字段
   ✅ 逐个复刻方法（每个方法验证）
   ✅ 扩展OlapTable.rs的tablet管理方法
   ✅ 测试验证
   ```

2. **Fragment划分和调度**
   ```
   任务：复刻 PlanFragment.java + Coordinator.java → fragment.rs + coordinator.rs
   
   步骤：
   ✅ 创建fragment.rs、fragment_instance.rs、scan_range.rs
   ✅ 1:1映射所有字段
   ✅ 逐个复刻Coordinator的3545行方法
   ✅ 实现ScanRange分配算法
   ✅ 测试验证
   ```

3. **FE高可用**
   ```
   任务：复刻 HAProtocol.java → ha.rs（改用raft-rs）
   
   步骤：
   ✅ 创建ha.rs和backend.rs
   ✅ 定义HAProtocol trait
   ✅ 集成raft-rs库
   ✅ 实现Leader选举和元数据共识
   ✅ 测试验证
   ```

### 每日复刻流程

```bash
# 1. 早上：选择一个Java类
选择: Tablet.java（800行）

# 2. 分析字段和方法
grep -n "private" Tablet.java  # 找所有字段
grep -n "public" Tablet.java   # 找所有方法

# 3. 创建Rust文件并映射字段
vim crates/fe-catalog/src/tablet.rs
# 1:1映射所有字段

# 4. 逐个复刻方法（每个方法验证）
for method in methods:
    复刻Java方法 → Rust方法
    测试验证（相同输入 → 相同输出）

# 5. 集成测试
cargo test
cargo clippy
cargo build --release

# 6. 晚上：提交git
git add tablet.rs
git commit -m "feat: 复刻 Tablet.java → tablet.rs"
```

---

## 💡 总结

**核心策略: 模块化1:1复刻 + Rust适配**

1. **不重设计，逐模块复刻**
2. **Java → Rust转换规则严格遵守**
3. **每个函数验证（相同输入 → 相同输出）**
4. **并发安全优先**
5. **错误处理完整（Exception → Result）**

**记住: 你的AI没那么聪明，所以要保持简单、可验证、可执行**