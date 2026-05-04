# RorisDB Roadmap

本目录包含 RorisDB 各模块的开发路线图，按优先级和模块拆分为独立文档，方便跟踪和执行。

## 文档索引

| 文档 | 优先级 | 模块 | 说明 |
|------|--------|------|------|
| [P0-dml-execution.md](P0-dml-execution.md) | P0 | DML 执行层 | UPDATE/DELETE 执行层实现 |
| [P0-partition.md](P0-partition.md) | P0 | 分区 | Range/List/Hash 分区 + Partition Pruning |
| [P0-fe-ha.md](P0-fe-ha.md) | P0 | 高可用 | FE Raft 共识、Master 选举、Quorum |
| [P1-materialized-view.md](P1-materialized-view.md) | P1 | 优化器 | 物化视图 DDL + 查询透明改写 |
| [P1-runtime-filter.md](P1-runtime-filter.md) | P1 | 优化器 | Runtime Filter Join 优化 |
| [P1-cbo.md](P1-cbo.md) | P1 | 优化器 | CBO 代价模型 + 统计信息收集 |
| [P1-rbac.md](P1-rbac.md) | P1 | 安全 | RBAC 角色权限控制 |
| [P1-backup-restore.md](P1-backup-restore.md) | P1 | 运维 | 备份恢复 |
| [P2-external-catalog.md](P2-external-catalog.md) | P2 | 集成 | Hive/Iceberg/Hudi 外部 Catalog |
| [P2-udf.md](P2-udf.md) | P2 | 扩展 | UDF/UDAF 框架 |
| [P2-auth.md](P2-auth.md) | P2 | 安全 | LDAP/Kerberos 认证 |
| [P2-multi-tenant.md](P2-multi-tenant.md) | P2 | 运维 | 多租户 + 资源隔离 |
| [P2-window-functions.md](P2-window-functions.md) | P2 | SQL | 窗口函数补全 |
| [P2-advanced-sql.md](P2-advanced-sql.md) | P2 | SQL | LATERAL VIEW / 高级 DDL |
| [P3-advanced-compaction.md](P3-advanced-compaction.md) | P3 | 存储 | 高级 Compaction 策略 |
| [P3-long-term.md](P3-long-term.md) | P3 | 综合 | 向量增强/存储过程/K8s/CDC 等长期规划 |

## 状态说明

- ❌ 未开始
- 🚧 进行中 / 部分实现
- ✅ 已完成

## 整体进度

详见 [feature-gap-analysis.md](../feature-gap-analysis.md) 中的完整对比表。
