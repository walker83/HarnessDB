# P2: UDF/UDAF 框架

**优先级**: P2
**模块**: fe-expression, fe-sql-parser
**状态**: ❌ 未开始

## 背景

用户自定义函数 (UDF) 和用户自定义聚合函数 (UDAF) 是数据库可扩展性的重要组成部分。RorisDB 当前不支持任何用户自定义函数。

## 任务清单

### 1. UDF 框架
- [ ] `CREATE FUNCTION` 语法解析
- [ ] 定义 UDF 注册接口
- [ ] 支持标量 UDF: 输入一行，输出一个值
- [ ] 调用方式: RPC 调用外部 UDF Service（Remote UDF）
- [ ] 或: WASM 内嵌执行（高性能方案）
- [ ] 函数生命周期: CREATE / DROP / SHOW FUNCTIONS

### 2. UDAF 框架
- [ ] `CREATE AGGREGATE FUNCTION` 语法解析
- [ ] 定义 UDAF 接口: init / update / merge / finalize
- [ ] 支持分布式聚合（merge 阶段合并中间结果）
- [ ] 与现有 Aggregate 框架集成

### 3. 类型系统支持
- [ ] UDF 参数类型和返回类型声明
- [ ] 类型检查和隐式转换
- [ ] NULL 处理

### 4. 测试
- [ ] Remote UDF: 启动外部服务，注册和调用
- [ ] UDAF: 自定义聚合函数正确性
- [ ] 性能测试: UDF 对查询性能的影响

## 涉及文件

- `crates/fe-expression/src/udf.rs` - 新建，UDF/UDAF 框架
- `crates/fe-sql-parser/src/parser.rs` - 语法解析
- `crates/fe-catalog/src/` - 函数注册和存储
