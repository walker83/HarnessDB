# P0: 分区支持

**优先级**: P0 (核心缺失)
**模块**: fe-catalog, fe-sql-planner, be-storage
**状态**: ❌ 未开始

## 背景

RorisDB 当前不支持任何分区策略。分区是 OLAP 数据库的基本功能，直接影响查询性能（通过 Partition Pruning 减少扫描量）和数据管理（分区级删除、过期数据清理等）。

## 任务清单

### 1. 分区元数据模型
- [ ] 在 `fe-catalog` 中定义 PartitionType 枚举（Range, List, Hash）
- [ ] 定义 PartitionSpec 结构，包含分区列、分区值范围、分区数量等
- [ ] 扩展 Table 结构，添加 partition_info 字段
- [ ] 支持二级分区（Composite Partition）

### 2. Range Partition
- [ ] 支持 `PARTITION BY RANGE(col)` 语法
- [ ] 支持显式定义 `PARTITION p1 VALUES LESS THAN ("2024-01-01")`
- [ ] LESS THAN 值比较和分区定位
- [ ] 支持多列 Range 分区

### 3. List Partition
- [ ] 支持 `PARTITION BY LIST(col)` 语法
- [ ] 支持 `PARTITION p1 VALUES IN ("北京", "上海")`
- [ ] IN 值匹配和分区定位

### 4. Hash Partition
- [ ] 支持 `PARTITION BY HASH(col) PARTITIONS N` 语法
- [ ] Hash 函数计算和分区定位
- [ ] 支持多列 Hash

### 5. 动态分区
- [ ] 支持按时间自动创建/删除分区
- [ ] `dynamic_partition` 属性配置
- [ ] 定时检查和自动管理任务

### 6. Partition Pruning (分区裁剪)
- [ ] 在 Planner 中分析 WHERE 条件
- [ ] 提取分区列的过滤条件
- [ ] 计算需要扫描的分区列表
- [ ] 将裁剪结果传递给 Scan 节点
- [ ] BE 端跳过不需要的 Tablet

### 7. SQL 解析支持
- [ ] Parser 支持 CREATE TABLE 中的 PARTITION BY 子句
- [ ] Parser 支持 ALTER TABLE ADD/DROP PARTITION
- [ ] Parser 支持 SHOW PARTITIONS

## 涉及文件

- `crates/fe-catalog/src/partition.rs` - 新建，分区元数据
- `crates/fe-sql-parser/src/parser.rs` - 分区语法解析
- `crates/fe-sql-planner/src/planner.rs` - 分区裁剪
- `crates/be-storage/src/tablet.rs` - Tablet 与分区映射
