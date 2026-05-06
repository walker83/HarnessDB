# External Table 和多 Catalog 支持

## 概述
当前 External Table 和多 Catalog 查询支持不完整。

## 现状分析
测试结果:
- `catalog_view/03_external_table.sql`: 118 errors
- `catalog_view/04_multi_catalog_query.sql`: 18 errors
- `catalog_view/02_catalog_operations.sql`: 46 errors

主要缺失:
- CREATE EXTERNAL TABLE 语法
- CREATE CATALOG 语法
- Catalog 查询切换
- 外部数据源连接

## 子任务

### Task 1: CATALOG DDL
- 实现 CREATE CATALOG 语句
- 实现 DROP CATALOG 语句
- 实现 SHOW CATALOGS 语句
- 实现 USE CATALOG 切换
- 验证: `catalog_view/02_catalog_operations.sql` 基本语句通过

### Task 2: External Table DDL
- 支持 CREATE TABLE ... ENGINE=ODBC/JDBC
- 支持 CREATE TABLE ... ENGINE=OB MySQL
- 支持 CREATE TABLE ... ENGINE=OSS/S3
- 存储外部表元数据
- 验证: `catalog_view/03_external_table.sql` CREATE 部分通过

### Task 3: External Table 查询
- 实现外部表查询路由
- 支持外部表列映射
- 支持谓词下推到外部表
- 验证: `catalog_view/03_external_table.sql` 查询部分通过

### Task 4: 多 Catalog 查询
- 实现跨 Catalog 查询
- 实现 Catalog 切换后的查询
- 支持 INFORMATION_SCHEMA 全局视图
- 验证: `catalog_view/04_multi_catalog_query.sql` 通过率 > 80%

## 验收标准
- [ ] 可以创建和删除 Catalog
- [ ] 可以创建 External Table
- [ ] 可以查询外部数据源
- [ ] 多 Catalog 查询正常工作
- [ ] Catalog 相关测试通过率 > 80%

## 影响范围
- `fe-sql-parser`: CATALOG/EXTERNAL TABLE 语法解析
- `fe-sql-planner`: 外部表查询计划
- `fe-catalog`: Catalog 和外部表元数据
- `be-execution`: 外部数据源连接
