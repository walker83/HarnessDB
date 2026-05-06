# 文本处理增强

## 概述
当前全文搜索和正则表达式功能支持不完整。

## 现状分析
测试结果:
- `text_processing/02_fulltext_search.sql`: 184 errors
- `text_processing/01_regex_patterns.sql`: 4 errors
- `text_processing/03_string_processing.sql`: 4 errors

主要缺失:
- MATCH ... AGAINST 全文搜索语法
- FULLTEXT INDEX
- 高级正则表达式函数

## 子任务

### Task 1: 全文搜索语法
- 支持 MATCH(col) AGAINST('text' IN NATURAL LANGUAGE MODE)
- 支持 MATCH(col) AGAINST('text' IN BOOLEAN MODE)
- 支持 WITH QUERY EXPANSION
- 验证: `text_processing/02_fulltext_search.sql` 基础部分通过

### Task 2: FULLTEXT INDEX DDL
- 支持 CREATE TABLE ... FULLTEXT INDEX
- 支持 ALTER TABLE ADD FULLTEXT INDEX
- 支持 DROP INDEX (FULLTEXT)
- 验证: `text_processing/02_fulltext_search.sql` DDL 部分通过

### Task 3: 全文搜索执行
- 实现全文索引构建
- 实现布尔模式搜索
- 实现相关性排序
- 验证: `text_processing/02_fulltext_search.sql` 执行部分通过

### Task 4: 正则表达式函数增强
- 实现 REGEXP 函数
- 实现 REGEXP_REPLACE 函数
- 实现 REGEXP_EXTRACT 函数
- 验证: `text_processing/01_regex_patterns.sql` 通过率 > 90%

## 验收标准
- [ ] MATCH ... AGAINST 语法正常解析
- [ ] FULLTEXT INDEX 可以创建
- [ ] 全文搜索返回正确结果
- [ ] 文本处理测试通过率 > 80%

## 影响范围
- `fe-sql-parser`: 全文搜索语法解析
- `fe-expression`: 正则表达式函数
- `be-storage`: 全文索引存储
- `be-execution`: 全文搜索执行
