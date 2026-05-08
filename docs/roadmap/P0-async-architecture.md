# P0-01: 异步架构实现

**优先级**: P0（最高）
**模块**: be-execution, be-storage
**状态**: ❌ 未开始
**预计工期**: 3个月
**价值**: ✅✅✅ 极高（性能提升3-5倍）

---

## 📋 问题分析

### Doris的架构限制

```
同步架构问题：
  1. Thread数量有限（几千个）
  2. Blocking IO浪费线程等待
  3. 线程切换开销大
  4. 大查询需要很多线程，成本高
  5. 并发受限（单线程处理1000任务）
```

### RorisDB的创新目标

```
异步架构优势：
  1. Tokio异步任务百万级（轻量）
  2. Non-blocking IO不浪费等待
  3. 无线程切换开销
  4. 大查询成本低，资源利用率高
  5. 单线程处理10000+并发任务
  
性能预期：
  - 并发数: 10倍提升（1000 → 10000+）
  - 吞吐: 3-5倍提升
  - 延迟: 降低50%
```

---

## 🎯 核心组件设计

### 1. Async IO层（io_uring零拷贝）

**技术选择**: tokio-uring（基于Linux io_uring）

**为什么选择io_uring？**
```
优势：
  1. 零拷贝读取（用户态直接访问内核缓冲区）
  2. 批量提交IO请求（减少系统调用）
  3. 完成事件通知（不阻塞等待）
  4. 性能提升：3-5倍（vs传统read）
  
限制：
  1. 仅支持Linux（macOS/Windows需要fallback）
  2. 需要tokio-uring库（tokio生态）
```

**组件设计:**

```rust
// be-storage/src/async_io.rs

use tokio_uring::File;
use std::os::unix::io::AsRawFd;

pub struct AsyncSegmentReader {
    file: File,
    segment_meta: SegmentMeta,
    column_readers: Vec<AsyncColumnReader>,
}

impl AsyncSegmentReader {
    pub async fn open(path: &str) -> Result<Self, Error> {
        let file = File::create(path)?;  // tokio-uring File
        let meta = Self::load_meta(&file).await?;
        let readers = Self::create_column_readers(&file, &meta)?;
        
        Ok(Self {
            file,
            segment_meta: meta,
            column_readers: readers,
        })
    }
    
    pub async fn read_batch(&self, batch_size: usize) -> Result<Block, Error> {
        // 并发读取多列（异步）
        let futures: Vec<_> = self.column_readers.iter()
            .map(|reader| reader.read_async(batch_size))
            .collect();
        
        // 等待所有列读取完成
        let columns = futures::future::join_all(futures).await;
        
        // 组装Block
        Ok(Block::from_columns(columns))
    }
}

pub struct AsyncColumnReader {
    file: File,
    offset: u64,
    length: u64,
    encoding: ColumnEncoding,
}

impl AsyncColumnReader {
    pub async fn read_async(&self, batch_size: usize) -> Result<ColumnRef, Error> {
        // 1. 计算读取范围
        let read_offset = self.offset;
        let read_length = self.calculate_batch_length(batch_size);
        
        // 2. io_uring零拷贝读取
        let buf = vec![0u8; read_length];
        let (res, buf) = self.file.read_at(buf, read_offset).await;
        res?;
        
        // 3. 解码列数据
        let column = self.decode_column(buf, batch_size)?;
        
        Ok(column)
    }
    
    fn decode_column(&self, buf: Vec<u8>, batch_size: usize) -> Result<ColumnRef, Error> {
        match self.encoding {
            ColumnEncoding::Plain => self.decode_plain(buf),
            ColumnEncoding::Rle => self.decode_rle(buf),
            ColumnEncoding::Dictionary => self.decode_dict(buf),
        }
    }
}

// 异步索引读取
pub struct AsyncIndexReader {
    file: File,
    zonemap_offset: u64,
    bloomfilter_offset: u64,
}

impl AsyncIndexReader {
    pub async fn read_zonemap(&self) -> Result<ZoneMap, Error> {
        let buf = vec![0u8; ZONEMAP_SIZE];
        let (res, buf) = self.file.read_at(buf, self.zonemap_offset).await;
        res?;
        
        ZoneMap::deserialize(&buf)
    }
    
    pub async fn read_bloomfilter(&self) -> Result<BloomFilter, Error> {
        let buf = vec![0u8; BLOOMFILTER_SIZE];
        let (res, buf) = self.file.read_at(buf, self.bloomfilter_offset).await;
        res?;
        
        BloomFilter::deserialize(&buf)
    }
}
```

