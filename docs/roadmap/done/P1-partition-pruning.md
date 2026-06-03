# P1-04: Partition Pruning（分区裁剪）

**优先级**: P1
**模块**: fe-sql-planner
**状态**: ❌ 未开始
**预计工期**: 1个月
**价值**: ✅ 中（查询优化）

---

## 📋 问题分析

### Doris的Partition Pruning

```java
// Doris: 多种分区裁剪策略
public class PartitionPruner {
    // 1. Range Partition Pruning
    public List<Long> pruneRangePartition(PartitionInfo info, List<Expr> conjuncts) {
        // 根据谓词裁剪Range分区
    }
    
    // 2. List Partition Pruning
    public List<Long> pruneListPartition(PartitionInfo info, List<Expr> conjuncts) {
        // 根据谓词裁剪List分区
    }
    
    // 3. Dynamic Partition Pruning
    public List<Long> pruneDynamicPartition(PartitionInfo info) {
        // 动态分区裁剪（运行时）
    }
}

// Doris有完整的：
// 1. Range/List/Dynamic分区裁剪
// 2. 多列分区裁剪
// 3. ZoneMap辅助裁剪
// 4. 谓词提取和转换
// 5. 分区统计信息
```

### HarnessDB的缺失

```
当前缺失：
  ❌ Partition Pruning未实现
  ❌ Range分区裁剪未实现
  ❌ List分区裁剪未实现
  ❌ 谓词提取未实现
  ❌ 分区统计未实现
  
影响：
  - 查询扫描所有分区（性能差）
  - 分区查询无法优化
  - 大表查询慢
```

---

## 🎯 核心组件设计

### 1. Range Partition Pruning

**Range分区裁剪逻辑:**

```
谓词分析 → 分区范围 → 交集计算 → 目标分区

例如：
  分区：p1 [1-100], p2 [101-200], p3 [201-300]
  谓词：id > 150 AND id < 250
  
  谓词范围：[150, 250]
  与分区交集：
    p1 [1-100]：无交集 → 裁剪
    p2 [101-200]：[150-200] → 保留
    p3 [201-300]：[201-250] → 保留
  
  目标分区：p2, p3
```

**组件设计:**

```rust
// fe-sql-planner/src/partition_pruning.rs

pub struct PartitionPruner {
    partition_info: PartitionInfo,
}

impl PartitionPruner {
    pub fn prune(&self, conjuncts: Vec<Expr>) -> Result<Vec<u64>, Error> {
        // 分区裁剪
        
        match self.partition_info.partition_type {
            PartitionType::Range => self.prune_range(conjuncts),
            PartitionType::List => self.prune_list(conjuncts),
            PartitionType::Hash => Ok(self.all_partitions()),  // Hash不裁剪
        }
    }
    
    fn prune_range(&self, conjuncts: Vec<Expr>) -> Result<Vec<u64>, Error> {
        // Range分区裁剪
        
        // 1. 提取分区列谓词
        let partition_preds = self.extract_partition_predicates(conjuncts)?;
        
        // 2. 计算谓词范围
        let predicate_range = self.calculate_predicate_range(partition_preds)?;
        
        // 3. 计算分区交集
        let matched_partitions = self.partition_info.partitions.iter()
            .filter(|partition| {
                // 检查谓词范围与分区范围是否有交集
                self.range_intersect(&predicate_range, &partition.range)
            })
            .map(|partition| partition.id)
            .collect();
        
        Ok(matched_partitions)
    }
    
    fn extract_partition_predicates(&self, conjuncts: Vec<Expr>) -> Result<Vec<Expr>, Error> {
        // 提取分区列相关的谓词
        
        let partition_column = self.partition_info.partition_column;
        
        conjuncts.iter()
            .filter(|expr| {
                // 检查表达式是否涉及分区列
                self.contains_column(expr, &partition_column)
            })
            .cloned()
            .collect()
    }
    
    fn calculate_predicate_range(&self, predicates: Vec<Expr>) -> Result<Range, Error> {
        // 计算谓词范围
        
        let mut range = Range::All;  // 初始范围：全范围
        
        for pred in predicates {
            match pred {
                Expr::BinaryOp { left, op, right } => {
                    let value = self.extract_value(right)?;
                    
                    match op {
                        BinaryOp::Gt => range.min = Some(value),  // id > 100
                        BinaryOp::Lt => range.max = Some(value),  // id < 200
                        BinaryOp::Ge => range.min = Some(value),  // id >= 100
                        BinaryOp::Le => range.max = Some(value),  // id <= 200
                        BinaryOp::Eq => {
                            // id = 100 → 单点范围
                            range.min = Some(value);
                            range.max = Some(value);
                        }
                    }
                }
                _ => continue,
            }
        }
        
        Ok(range)
    }
    
    fn range_intersect(&self, predicate_range: &Range, partition_range: &Range) -> bool {
        // 检查两个范围是否有交集
        
        // 茓谓词范围：[min, max]
        // 分区范围：[start, end]
        
        match (predicate_range.min, predicate_range.max) {
            (Some(pmin), Some(pmax)) => {
                // 茓谓词有范围 [pmin, pmax]
                match (partition_range.start, partition_range.end) {
                    (Some(smin), Some(smax)) => {
                        // 分区有范围 [smin, smax]
                        // 交集：max(pmin, smin) <= min(pmax, smax)
                        pmin <= smax && pmax >= smin
                    }
                    _ => true,  // 分区无范围，保留
                }
            }
            _ => true,  // 茓谓词无范围，保留所有
        }
    }
}

#[derive(Debug, Clone)]
pub struct Range {
    min: Option<ScalarValue>,
    max: Option<ScalarValue>,
}

#[derive(Debug, Clone)]
pub struct PartitionInfo {
    partition_type: PartitionType,
    partition_column: String,
    partitions: Vec<Partition>,
}

#[derive(Debug, Clone)]
pub struct Partition {
    id: u64,
    name: String,
    range: Range,
}
```

