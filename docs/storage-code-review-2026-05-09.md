# RorisDB 存储层代码审查报告

**审查日期**: 2026-05-09  
**审查范围**: `crates/be-storage/src/`  
**严重等级**: 🔴 Critical / 🟡 Medium / 🟢 Low

---

## 🔴 Critical Issues

### Issue #1: 数据无法持久化恢复

**位置**: `engine.rs:26-40`, `tablet.rs:336-348`

**问题描述**:
- `StorageEngine::open()` 只创建数据目录，不从磁盘加载现有 tablet
- `Tablet::new()` 创建空的内存结构，没有恢复逻辑
- **后果**: 重启进程后，所有已 flush 到磁盘的数据都无法恢复

**缺失的功能**:
```rust
// engine.rs 需要添加
impl StorageEngine {
    pub fn recover(&self) -> Result<()> {
        // 1. 扫描 data_dir 下的所有 tablet_N 目录
        // 2. 读取每个 tablet 的 rowset_*.json 元数据
        // 3. 重建 Tablet 对象并注册到 DashMap
    }
}

// tablet.rs 需要添加
impl Tablet {
    pub fn load_from_disk(tablet_id: u64, schema: TabletSchema, data_dir: PathBuf) -> Result<Self> {
        // 1. 读取已有的 rowset 元数据
        // 2. 重建 rowsets 列表
    }
}
```

**修复优先级**: **P0** - 阻塞生产使用

---

### Issue #2: MemTable 键提取 Bug

**位置**: `tablet.rs:280-290`

**代码**:
```rust
fn extract_key(&self, block: &Block, row_idx: usize, col_idx: usize) -> Result<MemTableKey, String> {
    let scalar = col.scalar_at(row_idx);
    Ok(match scalar {
        types::ScalarValue::Int64(v) => MemTableKey::from_i64(v),
        types::ScalarValue::Int32(v) => MemTableKey::from_i64(v as i64),
        types::ScalarValue::String(s) => MemTableKey::from_string(&s),
        other => MemTableKey(other.data_type().to_string().into_bytes()),  // 🔴 BUG!
    })
}
```

**问题**: 对于 Date, DateTime, Float32, Float64 等类型的键，只是把**类型名**转成字节作为键，完全错误！

**修复**:
```rust
        other => match scalar {
            types::ScalarValue::Date(d) => MemTableKey::from_i64(d as i64),
            types::ScalarValue::DateTime(d) => MemTableKey::from_i64(d),
            types::ScalarValue::Float32(f) => MemTableKey::from_i64(f.to_bits() as i64),
            types::ScalarValue::Float64(f) => MemTableKey::from_i64(f.to_bits() as i64),
            _ => return Err(format!("Unsupported key type: {}", scalar.data_type())),
        },
```

**后果**: 
- 使用非 Int/String 键的表，写入后会**数据损坏**或**查询不到**
- 可能导致 MemTable 中所有行都有相同的键

**修复优先级**: **P0**

---

### Issue #3: Flush 数据未真正落盘

**位置**: `segment/writer.rs:89-136`

**问题**:
```rust
file.write_all(&footer_json)...;
file.flush()...;  // 🟡 flush 只保证用户态 buffer 清空
// 缺少: file.sync_all()?;  // 保证 OS 层面落盘
```

**后果**: 
- 进程崩溃可能导致**已 flush 的数据丢失**
- 不符合数据库的 Durability 要求

**修复**:
```rust
file.write_all(&footer_json)...;
file.sync_all()  // 确保数据落到磁盘
    .map_err(|e| format!("Sync error: {}", e))?;
```

**修复优先级**: **P1**

---

## 🟡 Medium Issues

### Issue #4: Compaction 后旧 Segment 文件未删除

**位置**: `engine.rs:150-168`

**问题**:
```rust
match new_rowset {
    Ok(rowset) => {
        let old_ids: Vec<u64> = tablet.committed_rowsets()...
        tablet.remove_rowsets(&old_ids);
        tablet.add_rowset(rowset);
        // 🔴 缺少: 删除旧的 segment 文件
    }
}
```

**后果**: 磁盘空间泄漏，多次 compaction 后磁盘可能被占满

**修复**:
在 `remove_rowsets()` 中，遍历要删除的 rowset，删除其关联的 segment 文件。

---

### Issue #5: 并发控制逻辑不清晰

**位置**: `tablet.rs:351-368`

