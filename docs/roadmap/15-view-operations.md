# View 操作增强

## 概述
当前 View 相关操作支持不完整。

## 现状分析
测试结果:
- `catalog_view/01_view_operations.sql`: 112 errors
- `basic/03_table_ddl_special_features.sql`: 30 errors (部分 View 相关)

主要缺失:
- CREATE VIEW 完整语法
- ALTER VIEW 语句
- DROP VIEW 语句
- View 查询改写

## 子任务

### Task 1: View DDL 完整支持
- 支持 CREATE VIEW ... AS SELECT
- 支持 CREATE OR REPLACE VIEW
- 支持 IF NOT EXISTS
- 支持 WITH CHECK OPTION
- 验证: `catalog_view/01_view_operations.sql` CREATE VIEW 部分通过

### Task 2: View 元数据
- Catalog 中正确存储 View 定义
- 支持 SHOW CREATE VIEW
- 支持 DESCRIBE VIEW
- 验证: `catalog_view/01_view_operations.sql` SHOW CREATE VIEW 通过

### Task 3: View DML
- 支持 ALTER VIEW 语句
- 支持 DROP VIEW 语句
- 支持 DROP VIEW IF EXISTS
- 验证: `catalog_view/01_view_operations.sql` ALTER/DROP VIEW 部分通过

### Task 4: View 查询优化
- View 定义内联优化
- 支持物化视图自动选择
- 支持嵌套 View
- 验证: `catalog_view/01_view_operations.sql` 查询部分通过

## 验收标准
- [ ] CREATE VIEW 完整语法支持
- [ ] ALTER VIEW/DROP VIEW 正常工作
- [ ] SHOW CREATE VIEW 返回正确定义
- [ ] View 操作测试通过率 > 80%

## 影响范围
- `fe-sql-parser`: View 语法解析
- `fe-sql-planner`: View 计划生成
- `fe-catalog`: View 元数据存储
