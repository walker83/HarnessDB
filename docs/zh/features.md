# RorisDB 功能特性

本文档详细介绍 RorisDB 当前支持的功能特性。

## 版本说明

- **当前版本**：v0.1.3
- **项目状态**：Proof-of-Concept（概念验证阶段）
- **License**：MIT / Apache-2.0

## 已完成功能（v0.1.3）

### SQL 解析和规划

| 功能 | 状态 | 说明 |
|------|------|------|
| **SQL Parser** | ✅ | MySQL 兼容的 SQL 解析（通过 sqlparser crate） |
| **查询规划器** | ✅ | AST → 逻辑计划 → 物理计划，基于规则的优化 |
| **优化器** | ✅ | 谓词下推、列裁剪、Limit 下推、Join 重排序 |

### 表达式引擎

| 功能 | 状态 | 说明 |
|------|------|------|
| **向量化表达式** | ✅ | 批量求值，提升 CPU 缓存利用率 |
| **标量函数** | ✅ | 30+ 内置函数（数学、字符串、日期等） |
| **类型转换** | ✅ | 隐式和显式类型转换 |

### 聚合函数

| 函数 | 状态 | 说明 |
|------|------|------|
| `COUNT` | ✅ | 计数（支持 `COUNT(*)` 和 `COUNT(DISTINCT)`） |
| `SUM` | ✅ | 求和 |
| `AVG` | ✅ | 平均值 |
| `MIN` | ✅ | 最小值 |
| `MAX` | ✅ | 最大值 |
| `GROUP_CONCAT` | ✅ | 字符串拼接 |

### 窗口函数

| 函数 | 状态 | 说明 |
|------|------|------|
| `ROW_NUMBER` | ✅ | 行号（无并列） |
| `RANK` | ✅ | 排名（有并列，跳号） |
| `DENSE_RANK` | ✅ | 密集排名（有并列，不跳号） |
| `LAG` | ✅ | 访问前 n 行 |
| `LEAD` | ✅ | 访问后 n 行 |

### 数学函数

| 函数 | 状态 | 说明 |
|------|------|------|
| 基础数学 | ✅ | sin, cos, tan, asin, acos, atan |
| 指数对数 | ✅ | exp, log, log10, sqrt, pow |
| 其他 | ✅ | pi, rand, abs, ceil, floor, round, sign |

### 数据类型

| 类型 | 状态 | 说明 |
|------|------|------|
| **整数类型** | ✅ | Int8, Int16, Int32, Int64 |
| **浮点类型** | ✅ | Float32, Float64 |
| **字符串类型** | ✅ | String (VARCHAR) |
| **日期时间** | ✅ | Date, DateTime |
| **布尔类型** | ✅ | Boolean |
| **空值** | ✅ | Null（通过 Null Bitmap 跟踪） |

### 存储引擎

| 功能 | 状态 | 说明 |
|------|------|------|
| **向量化存储** | ✅ | 列式内存布局，支持多种数据类型 |
| **Null Bitmap** | ✅ | 位集空值跟踪，支持快速 AND/OR/NOT 操作 |
| **Block** | ✅ | 批量列式数据（schema + vectors），支持投影/过滤/切片 |
| **Tablet** | ✅ | 数据分片的基本单位 |
| **Rowset** | ✅ | 一次导入或 Compaction 产生的数据集合 |
| **Segment** | ✅ | 列式存储文件，包含多个 Column Page |
| **ZoneMap Index** | ✅ | 记录每列的最大/最小值，用于范围过滤 |
| **BloomFilter Index** | ✅ | 概率过滤器，用于高基数列过滤 |
| **LZ4 压缩** | ✅ | 轻量级压缩算法 |
| **RLE 编码** | ✅ | 游程编码，适合重复值 |

### Compaction

| 功能 | 状态 | 说明 |
|------|------|------|
| **Cumulative Compaction** | ✅ | 小文件合并，快速合并最新数据 |
| **Base Compaction** | ✅ | 大文件合并，优化查询性能 |
| **优先级调度** | ✅ | 基于优先队列的 Compaction 调度 |

### 查询执行

| 功能 | 状态 | 说明 |
|------|------|------|
| **Pipeline 执行** | ✅ | 流水线执行引擎 |
| **向量化执行** | ✅ | 批量处理数据，提升性能 |
| **Scan 算子** | ✅ | 表数据扫描 |
| **Filter 算子** | ✅ | 数据过滤 |
| **Project 算子** | ✅ | 列投影 |
| **Aggregate 算子** | ✅ | 聚合计算（HashAggregate） |
| **Join 算子** | ✅ | 连接操作（Hash Join、Nested Loop Join） |
| **Exchange 算子** | ✅ | 数据交换（HashPartition、Broadcast、Gather） |

### 子查询和集合操作

