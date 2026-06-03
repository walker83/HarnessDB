# P1-00: 列式存储优化（Arrow + SIMD）

**优先级**: P1
**模块**: be-storage, be-execution
**状态**: ❌ 未开始
**预计工期**: 3个月
**价值**: ✅✅ 高（吞吐5-10倍）

---

## 📋 问题分析

### Doris的列式存储问题

```
改造列式问题：
  1. 行式思维，性能损失
  2. 没有列式压缩优化
  3. 没有向量化执行深度集成
  4. 编码方式少（仅RLE）
  5. SIMD未深度集成
  
性能限制：
  - Scan吞吐: 1M rows/sec
  - 编码效率: 低
  - SIMD使用: 部分
```

### HarnessDB的列式原生设计

```
Arrow生态优势：
  1. Apache Arrow标准格式（列式原生）
  2. Arrow IPC零拷贝传输
  3. SIMD深度集成（Arrow compute kernels）
  4. 多种智能编码（Dictionary/RLE/Delta/BitPacking）
  5. 列式压缩优化（每列独立）
  
性能预期：
  - Scan吞吐: 5-10M rows/sec（5-10倍）
  - 编码效率: 高（智能选择）
  - SIMD使用: 深度集成
```

---

## 🎯 核心组件设计

### 1. Arrow数据格式集成

**为什么选择Arrow？**
```
优势：
  1. 列式内存布局（零拷贝）
  2. SIMD友好（连续内存）
  3. 跨语言支持（C++/Rust/Python/Java）
  4. Arrow IPC零拷贝传输
  5. Apache官方标准（生态成熟）
  
架构：
  HarnessDB存储层（Arrow格式）
    ↓ Arrow IPC零拷贝
  Datafusion查询引擎（Arrow格式）
    ↓ Arrow compute kernels
  SIMD向量化执行
```

**组件设计:**

```rust
// be-storage/src/arrow_format.rs

use arrow::array::{Int32Array, Float64Array, StringArray, BooleanArray};
use arrow::datatypes::{Schema, Field, DataType};
use arrow::ipc::writer::StreamWriter;
use arrow::ipc::reader::StreamReader;

pub struct ArrowSchema {
    schema: Schema,
}

impl ArrowSchema {
    pub fn from_columns(columns: Vec<ColumnDef>) -> Self {
        let fields = columns.iter()
            .map(|col| {
                Field::new(
                    col.name.clone(),
                    Self::convert_type(&col.data_type),
                    col.nullable,
                )
            })
            .collect();
        
        Self {
            schema: Schema::new(fields),
        }
    }
    
    fn convert_type(dtype: &DataType) -> arrow::datatypes::DataType {
        match dtype {
            DataType::Int32 => arrow::datatypes::DataType::Int32,
            DataType::Float64 => arrow::datatypes::DataType::Float64,
            DataType::String => arrow::datatypes::DataType::Utf8,
            DataType::Boolean => arrow::datatypes::DataType::Boolean,
        }
    }
}

pub struct ArrowSegmentWriter {
    schema: Arc<Schema>,
    file: tokio::fs::File,
}

impl ArrowSegmentWriter {
    pub async fn write_batch(&self, batch: RecordBatch) -> Result<u64, Error> {
        // Arrow IPC零拷贝写入
        let mut writer = StreamWriter::try_new(self.file, self.schema.clone())?;
        
        writer.write(&batch)?;
        writer.finish()?;
        
        Ok(self.file.metadata().len())
    }
}

pub struct ArrowSegmentReader {
    schema: Arc<Schema>,
    file: tokio::fs::File,
}

impl ArrowSegmentReader {
    pub async fn read_batch(&self) -> Result<RecordBatch, Error> {
        // Arrow IPC零拷贝读取
        let mut reader = StreamReader::try_new(self.file)?;
        
        reader.next()
            .ok_or(Error::NoBatch)?
    }
}

// Arrow RecordBatch（列式）
pub struct ArrowBatch {
    schema: Arc<Schema>,
    columns: Vec<Arc<dyn Array>>,  // Arrow列数组
    num_rows: usize,
}

impl ArrowBatch {
    pub fn get_column(&self, idx: usize) -> Arc<dyn Array> {
        self.columns[idx].clone()
    }
    
    pub fn slice(&self, offset: usize, length: usize) -> ArrowBatch {
        // Arrow零拷贝slice
        let sliced_columns = self.columns.iter()
            .map(|col| col.slice(offset, length))
            .collect();
        
        ArrowBatch {
            schema: self.schema.clone(),
            columns: sliced_columns,
            num_rows: length,
        }
    }
}
```

