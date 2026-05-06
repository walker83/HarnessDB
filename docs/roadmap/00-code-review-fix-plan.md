# 代码审查修复计划

> 基于 2026-05-06 深度代码评审，共发现 ~80 个问题（22 Critical / 30 High / 20 Medium / 10 Low）
> 目标：按优先级分阶段修复，每阶段有明确的验收标准

---

## 阶段概览

```
Phase 0 (数据安全)  →  Phase 1 (查询正确性)  →  Phase 2 (分布式执行)  →  Phase 3 (性能&稳定性)  →  Phase 4 (SQL兼容性)
   2-3 周                2-3 周                    2-3 周                   持续                      持续
```

---

## Phase 0: 数据安全基线 (P0-Critical)

> **目标**: 存储层能安全持久化数据，崩溃不丢数据，重启能恢复。
> **预计工期**: 2-3 周
> **验收标准**: 写入数据 → kill -9 进程 → 重启 → 数据完整可读

### 0.1 启动恢复机制
- **问题**: `StorageEngine::open()` 不加载磁盘数据 (#2)
- **修复内容**:
  - 扫描 `data_dir` 下所有 `tablet_*` 目录
  - 每个目录加载 `rowset_*.json` 元数据
  - 重建 `DashMap<TabletId, Tablet>` 内存结构
  - 检测并清理孤儿文件
- **涉及文件**: `be-storage/src/engine.rs`, `be-storage/src/tablet.rs`
- **测试**: 手动写入数据 → 重启 → 验证可读

### 0.2 WAL (Write-Ahead Log)
- **问题**: 无 WAL，崩溃丢数据 (#1)
- **修复内容**:
  - 每个 Tablet 一个 WAL 文件
  - write_batch 前先 append WAL (批量写)
  - 启动时回放 WAL 恢复 memtable
  - flush 成功后截断/归档 WAL
- **涉及文件**: 新增 `be-storage/src/wal.rs`, 修改 `tablet.rs`, `engine.rs`
- **测试**: 写入 → kill -9 → 重启 → 数据完整

### 0.3 Flush 原子化
- **问题**: Flush 三步不原子 (#6)
- **修复内容**:
  - segment 写入临时文件 `seg_*.dat.tmp`
  - metadata 写入 `rowset_*.json.tmp`
  - fsync 后 rename 为正式文件名
  - 最后更新内存状态
- **涉及文件**: `be-storage/src/tablet.rs`
- **测试**: flush 中途 kill → 重启 → 无孤儿文件，数据完整

### 0.4 错误传播
- **问题**: segment 读取错误/解压错误被静默吞掉 (#9, #10)
- **修复内容**:
  - `Tablet::read()` 中 segment 读取失败返回 `Err`
  - `codec::decode()` 解压失败返回 `Err` 而非 fallback
  - `ScanExecNode` 读取失败返回错误而非空 block (#59)
  - `execute_plan` 错误向上传播 (#54)
- **涉及文件**: `be-storage/src/tablet.rs`, `be-segment/src/codec.rs`, `be-execution/src/exec_node.rs`, `be-execution/src/planner.rs`

### 0.5 MemTable::to_block 修复
- **问题**: filter_map 丢 null 行 (#4)
- **修复内容**:
  - 用显式迭代检查 `is_null()` 
  - null 值 push `None` 到列向量
  - 确保所有列等长
- **涉及文件**: `be-storage/src/tablet.rs:143-220`
- **测试**: 写入含 null 的数据 → to_block → 验证行数和 null 位图

### 0.6 ZoneMap 反序列化修复
- **问题**: deserialize_scalar 总返回 Binary (#3)
- **修复内容**:
  - `ZoneMap` 存储列类型信息
  - 按类型反序列化 min/max
  - `compare_scalars` 正确比较类型化值
- **涉及文件**: `be-storage/src/index.rs`
- **测试**: 写入数据 → 验证 ZoneMap 裁剪生效

---

## Phase 1: 查询正确性 (P0-Critical)

> **目标**: 单机 SELECT/INSERT/UPDATE/DELETE 返回正确结果。
> **预计工期**: 2-3 周
> **验收标准**: TPC-H SF0.1 的 22 条查询返回正确结果

### 1.1 表达式求值引擎
- **问题**: Filter/Project/JOIN 条件存为字符串未执行 (#43, #44, #52)
- **修复内容**:
  - 定义 `ExecExpr` 类型化表达式树（非字符串）
  - 实现向量化求值：算术、比较、逻辑、字符串、日期
  - FilterExecNode 调用表达式求值
  - ProjectExecNode 执行列投影和表达式计算
  - HashJoinExecNode 用表达式求值 join 条件
- **涉及文件**: 新增 `be-execution/src/expr_engine.rs`, 重写 `exec_node.rs` 的 Filter/Project/Join
- **测试**: 单元测试覆盖每种运算符

### 1.2 NULL 传播
- **问题**: 算术/比较/逻辑不处理 NULL (#22)
- **修复内容**:
  - 所有运算检查 null bitmap
  - 任一操作数为 NULL → 结果为 NULL
  - 严格遵循 SQL 三值逻辑
- **涉及文件**: `fe-expression/src/evaluator.rs`, 新的 `expr_engine.rs`

### 1.3 CaseWhen/coalesce/nullif 修复
- **问题**: 返回 1 元素向量 (#23)
- **修复内容**:
  - 从完整 result 数组构建输出向量
  - 长度等于输入向量长度
- **涉及文件**: `fe-expression/src/evaluator.rs:103`, `fe-expression/src/functions.rs:231,245`

### 1.4 日期类型修复
- **问题**: 日期解析和序列化错误 (#46, #47)
- **修复内容**:
  - 使用 `chrono::NaiveDate::parse_from_str()` 解析
  - 存储为 epoch 天数 (i32)
  - MySQL 协议正确格式化输出
  - predicate_parser 日期比较用 epoch 天数
- **涉及文件**: `be-execution/src/predicate_parser.rs`, `mysql-protocol/src/packet.rs`

### 1.5 聚合函数类型分发
- **问题**: 只处理 Int64 (#48)
- **修复内容**:
  - sum/min/max/avg 按实际类型分发
  - 支持 Int8/16/32/64, Float32/64, Decimal
  - AVG 返回 Float64
- **涉及文件**: `be-execution/src/exec_node.rs:645-692`

### 1.6 Planner 传参修复
- **问题**: Aggregate/Sort/Join 参数为空 (#45)
- **修复内容**:
  - 从 PlanNode 提取 group_by/aggregates/sort_keys/join_keys
  - 转换为列索引 + 表达式
  - 传递给 ExecNode
- **涉及文件**: `be-execution/src/planner.rs:113-161`

### 1.7 SQL 注入修复
- **问题**: DESCRIBE/SHOW 拼接用户输入 (#25)
- **修复内容**:
  - 使用参数化谓词或标识符转义
- **涉及文件**: `fe-sql-planner/src/planner.rs:176-177`

### 1.8 DELETE tombstone 机制
- **问题**: DELETE 不持久 (#5)
- **修复内容**:
  - 每个 Rowset 维护 delete bitmap
  - 写 WAL 时记录 delete 操作
  - 读取时检查 delete bitmap 过滤已删行
  - Compaction 时物理清理
- **涉及文件**: `be-storage/src/tablet.rs`, `be-storage/src/compaction.rs`, `be-storage/src/rowset.rs`

---

## Phase 2: 分布式执行通路 (P1-High)

> **目标**: FE 能调度查询到 BE，BE 能执行并返回结果。
> **预计工期**: 2-3 周
> **验收标准**: 2 个 BE 节点 + 1 个 FE 节点，SELECT 查询分布式执行返回正确结果

### 2.1 BE Fragment 执行
- **问题**: BE gRPC 服务是空壳 (#50)
- **修复内容**:
  - 反序列化 `ExecPlanFragmentRequest` 中的计划
  - 创建执行 Pipeline
  - 调用 `pipeline.execute()` 执行
  - 结果缓冲到 fragment state
- **涉及文件**: `rpc/src/be_service.rs`, `roris-server/src/be_main.rs`

### 2.2 FE→BE 调度
- **问题**: 协调器模拟 (#49), FE 不走分布式 (#51)
- **修复内容**:
  - Coordinator 序列化 fragment → gRPC 发送到 BE
  - 调用 `fetch_data` 流式拉取结果
  - Exchange 节点合并多 BE 结果
  - FE 的 SELECT/INSERT 路由到 Coordinator
- **涉及文件**: `fe-scheduler/src/coordinator.rs`, `roris-server/src/fe_main.rs`

### 2.3 HashPartition 正确路由
- **问题**: 全互联 (#62)
- **修复内容**:
  - 每个子实例只发到 hash 匹配的父实例
- **涉及文件**: `fe-scheduler/src/scheduler.rs:371-393`

### 2.4 Fragment 并行度推断
- **问题**: 硬编码 1 (#63)
- **修复内容**:
  - Scan fragment 根据 tablet 数量推断并行度
  - 上游 fragment 根据子 fragment 并行度决定
- **涉及文件**: `fe-scheduler/src/fragment.rs`

---

## Phase 3: 性能与稳定性 (P1)

> **目标**: 消除关键性能瓶颈，内存可控。
> **预计工期**: 持续迭代

### 3.1 Compaction 修复
- **问题**: 竞争 + 磁盘泄漏 + 不去重 (#7)
- **修复内容**:
  - 原子交换 rowset（write lock 内 remove + add）
  - compaction 后删除旧 segment 文件
  - key 列去重（保留最新版本）
  - crash-safe: manifest 记录 compaction 状态
- **涉及文件**: `be-storage/src/engine.rs`, `be-storage/src/compaction.rs`

### 3.2 Auto-Flush 竞争修复
- **问题**: 锁间隙丢数据 (#8)
- **修复内容**:
  - 原子 swap memtable：安装新空 memtable，旧的无锁 flush
- **涉及文件**: `be-storage/src/tablet.rs`

### 3.3 内存管理
- **问题**: 无全局预算，Sort/Agg 无限缓冲 (#74, #58)
- **修复内容**:
  - MemoryTracker 用 `compare_exchange` 修复竞争
  - 执行节点接入 MemoryTracker
  - Sort/Agg 超限时 spill to disk
- **涉及文件**: `fe-scheduler/src/memory.rs`, `be-execution/src/exec_node.rs`

### 3.4 Segment 读取优化
- **问题**: 全文件读入内存 (#15), ZoneMap 不用 (#3 已修)
- **修复内容**:
  - 读取前检查 ZoneMap，跳过不匹配的 page
  - 使用 seek-based 读取或 mmap
  - 只加载需要的列
- **涉及文件**: `be-storage/src/segment/reader.rs`

### 3.5 Sort 批量化
- **问题**: O(n²) 逐行构建 (#57)
- **修复内容**:
  - 用 gather bitmap 批量提取排序后行
- **涉及文件**: `be-execution/src/exec_node.rs:927-935`

### 3.6 Catalog 一致性
- **问题**: Table ID=0 (#69), TOCTOU (#70), 不原子 (#71), 绕过 edit log (#72)
- **修复内容**:
  - CatalogManager 用 AtomicU64 生成唯一 table ID
  - 去掉外层 StdRwLock，直接用 DashMap
  - save 用 temp file + rename
  - 所有修改走 CatalogWriter 记录 edit log
  - INSERT 共享 catalog 不新建
- **涉及文件**: `fe-catalog/src/catalog.rs`, `roris-server/src/fe_main.rs`

### 3.7 事务原子性
- **问题**: 部分提交不回滚 (#76), Update 非原子 (#75)
- **修复内容**:
  - WAL 事务标记（begin/commit/rollback）
  - commit 失败时回滚
  - Update: 先写新数据再删旧数据，或用 tombstone

---

## Phase 4: SQL 兼容性增强 (P2)

> **目标**: 支持 SQL:2003 标准常用语法。
> **预计工期**: 持续迭代

### 4.1 Parser 修复
- Set 操作 UNION/EXCEPT/INTERSECT (#26)
- 未处理表达式返回错误而非 Wildcard (#27)
- 未知运算符返回错误而非 Eq (#28)
- CTE 解析所有表 (#38)
- DISTINCT 标志保留 (#39)
- SELECT DISTINCT → 加 Aggregate 节点 (#40)

### 4.2 谓词下推增强
- 推过 JOIN 节点 (#32)
- 推过 Project 时重映射列 (#33)

### 4.3 Runtime Filter 修复
- 匹配 Join 节点或 planner 转换为 HashJoin (#30)

### 4.4 物化视图修复
- 重写时保留 WHERE/GROUP BY (#29)
- 验证 MV schema 覆盖查询需求

### 4.5 Cost 模型改进
- Join selectivity 用 NDV 而非硬编码 (#34)
- Aggregate selectivity 用 group-by cardinality
- 谓词 selectivity 正确解析操作符

### 4.6 窗口函数 (#41)
- 新增 Window PlanNode
- 支持 PARTITION BY + ORDER BY + Frame
- row_number/rank/dense_rank/lag/lead 接入

### 4.7 子查询去关联
- EXISTS/IN key 推断修复 (#24)
- 相关子查询 decorrelation

### 4.8 MySQL 协议增强
- 错误返回 ERR 包 (#67)
- SELECT 无 FROM 表达式求值 (#66)
- COM_STMT_EXECUTE 参数绑定 (#68)
- SET 语句跟踪 (#65)

---

## 执行优先级总览

```
┌──────────────────────────────────────────────────────────┐
│ Phase 0: 数据安全 (Week 1-3)                             │
│   ✦ 启动恢复 → WAL → Flush 原子化 → 错误传播 →         │
│     to_block 修复 → ZoneMap 修复                         │
├──────────────────────────────────────────────────────────┤
│ Phase 1: 查询正确性 (Week 3-6)                           │
│   ✦ 表达式引擎 → NULL 传播 → 日期修复 →                 │
│     聚合类型 → Planner 传参 → SQL注入 → DELETE tombstone │
├──────────────────────────────────────────────────────────┤
│ Phase 2: 分布式执行 (Week 6-9)                           │
│   ✦ BE Fragment 执行 → FE→BE 调度 →                    │
│     HashPartition 路由 → 并行度推断                      │
├──────────────────────────────────────────────────────────┤
│ Phase 3: 性能稳定性 (Week 9+)                            │
│   ✦ Compaction 修复 → 内存管理 → Segment 优化 →         │
│     Catalog 一致性 → 事务原子性                          │
├──────────────────────────────────────────────────────────┤
│ Phase 4: SQL 兼容性 (持续)                               │
│   ✦ Parser 修复 → 谓词下推 → Runtime Filter →           │
│     窗口函数 → 子查询 → MySQL 协议                       │
└──────────────────────────────────────────────────────────┘
```

---

## 每阶段验收 Checklist

### Phase 0 验收
- [ ] 写入 10000 行 → kill -9 → 重启 → 行数完整
- [ ] flush 中途 kill → 无孤儿文件
- [ ] segment 文件损坏 → 查询返回 Error 而非错误结果
- [ ] ZoneMap 裁剪有效（验证读的 page 数 < 总 page 数）

### Phase 1 验收
- [ ] `SELECT * FROM t WHERE id > 10` 过滤正确
- [ ] `SELECT a + b, a * 2 FROM t` 计算正确
- [ ] NULL + 5 = NULL, NULL = NULL = NULL
- [ ] `SELECT DATE '2024-01-15'` 正确返回
- [ ] `SELECT SUM(score) FROM t` (score 是 Float64) 正确
- [ ] `DELETE FROM t WHERE id = 1` → flush → 重启 → 行仍不存在

### Phase 2 验收
- [ ] 2 BE + 1 FE，`SELECT COUNT(*) FROM t` 返回正确总数
- [ ] JOIN 查询分布式执行，结果与单机一致
- [ ] 单 BE 宕机，查询自动 reschedule

### Phase 3 验收
- [ ] 大表 compaction 后磁盘空间回收
- [ ] 内存使用不超过配置上限
- [ ] TPC-H SF1 全部 22 条查询完成

---

## 建议下一步

1. **立即开始 Phase 0** — 先提交当前代码 (git commit)
2. Phase 0.4 (错误传播) 改动最小、风险最低，建议第一个动手
3. Phase 0.1 (启动恢复) 和 0.2 (WAL) 是最大的两个任务，可并行
4. Phase 0.5 (to_block) 和 0.6 (ZoneMap) 是小修复，穿插进行