---

### 2. List Partition Pruning

**List分区裁剪逻辑:**

```
谓词分析 → 目标值列表 → 目标分区

例如：
  分区：p1 [1,2,3], p2 [4,5,6], p3 [7,8,9]
  茓谓词：id IN (2, 5, 8)
  
  目标值：{2, 5, 8}
  匹配分区：
    p1包含2 → 保留
    p2包含5 → 保留
    p3包含8 → 保留
  
  目标分区：p1, p2, p3
```

**组件设计:**

```rust
impl PartitionPruner {
    fn prune_list(&self, conjuncts: Vec<Expr>) -> Result<Vec<u64>, Error> {
        // List分区裁剪
        
        // 1. 提取分区列谓词
        let partition_preds = self.extract_partition_predicates(conjuncts)?;
        
        // 2. 提取目标值列表
        let target_values = self.extract_list_values(partition_preds)?;
        
        // 3. 匹配分区
        let matched_partitions = self.partition_info.partitions.iter()
            .filter(|partition| {
                // 检查分区值列表是否包含目标值
                partition.values.iter()
                    .any(|v| target_values.contains(v))
            })
            .map(|partition| partition.id)
            .collect();
        
        Ok(matched_partitions)
    }
    
    fn extract_list_values(&self, predicates: Vec<Expr>) -> Result<Vec<ScalarValue>, Error> {
        // 提取目标值列表
        
        let mut values = vec![];
        
        for pred in predicates {
            match pred {
                Expr::BinaryOp { left, op: BinaryOp::Eq, right } => {
                    // id = 100
                    let value = self.extract_value(right)?;
                    values.push(value);
                }
                Expr::InList { expr, list } => {
                    // id IN (1, 2, 3)
                    for value_expr in list {
                        let value = self.extract_value(value_expr)?;
                        values.push(value);
                    }
                }
                _ => continue,
            }
        }
        
        Ok(values)
    }
}

#[derive(Debug, Clone)]
pub struct Partition {
    id: u64,
    name: String,
    values: Vec<ScalarValue>,  // List分区值列表
}
```

---

### 3. 多列分区裁剪

**多列分区裁剪逻辑:**

```
多列分区：例如 (date, region)
  分区：p1 [(2024-01-01, 'east'), (2024-01-01, 'west')]
         p2 [(2024-01-02, 'east'), (2024-01-02, 'west')]
  
  茓谓词：date >= '2024-01-01' AND date <= '2024-01-02' AND region = 'east'
  
  分区列1：date → 茓谓词范围：[2024-01-01, 2024-01-02]
  分区列2：region → 目标值：'east'
  
  匹配：
    p1：date匹配 + region包含'east' → 保留
    p2：date匹配 + region包含'east' → 保留
```

**组件设计:**

