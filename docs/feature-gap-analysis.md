# RorisDB 功能缺失分析

本文档对比 Apache Doris 与 RorisDB 的功能差异，列出 RorisDB 尚未实现的功能。

> 参考仓库：~/code/doris（Apache Doris）
> RorisDB 版本：v0.1.3
> 更新时间：2026/05/04

---

## 1. SQL 支持

### 1.1 DDL 语句

| 功能 | Apache Doris | RorisDB | 状态 |
|------|-------------|---------|------|
| CREATE DATABASE | ✅ | ✅ | 已完成 |
| DROP DATABASE | ✅ | ✅ | 已完成 |
| CREATE TABLE | ✅ | ✅ | 已完成 |
| DROP TABLE | ✅ | ✅ | 已完成 |
| TRUNCATE TABLE | ✅ | ✅ | 已完成 |
| ALTER TABLE | ✅ | ❌ | 缺失 |
| CREATE INDEX | ✅ | ❌ | 缺失 |
| DROP INDEX | ✅ | ❌ | 缺失 |
| CREATE MATERIALIZED VIEW | ✅ | 🚧 | 进行中 |
| DROP MATERIALIZED VIEW | ✅ | ❌ | 缺失 |
| CREATE VIEW | ✅ | ✅ | 已完成 |
| DROP VIEW | ✅ | ✅ | 已完成 |
| CREATE CATALOG | ✅ | ❌ | 缺失 |
| DROP CATALOG | ✅ | ❌ | 缺失 |
| REFRESH CATALOG | ✅ | ❌ | 缺失 |
| CREATE RESOURCE | ✅ | ❌ | 缺失 |
| CREATE WORKLOAD GROUP | ✅ | ❌ | 缺失 |
| CREATE REPOSITORY (备份) | ✅ | ❌ | 缺失 |

### 1.2 DML 语句

| 功能 | Apache Doris | RorisDB | 状态 |
|------|-------------|---------|------|
| INSERT | ✅ | ✅ | 已完成 |
| INSERT OVERWRITE | ✅ | ❌ | 缺失 |
| UPDATE | ✅ | ❌ | 缺失 |
| DELETE | ✅ | ❌ | 缺失 |
| Stream Load | ✅ | ✅ | 已完成 |
| Broker Load | ✅ | ❌ | 缺失 |
| Routine Load | ✅ | ❌ | 缺失 |
| S3 Load | ✅ | ❌ | 缺失 |

### 1.3 查询类型

| 功能 | Apache Doris | RorisDB | 状态 |
|------|-------------|---------|------|
| SELECT | ✅ | ✅ | 已完成 |
| UNION / UNION ALL | ✅ | ✅ | 已完成 |
| INTERSECT | ✅ | ✅ | 已完成 |
| EXCEPT | ✅ | ✅ | 已完成 |
| WITH (CTE) | ✅ | ✅ | 已完成 |
| 递归 CTE | ✅ | ✅ | 已完成 |
| GROUP BY | ✅ | ✅ | 已完成 |
| ORDER BY | ✅ | ✅ | 已完成 |
| LIMIT / OFFSET | ✅ | ✅ | 已完成 |
| HAVING | ✅ | ✅ | 已完成 |
| JOIN (INNER/LEFT/RIGHT/FULL/CROSS) | ✅ | ✅ | 已完成 |
| LATERAL VIEW | ✅ | ❌ | 缺失 |
| 子查询 (IN/EXISTS) | ✅ | ✅ | 已完成 |

### 1.4 窗口函数

| 函数 | Apache Doris | RorisDB | 状态 |
|------|-------------|---------|------|
| ROW_NUMBER | ✅ | ✅ | 已完成 |
| RANK | ✅ | ✅ | 已完成 |
| DENSE_RANK | ✅ | ✅ | 已完成 |
| LAG | ✅ | ✅ | 已完成 |
| LEAD | ✅ | ✅ | 已完成 |
| FIRST_VALUE | ✅ | ❌ | 缺失 |
| LAST_VALUE | ✅ | ❌ | 缺失 |
| SUM (窗口) | ✅ | ❌ | 缺失 |
| AVG (窗口) | ✅ | ❌ | 缺失 |
| COUNT (窗口) | ✅ | ❌ | 缺失 |

---

## 2. 存储引擎

### 2.1 存储格式

