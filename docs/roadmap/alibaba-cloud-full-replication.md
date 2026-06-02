# RorisDB 阿里云全功能复刻 - 总路线图

> 目标：将 RorisDB 打造为复刻阿里云全部大数据+数据库产品功能的超级数据库

## 已完成 ✅

| 功能 | 对应阿里云产品 | 状态 |
|------|---------------|------|
| MySQL 协议 | RDS/PolarDB MySQL | ✅ |
| MaxCompute REST + Tunnel | MaxCompute (ODPS) | ✅ |
| Hologres PG 协议 | Hologres | ✅ |
| DataFusion SQL 引擎 | 通用查询引擎 | ✅ |
| Parquet 存储 | 列式存储基础 | ✅ |
| 基础 DDL/DML | 通用 | ✅ |
| 审计日志 | ActionTrail | ✅ |
| 1780 测试通过 | - | ✅ |

## 13 个 Agent 实施计划

### Agent 1: 性能优化引擎 🔧
**对应**: PolarDB 高性能内核
- MySQL 协议批量写入 + 零拷贝优化
- Arrow → MySQL 直接编码 (消除 String 中间层)
- SessionContext 复用
- 连接池优化
- 查询缓存 (Query Cache)
- **目标**: 129K 行从 894ms → <30ms

### Agent 2: 高级 SQL 引擎 📊
**对应**: PolarDB SQL 兼容性 + AnalyticDB
- 存储过程 (CREATE PROCEDURE/FUNCTION)
- 触发器 (CREATE TRIGGER)
- 事件调度器 (CREATE EVENT)
- 序列 (CREATE SEQUENCE)
- 游标支持
- EXPLAIN ANALYZE 执行计划分析
- 更多内置函数 (200+)

### Agent 3: 复杂类型系统 🗂️
**对应**: MaxCompute 复杂类型 + Lindorm 多模型
- ARRAY<T> 完整实现 (读取/写入/函数)
- STRUCT<...> 完整实现
- MAP<K,V> 完整实现
- JSON/JSONB 类型 + JSON 函数 (json_extract, json_set 等)
- GEOSPATIAL 类型 (POINT/LINE/POLYGON)
- ENUM 和 SET 类型

### Agent 4: 索引与全文检索 🔍
**对应**: Elasticsearch + AnalyticDB 索引
- 倒排索引 (Inverted Index)
- Bloom Filter 索引
- 全文检索 MATCH() 语法
- 分词器框架 (中文/英文/IK)
- Bitmap 索引
- 向量索引 (HNSW/IVF) 用于 ANN 搜索
- CREATE INDEX 增强

### Agent 5: 分区与表模型 📋
**对应**: MaxCompute 分区 + AnalyticDB 表模型
- Range 分区 + Partition Pruning
- List 分区
- Hash 分区
- 复合分区 (Range + Hash/List)
- Duplicate Key 表模型
- Aggregate Key 表模型 (预聚合)
- Unique Key 表模型 (主键去重)
- 动态分区添加/删除

### Agent 6: 物化视图与查询优化 ⚡
**对应**: AnalyticDB 优化器 + Hologres 加速
- 物化视图 CREATE/DROP/REFRESH/SHOW
- 查询自动改写到物化视图
- CBO (基于代价的优化器)
- 统计信息收集 (ANALYZE TABLE)
- Runtime Filter (Join 优化)
- Colocate Join
- 分区裁剪优化
- 谓词下推增强

### Agent 7: 安全与权限控制 🔒
**对应**: RAM + DataWorks 数据安全
- RBAC 角色权限系统
- GRANT/REVOKE 完整实现
- 用户认证增强 (LDAP)
- SSL/TLS 加密连接
- 数据脱敏 (动态/静态)
- SQL 审计增强
- 列级权限控制
- 白名单/黑名单 IP 访问控制

