# P0-03: 内存池零拷贝

**优先级**: P0
**模块**: types, be-execution
**状态**: ❌ 未开始
**预计工期**: 1个月
**价值**: ✅ 中（内存效率3倍）

---

## 📋 问题分析

### Doris的内存管理问题

```java
// Doris: 频繁分配和GC
public class Block {
    private byte[] data;  // 每次new分配
    
    public Block clone() {
        return new Block(Arrays.copyOf(data));  // 深拷贝
    }
}

问题：
  1. 频繁分配（GC压力大）
  2. 深拷贝（内存浪费）
  3. 大对象分配慢（直接内存）
  4. GC停顿（秒级停顿）
  5. 内存碎片（难以管理）
```

### RorisDB的内存池设计目标

```
内存池优势：
  1. 内存池预分配（减少分配次数）
  2. Arc零拷贝传递（共享所有权）
  3. PinnedBuffer回收（内存池复用）
  4. 无GC停顿（Rust手动管理）
  5. 内存可控（预分配大小）
  
性能预期：
  - 内存分配次数: 减少90%
  - GC停顿: 完全消除
  - 数据拷贝: 减少50%
  - 内存效率: 提升3倍
```

---

## 🎯 核心组件设计

### 1. GlobalMemoryPool

**设计目标:**
```
特性：
  1. 预分配内存块（减少分配）
  2. 分段管理（不同大小的池）
  3. 空闲列表（回收复用）
  4. 内存统计（监控使用）
  5. 零拷贝分配（PinnedBuffer）
```

**组件设计:**

```rust
// types/src/memory_pool.rs

use std::sync::{Arc, Mutex};
use std::collections::VecDeque;

pub struct GlobalMemoryPool {
    pools: Vec<Arc<MemoryPool>>,  // 不同大小的池
    stats: Arc<Mutex<MemoryStats>>,
}

pub struct MemoryPool {
    size: usize,                  // 每个块的大小
    buffers: Mutex<VecDeque<Pin<Box<[u8]>>>>,  // 预分配内存
    free_list: Mutex<VecDeque<usize>>,         // 空闲索引
}

pub struct MemoryStats {
    total_allocated: usize,
    total_used: usize,
    allocation_count: usize,
    free_count: usize,
}

impl GlobalMemoryPool {
    pub fn new(config: MemoryPoolConfig) -> Self {
        // 创建不同大小的池（4KB, 64KB, 1MB, 16MB）
        let pools = config.pool_sizes.iter()
            .map(|size| {
                Arc::new(MemoryPool::new(*size, config.initial_count))
            })
            .collect();
        
        Self {
            pools,
            stats: Arc::new(Mutex::new(MemoryStats::default())),
        }
    }
    
    pub fn alloc(&self, size: usize) -> PinnedBuffer {
        // 选择合适的池
        let pool = self.select_pool(size);
        
        // 从池分配
        pool.alloc()
    }
    
    pub fn free(&self, buffer: PinnedBuffer) {
        // 回收到池
        buffer.pool.free(buffer);
        
        // 更新统计
        self.stats.lock().free_count += 1;
    }
    
    fn select_pool(&self, size: usize) -> Arc<MemoryPool> {
        // 选择大小合适的池（最小满足）
        self.pools.iter()
            .find(|pool| pool.size >= size)
            .cloned()
            .unwrap_or_else(|| self.pools.last().unwrap().clone())
    }
    
    pub fn stats(&self) -> MemoryStats {
        self.stats.lock().clone()
    }
}

impl MemoryPool {
    pub fn new(size: usize, initial_count: usize) -> Self {
        // 预分配内存块
        let buffers = (0..initial_count)
            .map(|_| {
                let vec = vec![0u8; size];
                Pin::new(vec.into_boxed_slice())
            })
            .collect();
        
        let free_list = (0..initial_count).collect();
        
        Self {
            size,
            buffers: Mutex::new(buffers),
            free_list: Mutex::new(free_list),
        }
    }
    
    pub fn alloc(&self) -> PinnedBuffer {
        let idx = self.free_list.lock().pop()
            .unwrap_or_else(|| {
                // 池不足，动态扩展
                self.expand_pool();
                self.free_list.lock().pop().unwrap()
            });
        
        PinnedBuffer {
            pool: self as *const MemoryPool,
            idx,
            size: self.size,
        }
    }
    
    pub fn free(&self, buffer: PinnedBuffer) {
        // 回收索引到空闲列表
        self.free_list.lock().push_back(buffer.idx);
    }
    
    fn expand_pool(&self) {
        // 动态扩展池（加锁）
        let mut buffers = self.buffers.lock();
        let mut free_list = self.free_list.lock();
        
        let new_idx = buffers.len();
        let vec = vec![0u8; self.size];
        buffers.push_back(Pin::new(vec.into_boxed_slice()));
        free_list.push_back(new_idx);
    }
}

// PinnedBuffer（零拷贝）
pub struct PinnedBuffer {
    pool: *const MemoryPool,  // 所属池
    idx: usize,               // 池中索引
    size: usize,              // 大小
}

impl PinnedBuffer {
    pub fn as_slice(&self) -> &[u8] {
        // 获取内存切片（不拷贝）
        let pool = unsafe { &*self.pool };
        let buffers = pool.buffers.lock();
        &buffers[self.idx]
    }
    
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        // 获取可变切片（不拷贝）
        let pool = unsafe { &*self.pool };
        let mut buffers = pool.buffers.lock();
        &mut buffers[self.idx]
    }
}

impl Drop for PinnedBuffer {
    fn drop(&mut self) {
        // 自动回收到池
        let pool = unsafe { &*self.pool };
        pool.free_list.lock().push_back(self.idx);
    }
}

impl Clone for PinnedBuffer {
    fn clone(&self) -> Self {
        // 复制索引，不复制数据（零拷贝）
        Self {
            pool: self.pool,
            idx: self.idx,
            size: self.size,
        }
    }
}

// 配置
pub struct MemoryPoolConfig {
    pool_sizes: Vec<usize>,       // 不同大小的池（4KB, 64KB, 1MB, 16MB）
    initial_count: usize,         // 每个池初始数量
}

impl Default for MemoryPoolConfig {
    fn default() -> Self {
        Self {
            pool_sizes: vec![4 * 1024, 64 * 1024, 1024 * 1024, 16 * 1024 * 1024],
            initial_count: 100,
        }
    }
}
```

