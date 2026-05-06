# 索引管理

## 概述
当前 CREATE INDEX/DROP INDEX 语句无法解析，索引功能需要完善。

## 现状分析
测试结果:
- `ddl/02_index_rollup_operations.sql`: 182 errors
- `performance/03_index_performance.sql`: 158 errors
- `basic/02_table_ddl_positive.sql`: 36 errors (部分索引相关)

主要错误:
```
PARSE ERROR: Expected: an SQL statement, found: CREATE INDEX
Statement::CreateIndex not found in parser
```

## 子任务

### Task 1: 索引 DDL 解析
- 添加 CreateIndexStmt 到 Statement 枚举
- 添加 DropIndexStmt 到 Statement 枚举
- 实现 CREATE INDEX 语法解析
- 实现 DROP INDEX 语法解析
- 验证: `ddl/02_index_rollup_operations.sql` CREATE/DROP INDEX 部分通过

### Task 2: 索引元数据管理
- Catalog 中存储索引元信息
- 实现 SHOW INDEX 语句
- 支持 DESCRIBE table 显示索引信息
- 验证: `ddl/02_index_rollup_operations.sql` SHOW INDEX 部分通过

### Task 3: 索引执行实现
- 实现索引创建执行
- 实现索引删除执行
- 支持不同索引类型 (B-tree, Bitmap)
- 验证: `ddl/02_index_rollup_operations.sql` 执行部分通过

### Task 4: 倒排索引支持
- 支持 INVERTED INDEX 语法
- 实现全文搜索索引
- 验证: `advanced/03_inverted_index_positive.sql` 通过率 > 80%

## 验收标准
- [ ] CREATE INDEX 语句可以正常解析和执行
- [ ] DROP INDEX 语句可以正常解析和执行
- [ ] SHOW INDEX 返回正确的索引信息
- [ ] 索引 DDL 测试通过率 > 80%

## 影响范围
- `fe-sql-parser`: 添加索引语句解析
- `fe-sql-planner`: 索引计划生成
- `fe-catalog`: 索引元数据存储
- `be-storage`: 索引数据结构和查询
