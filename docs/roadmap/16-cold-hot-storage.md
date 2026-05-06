# 冷热存储分层

## 概述
当前冷热存储功能不支持。

## 现状分析
测试结果:
- `storage_engine/01_cold_hot_storage.sql`: 166 errors

主要缺失:
- STORAGE POLICY 语句
- ALTER TABLE SET STORAGE POLICY
- 存储分层配置

## 子任务

### Task 1: STORAGE POLICY 语法
- 添加 CreateStoragePolicyStmt 到 Statement 枚举
- 实现 CREATE STORAGE POLICY 语法
- 支持设置冷却时间
- 支持设置归档路径
- 验证: `storage_engine/01_cold_hot_storage.sql` POLICY 部分通过

### Task 2: 表级存储策略
- 支持 ALTER TABLE xxx SET ("storage_policy" = "xxx")
- 存储策略元数据
- 验证: `storage_engine/01_cold_hot_storage.sql` 表策略部分通过

### Task 3: 存储策略管理
- 实现 SHOW STORAGE POLICY 语句
- 实现 DROP STORAGE POLICY 语句
- 支持 CREATE/ALTER DATABASE SET STORAGE POLICY
- 验证: `storage_engine/01_cold_hot_storage.sql` 管理语句部分通过

### Task 4: 副本和分布策略
- 支持 SET ("replication_num" = "xxx")
- 支持 SET ("distribution" = "xxx")
- 支持 SET ("replication_allocation" = "xxx")
- 验证: `storage_engine/04_replication_distribution.sql` 副本策略部分通过

## 验收标准
- [ ] CREATE STORAGE POLICY 正常解析
- [ ] 表可以设置存储策略
- [ ] 存储分层功能基本可用
- [ ] 冷热存储测试通过率 > 80%

## 影响范围
- `fe-sql-parser`: STORAGE POLICY 语法解析
- `fe-catalog`: 存储策略元数据
- `be-storage`: 冷热数据管理