---

### 2. 智能列式编码

**编码类型:**

```
智能编码选择：
  1. Dictionary编码（低NDV列）
  2. RLE编码（重复值多）
  3. Delta编码（有序数值）
  4. BitPacking编码（小范围数值）
  5. Plain编码（通用）
  
选择策略：
  - NDV < 100: Dictionary
  - Repeats > 50%: RLE
  - Sorted + Delta small: Delta
  - Range < 1000: BitPacking
  - 其他: Plain
```

**组件设计:**

```rust
// be-storage/src/smart_encoder.rs

pub struct SmartEncoder {
    statistics: ColumnStatistics,
}

impl SmartEncoder {
    pub fn select_encoding(stats: &ColumnStatistics) -> EncodingType {
        // 基于统计选择最佳编码
        
        // 1. Dictionary编码（低NDV）
        if stats.ndv < 100 {
            return EncodingType::Dictionary;
        }
        
        // 2. RLE编码（重复值多）
        if stats.repeat_ratio > 0.5 {
            return EncodingType::Rle;
        }
        
        // 3. Delta编码（有序数值）
        if stats.is_sorted && stats.max_delta < 1000 {
            return EncodingType::Delta;
        }
        
        // 4. BitPacking编码（小范围数值）
        let range = stats.max_value - stats.min_value;
        if range < 1000 {
            return EncodingType::BitPacking { bits: Self::calculate_bits(range) };
        }
        
        // 5. Plain编码（默认）
        EncodingType::Plain
    }
    
    fn calculate_bits(range: u64) -> u8 {
        // 计算需要的bit数
        if range < 256 { 8 }
        else if range < 65536 { 16 }
        else { 32 }
    }
    
    pub fn encode(&self, array: Arc<dyn Array>) -> Result<EncodedColumn, Error> {
        let stats = self.calculate_statistics(array);
        let encoding = Self::select_encoding(&stats);
        
        match encoding {
            EncodingType::Dictionary => self.encode_dictionary(array),
            EncodingType::Rle => self.encode_rle(array),
            EncodingType::Delta => self.encode_delta(array),
            EncodingType::BitPacking { bits } => self.encode_bitpacking(array, bits),
            EncodingType::Plain => self.encode_plain(array),
        }
    }
    
    fn encode_dictionary(&self, array: Arc<dyn Array>) -> Result<EncodedColumn, Error> {
        // Dictionary编码：值 → code
        let dict = Self::build_dictionary(array)?;
        let codes = Self::encode_codes(array, &dict)?;
        
        Ok(EncodedColumn {
            encoding: EncodingType::Dictionary,
            dict: Some(dict),
            data: codes,
        })
    }
    
    fn encode_rle(&self, array: Arc<dyn Array>) -> Result<EncodedColumn, Error> {
        // RLE编码：(value, count)
        let runs = Self::detect_runs(array);
        
        Ok(EncodedColumn {
            encoding: EncodingType::Rle,
            data: runs.encode(),
        })
    }
    
    fn encode_delta(&self, array: Arc<dyn Array>) -> Result<EncodedColumn, Error> {
        // Delta编码：base + deltas
        let base = array.first_value();
        let deltas = array.delta_values();
        
        Ok(EncodedColumn {
            encoding: EncodingType::Delta,
            base: Some(base),
            data: deltas,
        })
    }
    
    fn encode_bitpacking(&self, array: Arc<dyn Array>, bits: u8) -> Result<EncodedColumn, Error> {
        // BitPacking编码：紧凑存储
        let packed = Self::pack_bits(array, bits);
        
        Ok(EncodedColumn {
            encoding: EncodingType::BitPacking { bits },
            data: packed,
        })
    }
}

// 编码效果对比
#[derive(Debug)]
pub struct EncodingStats {
    encoding_type: EncodingType,
    original_size: usize,
    encoded_size: usize,
    compression_ratio: f64,
}

impl SmartEncoder {
    pub fn benchmark_encodings(array: Arc<dyn Array>) -> Vec<EncodingStats> {
        let stats = self.calculate_statistics(array);
        
        // 测试所有编码
        let encodings = vec![
            EncodingType::Plain,
            EncodingType::Dictionary,
            EncodingType::Rle,
            EncodingType::Delta,
        ];
        
        encodings.iter()
            .map(|encoding| {
                let encoded = self.encode_with_type(array, encoding);
                EncodingStats {
                    encoding_type: encoding.clone(),
                    original_size: array.len(),
                    encoded_size: encoded.len(),
                    compression_ratio: encoded.len() as f64 / array.len() as f64,
                }
            })
            .collect()
    }
}
```