---

### 2. Block零拷贝

**设计目标:**
```
特性：
  1. Block使用内存池分配
  2. Block slice零拷贝（Arc共享）
  3. Block传递零拷贝（Arc clone）
  4. Block自动回收（Drop回收）
```

**组件设计:**

```rust
// types/src/block.rs

use std::sync::Arc;

pub struct Block {
    pool_ref: Arc<GlobalMemoryPool>,
    buffer_idx: usize,            // 内存池索引
    columns: Vec<ColumnRef>,      // 列引用
    num_rows: usize,
}

impl Block {
    pub fn new(pool: Arc<GlobalMemoryPool>, num_rows: usize) -> Self {
        // 从内存池分配
        let size = num_rows * ROW_SIZE;
        let buffer = pool.alloc(size);
        
        Self {
            pool_ref: pool,
            buffer_idx: buffer.idx,
            columns: vec![],
            num_rows,
        }
    }
    
    pub fn slice(&self, start: usize, len: usize) -> Block {
        // 零拷贝slice（Arc共享）
        Block {
            pool_ref: self.pool_ref.clone(),  // Arc不拷贝数据
            buffer_idx: self.buffer_idx,
            columns: self.columns.iter()
                .map(|col| col.slice(start, len))
                .collect(),
            num_rows: len,
        }
    }
    
    pub fn from_columns(columns: Vec<ColumnRef>) -> Self {
        // 从列构建Block
        let num_rows = columns.first().map(|c| c.len).unwrap_or(0);
        
        Self {
            pool_ref: Arc::new(GlobalMemoryPool::default()),  // 临时池
            buffer_idx: 0,
            columns,
            num_rows,
        }
    }
    
    pub fn get_column(&self, idx: usize) -> Option<&ColumnRef> {
        self.columns.get(idx)
    }
}

impl Clone for Block {
    fn clone(&self) -> Self {
        // Arc clone，不拷贝数据
        Self {
            pool_ref: self.pool_ref.clone(),
            buffer_idx: self.buffer_idx,
            columns: self.columns.iter().cloned().collect(),
            num_rows: self.num_rows,
        }
    }
}

// ColumnRef零拷贝
pub struct ColumnRef {
    data: Arc<Vec<u8>>,           // Arc共享数据
    null_bitmap: Arc<Vec<u8>>,
    start: usize,
    len: usize,
    data_type: DataType,
}

impl ColumnRef {
    pub fn slice(&self, start: usize, len: usize) -> ColumnRef {
        // 零拷贝slice（Arc共享）
        ColumnRef {
            data: self.data.clone(),  // Arc不拷贝数据
            null_bitmap: self.null_bitmap.clone(),
            start: self.start + start,
            len,
            data_type: self.data_type.clone(),
        }
    }
    
    pub fn get(&self, idx: usize) -> Option<ScalarValue> {
        if idx >= self.len {
            return None;
        }
        
        let offset = self.start + idx;
        
        // 检查null
        if self.is_null(offset) {
            return None;
        }
        
        // 解析值（不拷贝）
        Some(self.parse_value(offset))
    }
    
    fn is_null(&self, idx: usize) -> bool {
        let byte_idx = idx / 8;
        let bit_idx = idx % 8;
        
        if byte_idx < self.null_bitmap.len() {
            (self.null_bitmap[byte_idx] & (1 << bit_idx)) != 0
        } else {
            false
        }
    }
    
    fn parse_value(&self, idx: usize) -> ScalarValue {
        match &self.data_type {
            DataType::Int32 => {
                let offset = idx * 4;
                let bytes = &self.data[offset..offset+4];
                ScalarValue::Int32(i32::from_le_bytes(bytes.try_into().unwrap()))
            }
            DataType::Float64 => {
                let offset = idx * 8;
                let bytes = &self.data[offset..offset+8];
                ScalarValue::Float64(f64::from_le_bytes(bytes.try_into().unwrap()))
            }
            DataType::String => {
                // String类型特殊处理（偏移量数组）
                ScalarValue::String("".to_string())
            }
        }
    }
}

impl Clone for ColumnRef {
    fn clone(&self) -> Self {
        // Arc clone，不拷贝数据
        Self {
            data: self.data.clone(),
            null_bitmap: self.null_bitmap.clone(),
            start: self.start,
            len: self.len,
            data_type: self.data_type.clone(),
        }
    }
}
```

