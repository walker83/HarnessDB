# RorisDB 开发路线图

基于 Doris Test Suite 测试结果，将开发任务拆分为以下子任务。

## 任务总览

| 编号 | 任务名称 | 优先级 | 错误数 | 难度 |
|------|----------|--------|--------|------|
| 01 | DML 执行层实现 | P0 | 281 (进行中) | 高 |
| 02 | 分区表支持 | P0 | ~250 | 中 |
| 03 | 索引管理 | P1 | ~340 | 中 |
| 04 | 物化视图 | P1 | ~200 | 高 |
| 05 | Compaction 和 Schema Change | P1 | ~500 | 高 |
| 06 | 安全模块 - 用户和权限 | P1 | ~800 | 高 |
| 07 | BITMAP 和 HLL 函数 | P1 | ~180 | 中 |
| 08 | 备份恢复 (BACKUP/RESTORE) | P2 | ~150 | 中 |
| 09 | 复杂类型支持 (ARRAY/MAP/JSON) | P1 | ~330 | 中 |
| 10 | Information Schema 增强 | P2 | ~150 | 低 |
| 11 | 资源管理 - WORKLOAD GROUP | P2 | ~100 | 中 |
| 12 | INSERT ON DUPLICATE KEY 语法 | P0 | ~500+ | 低 |
| 13 | External Table 和多 Catalog | P1 | ~180 | 中 |
| 14 | EXPORT/IMPORT 数据导出导入 | P2 | ~330 | 中 |
| 15 | View 操作增强 | P2 | ~150 | 低 |
| 16 | 冷热存储分层 | P3 | ~170 | 高 |
| 17 | 文本处理增强 | P2 | ~190 | 中 |
| 18 | 管理语句增强 | P2 | ~180 | 低 |
| 19 | 高级窗口函数 | P2 | ~30 | 中 |
| 20 | CTE 和递归查询 | P2 | ~60 | 中 |

## 优先级说明

### P0 (核心缺失，影响基础使用)
- 01: DML 执行层实现
- 12: INSERT ON DUPLICATE KEY 语法

### P1 (重要功能，影响主要业务)
- 02: 分区表支持
- 03: 索引管理
- 04: 物化视图
- 05: Compaction 和 Schema Change
- 06: 安全模块
- 07: BITMAP 和 HLL 函数
- 09: 复杂类型支持
- 13: External Table 和多 Catalog

### P2 (高级功能，建议实现)
- 08: 备份恢复
- 10: Information Schema 增强
- 11: WORKLOAD GROUP
- 14: EXPORT/IMPORT
- 15: View 操作增强
- 17: 文本处理增强
- 18: 管理语句增强
- 19: 高级窗口函数
- 20: CTE 和递归查询

### P3 (可选功能)
- 16: 冷热存储分层

## 建议开发顺序

1. **先修 P0**: DML 执行、INSERT ON DUPLICATE KEY
2. **再修 P1 基础**: 分区表、复杂类型、索引管理
3. **然后 P1 核心**: 物化视图、Compaction、安全模块
4. **最后 P2**: 其他高级功能

## 已验证正常功能 (无需开发)

以下模块测试通过率较高，暂不需要修改：

- `query/03_subquery_positive.sql`: 0 errors ✅
- `query/04_window_function_positive.sql`: 0 errors ✅
- `query/05_query_negative.sql`: 0 errors ✅
- `advanced/05_advanced_negative.sql`: 0 errors ✅
- `functions/01_string_functions.sql`: 4 errors ✅
- `functions/02_math_functions.sql`: 4 errors ✅
- `functions/03_date_time_functions.sql`: 4 errors ✅
- `mysql_compat/01_mysql_string_functions.sql`: 4 errors ✅
- `mysql_compat/02_mysql_numeric_functions.sql`: 4 errors ✅
- `mysql_compat/03_mysql_date_functions.sql`: 4 errors ✅
- `text_processing/01_regex_patterns.sql`: 4 errors ✅
- `text_processing/03_string_processing.sql`: 4 errors ✅

## 文件结构

```
docs/roadmap/
├── 00-overview.md              # 本文件 - 总览和索引
├── 01-dml-execution.md         # DML 执行层实现
├── 02-partition-ddl.md         # 分区表支持
├── 03-index-management.md       # 索引管理
├── 04-materialized-view.md     # 物化视图
├── 05-compaction-schema-change.md  # Compaction 和 Schema Change
├── 06-security-user-management.md # 安全模块
├── 07-bitmap-hll.md             # BITMAP 和 HLL 函数
├── 08-backup-restore.md         # 备份恢复
├── 09-complex-types.md          # 复杂类型支持
├── 10-information-schema.md     # Information Schema 增强
├── 11-workload-group.md         # WORKLOAD GROUP
├── 12-insert-on-duplicate.md    # INSERT ON DUPLICATE KEY
├── 13-external-table-catalog.md # External Table 和多 Catalog
├── 14-export-import.md          # EXPORT/IMPORT
├── 15-view-operations.md       # View 操作增强
├── 16-cold-hot-storage.md       # 冷热存储分层
├── 17-text-processing.md        # 文本处理增强
├── 18-admin-statements.md       # 管理语句增强
├── 19-window-functions.md        # 高级窗口函数
└── 20-CTE-recursive.md           # CTE 和递归查询
```

## 统计

- 总任务数: 20
- P0 任务: 2
- P1 任务: 9
- P2 任务: 8
- P3 任务: 1
- 预估总错误数覆盖: ~5000+
