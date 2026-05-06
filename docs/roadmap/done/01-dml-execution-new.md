# DML 执行层实现

## 概述
实现 INSERT/UPDATE/DELETE 语句的完整执行功能。

## 现状分析 (2026-05-06)

### 测试结果

| 文件 | 错误数 (之前) | 错误数 (当前) | 改进 |
|------|--------------|--------------|------|
| basic/04_dml_insert_update_delete.sql | 32 | 16 | -50% |
| dml/01_insert_operations.sql | 96 | 48 | -50% |
| dml/02_update_delete_operations.sql | 90 | 45 | -50% |
| dml/03_upsert_merge_operations.sql | 82 | 41 | -50% |
| dml/04_transaction_isolation.sql | 262 | 131 | -50% |
| **总计** | **562** | **281** | **-50%** |

### 剩余错误类型

1. **执行层错误** (约 200+)
   ```
   ERROR: INSERT execution not yet implemented - table: xxx
   ERROR: UPDATE execution not yet implemented
   ERROR: DELETE execution not yet implemented
   ```

2. **解析层错误** (约 80+)
   ```
   PARSE ERROR: ... found: DUPLICATE (INSERT ON DUPLICATE KEY)
   PARSE ERROR: ... found: UNIQUE (UNIQUE 约束)
   PARSE ERROR: ... found: ON (INSERT...ON DUPLICATE KEY)
   ```

---

## 子任务

### Task 1: INSERT 执行实现

**当前状态**: Planner 解析已通过，Execution 执行未实现

**验证语句**:
```sql
-- 1. 基础 INSERT
INSERT INTO test_table (id, name) VALUES (1, 'test');

-- 2. 多行 INSERT
INSERT INTO test_table (id, name) VALUES (1, 'a'), (2, 'b'), (3, 'c');

-- 3. INSERT ... SELECT
INSERT INTO test_table (id, name) SELECT id, name FROM source_table;

-- 4. INSERT with DEFAULT
INSERT INTO test_table (id, name) VALUES (1, DEFAULT);

-- 5. INSERT with SET clause (MySQL specific)
INSERT INTO test_table SET id = 1, name = 'test';
```

**验收标准**:
- [ ] 单行 INSERT 成功执行
- [ ] 多行 INSERT 成功执行
- [ ] INSERT ... SELECT 成功执行
- [ ] 相关测试用例通过率 > 80%

---

### Task 2: INSERT ... ON DUPLICATE KEY 实现

**当前状态**: 语法不支持，需要解析层支持

**验证语句**:
```sql
-- 1. 基础 ON DUPLICATE KEY UPDATE
INSERT INTO test_table (id, name, value) VALUES (1, 'test', 100)
    ON DUPLICATE KEY UPDATE value = value + 100;

-- 2. 多列更新
INSERT INTO test_table (id, name, value) VALUES (1, 'test', 100)
    ON DUPLICATE KEY UPDATE name = 'updated', value = 200;

-- 3. 使用 VALUES() 引用原始值
INSERT INTO test_table (id, name, value) VALUES (1, 'test', 100)
    ON DUPLICATE KEY UPDATE value = VALUES(value) + 100;

-- 4. 多行 + ON DUPLICATE KEY
INSERT INTO test_table (id, name, value) VALUES (1, 'a', 100), (2, 'b', 200)
    ON DUPLICATE KEY UPDATE value = VALUES(value) + 10;
```

**验收标准**:
- [ ] ON DUPLICATE KEY UPDATE 语法正确解析
- [ ] 重复键时执行 UPDATE
- [ ] 非重复键时执行 INSERT
- [ ] VALUES() 函数正确引用

---

### Task 3: UPDATE 执行实现

**当前状态**: 语法不支持，需要 Execution 层实现

**验证语句**:
```sql
-- 1. 基础 UPDATE
UPDATE test_table SET name = 'new_name' WHERE id = 1;

-- 2. 多列 UPDATE
UPDATE test_table SET name = 'new_name', value = 100 WHERE id = 1;

-- 3. UPDATE with ORDER BY
UPDATE test_table SET value = value + 10 ORDER BY id DESC;

-- 4. UPDATE with LIMIT
UPDATE test_table SET value = 0 LIMIT 10;

-- 5. UPDATE with ORDER BY + LIMIT
UPDATE test_table SET value = value + 10 ORDER BY id LIMIT 5;

-- 6. UPDATE with subquery
UPDATE test_table SET value = (SELECT MAX(value) FROM other_table);
```

**验收标准**:
- [ ] 单表 UPDATE 成功执行
- [ ] WHERE 条件正确过滤
- [ ] ORDER BY + LIMIT 正确工作
- [ ] 相关测试用例通过率 > 80%

---

### Task 4: DELETE 执行实现

**当前状态**: 语法不支持，需要 Execution 层实现