**问题**:
```rust
pub fn write(&self, block: &Block) -> Result<(), String> {
    let mut memtable = self.memtable.write();
    memtable.insert(block, key_col_idx)?;
    
    if memtable.should_flush() {
        drop(memtable);  // 释放锁
        self.flush()?;   // flush() 内部又会获取写锁
    }
}
```

虽然 Rust 的 `RwLock` 支持重入（同一个线程），但逻辑不清晰，建议重构为：
```rust
pub fn write(&self, block: &Block) -> Result<(), String> {
    {
        let mut memtable = self.memtable.write();
        memtable.insert(block, key_col_idx)?;
        if !memtable.should_flush() {
            return Ok(());
        }
    }
    // 走到这里说明需要 flush
    self.flush()
}
```

---

### Issue #6: Tablet 读取逻辑中的索引错误

**位置**: `tablet.rs:488-502`

**问题**:
代码试图区分"第一个 block 是 memtable"还是"全部是 segment"，但逻辑不正确：
```rust
for (i, block) in blocks.into_iter().enumerate() {
    if i == 0 && !predicates.is_empty() && !rowsets.is_empty() {
        // 假设第一个是 memtable - 但这个假设不一定对
    }
}
```

**后果**: 可能导致 memtable 的数据未被正确过滤。

**修复**: 应该显式标记 memtable 的 block，而不是依赖索引假设。

---

## 🟢 Low Issues / 建议

### Issue #7: 缺少 WAL (Write-Ahead Log)

当前架构只有 MemTable + Flush to Segment，没有 WAL。
- **风险**: 写入 MemTable 后、flush 前宕机 = 数据丢失
- **建议**: 实现 WAL 保证 Durability

---

### Issue #8: Segment 文件格式缺少 Checksum

**位置**: `segment/writer.rs`

写入的 segment 文件没有 checksum，无法检测数据损坏。

**建议**: 在 footer 中添加 CRC32 或 XXHash 校验和。

---

### Issue #9: 错误修复 - `StorageEngine::create_tablet()` 中的拼写错误

**位置**: `engine.rs:44`

```rust
if self.tablets.contains_key(&tablet_id) {  // 🔴 拼写错误
```

应该是 `&tablet_id`（变量名写错了）。

---

## 测试相关发现

### TPC-C 测试缺失

项目只有 TPC-H 测试（`benches/tpch/`），没有 TPC-C。

- **是否合理**: 对于 OLAP 数据库（RorisDB 的定位），TPC-H 比 TPC-C 更合适
- **建议**: 
  - 如果目标是 OLAP，完善 TPC-H 测试即可
  - 如果需要 OLTP 能力，才需要添加 TPC-C

### 性能测试不完整

`PERFORMANCE_REPORT.md` 显示只测试了：
- Filter 操作
- Query 规划时间
- 数据生成速度

**缺失的性能测试**:
- 实际 TPC-H 查询执行时间（Q1-Q22）
- 数据加载性能
- Compaction 性能
- 并发读写性能
- 内存使用量

---

## 修复优先级排序

| 优先级 | Issue | 工作量 | 影响 |
|--------|-------|--------|------|
| P0 | #1 数据恢复 | 大 | 数据丢失 |
| P0 | #2 MemTable键提取 | 小 | 数据损坏 |
| P1 | #3 Flush落盘 | 小 | 数据丢失 |
| P1 | #4 Compaction文件清理 | 中 | 磁盘泄漏 |
| P2 | #5 并发逻辑 | 小 | 可维护性 |
| P2 | #6 读取逻辑 | 中 | 查询错误 |
| P3 | #7 WAL | 大 | 持久性 |
| P3 | #8 Checksum | 小 | 数据完整性 |

---

## 行动建议

### 立即修复（本周）
1. 修复 MemTable 键提取 bug（5行代码）
2. 在 flush 后添加 `sync_all()` 调用（1行代码）

### 短期（2周内）
3. 实现 `StorageEngine::recover()` 和 `Tablet::load_from_disk()`
4. 修复 compaction 文件清理

### 中期（1个月内）
5. 完善并发控制逻辑
6. 修复 tablet 读取逻辑
7. 添加 WAL 支持

---

## 关于"数据存储有问题"的验证

**你的直觉是对的** 🎯

1. ✅ **数据持久化有问题** - 重启后数据无法恢复
2. ✅ **数据存储有bug** - MemTable 键提取错误会导致数据损坏
3. ✅ **性能测试不完整** - 缺少端到端的查询性能测试

建议优先修复 P0 问题，否则数据库无法用于生产环境。