| 功能 | Apache Doris | RorisDB | 状态 |
|------|-------------|---------|------|
| Tablet/Rowset/Segment | ✅ | ✅ | 已完成 |
| 列式存储 (Vectorized) | ✅ | ✅ | 已完成 |
| Alpha Rowset (Legacy) | ✅ | ❌ | 缺失 |
| Beta Rowset (New) | ✅ | ❌ | 缺失 |
| Primary Key Index | ✅ | ❌ | 缺失 |

### 2.2 索引类型

| 功能 | Apache Doris | RorisDB | 状态 |
|------|-------------|---------|------|
| ZoneMap Index | ✅ | ✅ | 已完成 |
| BloomFilter Index | ✅ | ✅ | 已完成 |
| Bitmap Index | ✅ | ❌ | 缺失 |
| Inverted Index | ✅ | ❌ | 缺失 |
| NGram Bloom Filter | ✅ | ❌ | 缺失 |
| ANN Index (向量检索) | ✅ | ❌ | 缺失 |

### 2.3 压缩算法

| 功能 | Apache Doris | RorisDB | 状态 |
|------|-------------|---------|------|
| LZ4 | ✅ | ✅ | 已完成 |
| zstd | ✅ | ❌ | 缺失 |
| Zlib | ✅ | ❌ | 缺失 |
| RLE | ✅ | ✅ | 已完成 |

### 2.4 Compaction

| 功能 | Apache Doris | RorisDB | 状态 |
|------|-------------|---------|------|
| Base Compaction | ✅ | ✅ | 已完成 |
| Cumulative Compaction | ✅ | ✅ | 已完成 |
| Full Compaction | ✅ | ❌ | 缺失 |
| Single Replica Compaction | ✅ | ❌ | 缺失 |
| Segment Compaction | ✅ | ❌ | 缺失 |
| 时间序列 Compaction | ✅ | ❌ | 缺失 |
| 优先级调度 | ✅ | ✅ | 已完成 |

---

## 3. 查询优化

### 3.1 优化器

| 功能 | Apache Doris | RorisDB | 状态 |
|------|-------------|---------|------|
| RBO (基于规则) | ✅ | ✅ | 已完成 |
| CBO (基于代价) | ✅ (Nereids) | ❌ | 缺失 |
| 统计信息管理 | ✅ | ❌ | 缺失 |
| 列统计信息 | ✅ | ❌ | 缺失 |
| 直方图 | ✅ | ❌ | 缺失 |
| NDV (独立值数量) | ✅ | ❌ | 缺失 |

### 3.2 优化规则

| 功能 | Apache Doris | RorisDB | 状态 |
|------|-------------|---------|------|
| 谓词下推 | ✅ | ✅ | 已完成 |
| 列裁剪 | ✅ | ✅ | 已完成 |
| Limit 下推 | ✅ | ✅ | 已完成 |
| Join 重排序 | ✅ | ✅ | 已完成 |
| 子查询解嵌套 | ✅ | ✅ | 已完成 |
| 常量折叠 | ✅ | ❌ | 缺失 |
| Runtime Filter | ✅ | ❌ | 缺失 |
| Partition Pruning | ✅ | ❌ | 缺失 |
| Short Circuit Query | ✅ | ❌ | 缺失 |
| CTE 复用 | ✅ | ❌ | 缺失 |
| Outer Join 转 Inner Join | ✅ | ❌ | 缺失 |

### 3.3 执行计划优化

| 功能 | Apache Doris | RorisDB | 状态 |
|------|-------------|---------|------|
| Broadcast Join | ✅ | ✅ | 已完成 |
| Shuffle Join | ✅ | ✅ | 已完成 |
| Colocate Join | ✅ | ❌ | 缺失 |
| Bucket Shuffle Join | ✅ | ❌ | 缺失 |
| 物化视图透明改写 | ✅ | 🚧 | 进行中 |
| DPP (分布式处理) | ✅ | ✅ | 已完成 |

---

## 4. 分布与分区

### 4.1 分区策略

| 功能 | Apache Doris | RorisDB | 状态 |
|------|-------------|---------|------|
| Range Partition | ✅ | ❌ | 缺失 |
| List Partition | ✅ | ❌ | 缺失 |
| Hash Partition | ✅ | ❌ | 缺失 |
| 二級分区 (Composite) | ✅ | ❌ | 缺失 |
| 动态分区 | ✅ | ❌ | 缺失 |
| Auto Partition | ✅ | ❌ | 缺失 |

### 4.2 副本管理

