# P3: 长期规划

**优先级**: P3 (长期规划)
**状态**: ❌ 未开始

以下功能属于长期规划，优先级较低但对企业级产品很重要。

---

## 1. 向量检索增强

已有基础 ANN Index 实现（L2 距离、KNN 搜索、线性扫描），需要优化：

- [ ] HNSW 索引（近似最近邻，适合大规模向量）
- [ ] IVF (Inverted File) 索引
- [ ] 余弦相似度度量
- [ ] 向量索引的增量更新
- [ ] GPU 加速（长期）

## 2. 存储过程

- [ ] `CREATE PROCEDURE` 语法
- [ ] 存储过程执行引擎
- [ ] 变量声明、控制流（IF/WHILE/FOR）
- [ ] 异常处理
- [ ] 与 SQL 执行集成

## 3. Kubernetes Operator

- [ ] CRD 定义: RorisDBCluster
- [ ] FE/BE StatefulSet 管理
- [ ] 自动扩缩容
- [ ] 滚动升级
- [ ] 监控集成（Prometheus Operator）
- [ ] Helm Chart

## 4. Binlog CDC

- [ ] Tablet 级别 Binlog 记录
- [ ] Binlog 消费 API
- [ ] Flink CDC Connector
- [ ] Kafka Connect Connector
- [ ] 增量数据同步到下游系统

## 5. 优化器长期目标

- [ ] Outer Join → Inner Join 自动转换
- [ ] Short Circuit Query（直接从 Tablet 读取结果，跳过分布式执行）
- [ ] CTE 复用（共享计算结果）
- [ ] Colocate Join（本地化 Join，减少网络传输）
- [ ] Bucket Shuffle Join（按 Bucket 分区减少 Shuffle 数据量）

## 6. 数据集成长期目标

- [ ] Paimon Catalog
- [ ] JDBC Catalog（MySQL、PostgreSQL 等外部数据库）
- [ ] Elasticsearch Catalog
- [ ] MaxCompute Catalog
- [ ] Trino/Presto Connector

## 7. 其他

- [ ] Group Commit（小批量高频写入优化）
- [ ] Sequence Import（数据版本管理）
- [ ] Partial Update（列级别部分更新）
- [ ] HLL (HyperLogLog) 类型实现
- [ ] IPV4/IPV6 类型
- [ ] BINARY/VARBINARY 类型
- [ ] QUANTILE_STATE / AGG_STATE 类型