---

### 2. Async Pipeline执行引擎

**架构设计:**

```
Pipeline执行流程：
  SQL → LogicalPlan → PhysicalPlan → AsyncPipeline
    ↓
  Fragment划分（分布式）
    ↓
  AsyncStage（每个Fragment）
    ↓
  AsyncOperator（算子）
    ↓
  AsyncQueue（数据流）
    ↓
  Result（结果）
```

**组件设计:**

```rust
// be-execution/src/async_pipeline.rs

pub struct AsyncPipelineExecutor {
    runtime: tokio::runtime::Runtime,
    pipelines: Vec<AsyncPipeline>,
}

impl AsyncPipelineExecutor {
    pub fn execute(&self) -> Result<QueryResult, Error> {
        self.runtime.block_on(async {
            // 并发执行多个Pipeline
            let futures: Vec<_> = self.pipelines.iter()
                .map(|pipeline| pipeline.execute_async())
                .collect();
            
            let results = futures::future::join_all(futures).await;
            
            // 合并结果
            Ok(QueryResult::merge(results))
        })
    }
}

pub struct AsyncPipeline {
    stages: Vec<AsyncStage>,
    input_queue: AsyncQueue<Block>,
    output_queue: AsyncQueue<Block>,
}

impl AsyncPipeline {
    pub async fn execute_async(&self) -> Result<BlockStream, Error> {
        // 启动所有Stage（并发）
        let stage_futures: Vec<_> = self.stages.iter()
            .enumerate()
            .map(|(i, stage)| {
                let input = if i == 0 { 
                    self.input_queue.clone() 
                } else { 
                    self.stages[i-1].output_queue.clone() 
                };
                
                let output = if i == self.stages.len() - 1 {
                    self.output_queue.clone()
                } else {
                    self.stages[i+1].input_queue.clone()
                };
                
                stage.execute(input, output)
            })
            .collect();
        
        // 等待所有Stage完成
        futures::future::join_all(stage_futures).await;
        
        Ok(self.output_queue.clone())
    }
}

pub struct AsyncStage {
    operators: Vec<AsyncOperator>,
    input_queue: AsyncQueue<Block>,
    output_queue: AsyncQueue<Block>,
}

impl AsyncStage {
    pub async fn execute(&self, input: AsyncQueue<Block>, output: AsyncQueue<Block>) -> Result<(), Error> {
        // 从输入队列读取Block
        while let Some(block) = input.recv().await {
            // 流经所有算子
            let mut current_block = block;
            for operator in &self.operators {
                current_block = operator.process(current_block).await?;
            }
            
            // 发送到输出队列
            output.send(current_block).await;
        }
        
        Ok(())
    }
}

// 异步算子接口
pub trait AsyncOperator {
    async fn process(&self, input: Block) -> Result<Block, Error>;
}

// 异步Scan算子
pub struct AsyncScanOperator {
    reader: AsyncSegmentReader,
    batch_size: usize,
}

impl AsyncOperator for AsyncScanOperator {
    async fn process(&self, _input: Block) -> Result<Block, Error> {
        // 异步读取Segment
        self.reader.read_batch(self.batch_size).await
    }
}

// 异步Filter算子
pub struct AsyncFilterOperator {
    predicate: AsyncPredicate,
}

impl AsyncOperator for AsyncFilterOperator {
    async fn process(&self, input: Block) -> Result<Block, Error> {
        // SIMD并行过滤（不阻塞）
        self.predicate.filter_block(&input).await
    }
}

// 异步Aggregate算子
pub struct AsyncAggregateOperator {
    group_by: Vec<usize>,
    aggregates: Vec<AggregateFunc>,
}

impl AsyncOperator for AsyncAggregateOperator {
    async fn process(&self, input: Block) -> Result<Block, Error> {
        // 流式聚合（不阻塞）
        self.aggregate_block(&input).await
    }
}

// 异步Join算子
pub struct AsyncHashJoinOperator {
    build_side: AsyncHashTable,
    probe_side: AsyncQueue<Block>,
}

impl AsyncOperator for AsyncHashJoinOperator {
    async fn process(&self, input: Block) -> Result<Block, Error> {
        // 异步Hash Join
        self.hash_join(input).await
    }
}
```