| 功能 | Apache Doris | RorisDB | 状态 |
|------|-------------|---------|------|
| Tablet 副本数配置 | ✅ | ❌ | 缺失 |
| 副本自动分配 | ✅ | ❌ | 缺失 |
| 副本迁移 | ✅ | ❌ | 缺失 |
| 副本均衡 | ✅ | ❌ | 缺失 |
| Colocate Table | ✅ | ❌ | 缺失 |
| 单副本读取 | ✅ | ❌ | 缺失 |

### 4.3 负载均衡

| 功能 | Apache Doris | RorisDB | 状态 |
|------|-------------|---------|------|
| Tablet Scheduler | ✅ | ❌ | 缺失 |
| BeLoadRebalancer | ✅ | ❌ | 缺失 |
| DiskRebalancer | ✅ | ❌ | 缺失 |
| PartitionRebalancer | ✅ | ❌ | 缺失 |

---

## 5. 安全

### 5.1 认证

| 功能 | Apache Doris | RorisDB | 状态 |
|------|-------------|---------|------|
| MySQL 协议认证 | ✅ | ✅ | 已完成 |
| LDAP 认证 | ✅ | ❌ | 缺失 |
| Kerberos 认证 | ✅ | ❌ | 缺失 |
| AWS IAM 认证 | ✅ | ❌ | 缺失 |
| Token 认证 | ✅ | ❌ | 缺失 |

### 5.2 授权

| 功能 | Apache Doris | RorisDB | 状态 |
|------|-------------|---------|------|
| RBAC (角色权限) | ✅ | ❌ | 缺失 |
| 列级权限 | ✅ | ❌ | 缺失 |
| 行级权限 | ✅ | ❌ | 缺失 |
| Apache Ranger 集成 | ✅ | ❌ | 缺失 |
| Workload Group | ✅ | ❌ | 缺失 |

---

## 6. 数据类型

### 6.1 基础类型

| 类型 | Apache Doris | RorisDB | 状态 |
|------|-------------|---------|------|
| TINYINT | ✅ | ✅ | 已完成 |
| SMALLINT | ✅ | ✅ | 已完成 |
| INT | ✅ | ✅ | 已完成 |
| BIGINT | ✅ | ✅ | 已完成 |
| LARGEINT | ✅ | ❌ | 缺失 |
| FLOAT | ✅ | ✅ | 已完成 |
| DOUBLE | ✅ | ✅ | 已完成 |
| DECIMAL | ✅ | ❌ | 缺失 |
| CHAR | ✅ | ❌ | 缺失 |
| VARCHAR | ✅ | ✅ | 已完成 |
| STRING | ✅ | ✅ | 已完成 |
| DATE | ✅ | ✅ | 已完成 |
| DATETIME | ✅ | ✅ | 已完成 |
| TIME | ✅ | ❌ | 缺失 |
| TIMESTAMP | ✅ | ❌ | 缺失 |
| BOOLEAN | ✅ | ✅ | 已完成 |

### 6.2 复杂类型

| 类型 | Apache Doris | RorisDB | 状态 |
|------|-------------|---------|------|
| ARRAY | ✅ | ❌ | 缺失 |
| MAP | ✅ | ❌ | 缺失 |
| STRUCT | ✅ | ❌ | 缺失 |
| JSON | ✅ | ❌ | 缺失 |
| VARIANT | ✅ | ❌ | 缺失 |
| HLL (HyperLogLog) | ✅ | ❌ | 缺失 |
| BITMAP | ✅ | ❌ | 缺失 |
| IPV4 | ✅ | ❌ | 缺失 |
| IPV6 | ✅ | ❌ | 缺失 |
| BINARY | ✅ | ❌ | 缺失 |
| VARBINARY | ✅ | ❌ | 缺失 |
| QUANTILE_STATE | ✅ | ❌ | 缺失 |
| AGG_STATE | ✅ | ❌ | 缺失 |

---

## 7. 外部表和集成

### 7.1 外部 Catalog

| 功能 | Apache Doris | RorisDB | 状态 |
|------|-------------|---------|------|
| Hive Catalog | ✅ | 🚧 | 进行中 |
| Iceberg Catalog | ✅ | 🚧 | 进行中 |
| Hudi Catalog | ✅ | 🚧 | 进行中 |
| Paimon Catalog | ✅ | ❌ | 缺失 |
| JDBC Catalog | ✅ | ❌ | 缺失 |
| MaxCompute | ✅ | ❌ | 缺失 |
| Elasticsearch | ✅ | ❌ | 缺失 |
| MySQL 外部表 | ✅ | ❌ | 缺失 |
| PostgreSQL 外部表 | ✅ | ❌ | 缺失 |
| Trino/Presto | ✅ | ❌ | 缺失 |

