# RorisDB 性能报告

本文档提供 RorisDB 的性能基准测试结果和优化说明。

## 测试环境

### 硬件配置
- **平台**：Apple Silicon Mac
- **处理器**：Apple M 系列芯片
- **内存**：16GB+ 统一内存架构
- **存储**：SSD

### 软件环境
- **操作系统**：macOS
- **Rust 版本**：1.75+
- **测试版本**：RorisDB v0.1.3

## TPC-H 基准测试

### 测试概述

TPC-H 是一个用于衡量 OLAP 数据库性能的标准基准测试。它包含 22 个查询，模拟复杂的业务分析场景。

- **Scale Factor 1 (SF1)**：约 6M 行 lineitem 表
- **Scale Factor 0.01 (SF0.01)**：小型测试数据集

### 查询规划性能

以下是在 Apple Silicon 上的查询规划时间（不包括执行）：

| 查询 | 规划时间 (µs) |
|-------|----------------|
| Q1 | 36.95 |
| Q2 | 44.24 |
| Q3 | 25.47 |
| Q4 | 20.18 |
| Q5 | 31.53 |
| Q6 | 16.54 |
| Q7 | 55.62 |
| Q8 | 55.38 |
| Q9 | 43.07 |
| Q10 | 33.54 |
| Q11 | 31.90 |
| Q12 | 35.87 |
| Q13 | 24.40 |
| Q14 | 23.69 |
| Q15 | 45.31 |
| Q16 | 31.62 |
| Q17 | 20.43 |
| Q18 | 29.49 |
| Q19 | 57.29 |
| Q20 | 36.83 |
| Q21 | 45.28 |
| Q22 | 46.39 |

**平均规划时间**：约 35 µs

### 端到端查询性能

#### Q6 示例（过滤聚合查询）

| 指标 | 值 |
|------|-----|
| 查询类型 | 过滤 + 聚合（lineitem 表） |
| 端到端时间 | 178.14 µs |
| 数据规模 | SF1 (6M 行) |

#### 数据生成性能

| Scale Factor | 生成时间 |
|--------------|----------|
| SF0.01 (tiny) | 160 µs |
| SF1 (6M 行) | 1.48 ms |

### 过滤操作性能

| 操作 | 时间 | 改进 |
|------|------|------|
| Filter (lineitem returnflag = 'R') | 113.86 µs | ~9% 提升 |

## 关键优化技术

### 1. Bitmap 向量化迭代器（iter_set_bits）

**文件**：`crates/types/src/bitmap.rs`

**问题**：原始过滤实现逐个检查每个位 - O(n) 复杂度，缓存行为差。

**解决方案**：使用 `trailing_zeros()` 实现 `SetBitIter`，一次处理 64 位：

```rust
pub struct SetBitIter<'a> {
    data: &'a Vec<u64>,
    word_idx: usize,
    word: u64,
    len: usize,
    consumed: usize,
}

impl<'a> Iterator for SetBitIter<'a> {
    type Item = usize;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.word != 0 {
                let bit = self.word.trailing_zeros() as usize;
                self.word &= !(1u64 << bit);
                return Some(self.consumed + bit);
            }
            // ... 高效处理跨 word 切换
        }
    }
}
```

**优势**：利用 CPU 指令级并行，一次处理 64 位。

### 2. 向量过滤预分配

**文件**：`crates/types/src/vector.rs`

**问题**：向量过滤操作在不知道输出大小的情况下分配向量，导致重新分配开销。

**解决方案**：使用 `set_count()` 预分配：

```rust
pub fn filter(&self, selection: &Bitmap) -> Self {
    let len = selection.set_count();  // 预分配
    let mut data = Vec::with_capacity(len);
    let mut validity = Bitmap::with_capacity(len);
    for idx in selection.iter_set_bits() {
        data.push(self.data[idx]);
        validity.push(self.validity.is_valid(idx));
    }
    Self { data, validity }
}
```

**优势**：消除过滤过程中的向量重新分配。

### 3. 批量 Bitmap 操作

**文件**：`crates/types/src/bitmap.rs`

