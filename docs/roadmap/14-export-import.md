# EXPORT/IMPORT 数据导出导入

## 概述
当前 EXPORT 和 IMPORT 语句无法解析。

## 现状分析
测试结果:
- `catalog_view/05_export_import.sql`: 80 errors
- `consistency_advanced/06_import_export_consistency.sql`: 252 errors

主要错误:
```
PARSE ERROR: syntax error at position 0: sql parser error: Expected: an SQL statement, found: EXPORT
Statement::ExportTable not found in parser
```

## 子任务

### Task 1: EXPORT 语法解析
- 添加 ExportTableStmt 到 Statement 枚举
- 实现 EXPORT TABLE 语法解析
- 支持 EXPORT TO SQL 文件
- 支持 EXPORT TO PARQUET/CSV/JSON
- 验证: `catalog_view/05_export_import.sql` EXPORT 部分通过

### Task 2: EXPORT 执行
- 实现数据导出执行
- 支持并行导出
- 支持导出进度查询
- 支持 CANCEL EXPORT
- 验证: `catalog_view/05_export_import.sql` EXPORT 执行部分通过

### Task 3: IMPORT 语法解析
- 添加 ImportStatement 到 Statement 枚举 (如果不存在)
- 实现 LOAD DATA 语法 (如果未实现)
- 支持 STREAM LOAD
- 验证: `consistency_advanced/06_import_export_consistency.sql` IMPORT 部分通过

### Task 4: EXPORT/IMPORT 状态查询
- 实现 SHOW EXPORT 语句
- 实现 EXPORT 进度查询
- 支持 EXPORT 失败重试
- 验证: `catalog_view/05_export_import.sql` SHOW EXPORT 部分通过

## 验收标准
- [ ] EXPORT TABLE 语句可以正常解析
- [ ] 可以导出数据到不同格式
- [ ] SHOW EXPORT 语句正常返回状态
- [ ] 导出导入测试通过率 > 80%

## 影响范围
- `fe-sql-parser`: EXPORT/IMPORT 语法解析
- `fe-sql-planner`: 导出导入计划
- `be-execution`: 数据导出执行
- `data-io`: 文件格式处理