---

### 3. SIMD向量化执行

**SIMD集成:**

```
Arrow Compute Kernels（SIMD优化）：
  1. Filter SIMD（并行1024行）
  2. Project SIMD（并行1024行）
  3. Aggregate SIMD（并行1024行）
  4. Sort SIMD（并行排序）
  5. Join SIMD（Hash构建）
  
SIMD优势：
  - 单指令处理多数据（AVX2: 8个float/次）
  - 并行处理1024行（batch处理）
  - 性能提升：5-10倍
```

**组件设计:**

```rust
// be-execution/src/simd_operator.rs

use arrow::compute::{filter, take, comparison, aggregate};

pub struct SimdFilterOperator {
    predicate: Expr,
}

impl SimdFilterOperator {
    pub fn process(&self, batch: &RecordBatch) -> Result<RecordBatch, Error> {
        // SIMD并行过滤
        let predicate_array = self.evaluate_predicate(batch)?;
        
        // Arrow filter kernel（SIMD优化）
        let filtered = filter::filter(batch, &predicate_array)?;
        
        Ok(filtered)
    }
    
    fn evaluate_predicate(&self, batch: &RecordBatch) -> Result<BooleanArray, Error> {
        // SIMD并行计算predicate
        match &self.predicate {
            Expr::BinaryOp { left, op, right } => {
                let left_array = self.evaluate_expr(left, batch)?;
                let right_array = self.evaluate_expr(right, batch)?;
                
                // Arrow comparison kernel（SIMD优化）
                match op {
                    BinaryOp::Gt => comparison::gt(&left_array, &right_array),
                    BinaryOp::Lt => comparison::lt(&left_array, &right_array),
                    BinaryOp::Eq => comparison::eq(&left_array, &right_array),
                }
            }
        }
    }
}

pub struct SimdAggregateOperator {
    group_by: Vec<usize>,
    aggregates: Vec<AggregateFunc>,
}

impl SimdAggregateOperator {
    pub fn process(&self, batch: &RecordBatch) -> Result<RecordBatch, Error> {
        // SIMD并行聚合
        
        // 1. Group by（Hash分组）
        let groups = self.group_rows(batch)?;
        
        // 2. Aggregate（SIMD聚合）
        let agg_results = self.aggregate_groups(&groups)?;
        
        Ok(agg_results)
    }
    
    fn aggregate_groups(&self, groups: &HashMap<u64, Vec<usize>>) -> Result<RecordBatch, Error> {
        let mut results = vec![];
        
        for (_, indices) in groups {
            for agg in &self.aggregates {
                match agg {
                    AggregateFunc::Sum => {
                        // Arrow sum kernel（SIMD优化）
                        let sum = aggregate::sum(self.get_column(batch, indices))?;
                        results.push(sum);
                    }
                    AggregateFunc::Avg => {
                        let avg = aggregate::avg(self.get_column(batch, indices))?;
                        results.push(avg);
                    }
                }
            }
        }
        
        Ok(RecordBatch::new(results))
    }
}

pub struct SimdSortOperator {
    sort_keys: Vec<(usize, SortOrder)>,
}

impl SimdSortOperator {
    pub fn process(&self, batch: &RecordBatch) -> Result<RecordBatch, Error> {
        // SIMD并行排序
        
        // Arrow sort kernel（SIMD优化）
        let sorted_indices = arrow::compute::sort_to_indices(
            batch.column(self.sort_keys[0].0),
            self.sort_keys[0].1,
        )?;
        
        // 根据索引重排
        let sorted_batch = arrow::compute::take(batch, &sorted_indices)?;
        
        Ok(sorted_batch)
    }
}

pub struct SimdJoinOperator {
    left_key: usize,
    right_key: usize,
}

impl SimdJoinOperator {
    pub fn process(&self, left: &RecordBatch, right: &RecordBatch) -> Result<RecordBatch, Error> {
        // SIMD Hash Join
        
        // 1. Build Hash Table（SIMD构建）
        let hash_table = self.build_hash_table(left)?;
        
        // 2. Probe（SIMD探测）
        let matched_indices = self.probe_hash_table(right, &hash_table)?;
        
        // 3. Join结果
        let joined = self.join_rows(left, right, &matched_indices)?;
        
        Ok(joined)
    }
    
    fn build_hash_table(&self, batch: &RecordBatch) -> Result<HashMap<u64, Vec<usize>>, Error> {
        // SIMD并行构建Hash Table
        let key_column = batch.column(self.left_key);
        
        // Arrow hash kernel（SIMD优化）
        let hashes = arrow::compute::hash(key_column)?;
        
        // 构建Hash Table
        let mut hash_table = HashMap::new();
        for (i, hash) in hashes.iter().enumerate() {
            hash_table.entry(*hash).or_insert(vec![]).push(i);
        }
        
        Ok(hash_table)
    }
}
```