---

### 3. Result零拷贝传递

**设计目标:**
```
特性：
  1. QueryResult使用Block引用
  2. Result传递零拷贝（Arc传递）
  3. Result合并零拷贝（Block拼接）
```

**组件设计:**

```rust
// be-execution/src/result.rs

pub struct QueryResult {
    blocks: Arc<Vec<Block>>,      // Arc共享Block
    schema: Schema,
}

impl QueryResult {
    pub fn new(blocks: Vec<Block>, schema: Schema) -> Self {
        Self {
            blocks: Arc::new(blocks),  // Arc共享
            schema,
        }
    }
    
    pub fn merge(results: Vec<QueryResult>) -> Self {
        // 合并多个Result（零拷贝）
        let blocks: Vec<Block> = results.iter()
            .flat_map(|r| r.blocks.iter().cloned())
            .collect();
        
        let schema = results.first().unwrap().schema.clone();
        
        Self {
            blocks: Arc::new(blocks),  // Arc共享
            schema,
        }
    }
    
    pub fn get_block(&self, idx: usize) -> Option<Block> {
        self.blocks.get(idx).cloned()  // Block clone（Arc不拷贝）
    }
    
    pub fn num_rows(&self) -> usize {
        self.blocks.iter().map(|b| b.num_rows).sum()
    }
}

impl Clone for QueryResult {
    fn clone(&self) -> Self {
        // Arc clone，不拷贝数据
        Self {
            blocks: self.blocks.clone(),
            schema: self.schema.clone(),
        }
    }
}
```

---

### 4. 内存池对比

| 方案 | 分配次数 | GC停顿 | 数据拷贝 | 内存效率 |
|------|---------|---------|---------|---------|
| **new分配** | ❌ 高（100万次） | ❌ 有（秒级） | ❌ 深拷贝 | ❌ 低 |
| **内存池** | ✅ 低（100次） | ✅ 无 | ✅ 零拷贝 | ✅ 高（3倍） |

---

## 📅 实施路线（1个月）

### Week 1-2: 内存池实现

- [ ] GlobalMemoryPool实现
- [ ] MemoryPool分段实现
- [ ] PinnedBuffer零拷贝
- [ ] 内存池测试

**验收标准:**
```
- 内存分配次数：减少90%
- 内存池命中率：≥95%
- 分配延迟：<1ms
```

---

### Week 3-4: Block零拷贝