| 功能 | 状态 | 说明 |
|------|------|------|
| **IN 子查询** | ✅ | `WHERE col IN (SELECT ...)` |
| **EXISTS 子查询** | ✅ | `WHERE EXISTS (SELECT ...)` |
| **NOT IN** | ✅ | `WHERE col NOT IN (SELECT ...)` |
| **NOT EXISTS** | ✅ | `WHERE NOT EXISTS (SELECT ...)` |
| **SemiJoin/AntiSemiJoin** | ✅ | 子查询优化执行 |
| **UNION** | ✅ | 并集（去重） |
| **UNION ALL** | ✅ | 并集（保留重复） |
| **INTERSECT** | ✅ | 交集 |
| **EXCEPT** | ✅ | 差集 |

### CTE 和视图

| 功能 | 状态 | 说明 |
|------|------|------|
| **CTE (WITH)** | ✅ | 公用表表达式，支持递归 |
| **CREATE VIEW** | ✅ | 视图创建和元数据管理 |
| **SHOW CREATE TABLE** | ✅ | 查看建表语句 |

### 数据导入和导出

| 功能 | 状态 | 说明 |
|------|------|------|
| **CSV 读写** | ✅ | CSV 格式导入导出 |
| **JSON Lines 解析** | ✅ | JSON Lines 格式解析 |
| **Stream Load** | ✅ | HTTP 流式导入框架 |

### 网络协议

| 功能 | 状态 | 说明 |
|------|------|------|
| **MySQL 协议** | ✅ | MySQL 线协议服务器（握手、认证、查询、结果集） |
| **gRPC FE-BE** | ✅ | FE 和 BE 之间的 gRPC 通信（tonic/prost） |

### 分布式查询

| 功能 | 状态 | 说明 |
|------|------|------|
| **Fragment 规划** | ✅ | 将物理计划切分为可分布式执行的 Fragment |
| **分布式调度** | ✅ | 负载感知的 BE 节点选择、轮询分配 |
| **查询协调器** | ✅ | 完整查询生命周期管理（plan → fragment → schedule → execute → collect） |
| **失败重调度** | ✅ | 查询失败时重新调度 |

### 集群管理

| 功能 | 状态 | 说明 |
|------|------|------|
| **BE 节点注册** | ✅ | BE 启动时向 FE 注册 |
| **心跳机制** | ✅ | BE 定期向 FE 发送心跳（包含负载信息） |
| **负载跟踪** | ✅ | FE 跟踪每个 BE 的负载分数（load score） |

### 客户端工具

| 功能 | 状态 | 说明 |
|------|------|------|
| **roris-cli** | ✅ | 命令行客户端（REPL），支持 SQL 解析和计划可视化 |
| **MySQL 客户端兼容** | ✅ | 可直接使用 mysql 命令行工具连接 |

### DDL 和 DML

| 功能 | 状态 | 说明 |
|------|------|------|
| **CREATE DATABASE** | ✅ | 创建数据库 |
| **DROP DATABASE** | ✅ | 删除数据库 |
| **ALTER DATABASE** | ✅ | 修改数据库属性 |
| **SHOW CREATE DATABASE** | ✅ | 查看建库语句 |
| **CREATE TABLE** | ✅ | 创建表（支持 DUPLICATE KEY、分区表） |
| **ALTER TABLE** | ✅ | 修改表（重命名列、注释、设置属性） |
| **DROP TABLE** | ✅ | 删除表 |
| **TRUNCATE TABLE** | ✅ | 快速清空表 |
| **INSERT** | ✅ | 插入数据（单条和多条） |
| **SELECT** | ✅ | 查询数据（支持复杂查询） |
| **CREATE VIEW** | ✅ | 创建视图 |
| **DROP VIEW** | ✅ | 删除视图 |
| **ALTER VIEW** | ✅ | 修改视图定义 |
| **SHOW CREATE VIEW** | ✅ | 查看建视图语句 |

### 分区支持

| 功能 | 状态 | 说明 |
|------|------|------|
| **Range 分区** | ✅ | 按值范围分区 |
| **List 分区** | ✅ | 按值列表分区 |
| **Hash 分区** | ✅ | 按哈希值分区 |
| **分区管理** | ✅ | 动态添加/删除分区 |

### 物化视图

| 功能 | 状态 | 说明 |
|------|------|------|
| **MV 框架** | ✅ | 物化视图创建和元数据管理 |
| **查询重写** | ✅ | 透明查询重写使用物化视图 |
| **MV 维护** | 🚧 | 自动刷新和一致性维护 |

### CBO 优化器

| 功能 | 状态 | 说明 |
|------|------|------|
| **代价模型** | ✅ | 基于代价的优化（CPU/I/O 估算） |
| **统计信息** | ✅ | 通过 ANALYZE TABLE 收集表统计信息 |
| **Join 重排序** | ✅ | 基于代价的 Join 顺序优化 |
| **计划选择** | ✅ | 基于统计信息的最优计划选择 |

### Runtime Filter

| 功能 | 状态 | 说明 |
|------|------|------|
| **Runtime Filter 下推** | ✅ | 动态过滤条件下推优化 Join |
| **Bloom Filter** | ✅ | 运行时 Bloom Filter 用于选择性 Join |
| **过滤器传播** | ✅ | 跨 Fragment 过滤器传播 |

### 外部 Catalog