---

## 8. 高可用

### 8.1 Frontend HA

| 功能 | Apache Doris | RorisDB | 状态 |
|------|-------------|---------|------|
| BDBJE 共识 | ✅ | 🚧 | 进行中 (Raft) |
| Master 选举 | ✅ | 🚧 | 进行中 |
| Quorum 协议 | ✅ | 🚧 | 进行中 |
| 事务协调器 | ✅ | ❌ | 缺失 |

### 8.2 Backend HA

| 功能 | Apache Doris | RorisDB | 状态 |
|------|-------------|---------|------|
| 心跳机制 | ✅ | ✅ | 已完成 |
| Tablet 自动修复 | ✅ | ❌ | 缺失 |
| 副本修复 | ✅ | ❌ | 缺失 |
| Publish Version Daemon | ✅ | ❌ | 缺失 |

### 8.3 其他 HA

| 功能 | Apache Doris | RorisDB | 状态 |
|------|-------------|---------|------|
| Binlog CDC | ✅ | ❌ | 缺失 |
| 多租户支持 | ✅ | ❌ | 缺失 |
| 热数据存储分层 | ✅ | ❌ | 缺失 |

---

## 9. 管理与监控

### 9.1 监控

| 功能 | Apache Doris | RorisDB | 状态 |
|------|-------------|---------|------|
| Information Schema | ✅ | ❌ | 缺失 |
| Metrics API (Prometheus) | ✅ | ❌ | 缺失 |
| FE/BE Metrics | ✅ | ❌ | 缺失 |
| Query Profile | ✅ | ❌ | 缺失 |
| Audit Log | ✅ | ❌ | 缺失 |

### 9.2 备份恢复

| 功能 | Apache Doris | RorisDB | 状态 |
|------|-------------|---------|------|
| Backup Handler | ✅ | ❌ | 缺失 |
| Repository (S3/HDFS/OSS) | ✅ | ❌ | 缺失 |
| 增量备份 | ✅ | ❌ | 缺失 |
| 恢复任务 | ✅ | ❌ | 缺失 |

### 9.3 集群管理

| 功能 | Apache Doris | RorisDB | 状态 |
|------|-------------|---------|------|
| FE 管理 (添加/删除) | ✅ | ✅ | 已完成 |
| BE 管理 | ✅ | ✅ | 已完成 |
| Broker 管理 | ✅ | ❌ | 缺失 |
| Resource 管理 | ✅ | ❌ | 缺失 |
| Workload Groups | ✅ | ❌ | 缺失 |

---

## 10. 特殊功能

| 功能 | Apache Doris | RorisDB | 状态 |
|------|-------------|---------|------|
| UDF (用户自定义函数) | ✅ | ❌ | 缺失 |
| UDAF (用户自定义聚合) | ✅ | ❌ | 缺失 |
| 存储过程 | ✅ | ❌ | 缺失 |
| 触发器 | ✅ | ❌ | 缺失 |
| Group Commit | ✅ | ❌ | 缺失 |
| Sequence Import | ✅ | ❌ | 缺失 |
| Partial Update (列更新) | ✅ | ❌ | 缺失 |
| CDC (Change Data Capture) | ✅ | ❌ | 缺失 |
| Kubernetes Operator | ✅ | ❌ | 缺失 |
| TPC-H 端到端 | ❌ | 🚧 | 进行中 |

---

## 优先级建议

### P0 (核心缺失，影响基本使用)

1. **UPDATE / DELETE** - 行级数据修改
2. **ALTER TABLE** - 表结构变更
3. **分区支持** - Range/List/Hash Partition
4. **CBO 优化器** - 基于代价的查询优化
5. **统计信息** - 列统计、直方图

### P1 (重要功能，影响性能)

1. **物化视图** - 查询加速
2. **Bitmap/HLL 类型** - 精确去重和计数
3. **Inverted Index** - 全文搜索
4. **Runtime Filter** - Join 优化
5. **备份恢复** - 数据安全

### P2 (完善功能，企业级)

1. **外部表** - Hive/Iceberg/Hudi
2. **LDAP 认证** - 企业集成
3. **Information Schema** - 系统监控
4. **UDF/UDAF** - 可扩展性
5. **多租户** - 资源隔离

### P3 (长期规划)

1. **向量检索 (ANN)** - AI 应用
2. **存储过程** - 复杂业务逻辑
3. **Kubernetes Operator** - 云原生部署
4. **Binlog CDC** - 数据同步