```rust
impl PartitionPruner {
    fn prune_multi_column(&self, conjuncts: Vec<Expr>) -> Result<Vec<u64>, Error> {
        // 多列分区裁剪
        
        let partition_columns = self.partition_info.partition_columns;
        
        // 1. 为每个分区列提取谓词
        let column_predicates = partition_columns.iter()
            .map(|col| self.extract_column_predicates(conjuncts, col))
            .collect();
        
        // 2. 为每个分区列计算范围/值
        let column_ranges = column_predicates.iter()
            .map(|preds| self.calculate_column_range(preds))
            .collect();
        
        // 3. 匹配分区（所有列都匹配）
        let matched_partitions = self.partition_info.partitions.iter()
            .filter(|partition| {
                // 检查所有分区列是否匹配
                column_ranges.iter()
                    .enumerate()
                    .all(|(col_idx, range)| {
                        self.column_range_match(range, &partition.column_ranges[col_idx])
                    })
            })
            .map(|partition| partition.id)
            .collect();
        
        Ok(matched_partitions)
    }
}
```

---

### 4. Dynamic Partition Pruning

**动态分区裁剪逻辑:**

```
动态分区裁剪（运行时）：
  1. 编译时无法确定分区（例如子查询）
  2. 运行时根据实际数据裁剪
  3. 使用ZoneMap辅助裁剪
```

**组件设计:**

```rust
pub struct DynamicPartitionPruner {
    partition_info: PartitionInfo,
}

impl DynamicPartitionPruner {
    pub async fn prune_runtime(&self, subquery_result: Block) -> Result<Vec<u64>, Error> {
        // 运行时分区裁剪
        
        // 1. 从子查询结果提取值
        let values = self.extract_values_from_block(subquery_result)?;
        
        // 2. 动态匹配分区
        let matched_partitions = self.partition_info.partitions.iter()
            .filter(|partition| {
                // 检查分区是否包含子查询值
                partition.range.contains_any(&values)
            })
            .map(|partition| partition.id)
            .collect();
        
        Ok(matched_partitions)
    }
}
```

---

## 📅 实施路线（1个月）

### Week 1-2: Range/List裁剪

- [ ] PartitionPruner实现
- [ ] Range分区裁剪
- [ ] List分区裁剪
- [ ] 茓谓词提取
- [ ] 单元测试

**验收标准:**
```
- Range裁剪正确
- List裁剪正确
- 茓谓词提取正确
```

---

### Week 3-4: 多列裁剪 + Dynamic

- [ ] 多列分区裁剪
- [ ] Dynamic分区裁剪
- [ ] ZoneMap集成
- [ ] 性能测试
- [ ] 集成测试

**验收标准:**
```
- 多列裁剪正确
- Dynamic裁剪正确
- 查询性能提升明显
```

---

## 📊 性能预期

| 场景 | 无裁剪 | 有裁剪 | 性能提升 |
|------|--------|--------|---------|
| **分区表查询** | 扫描所有分区 | 扫描目标分区 | 10-100倍 |
| **大表查询** | 扫描100GB | 扫描10GB | 10倍 |
| **分区列谓词** | 全扫描 | 智能裁剪 | 明显改善 |

---

## 📁 涉及文件

### 新建文件

```
fe-sql-planner/src/
├── partition_pruning.rs       # 分区裁剪（~400行）
├── range_pruning.rs           # Range裁剪（~250行）
├── list_pruning.rs            # List裁剪（~200行）
├── dynamic_pruning.rs         # Dynamic裁剪（~150行）
└── predicate_extractor.rs     # 茓谓词提取（~200行）

tests/integration/
└── partition_pruning_test.rs  # 分区裁剪测试（~300行）
```

### 修改文件

```
fe-sql-planner/src/planner.rs  # 集成分区裁剪
fe-sql-planner/src/lib.rs      # 导出模块
```

---

## 💡 创新价值

**这是查询优化的关键：**

1. ✅ **Range/List裁剪**：智能分区选择
2. ✅ **多列裁剪**：复杂分区支持
3. ✅ **Dynamic裁剪**：运行时优化
4. ✅ **性能提升**：10-100倍（分区表）
5. ✅ **ZoneMap集成**：深度优化

**Partition Pruning是HarnessDB查询优化的关键！**

---

## 🔗 相关文档

- [创新路线总览](00-overview.md)
- [P1-01 Tablet/Replica](P1-tablet-replica.md)

---

## 📝 备注

**为什么Partition Pruning是P1？**

1. ✅ 查询优化关键（分区表性能）
2. ✅ 依赖P1-01（Partition管理）
3. ✅ 性能提升明显（10-100倍）
4. ✅ 标准优化功能（数据库必备）

**P1-04是HarnessDB查询优化的重要组成部分！**