| 功能 | 状态 | 说明 |
|------|------|------|
| **Catalog 框架** | ✅ | 外部 Catalog 框架（Hive/Iceberg/Hudi） |
| **联邦查询** | 🚧 | 直接查询外部数据源 |
| **元数据同步** | 🚧 | Catalog 元数据同步 |

### 认证框架

| 功能 | 状态 | 说明 |
|------|------|------|
| **MySQL 本地密码** | ✅ | MySQL 本地密码认证 |
| **LDAP 认证** | ✅ | 外部 LDAP 认证支持 |
| **Token 认证** | ✅ | 基于 Token 的认证 |
| **可插拔认证** | ✅ | 可插拔认证框架 |

### 备份恢复

| 功能 | 状态 | 说明 |
|------|------|------|
| **备份框架** | ✅ | 备份和恢复框架 |
| **增量备份** | 🚧 | 增量备份支持 |
| **远程存储** | 🚧 | 备份到 S3/GCS 远程存储 |

### 编解码和压缩

| 功能 | 状态 | 说明 |
|------|------|------|
| **LZ4 压缩** | ✅ | 改进的 LZ4 压缩和优化 |
| **编解码框架** | ✅ | 可扩展编解码框架 |
| **外部文件扫描** | ✅ | 直接扫描外部文件（CSV、JSON） |

## 进行中的功能

| 功能 | 状态 | 说明 |
|------|------|------|
| **物化视图** | 🚧 | 透明查询重写（Materialized Views） |
| **HA 高可用** | 🚧 | 基于 Raft 的 FE 元数据复制 |
| **Catalog 持久化** | 🚧 | EditLog + BDBJE 风格的元数据持久化 |
| **联邦查询** | 🚧 | Hive/Iceberg/Hudi 外部 Catalog |
| **云原生模式** | 🚧 | S3 共享存储、元数据服务 |

## 尚未实现的功能

| 功能 | 说明 |
|------|------|
| **UDF / UDAF** | 用户自定义函数和聚合函数 |
| **多数据库事务** | 跨数据库事务支持 |
| **行级安全** | 行级权限控制 |
| **工作负载管理** | 查询资源隔离和优先级 |
| **TPC-H 端到端** | 完整的 TPC-H 基准测试 |
| **Kubernetes Operator** | K8s 部署和管理工具 |
| **UPDATE / DELETE** | 数据更新和删除操作 |
| **外键约束** | 表间外键约束 |
| **存储过程** | 存储过程支持 |
| **触发器** | 触发器支持 |

## 与 Apache Doris 的功能对比

| 功能类别 | Apache Doris | RorisDB | 说明 |
|---------|-------------|---------|------|
| **语言** | C++ | Rust | 内存安全 |
| **SQL 兼容** | MySQL | MySQL | 通过 mysql-protocol |
| **存储格式** | Tablet/Rowset/Segment | Tablet/Rowset/Segment | 类似设计 |
| **索引** | ZoneMap, BloomFilter, Inverted | ZoneMap, BloomFilter | RorisDB 添加了 Inverted |
| **压缩算法** | zstd, LZ4, Zlib | LZ4 | 更多编解码器规划中 |
| **执行模型** | 向量化 + Pipeline | 向量化 + Pipeline | 相同理念 |
| **Compaction** | Cumulative + Base | Cumulative + Base | 相同策略 |
| **高可用** | BDBJE Master/Follower | Raft（规划中） | 不同共识机制 |
| **云模式** | Shared-nothing + S3 | Shared-nothing + S3（规划中） | |
| **物化视图** | ✅ | 🚧 | 规划中 |
| **联邦查询** | ✅ Hive/Iceberg/Hudi | 🚧 | 规划中 |
| **事务** | ✅ | ❌ | 规划中 |

## 性能特性

### 向量化执行

RorisDB 使用向量化执行模型，批量处理数据：

- **批量大小**：默认 1024 行/批次
- **CPU 缓存友好**：连续内存布局，减少 cache miss
- **SIMD 友好**：紧密循环，便于编译器优化

### 零拷贝

- 使用 Rust 借用机制避免不必要的数据拷贝
- 在可能的地方使用引用而非所有权转移

### 延迟物化

- 只在必要时物化数据
- 尽早过滤数据，减少后续处理的数据量

### 索引优化

- **ZoneMap**：快速跳过不满足范围条件的 Segment
- **BloomFilter**：快速判断值是否存在于 Segment 中
- **列裁剪**：只读取查询需要的列

## 可扩展性

### 水平扩展

- 通过添加 BE 节点实现存储和计算能力的水平扩展
- FE 负责查询规划和调度，BE 负责数据存储和执行

### 分布式查询

- Fragment 级别的并行执行
- 支持多种数据交换模式：
  - **HashPartition**：按哈希分区
  - **Broadcast**：广播小表
  - **Gather**：收集结果到单一节点

## 下一步

- 查看[产品概述](product-overview.md)了解 RorisDB 定位
- 阅读[架构设计文档](architecture.md)了解系统架构
- 参考[SQL 参考手册](sql-reference.md)学习 SQL 语法
- 查看[开发者指南](developer-guide.md)参与项目开发
