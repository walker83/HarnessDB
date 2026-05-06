# 复杂类型支持 (ARRAY/MAP/JSON/STRUCT)

## 概述
当前 ARRAY、MAP、JSON、STRUCT 等复杂类型支持不完整。

## 现状分析
测试结果:
- `complex_types/01_array_operations.sql`: 56 errors
- `complex_types/02_map_operations.sql`: 58 errors
- `complex_types/03_json_operations.sql`: 34 errors
- `complex_types/04_struct_variant_operations.sql`: 86 errors
- `complex_types/05_type_interop.sql`: 92 errors
- `advanced/04_array_json_positive.sql`: 14 errors

主要缺失:
- ARRAY 类型 DDL 支持
- ARRAY 索引访问
- MAP 类型和函数
- JSON 函数
- STRUCT 类型

## 子任务

### Task 1: ARRAY 类型增强
- 支持 ARRAY 类型列定义
- 支持 ARRAY[...] 字面量
- 支持数组索引访问 arr[0]
- 支持数组切片
- 验证: `complex_types/01_array_operations.sql` 基础部分通过

### Task 2: ARRAY 函数
- 实现 array_length() 函数
- 实现 array_contains() 函数
- 实现 array_position() 函数
- 实现 array_distinct() 函数
- 验证: `complex_types/01_array_operations.sql` 函数部分通过

### Task 3: MAP 类型支持
- 支持 MAP 类型列定义
- 支持 MAP[...] 字面量
- 实现 map_keys() 函数
- 实现 map_values() 函数
- 实现 map_from_arrays() 函数
- 验证: `complex_types/02_map_operations.sql` 通过率 > 80%

### Task 4: JSON 类型支持
- 支持 JSON 类型列定义
- 实现 json_extract() 函数
- 实现 json_object() 函数
- 实现 json_array() 函数
- 实现 json_length() 函数
- 验证: `complex_types/03_json_operations.sql` 基础部分通过

### Task 5: STRUCT 类型支持
- 支持 STRUCT 类型列定义
- 支持 struct(...) 字面量
- 支持字段访问 struct.field
- 验证: `complex_types/04_struct_variant_operations.sql` STRUCT 部分通过

## 验收标准
- [ ] 可以创建包含 ARRAY/MAP/JSON 列的表
- [ ] 复杂类型字面量可以正常解析
- [ ] 复杂类型函数正常工作
- [ ] 复杂类型测试通过率 > 80%

## 影响范围
- `fe-sql-parser`: 复杂类型语法解析
- `fe-expression`: 复杂类型函数注册
- `types`: 复杂类型数据表示
- `be-execution`: 复杂类型算子实现