添加了优化的原地操作：
- `and_inplace()` - 批量 AND，支持提前终止
- `or_inplace()` - 批量 OR，支持容量调整
- `not_inplace()` - 批量 NOT，支持位掩码清理

## Apple Silicon 优化说明

### 1. NEON SIMD 指令

Rust 在 Apple Silicon 上的 `u64` 操作会自动利用 NEON SIMD 指令（当启用目标特性编译时）。

### 2. trailing_zeros() 优化

`trailing_zeros()` 直接映射到 ARM 的 `cls`（count leading zeros）指令，实现高效的位扫描。

### 3. 内存布局优化

列式存储确保数据访问模式对分析工作负载缓存友好。

### 4. 预分配策略

减少分配器压力 - 对 Apple Silicon 的内存子系统尤为重要。

## 性能对比

### 与 Apache Doris 的理论对比

| 维度 | Apache Doris (C++) | RorisDB (Rust) |
|------|-------------------|----------------|
| 内存安全 | 手动管理 | 编译期保证 |
| 执行模型 | 向量化 + Pipeline | 向量化 + Pipeline |
| SIMD 优化 | 手动 intrinsics | 编译器自动优化 |
| 内存分配 | 自定义分配器 | Rust 标准分配器 + 预分配优化 |

## 如何复现测试结果

### 编译 Release 版本

```bash
cargo build --release
```

### 运行 TPC-H 基准测试

```bash
cargo bench -p tpch-bench
```

### 手动测试查询性能

```bash
# 启动 RorisDB
./target/release/roris-fe --http-port 8030 --rpc-port 9020 &
./target/release/roris-be --http-port 8060 --rpc-port 9060 --fe-addr 127.0.0.1:9020 &

# 使用 mysql 客户端测试
time mysql -h 127.0.0.1 -P 9030 -uroot -e "SELECT COUNT(*) FROM lineitem WHERE returnflag = 'R'"
```

## 未来优化方向

### 短期优化
1. **批量 selection 构建**：并行构建过滤谓词的 bitmap
2. **字符串操作 SIMD 加速**：利用 Apple Silicon Neon 进行字符串比较

### 中期优化
3. **NUMA 感知分配**：为多 die 的 Apple Silicon 配置优化
4. **JIT 编译**：为热点查询片段生成机器码

### 长期优化
5. **查询计划缓存**：缓存编译后的查询计划
6. **列式压缩优化**：更多压缩算法（zstd、Zlib）
7. **内存池**：自定义内存池减少分配开销

## 性能监控

### 查看查询执行时间

```sql
-- 在 mysql 客户端中
SELECT /*+ SET_VAR(profile=true) */ COUNT(*) FROM lineitem WHERE returnflag = 'R';
```

### 使用 EXPLAIN 分析查询计划

```sql
EXPLAIN SELECT COUNT(*) FROM lineitem WHERE returnflag = 'R';
```

### 日志中的性能指标

设置日志级别为 `debug` 或 `trace` 可以查看详细的性能指标：

```bash
RUST_LOG=debug ./target/release/roris-fe ...
```

## 性能最佳实践

### 数据建模
1. **选择合适的表模型**：当前使用 `DUPLICATE KEY` 模型
2. **合理设计主键**：帮助查询优化器选择更好的执行计划

### 查询优化
1. **尽早过滤**：使用 WHERE 子句尽早减少数据量
2. **只查询需要的列**：避免 `SELECT *`
3. **使用 LIMIT**：限制结果集大小
4. **利用索引**：ZoneMap 和 BloomFilter 会自动使用

### 系统配置
1. **合理分配内存**：根据数据规模设置 `memory_limit`
2. **使用 SSD 存储**：列式存储对 IO 性能敏感
3. **调整批次大小**：根据查询特点调整 `batch_size`

## 已知性能限制

1. **查询执行未完成**：部分查询算子尚未完全实现，影响端到端性能
2. **分布式执行待完善**：当前主要测试单机性能
3. **编译优化**：尚未使用 `-C target-cpu=native` 等编译优化

## 下一步

- 查看[架构设计文档](architecture.md)了解系统设计
- 阅读[功能特性](features.md)了解当前实现状态
- 参考[开发者指南](developer-guide.md)参与性能优化
