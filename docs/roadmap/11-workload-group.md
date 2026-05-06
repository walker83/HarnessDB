# 资源管理 - WORKLOAD GROUP

## 概述
当前 CREATE WORKLOAD GROUP 语句无法解析，资源管理功能缺失。

## 现状分析
测试结果:
- `admin/04_resource_management.sql`: 100 errors (大部分 WORKLOAD GROUP 相关)

主要错误:
```
PARSE ERROR: syntax error at position 0: sql parser error: Expected: an object type after CREATE, found: WORKLOAD
Statement::CreateWorkloadGroup not found in parser
```

## 子任务

### Task 1: WORKLOAD GROUP 语法解析
- 添加 CreateWorkloadGroupStmt 到 Statement 枚举
- 添加 DropWorkloadGroupStmt 到 Statement 枚举
- 实现 CREATE WORKLOAD GROUP 语法解析
- 实现 DROP WORKLOAD GROUP 语法解析
- 验证: `admin/04_resource_management.sql` 解析部分通过

### Task 2: WORKLOAD GROUP 属性
- 支持 CPU_SHARE 配置
- 支持 MEMORY_LIMIT 配置
- 支持 QUERY_TIMEOUT 配置
- 支持 MAX_CONCURRENT_QUERIES 配置
- 验证: `admin/04_resource_management.sql` 属性部分通过

### Task 3: WORKLOAD GROUP 执行
- 实现 WORKLOAD GROUP 创建逻辑
- 实现查询到 WORKLOAD GROUP 的分配
- 支持 WORKLOAD GROUP 切换
- 验证: `admin/04_resource_management.sql` 执行部分通过

### Task 4: WORKLOAD GROUP 管理
- 实现 SHOW WORKLOAD GROUP 语句
- 实现 ALTER WORKLOAD GROUP 语句
- 支持 WORKLOAD GROUP 资源分配
- 验证: `admin/04_resource_management.sql` 管理语句部分通过

## 验收标准
- [ ] CREATE/DROP WORKLOAD GROUP 正常解析
- [ ] WORKLOAD GROUP 属性正确应用
- [ ] 查询可以分配到 WORKLOAD GROUP
- [ ] 资源管理测试通过率 > 80%

## 影响范围
- `fe-sql-parser`: WORKLOAD GROUP 语法解析
- `fe-sql-planner`: WORKLOAD GROUP 计划
- `fe-scheduler`: WORKLOAD GROUP 资源调度
- `be-execution`: 资源限制执行
