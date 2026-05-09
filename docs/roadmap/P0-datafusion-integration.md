# P0: DataFusion 集成 — 废弃手写 SQL 引擎

## 背景

当前 RorisDB 的 SQL 引擎完全手写（Parser → Planner → Optimizer → Execution），存在以下根本问题：
- 表达式在 Planner 和 Execution 之间用字符串传递，需要序列化→反序列化，导致信息丢失（如 DISTINCT 标志）
- 聚合结果没有列名，下游只能靠索引猜测
- 表达式解析器用手写字符串匹配，遇到括号就放弃二元运算符解析
- 每新增一个 SQL 特性需要同时维护 3 处代码（Parser/Planner/Execution）

DataFusion 已在依赖中（v47）但只用于 logging，未真正接入。

## 方案

### 架构

```
MySQL Protocol → fe_main.rs (DDL dispatch)
                    ↓ (DML)
               SessionContext.sql()
                    ↓
            DataFusion (Parser → LogicalPlan → Optimizer → PhysicalPlan → Execution)
                    ↓
             RorisTableProvider (MemTable wrapper)
                    ↕
             RorisCatalogProvider → fe-catalog::CatalogManager
```

### 选择
- **一次性替换**：DML 查询全部走 DataFusion，不再维护手写引擎
- **DataFusion MemTable**：数据存内存，不用 be-storage（后续可加回）
- **单进程**：SessionContext 全在 FE 进程中

## 实施步骤

### Phase 1: 基础框架
1. [x] 创建 `fe-datafusion` crate
2. [ ] 实现类型转换 `DataType ↔ Arrow DataType`
3. [ ] 实现 `RorisCatalogProvider` (CatalogProvider + SchemaProvider)
4. [ ] 实现 `RorisTableProvider` (TableProvider wrapping MemTable)

### Phase 2: 查询通道
5. [ ] 改造 `fe_main.rs` 的 DML 路径，用 `SessionContext::sql()` 替换手写引擎
6. [ ] 实现 `RecordBatch → QueryResult` 转换
7. [ ] DDL (CREATE/DROP DATABASE/TABLE) 保留在 fe_main.rs，同步更新 CatalogProvider

### Phase 3: DML 支持
8. [ ] INSERT 支持 — 注册数据到 MemTable
9. [ ] DELETE 支持 — `ctx.sql("DELETE FROM ...")` 或手动过滤
10. [ ] UPDATE 支持 — 同 DELETE

### Phase 4: 验证 & 清理
11. [ ] 53/53 测试全部通过
12. [ ] 移除 fe-sql-parser (手写部分), fe-sql-planner, fe-expression, be-execution 依赖

## 影响范围

### 新增
- `crates/fe-datafusion/` — DataFusion 适配层

### 修改
- `roris-server/src/fe_main.rs` — DML 走 DataFusion, DDL 保留
- `roris-server/Cargo.toml` — 添加 fe-datafusion 依赖
- `Cargo.toml` — 添加 workspace member

### 保留不动
- `crates/mysql-protocol/` — MySQL 协议层
- `crates/fe-catalog/` — 元数据管理
- `crates/be-storage/` — 存储引擎（暂不用，后续可接回）
- `crates/common/`, `crates/types/` — 基础类型

### 废弃（但暂不删除）
- `crates/fe-sql-parser/` (手写 parser 部分)
- `crates/fe-sql-planner/`
- `crates/fe-expression/`
- `crates/be-execution/`
