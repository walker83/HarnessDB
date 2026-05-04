# P1: 备份恢复

**优先级**: P1
**模块**: fe-common, be-storage, data-io
**状态**: ❌ 未开始

## 背景

RorisDB 当前无数据备份恢复能力，数据丢失风险高。需要支持全量/增量备份，以及到 S3/HDFS/本地文件系统的存储。

## 任务清单

### 1. Repository 管理
- [ ] `CREATE REPOSITORY` 语法解析和元数据管理
- [ ] 支持本地文件系统 Repository
- [ ] 支持 S3 Repository
- [ ] 支持 HDFS Repository（长期）
- [ ] `DROP REPOSITORY` / `SHOW REPOSITORIES`

### 2. 全量备份
- [ ] `BACKUP DATABASE db TO repo` 语法
- [ ] 备份元数据（Database、Table、Partition Schema）
- [ ] 备份数据（导出 Tablet 数据到 Repository）
- [ ] 备份进度追踪
- [ ] 备份版本管理

### 3. 恢复
- [ ] `RESTORE DATABASE db FROM repo` 语法
- [ ] 恢复元数据（重建 Schema）
- [ ] 恢复数据（从 Repository 导入 Tablet）
- [ ] 恢复进度追踪
- [ ] 恢复后数据校验

### 4. 增量备份（长期）
- [ ] 基于 Snapshot 差异计算增量
- [ ] 增量数据导出
- [ ] 增量 + 全量合并恢复

### 5. 集成测试
- [ ] 备份 → 删库 → 恢复 → 验证数据一致性
- [ ] 大数据量备份性能
- [ ] 并发备份安全性

## 涉及文件

- `crates/fe-common/src/backup.rs` - 新建，备份恢复协调
- `crates/be-storage/src/` - Tablet 数据导出/导入
- `crates/data-io/src/` - 数据序列化
- `crates/fe-sql-parser/src/parser.rs` - 语法解析