**验证语句**:
```sql
-- 1. 基础 DELETE
DELETE FROM test_table WHERE id = 1;

-- 2. DELETE with ORDER BY
DELETE FROM test_table ORDER BY id DESC;

-- 3. DELETE with LIMIT
DELETE FROM test_table LIMIT 10;

-- 4. DELETE with ORDER BY + LIMIT
DELETE FROM test_table ORDER BY id LIMIT 5;

-- 5. 多表 DELETE (MySQL 语法)
DELETE t1 FROM test_table t1 INNER JOIN other_table t2 ON t1.id = t2.id;

-- 6. Quick DELETE (不加 WHERE)
DELETE FROM test_table;
```

**验收标准**:
- [ ] 单表 DELETE 成功执行
- [ ] WHERE 条件正确过滤
- [ ] ORDER BY + LIMIT 正确工作
- [ ] 相关测试用例通过率 > 80%

---

### Task 5: 事务支持

**当前状态**: 部分支持，需要完善

**验证语句**:
```sql
-- 1. 基础事务
START TRANSACTION;
INSERT INTO test_table (id, name) VALUES (1, 'test');
UPDATE test_table SET name = 'updated' WHERE id = 1;
COMMIT;

-- 2. Rollback
START TRANSACTION;
INSERT INTO test_table (id, name) VALUES (2, 'test2');
ROLLBACK;

-- 3. 自动提交模式
SET autocommit = 1;
INSERT INTO test_table (id, name) VALUES (3, 'test3');
COMMIT;

-- 4. Savepoint
START TRANSACTION;
INSERT INTO test_table (id, name) VALUES (1, 'a');
SAVEPOINT sp1;
INSERT INTO test_table (id, name) VALUES (2, 'b');
ROLLBACK TO sp1;
COMMIT;

-- 5. 事务隔离级别
SET TRANSACTION ISOLATION LEVEL READ COMMITTED;
SET TRANSACTION ISOLATION LEVEL REPEATABLE READ;
SET TRANSACTION ISOLATION LEVEL SERIALIZABLE;
```

**验收标准**:
- [ ] START TRANSACTION / BEGIN 正常开始事务
- [ ] COMMIT 成功提交
- [ ] ROLLBACK 成功回滚
- [ ] Savepoint 正常工作
- [ ] 隔离级别设置生效
- [ ] 相关测试用例通过率 > 80%

---

### Task 6: UNIQUE 约束支持

**当前状态**: 语法不支持

**验证语句**:
```sql
-- 1. 建表时指定 UNIQUE
CREATE TABLE test_unique (
    id INT,
    name VARCHAR(50),
    UNIQUE (id),
    UNIQUE KEY uk_name (name)
);

-- 2. ALTER TABLE ADD UNIQUE
ALTER TABLE test_table ADD UNIQUE (id);
ALTER TABLE test_table ADD UNIQUE KEY uk_name (name);

-- 3. UNIQUE 约束冲突检测
INSERT INTO test_table (id, name) VALUES (1, 'a');
INSERT INTO test_table (id, name) VALUES (1, 'b'); -- Error: Duplicate entry
```

**验收标准**:
- [ ] UNIQUE 约束正确解析
- [ ] 唯一性约束冲突正确报错
- [ ] 相关测试用例通过率 > 80%

---

## 影响范围

- `fe-sql-parser`: INSERT ... ON DUPLICATE KEY, UNIQUE KEY 语法
- `fe-sql-planner`: InsertNode, UpdateNode, DeleteNode 计划生成
- `be-execution`: InsertExecNode, UpdateExecNode, DeleteExecNode 执行算子
- `fe-scheduler`: DML 语句的调度
- `be-storage`: 事务和锁管理

## 预估工作量

| 子任务 | 难度 | 预估时间 |
|--------|------|----------|
| INSERT 执行实现 | 中 | 2-3 天 |
| ON DUPLICATE KEY | 中 | 2-3 天 |
| UPDATE 执行 | 中 | 2 天 |
| DELETE 执行 | 中 | 2 天 |
| 事务支持 | 高 | 3-4 天 |
| UNIQUE 约束 | 低 | 1-2 天 |

---

## 验证命令

```bash
# 测试所有 DML 文件
mysql -h 127.0.0.1 -P 9030 -uroot < /Users/walker/workspace/doris_test_suite/basic/04_dml_insert_update_delete.sql 2>&1 | grep -c "ERROR"
mysql -h 127.0.0.1 -P 9030 -uroot < /Users/walker/workspace/doris_test_suite/dml/01_insert_operations.sql 2>&1 | grep -c "ERROR"
mysql -h 127.0.0.1 -P 9030 -uroot < /Users/walker/workspace/doris_test_suite/dml/02_update_delete_operations.sql 2>&1 | grep -c "ERROR"
mysql -h 127.0.0.1 -P 9030 -uroot < /Users/walker/workspace/doris_test_suite/dml/03_upsert_merge_operations.sql 2>&1 | grep -c "ERROR"
mysql -h 127.0.0.1 -P 9030 -uroot < /Users/walker/workspace/doris_test_suite/dml/04_transaction_isolation.sql 2>&1 | grep -c "ERROR"

# 目标: 每个文件错误数 < 5
```
