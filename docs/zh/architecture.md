# RorisDB 架构设计文档

## 系统架构概览

RorisDB 采用经典的 FE（Frontend）+ BE（Backend）分布式架构，基于 MPP（大规模并行处理）模型设计。

### 整体架构

```
┌─────────────────────────────────────────────────────────────────┐
│                        RorisDB Cluster                            │
│                                                                 │
│   ┌──────────────┐                        ┌──────────────────┐    │
│   │  FE (Rust)  │◄────── RPC ─────────►│   BE 1 (Rust)   │    │
│   │              │     (gRPC)            │                  │    │
│   │  ┌────────┐ │                        │  ┌────────────┐  │    │
│   │  │ Parser │ │                        │  │ Storage   │  │    │
│   │  │Planner │ │                        │  │ Engine    │  │    │
│   │  │Scheduler│ │                        │  │ (Segment) │  │    │
│   │  │Catalog │ │                        │  └────────────┘  │    │
│   │  └────────┘ │                        │  ┌────────────┐  │    │
│   │              │                        │  │ Execution  │  │    │
│   │  MySQL      │                        │  │ Pipeline   │  │    │
│   │  Protocol   │                        │  └────────────┘  │    │
│   └──────────────┘                        └──────────────────┘    │
│          │                                        │               │
│          │             ┌──────────────────┐    │               │
│          └───────────►│   BE 2 (Rust)   │◄───┘               │
│                         └──────────────────┘                   │
└─────────────────────────────────────────────────────────────────┘
```

## FE（Frontend）架构

### 核心模块

#### 1. SQL Parser（`fe-sql-parser`）
- 基于 `sqlparser` crate 实现
- 将 SQL 文本解析为 AST（抽象语法树）
- 支持 MySQL 兼容的 SQL 语法

#### 2. Query Planner（`fe-sql-planner`）
- AST → 逻辑计划 → 物理计划
- 基于规则的优化（RBO）：
  - 谓词下推（Predicate Pushdown）
  - 列裁剪（Column Pruning）
  - Limit 下推（Limit Pushdown）
  - Join 重排序（Join Reordering）

#### 3. Catalog（`fe-catalog`）
- 管理数据库、表、分区等元数据
- 当前使用内存存储（计划持久化到 EditLog + BDBJE）
- 支持数据库、表、视图的创建和查询

#### 4. Scheduler（`fe-scheduler`）
- Fragment 规划：将物理计划切分为可分布式执行的 Fragment
- 分布式调度：
  - 负载感知的 BE 节点选择
  - 轮询分配策略
  - 失败重调度

#### 5. Expression Engine（`fe-expression`）
- 向量化表达式求值
- 30+ 内置标量函数
- 批量处理，优化 CPU 缓存利用

#### 6. MySQL Protocol（`mysql-protocol`）
- 实现 MySQL 线协议
- 支持握手、认证、查询、结果集返回
- 兼容 MySQL 客户端工具

### FE 主要流程

```
SQL Text
    ↓
[Parser] → AST
    ↓
[Planner] → Logical Plan → Physical Plan
    ↓
[Optimizer] → Optimized Physical Plan
    ↓
[Scheduler] → Fragments + Execution Plan
    ↓
[Coordinator] → 分发到 BE 执行
    ↓
[Collector] → 收集结果返回给客户端
```

## BE（Backend）架构

### 核心模块

#### 1. Storage Engine（`be-storage`）
- **Tablet**：数据分片的基本单位
- **Rowset**：一次导入或 Compaction 产生的数据集合
- **Segment**：列式存储文件，包含多个 Column Page
- **MemTable**：内存缓冲区，用于实时写入

存储层次：
```
Table
  └── Partition (可选)
       └── Tablet (分片)
            └── Rowset
                 └── Segment
                      ├── Column Page (列式数据)
                      ├── ZoneMap Index (范围索引)
                      ├── BloomFilter Index (布隆过滤器)
                      └── Null Bitmap
```

