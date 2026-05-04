# P3: 高级 Compaction 策略

**优先级**: P3
**模块**: be-storage
**状态**: ❌ 未开始

## 背景

RorisDB 已实现 Base/Cumulative/Full Compaction 和优先级调度。以下高级 Compaction 策略待实现。

## 已实现

- ✅ Base Compaction
- ✅ Cumulative Compaction
- ✅ Full Compaction
- ✅ 优先级调度

## 任务清单

### 1. Segment Compaction
- [ ] 在导入阶段，Segment 级别预合并
- [ ] 减少导入后的小 Segment 数量
- [ ] 控制导入过程中的内存使用

### 2. Single Replica Compaction
- [ ] 只在一个副本上执行 Compaction
- [ ] 合并结果通过复制同步到其他副本
- [ ] 减少 Compaction 对集群计算资源的占用

### 3. 时间序列 Compaction
- [ ] 按时间维度优化数据布局
- [ ] 冷热数据分层存储
- [ ] 过期分区数据自动归档

### 4. 其他存储优化
- [ ] Zlib 压缩算法支持
- [ ] NGram Bloom Filter（用于 LIKE 查询优化）
- [ ] 副本管理: 自动分配、迁移、均衡
- [ ] Tablet 自动修复
- [ ] Colocate Table 支持
- [ ] Publish Version Daemon

## 涉及文件

- `crates/be-storage/src/compaction.rs` - Compaction 策略扩展
- `crates/be-segment/src/codec.rs` - Zlib 压缩
- `crates/be-storage/src/index.rs` - NGram Bloom Filter
- `crates/fe-catalog/src/replica.rs` - 副本管理