- [ ] Block内存池集成
- [ ] ColumnRef Arc共享
- [ ] Block slice零拷贝
- [ ] QueryResult零拷贝
- [ ] 全链路测试

**验收标准:**
```
- Block拷贝次数：减少90%
- Block传递延迟：<1ms
- 内存使用：节省50%
```

---

## 📊 性能预期对比

| 指标 | Doris（new分配） | RorisDB（内存池） | 提升倍数 |
|------|-----------------|------------------|---------|
| **内存分配次数** | 100万次 | 100次 | 减少90% |
| **GC停顿** | 5次/秒 | 0次 | 完全消除 |
| **数据拷贝** | 20GB | 10GB | 减少50% |
| **内存效率** | 基准 | 3倍 | 提升3倍 |
| **分配延迟** | 10ms | 1ms | 10倍改善 |

---

## 📁 涉及文件

### 新建文件

```
types/src/
├── memory_pool.rs            # GlobalMemoryPool（~400行）
├── pinned_buffer.rs          # PinnedBuffer（~150行）
├── block.rs                  # Block零拷贝（~300行）
└── column_ref.rs             # ColumnRef零拷贝（~250行）

be-execution/src/
└── result.rs                 # QueryResult零拷贝（~150行）

tests/integration/
└── memory_pool_test.rs       # 内存池测试（~300行）
```

### 修改文件

```
types/src/lib.rs              # 导出memory_pool
be-execution/src/exec_node.rs # Block使用内存池
Cargo.toml                    # 无需额外依赖（标准库）
```

---

## ⚠️ 技术挑战和应对

### 挑战1: 内存池大小配置

**应对:**
```rust
// 动态调整池大小
pub struct MemoryPoolMonitor {
    pool: Arc<GlobalMemoryPool>,
}

impl MemoryPoolMonitor {
    pub async fn monitor(&self) {
        loop {
            let stats = self.pool.stats();
            
            // 如果使用率低，收缩池
            if stats.total_used < stats.total_allocated * 0.3 {
                self.shrink_pool();
            }
            
            // 如果使用率高，扩展池
            if stats.total_used > stats.total_allocated * 0.8 {
                self.expand_pool();
            }
            
            tokio::time::sleep(Duration::from_secs(10)).await;
        }
    }
}
```

### 挑战2: 大对象处理

**应对:**
```rust
// 大对象单独分配（不进池）
pub fn alloc_block(size: usize) -> PinnedBuffer {
    if size > MAX_POOL_SIZE {
        // 大对象直接分配
        let vec = vec![0u8; size];
        PinnedBuffer::from_vec(vec)
    } else {
        // 正常对象从池分配
        GLOBAL_POOL.alloc(size)
    }
}
```

### 挑战3: 内存泄漏检测

**应对:**
```rust
// 内存泄漏检测
pub struct MemoryLeakDetector {
    allocated_buffers: Mutex<HashMap<usize, AllocationInfo>>,
}

impl MemoryLeakDetector {
    pub fn check_leak(&self) -> Vec<AllocationInfo> {
        let buffers = self.allocated_buffers.lock();
        
        // 检查长时间未回收的Buffer
        buffers.iter()
            .filter(|(_, info)| {
                info.timestamp.elapsed() > Duration::from_secs(60)
            })
            .map(|(_, info)| info.clone())
            .collect()
    }
}
```

---

## 💡 创新价值

**这是中等价值的创新点：**

1. ✅ **内存效率提升**：3倍（减少分配和拷贝）
2. ✅ **GC停顿消除**：完全无停顿
3. ✅ **内存可控**：预分配，避免碎片
4. ✅ **零拷贝传递**：Arc共享，减少拷贝
5. ✅ **性能提升**：分配延迟降低10倍

**内存池是RorisDB内存管理的优化！**

---

## 🔗 相关文档

- [创新路线总览](00-overview.md)
- [P0-01 异步架构](P0-async-architecture.md)
- [P0-02 无锁并发](P0-lock-free-concurrency.md)

---

## 📝 备注

**为什么内存池设计重要？**

1. ✅ 减少GC停顿（Doris的GC风险）
2. ✅ 提升内存效率（减少分配和拷贝）
3. ✅ 内存可控（预分配避免OOM）
4. ✅ 性能提升（分配延迟降低）
5. ✅ Rust独有优势（Arc零拷贝）

**P0-03是RorisDB内存效率的保障！**