---

### 3. Async Queue无锁队列

**设计目标:**
```
特性：
  1. 无锁并发（基于async_channel）
  2. 异步send/recv（不阻塞）
  3. 多生产者多消费者（MPMC）
  4. 背压控制（bounded queue）
  5. 性能：5-10倍（vs Mutex队列）
```

**组件设计:**

```rust
// be-execution/src/async_queue.rs

use async_channel::{Sender, Receiver, bounded};

pub struct AsyncQueue<T> {
    sender: Sender<T>,
    receiver: Receiver<T>,
    capacity: usize,
}

impl<T> AsyncQueue<T> {
    pub fn new(capacity: usize) -> Self {
        let (sender, receiver) = bounded(capacity);
        Self {
            sender,
            receiver,
            capacity,
        }
    }
    
    pub async fn send(&self, item: T) -> Result<(), Error> {
        self.sender.send(item).await
            .map_err(|_| Error::QueueClosed)?;
        Ok(())
    }
    
    pub async fn recv(&self) -> Result<T, Error> {
        self.receiver.recv().await
            .map_err(|_| Error::QueueClosed)
    }
    
    pub fn try_send(&self, item: T) -> Result<(), Error> {
        self.sender.try_send(item)
            .map_err(|_| Error::QueueFull)
    }
    
    pub fn len(&self) -> usize {
        self.receiver.len()
    }
    
    pub fn is_empty(&self) -> bool {
        self.receiver.is_empty()
    }
    
    pub fn is_full(&self) -> bool {
        self.receiver.len() == self.capacity
    }
}

// Block流（Block队列）
pub type BlockQueue = AsyncQueue<Block>;
pub type BlockStream = Receiver<Block>;

// 背压控制示例
pub struct BackpressureController {
    queue: AsyncQueue<Block>,
    threshold: usize,
}

impl BackpressureController {
    pub async fn send_with_backpressure(&self, block: Block) -> Result<(), Error> {
        // 如果队列接近满，等待消费
        while self.queue.len() > self.threshold {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        
        self.queue.send(block).await
    }
}
```

---

### 4. 异步调度器

**设计目标:**
```
特性：
  1. 异步Fragment调度（不阻塞）
  2. 异步Task管理（async_task）
  3. 异步RPC调用（tonic异步）
  4. 异步结果收集（join_all）
  5. 异步超时检测（tokio::time）
```

**组件设计:**