### Agent 8: 数据湖集成 🏔️
**对应**: EMR + DLF (Data Lake Formation)
- External Catalog 框架
- Hive Metastore 集成
- Apache Iceberg 表读取
- Apache Hudi 表读取
- Delta Lake 表读取
- OSS/S3/HDFS 存储对接
- 外表 (Foreign Table) 增强
- 跨 Catalog 联邦查询

### Agent 9: 流处理与 CDC 🌊
**对应**: Flink/Realtime Compute + DTS
- Change Data Capture (CDC) 框架
- Binlog 兼容输出
- 流式 INSERT (Stream Load)
- Kafka Connector (Source + Sink)
- 实时物化视图 (增量刷新)
- Watermark 处理
- 窗口聚合 (Tumble/Hop/Session)
- SQL 流作业管理

### Agent 10: 时序引擎 ⏱️
**对应**: Lindorm TS + TSDB
- 时序表模型 (Time Series Table)
- 自动时间分区
- 数据降采样 (Downsampling)
- 数据保留策略 (Retention Policy)
- 连续查询 (Continuous Query)
- 时序函数 (interpolate, rate, delta)
- 超级标签 (Supertag)
- TTL 自动过期

### Agent 11: 多模型引擎 🔄
**对应**: Lindorm + Tair + TableStore + GDB
- Wide-Column 模型 (类似 HBase/TableStore)
- Key-Value 引擎 (类似 Tair/Redis)
  - Redis Protocol 兼容
  - 数据结构: String/Hash/List/Set/ZSet
- Document 模型 (JSON Document Store)
- Graph 模型 (属性图)
  - Gremlin 查询语言基础
  - 图算法 (最短路径/PageRank)
- 统一 Catalog 管理多模型

### Agent 12: 备份与高可用 🛡️
**对应**: DBS + DTS + PolarDB 高可用
- 全量备份 (物理/逻辑)
- 增量备份 (基于 WAL)
- Point-in-Time Recovery (PITR)
- 备份到 OSS/S3
- 克隆实例 (Clone Instance)
- Raft 共识 (FE HA)
- Master/Follower 架构
- 读写分离

### Agent 13: 运维与可观测性 📈
**对应**: CloudMonitor + ARMS + DMS
- Prometheus 指标导出
- 慢查询日志 + 分析
- Query Profile (详细执行分析)
- Web 管理控制台增强
  - SQL Editor
  - 表管理
  - 用户管理
  - 监控仪表板
- SHOW PROCESSLIST / KILL
- 配置热更新
- 资源队列 (Resource Queue)
- 多租户 + 资源隔离
- UDF/Plugin 框架

## 配置开关设计

```toml
# roris.toml
[server]
mysql_port = 9030
maxcompute_port = 9031
hologres_port = 15432
redis_port = 6379        # Agent 11
http_port = 8080

[features]
# 模块开关
sql_engine = true         # 基础SQL (始终开启)
streaming = false         # Agent 9: 流处理
timeseries = false        # Agent 10: 时序
multimodel = false        # Agent 11: 多模型
search = false            # Agent 4: 全文检索
datalake = false          # Agent 8: 数据湖

[security]
rbac = false              # Agent 7
tls = false
encryption_at_rest = false

[performance]
query_cache = true
materialized_view = false # Agent 6
runtime_filter = false    # Agent 6

[ha]
raft_enabled = false      # Agent 12
replication_factor = 1
```

## 执行顺序

1. **Agent 1** (性能优化) - 基础，所有后续功能都受益
2. **Agent 5** (分区与表模型) - 存储基础
3. **Agent 3** (复杂类型) - 类型系统基础
4. **Agent 6** (物化视图与优化) - 查询性能
5. **Agent 4** (索引与检索) - 查询加速
6. **Agent 7** (安全) - 企业级必备
7. **Agent 2** (高级SQL) - 兼容性
8. **Agent 10** (时序) - 独立功能
9. **Agent 8** (数据湖) - 集成
10. **Agent 9** (流处理) - 依赖存储层
11. **Agent 11** (多模型) - 独立引擎
12. **Agent 12** (备份HA) - 运维基础
13. **Agent 13** (运维) - 最后完善