#### 2. Segment Format（`be-segment`）
- 列式存储格式
- 支持多种编码：
  - **Plain Encoding**：直接存储
  - **RLE (Run-Length Encoding)**：游程编码，适合重复值
  - **LZ4 Compression**：轻量级压缩
- 索引支持：
  - **ZoneMap**：记录每列的最大值、最小值，用于范围过滤
  - **BloomFilter**：用于高基数列的等值过滤

#### 3. Execution Engine（`be-execution`）
- Pipeline 执行模型
- 向量化执行：批量处理数据
- 算子类型：
  - **Scan**：扫描表数据
  - **Filter**：过滤数据
  - **Project**：投影列
  - **Aggregate**：聚合计算
  - **Join**：连接操作（Hash Join、Nested Loop Join）
  - **Exchange**：数据交换（HashPartition、Broadcast、Gather）

#### 4. Compaction
- **Cumulative Compaction**：小文件合并，快速合并最新数据
- **Base Compaction**：大文件合并，优化查询性能
- 基于优先队列的调度策略

### BE 主要流程

```
[接收 Fragment]
    ↓
[Pipeline Builder] → 构建执行 Pipeline
    ↓
[Executor] → 向量化执行
    ↓
[Operator Chain] → Scan → Filter → Aggregate → ...
    ↓
[Result Sender] → 发送结果给 FE
```

## 数据类型系统（`types`）

### 基本类型
- **整数类型**：Int8, Int16, Int32, Int64
- **浮点类型**：Float32, Float64
- **字符串类型**：String (UTF-8)
- **日期时间**：Date, DateTime
- **布尔类型**：Boolean
- **空值**：Null（通过 Null Bitmap 跟踪）

### 向量化表示
- 每种类型对应一个 Vector 实现
- 批量存储数据，优化缓存局部性
- 支持 Null Bitmap 跟踪空值

```rust
pub enum Vector {
    Int64(Int64Vector),
    Float64(Float64Vector),
    String(StringVector),
    Boolean(BooleanVector),
    // ...
}
```

## 表达式系统（`fe-expression`）

### 表达式类型
- **Literal**：常量表达式
- **ColumnRef**：列引用
- **BinaryOp**：二元运算（+, -, *, /, etc.）
- **UnaryOp**：一元运算（NOT, -, etc.）
- **FunctionCall**：函数调用
- **Cast**：类型转换
- **Subquery**：子查询

### 向量化求值
表达式求值采用批量处理模式：

```rust
impl Expression for BinaryOpExpr {
    fn eval(&self, batch: &Batch) -> Vector {
        let left = self.left.eval(batch);
        let right = self.right.eval(batch);
        // 批量计算，一次处理多行
        vector_binary_op(&left, &right, self.op)
    }
}
```

## 查询执行流程

### 1. SQL 解析阶段
```sql
SELECT age, COUNT(*) FROM user WHERE age > 20 GROUP BY age
```
↓
```rust
AST: Query {
  select: [ColumnRef("age"), FunctionCall(COUNT, *)],
  from: Table("user"),
  filter: BinaryOp(ColumnRef("age"), >, Literal(20)),
  group_by: [ColumnRef("age")]
}
```

### 2. 逻辑计划阶段
```
LogicalPlan:
  Aggregate {
    group_by: [age],
    aggr_exprs: [COUNT(*)],
    input: Filter {
      predicate: age > 20,
      input: Scan { table: "user" }
    }
  }
```

### 3. 物理计划阶段
```
PhysicalPlan:
  HashAggregate {
    group_by: [age],
    aggr_exprs: [COUNT(*)],
    input: Filter {
      predicate: age > 20,
      input: TableScan { table: "user", projections: [age] }
    }
  }
```

### 4. Fragment 切分
```
Fragment 1 (BE Local):
  TableScan → Filter → HashAggregate (Partial)
  
Fragment 2 (BE Local):
  HashAggregate (Final) → Output
  
Exchange: HashPartition (by age) 从 Fragment 1 → Fragment 2
```