---

### 4. 列式压缩优化

**压缩策略:**

```
列式压缩优化：
  1. 每列独立压缩（不同列不同算法）
  2. 选择最佳压缩算法（LZ4/ZSTD/Snappy）
  3. 压缩级别自适应（基于数据特征）
  4. 压缩统计（监控压缩率）
```

**组件设计:**

```rust
// be-storage/src/column_compressor.rs

pub struct ColumnCompressor {
    algorithms: Vec<CompressionAlgorithm>,
}

pub enum CompressionAlgorithm {
    LZ4 { level: u8 },
    ZSTD { level: u8 },
    Snappy,
    None,
}

impl ColumnCompressor {
    pub fn select_algorithm(stats: &ColumnStatistics) -> CompressionAlgorithm {
        // 基于统计选择最佳压缩
        
        // 1. 已编码列（低压缩率）
        if stats.encoding != EncodingType::Plain {
            return CompressionAlgorithm::None;  // 编码已经压缩
        }
        
        // 2. 高压缩率列（ZSTD）
        if stats.compression_ratio > 0.3 {
            return CompressionAlgorithm::ZSTD { level: 3 };
        }
        
        // 3. 中压缩率列（LZ4）
        if stats.compression_ratio > 0.5 {
            return CompressionAlgorithm::LZ4 { level: 1 };
        }
        
        // 4. 低压缩率列（Snappy）
        CompressionAlgorithm::Snappy
    }
    
    pub fn compress(&self, data: &[u8], algorithm: &CompressionAlgorithm) -> Result<Vec<u8>, Error> {
        match algorithm {
            CompressionAlgorithm::LZ4 { level } => {
                lz4::compress(data, *level)
            }
            CompressionAlgorithm::ZSTD { level } => {
                zstd::compress(data, *level)
            }
            CompressionAlgorithm::Snappy => {
                snap::compress(data)
            }
            CompressionAlgorithm::None => {
                Ok(data.to_vec())
            }
        }
    }
    
    pub fn decompress(&self, data: &[u8], algorithm: &CompressionAlgorithm) -> Result<Vec<u8>, Error> {
        match algorithm {
            CompressionAlgorithm::LZ4 { .. } => {
                lz4::decompress(data)
            }
            CompressionAlgorithm::ZSTD { .. } => {
                zstd::decompress(data)
            }
            CompressionAlgorithm::Snappy => {
                snap::decompress(data)
            }
            CompressionAlgorithm::None => {
                Ok(data.to_vec())
            }
        }
    }
}
```

---

## 📅 实施路线（3个月）

### Month 1: Arrow集成

**Week 1-2: Arrow数据格式**
- [ ] Arrow Schema定义
- [ ] Arrow RecordBatch使用
- [ ] Arrow IPC读写
- [ ] Arrow测试

**Week 3-4: Arrow存储层**
- [ ] ArrowSegmentWriter
- [ ] ArrowSegmentReader
- [ ] Arrow列式存储
- [ ] Arrow集成测试