```rust
// fe-scheduler/src/async_coordinator.rs

use tokio::time::{timeout, Duration};
use tonic::transport::Channel;

pub struct AsyncCoordinator {
    fragments: Vec<AsyncFragment>,
    backends: Vec<AsyncBackendClient>,
    timeout_ms: u64,
}

impl AsyncCoordinator {
    pub async fn execute_query(&self) -> Result<QueryResult, Error> {
        // 1. 异步调度Fragment到BE
        let schedule_futures: Vec<_> = self.fragments.iter()
            .zip(self.backends.iter())
            .map(|(fragment, backend)| {
                self.schedule_fragment(fragment, backend)
            })
            .collect();
        
        // 2. 等待调度完成（带超时）
        let scheduled = timeout(
            Duration::from_millis(self.timeout_ms),
            futures::future::join_all(schedule_futures)
        ).await??;
        
        // 3. 异步收集结果
        let result_futures: Vec<_> = scheduled.iter()
            .map(|task_id| self.collect_result(*task_id))
            .collect();
        
        let results = futures::future::join_all(result_futures).await;
        
        // 4. 合并结果
        Ok(QueryResult::merge(results))
    }
    
    async fn schedule_fragment(&self, fragment: &AsyncFragment, backend: &AsyncBackendClient) -> Result<u64, Error> {
        // 异步RPC调用
        let request = FragmentRequest {
            fragment_id: fragment.id,
            plan: fragment.plan.clone(),
        };
        
        let response = backend.execute_fragment(request).await?;
        Ok(response.task_id)
    }
    
    async fn collect_result(&self, task_id: u64) -> Result<Block, Error> {
        // 异步轮询结果
        loop {
            let response = self.get_task_status(task_id).await?;
            
            match response.status {
                TaskStatus::Running => {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                TaskStatus::Finished => {
                    return self.fetch_result(task_id).await;
                }
                TaskStatus::Failed => {
                    return Err(Error::TaskFailed(task_id));
                }
            }
        }
    }
}

// 异步Backend客户端（gRPC）
pub struct AsyncBackendClient {
    channel: Channel,
    client: BackendServiceClient<Channel>,
}

impl AsyncBackendClient {
    pub async fn connect(addr: &str) -> Result<Self, Error> {
        let channel = Channel::from_static(addr).connect().await?;
        let client = BackendServiceClient::new(channel);
        Ok(Self { channel, client })
    }
    
    pub async fn execute_fragment(&self, request: FragmentRequest) -> Result<FragmentResponse, Error> {
        let response = self.client.execute_fragment(request).await?;
        Ok(response.into_inner())
    }
    
    pub async fn get_task_status(&self, task_id: u64) -> Result<TaskStatusResponse, Error> {
        let request = TaskStatusRequest { task_id };
        let response = self.client.get_task_status(request).await?;
        Ok(response.into_inner())
    }
    
    pub async fn fetch_result(&self, task_id: u64) -> Result<Block, Error> {
        let request = FetchResultRequest { task_id };
        let response = self.client.fetch_result(request).await?;
        
        // 解析Block
        Block::from_bytes(response.into_inner().data)
    }
}
```

---

## 📅 实施路线（3个月）

### Month 1: Async IO层实现

**Week 1-2: io_uring集成**
- [ ] 添加tokio-uring依赖
- [ ] 实现AsyncSegmentReader
- [ ] 实现AsyncColumnReader
- [ ] 实现AsyncIndexReader
- [ ] 测试异步读取性能

**Week 3-4: 异步存储层**
- [ ] AsyncTablet实现
- [ ] AsyncRowset实现
- [ ] AsyncCompaction实现
- [ ] 异步存储测试

**验收标准:**
```
- io_uring读取速度：≥100MB/s
- 异步并发数：≥1000（单线程）
- 延迟：≤10ms
```

---

### Month 2: Async Pipeline引擎

**Week 1-2: Pipeline框架**
- [ ] AsyncPipelineExecutor实现
- [ ] AsyncPipeline实现
- [ ] AsyncStage实现
- [ ] AsyncQueue实现

**Week 3-4: 异步算子**
- [ ] AsyncScanOperator
- [ ] AsyncFilterOperator
- [ ] AsyncAggregateOperator
- [ ] AsyncHashJoinOperator
- [ ] AsyncSortOperator

**验收标准:**
```
- Pipeline吞吐：≥1M rows/sec
- 算子并发：≥1000个算子同时执行
- 内存效率：无阻塞等待
```

---

### Month 3: 异步调度和集成测试

**Week 1-2: 异步调度器**
- [ ] AsyncCoordinator实现
- [ ] AsyncBackendClient实现
- [ ] 异步RPC调用
- [ ] 异步超时检测

**Week 3-4: 全链路测试**
- [ ] 单机异步查询测试
- [ ] 多线程并发测试
- [ ] 性能对比测试（vs同步）
- [ ] 稳定性测试

**验收标准:**
```
- 并发查询数：≥10000（vs Doris: 1000）
- 吞吐提升：≥3倍
- 延迟降低：≥50%
```

---

## 📊 性能预期对比