### 5. 分布式执行
- FE 将 Fragment 分发到多个 BE 节点
- 每个 BE 执行本地 Fragment
- 通过 Exchange 算子进行数据交换
- FE 收集最终结果返回给客户端

## 数据导入

### 支持格式
- **CSV**：逗号分隔值
- **JSON Lines**：每行一个 JSON 对象
- **Stream Load**：HTTP 流式导入

### 导入流程
```
客户端
  ↓
[FE] 接收导入请求
  ↓
[BE] 数据写入 MemTable
  ↓
[BE] Flush 到磁盘（生成 Rowset/Segment）
  ↓
[BE] 触发 Compaction（可选）
```

## 网络协议

### MySQL 协议（`mysql-protocol`）
- 支持 MySQL 握手和认证
- 支持 COM_QUERY、COM_PING、COM_QUIT 等命令
- 返回标准 MySQL 结果集格式

### gRPC 协议（`rpc`）
- FE 和 BE 之间通过 gRPC 通信
- 使用 Protocol Buffers 定义消息格式
- 主要服务：
  - `BackendService`：BE 注册、心跳、查询执行
  - `QueryService`：查询协调和执行

## 集群管理

### BE 节点管理
- BE 节点启动时向 FE 注册
- 定期发送心跳（包含负载信息）
- FE 跟踪每个 BE 的负载分数（load score）
- 查询调度时选择负载较低的 BE

### 高可用（规划中）
- 当前 FE 为单点
- 计划实现基于 Raft 的 FE 元数据复制
- BE 节点支持多副本

## 存储格式详解

### Segment 文件结构
```
Segment File:
├── Header (magic number, version)
├── Column Pages
│   ├── Page 1: column data + metadata
│   ├── Page 2: column data + metadata
│   └── ...
├── ZoneMap Index
│   ├── min value per column
│   ├── max value per column
│   └── null count
├── BloomFilter Index (可选)
│   └── bloom filter per column
└── Footer (offset table, checksum)
```

### Compaction 策略
1. **Cumulative Compaction**
   - 合并小 Rowset（最近导入的数据）
   - 快速合并，减少小文件数量
   - 触发条件：Rowset 数量超过阈值

2. **Base Compaction**
   - 合并大 Rowset（历史数据）
   - 深度合并，优化查询性能
   - 触发条件：Cumulative 文件过多或定期触发

## 性能优化技术

### 向量化执行
- 批量处理数据（Batch size = 1024 或更大）
- 减少函数调用开销
- 提高 CPU 缓存命中率

### 零拷贝
- 使用 Rust 的借用机制避免数据拷贝
- 在可能的地方使用引用而非所有权转移

### 延迟物化
- 只在必要时物化数据
- 尽早过滤数据，减少后续处理的数据量

### 索引优化
- ZoneMap：快速跳过不满足范围条件的 Segment
- BloomFilter：快速判断值是否存在于 Segment 中
- 列裁剪：只读取查询需要的列

## 与 Apache Doris 的架构对比

| 架构组件 | Apache Doris | RorisDB |
|---------|-------------|---------|
| 语言 | C++ | Rust |
| FE 元数据 | BDBJE | EditLog（计划 BDBJE） |
| 高可用 | BDBJE Master/Follower | Raft（规划中） |
| 存储格式 | Tablet/Rowset/Segment | Tablet/Rowset/Segment |
| 执行模型 | 向量化 + Pipeline | 向量化 + Pipeline |
| 网络协议 | MySQL + Thrift | MySQL + gRPC |
| 压缩算法 | zstd, LZ4, Zlib | LZ4（更多规划中） |

## 未来规划

### 短期（v0.2）
- Catalog 持久化（EditLog + BDBJE）
- 物化视图透明查询重写
- HA 高可用（Raft 共识）

### 中期（v0.3）
- 联邦查询（Hive/Iceberg/Hudi）
- 更多压缩算法（zstd, Zlib）
- 云原生模式（S3 共享存储）

### 长期
- Kubernetes Operator
- 多数据库事务
- UDF/UDAF 支持
- 行级安全