**验收标准:**
```
- Arrow IPC读写速度：≥100MB/s
- Arrow零拷贝slice：性能验证
- Arrow内存布局：符合标准
```

---

### Month 2: 智能编码 + SIMD

**Week 1-2: 智能编码**
- [ ] Dictionary编码
- [ ] RLE编码
- [ ] Delta编码
- [ ] BitPacking编码
- [ ] 编码选择算法

**Week 3-4: SIMD集成**
- [ ] SimdFilterOperator
- [ ] SimdAggregateOperator
- [ ] SimdSortOperator
- [ ] SimdJoinOperator
- [ ] SIMD性能测试

**验收标准:**
```
- 编码压缩率：≥50%
- SIMD性能：≥5倍（vs scalar）
- 编码选择准确性：≥90%
```

---

### Month 3: 列式压缩 + 测试

**Week 1-2: 列式压缩**
- [ ] ColumnCompressor
- [ ] 压缩算法选择
- [ ] 压缩统计监控
- [ ] 压缩测试

**Week 3-4: 全链路测试**
- [ ] 列式存储全流程
- [ ] Scan吞吐测试
- [ ] 编码+压缩+SIMD集成
- [ ] 性能对比（vs改造列式）

**验收标准:**
```
- Scan吞吐：≥5M rows/sec（vs Doris: 1M）
- 编码+压缩率：≥70%
- SIMD性能：≥5倍
```

---

## 📊 性能预期对比

| 指标 | Doris（改造列式） | HarnessDB（原生列式） | 提升倍数 |
|------|-----------------|-------------------|---------|
| **Scan吞吐** | 1M rows/sec | 5-10M rows/sec | 5-10倍 |
| **编码压缩率** | 30% | 70% | 2倍改善 |
| **SIMD性能** | 部分 | 深度集成 | 5倍提升 |
| **内存效率** | 基准 | Arrow零拷贝 | 2倍提升 |
| **跨语言支持** | ❌ 无 | ✅ Arrow标准 | 生态成熟 |

---

## 📁 涉及文件

### 新建文件

```
be-storage/src/
├── arrow_format.rs            # Arrow格式集成（~400行）
├── arrow_writer.rs            # Arrow Segment写入（~300行）
├── arrow_reader.rs            # Arrow Segment读取（~300行）
├── smart_encoder.rs           # 智能编码（~600行）
├── column_compressor.rs       # 列式压缩（~300行）
└── encoding_stats.rs          # 编码统计（~200行）

be-execution/src/
├── simd_operator.rs           # SIMD算子（~500行）
├── simd_filter.rs             # SIMD Filter（~300行）
├── simd_aggregate.rs          # SIMD Aggregate（~400行）
├── simd_sort.rs               # SIMD Sort（~300行）
└── simd_join.rs               # SIMD Join（~400行）

tests/integration/
└── arrow_simd_test.rs         # Arrow+SIMD测试（~600行）
```

### 修改文件

```
Cargo.toml                     # 添加arrow依赖
be-storage/src/lib.rs          # 导出arrow模块
be-execution/src/lib.rs        # 导出simd模块
```

---

## 💡 创新价值

**这是高价值的创新点：**

1. ✅ **吞吐突破**：5-10倍（1M → 5-10M）
2. ✅ **编码智能**：自动选择最佳编码
3. ✅ **SIMD深度集成**：Arrow compute kernels
4. ✅ **Arrow生态**：跨语言支持（C++/Python/Java）
5. ✅ **零拷贝传输**：Arrow IPC

**列式存储原生设计是HarnessDB性能的核心！**

---

## 🔗 相关文档

- [创新路线总览](00-overview.md)
- [P0-01 异步架构](P0-async-architecture.md)
- [Apache Arrow官方文档](https://arrow.apache.org/)

---

## 📝 备注

**为什么选择Arrow生态？**

1. ✅ 列式原生设计（不是改造）
2. ✅ SIMD深度集成（Arrow compute kernels）
3. ✅ 跨语言支持（生态成熟）
4. ✅ 零拷贝传输（Arrow IPC）
5. ✅ Apache官方标准（技术领先）

**P1-00是HarnessDB列式存储的核心竞争力！**