| 指标 | Doris（同步） | RorisDB（异步） | 提升倍数 |
|------|--------------|----------------|---------|
| **并发任务数** | 1000 | 10000+ | 10倍 |
| **吞吐量** | 基准 | 3-5倍 | 3-5倍 |
| **延迟** | 10秒 | 3秒 | 50%降低 |
| **内存占用** | 1GB（线程栈） | 100MB（异步任务） | 10倍节省 |
| **线程数** | 1000 | 4（Tokio runtime） | 250倍减少 |

---

## 📁 涉及文件

### 新建文件

```
be-storage/src/
├── async_io.rs               # Async IO层（~300行）
├── async_reader.rs           # Async Segment/Column Reader（~400行）
├── async_tablet.rs           # Async Tablet管理（~200行）
├── async_rowset.rs           # Async Rowset管理（~150行）
└── async_compaction.rs       # Async Compaction（~200行）

be-execution/src/
├── async_pipeline.rs         # Async Pipeline框架（~500行）
├── async_operator.rs         # Async Operator trait + 实现（~800行）
├── async_queue.rs            # Async Queue无锁队列（~150行）
├── async_executor.rs         # Async Executor入口（~200行）
└── async_stage.rs            # Async Stage实现（~300行）

fe-scheduler/src/
├── async_coordinator.rs      # Async Coordinator（~600行）
├── async_backend_client.rs   # Async Backend RPC（~200行）
└── async_task_manager.rs     # Async Task管理（~300行）

tests/integration/
└── async_performance_test.rs # 异步性能测试（~400行）
```

### 修改文件

```
Cargo.toml                    # 添加tokio-uring, async-channel依赖
be-storage/src/lib.rs         # 导出async模块
be-execution/src/lib.rs       # 导出async模块
fe-scheduler/src/lib.rs       # 导出async模块
```

---

## ⚠️ 技术挑战和应对

### 挑战1: io_uring仅支持Linux

**应对:**
```rust
// 平台适配层
#[cfg(target_os = "linux")]
use tokio_uring::File as AsyncFile;

#[cfg(not(target_os = "linux"))]
use tokio::fs::File as AsyncFile;  // Fallback到普通异步IO
```

### 挑战2: 异步算子状态管理

**应对:**
```rust
// 使用Arc<Mutex>管理共享状态（少量使用）
pub struct AsyncAggregateState {
    hash_table: Arc<Mutex<HashMap<Key, AggValue>>>,
}

// 或使用Actor模型（完全无锁）
pub struct AggregateActor {
    state: HashMap<Key, AggValue>,
    receiver: Receiver<AggregateCommand>,
}
```

### 挑战3: 异步错误传播

**应对:**
```rust
// 使用Result + ?运算符自动传播
impl AsyncOperator for AsyncFilterOperator {
    async fn process(&self, input: Block) -> Result<Block, Error> {
        let filtered = self.predicate.filter(&input)?;  // ?自动传播错误
        Ok(filtered)
    }
}
```

---

## 💡 创新价值

**这是最大价值的创新点：**

1. ✅ **并发能力突破**：10倍提升（1000 → 10000+）
2. ✅ **吞吐突破**：3-5倍提升
3. ✅ **延迟突破**：50%降低
4. ✅ **资源效率**：内存节省10倍，线程减少250倍
5. ✅ **技术领先**：Doris无法实现（线程模型限制）

**异步架构是RorisDB的核心竞争力！**

---

## 🔗 相关文档

- [创新路线总览](00-overview.md)
- [P0-00 SQL Parser重构](P0-sql-parser-refactor.md)
- [P0-02 无锁并发](P0-lock-free-concurrency.md)
- [tokio-uring官方文档](https://github.com/tokio-rs/tokio-uring)

---

## 📝 备注

**为什么异步架构是最高价值？**

1. ✅ 性能提升最显著（3-5倍）
2. ✅ 并发能力突破最大（10倍）
3. ✅ Doris无法实现（架构限制）
4. ✅ Rust独有优势（async/await生态）
5. ✅ 为后续创新打基础（无锁并发、内存池依赖异步）

**P0-01是RorisDB最核心的创新！**