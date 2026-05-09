# P0: SQL Parser 重构（基于Datafusion集成）

**优先级**: P0（最高）
**模块**: fe-sql-parser
**状态**: ❌ 未开始
**预计工期**: 2周
**价值**: ✅✅✅ 极高（解决80% SQL语法问题）

---

## 📋 问题诊断

### 当前Parser的核心问题

```
✅ 使用sqlparser-rs（基础正确）
❌ 手工预处理过于复杂（3000+行代码）
❌ 缺少深度语义分析（类型检查、列引用解析）
❌ Doris特有语法处理不完整（导致语法报错）
❌ 错误处理不友好（Error recovery机制缺失）
❌ 维护成本高（每次新语法都要手工添加）
```

### 具体错误点

1. **预处理逻辑复杂**：`preprocess_create_table()` 手工解析DISTRIBUTED BY等，容易出错
2. **手工解析过多**：100多个if判断，如`if starts_with("CREATE REPOSITORY")`
3. **缺少语义分析**：只转换AST，没有列引用解析、类型检查
4. **错误提示不友好**：解析失败没有详细错误位置

---

## 🎯 目标

### 短期目标（Week 1）
- ✅ 集成Datafusion SQL Parser
- ✅ 替换现有手工预处理逻辑
- ✅ 支持完整SQL语法（CTE、子查询、窗口函数）
- ✅ 错误提示友好（详细错误位置）

### 中期目标（Week 2）
- ✅ 实现深度语义分析（列引用解析、类型检查）
- ✅ 支持Doris特有语法（DISTRIBUTED BY、PARTITION BY等）
- ✅ Arrow生态集成（为后续创新做准备）

### 长期目标
- ✅ 降低维护成本（使用现成框架）
- ✅ 提升SQL兼容性（达到Doris 80%兼容度）
- ✅ 支持更多SQL特性（LATERAL VIEW、TABLE FUNCTION等）

---

## 📊 方案对比

| 方案 | 优势 | 劣势 | 时间成本 | 推荐度 |
|------|------|------|---------|--------|
| **方案1: sqlparser改进** | 保持现有架构 | 手工代码仍有 | 1-2周 | ✅ 可行 |
| **方案2: Datafusion集成** | 现代Parser，功能完整 | 需要适配 | 2周 | ✅✅✅ 强烈推荐 |
| **方案3: Analyzer实现** | 深度语义分析 | 工作量大 | 3-4周 | ✅✅ 必须做（可延后） |

**最终选择：方案2（Datafusion集成）+ 方案3（语义分析）**

---

## 🚀 实施路线（2周）

### Week 1: Datafusion集成（7天）

#### Day 1-2: 添加依赖和基础框架

**任务清单:**
- [ ] 添加Datafusion依赖到Cargo.toml
- [ ] 创建`datafusion_parser.rs`文件
- [ ] 定义`RorisParser`结构体
- [ ] 实现`parse()`入口函数

**代码框架:**
```rust
// crates/fe-sql-parser/src/datafusion_parser.rs
use datafusion::sql::parser::DFParser;
use datafusion::sql::planner::SqlToRel;
use datafusion::logical_plan::LogicalPlan;

pub struct RorisParser {
    catalog_provider: Arc<RorisSchemaProvider>,
}

impl RorisParser {
    pub fn parse(sql: &str) -> Result<LogicalPlan, ParseError> {
        let statements = DFParser::parse_sql(sql)?;
        let planner = SqlToRel::new(&self.catalog_provider);
        planner.plan(&statements[0])
    }
}
```

**验收标准:**
```bash
cargo add datafusion --version 42
cargo build -p fe-sql-parser  # 编译成功
```

---

#### Day 3-5: 适配Schema Provider

**任务清单:**
- [ ] 实现`RorisSchemaProvider` trait
- [ ] 实现`RorisTableProvider` trait
- [ ] 连接RorisDB Catalog和Datafusion
- [ ] 支持表名解析

**代码框架:**
```rust
// crates/fe-catalog/src/schema_provider.rs
use datafusion::catalog::SchemaProvider;

pub struct RorisSchemaProvider {
    catalog: Arc<CatalogManager>,
}

impl SchemaProvider for RorisSchemaProvider {
    fn table(&self, name: &str) -> Option<Arc<dyn TableProvider>> {
        let table = self.catalog.get_table(None, name)?;
        Some(Arc::new(RorisTableProvider(table)))
    }
    
    fn table_names(&self) -> Vec<String> {
        self.catalog.list_table_names()
    }
}

pub struct RorisTableProvider(Arc<Table>);

impl TableProvider for RorisTableProvider {
    fn schema(&self) -> SchemaRef {
        self.0.arrow_schema()  // 返回Arrow Schema
    }
    
    fn scan(&self, projection, filters, limit) -> ExecutionPlan {
        Arc::new(RorisScanExecNode::new(...))
    }
}
```

**验收标准:**
```bash
cargo test -p fe-catalog --test schema_provider  # 能解析表名
```

---

#### Day 6-7: 测试验证

**任务清单:**
- [ ] 创建集成测试文件
- [ ] 测试基础SQL解析（SELECT/INSERT/UPDATE/DELETE）
- [ ] 测试CTE和子查询
- [ ] 测试窗口函数
- [ ] 测试错误提示

**测试文件:**
```rust
// tests/integration/test_datafusion_parser.rs
#[test]
fn test_basic_select() {
    let sql = "SELECT id, name FROM users WHERE age > 18";
    let plan = RorisParser::parse(sql).unwrap();
    assert!(plan.is_select());
}

#[test]
fn test_cte() {
    let sql = "WITH cte AS (SELECT * FROM t1) SELECT * FROM cte";
    let plan = RorisParser::parse(sql).unwrap();
    assert!(plan.has_cte());
}

#[test]
fn test_window_function() {
    let sql = "SELECT id, ROW_NUMBER() OVER (ORDER BY id) FROM t";
    let plan = RorisParser::parse(sql).unwrap();
    assert!(plan.has_window_function());
}
```

**验收标准:**
```bash
cargo test -p fe-sql-parser --test datafusion_integration  # 所有测试通过
```

---

### Week 2: 语义分析 + Doris扩展（7天）

#### Day 8-10: 实现Analyzer（深度语义分析）

**任务清单:**
- [ ] 创建`analyzer.rs`文件
- [ ] 实现`SymbolTable`（作用域管理）
- [ ] 实现列引用解析
- [ ] 实现类型检查
- [ ] 实现错误提示

**代码框架:**
```rust
// crates/fe-sql-parser/src/analyzer.rs
pub struct Analyzer {
    catalog: Arc<CatalogManager>,
    symbol_table: SymbolTable,
}

impl Analyzer {
    pub fn analyze(&self, stmt: &Statement) -> Result<TypedStatement, AnalysisError> {
        match stmt {
            Statement::Query(query) => self.analyze_query(query),
            Statement::Insert(insert) => self.analyze_insert(insert),
            _ => Ok(TypedStatement::from(stmt)),
        }
    }
    
    fn analyze_query(&self, query: &QueryStmt) -> Result<TypedStatement, AnalysisError> {
        // 1. 解析FROM clause（表引用）
        let tables = self.analyze_from(&query.from)?;
        
        // 2. 解析SELECT list（列引用）
        let columns = self.analyze_select_list(&query.select_list, &tables)?;
        
        // 3. 类型检查（WHERE/HAVING/ORDER BY）
        let typed_where = self.analyze_where(&query.where, &columns)?;
        
        Ok(TypedStatement::Query(...))
    }
    
    fn analyze_expr(&self, expr: &Expr, tables: &[ResolvedTable]) -> Result<TypedExpr, AnalysisError> {
        match expr {
            Expr::ColumnRef { name, table_alias } => {
                // 从SymbolTable解析列
                let column = self.symbol_table.resolve_column(table_alias, name)?;
                Ok(TypedExpr::ColumnRef {
                    column,
                    data_type: column.data_type.clone(),
                })
            }
            Expr::BinaryOp { left, op, right } => {
                // 类型检查
                let left_typed = self.analyze_expr(left, tables)?;
                let right_typed = self.analyze_expr(right, tables)?;
                self.check_binary_op_type(op, &left_typed.data_type(), &right_typed.data_type())?;
                Ok(...)
            }
        }
    }
}

pub struct SymbolTable {
    tables: HashMap<String, ResolvedTable>,
    columns: HashMap<(Option<String>, String), Column>,
}

impl SymbolTable {
    pub fn resolve_column(&self, table_alias: &Option<String>, col_name: &str) -> Result<Column, AnalysisError> {
        // 尝试带alias查找
        if let Some(alias) = table_alias {
            return self.columns.get(&(Some(alias.clone()), col_name.clone()))
                .cloned()
                .ok_or_else(|| AnalysisError::ColumnNotFound(format!("{}.{}", alias, col_name)));
        }
        
        // 不带alias，搜索所有表
        for (key, column) in &self.columns {
            if key.1 == col_name {
                return Ok(column.clone());
            }
        }
        
        Err(AnalysisError::ColumnNotFound(col_name.to_string()))
    }
}
```

**验收标准:**
```rust
// 测试：列不存在应该报错
#[test]
fn test_column_not_found() {
    let sql = "SELECT nonexistent_col FROM users";
    let result = analyzer.analyze(&parse(sql));
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), AnalysisError::ColumnNotFound("nonexistent_col"));
}

// 测试：类型不匹配应该报错
#[test]
fn test_type_mismatch() {
    let sql = "SELECT * FROM users WHERE id > 'abc'";  // int vs string
    let result = analyzer.analyze(&parse(sql));
    assert!(result.is_err());
}
```

---

#### Day 11-12: Doris扩展语法支持

**任务清单:**
- [ ] 创建`doris_extensions.rs`文件
- [ ] 实现DISTRIBUTED BY解析
- [ ] 实现PARTITION BY解析
- [ ] 实现PROPERTIES解析
- [ ] 实现KEYS类型解析（AGGREGATE KEY等）

**代码框架:**
```rust
// crates/fe-sql-parser/src/doris_extensions.rs
pub struct DorisExtensions {
    pub keys_type: KeysType,
    pub partition: Option<PartitionDef>,
    pub distribution: Option<DistributionDef>,
    pub properties: Vec<(String, String)>,
}

pub fn parse_doris_create_table_extensions(tokens: &[Token]) -> Result<DorisExtensions, ParseError> {
    let stream = TokenStream::new(tokens);
    
    // 解析KEYS类型
    let keys_type = match stream.peek_keyword()? {
        "AGGREGATE" => {
            stream.expect_keywords(&["AGGREGATE", "KEY"])?;
            KeysType::Aggregate
        }
        "UNIQUE" => {
            stream.expect_keywords(&["UNIQUE", "KEY"])?;
            KeysType::Unique
        }
        "PRIMARY" => {
            stream.expect_keywords(&["PRIMARY", "KEY"])?;
            KeysType::Primary
        }
        "DUPLICATE" => {
            stream.expect_keywords(&["DUPLICATE", "KEY"])?;
            KeysType::Duplicate
        }
        _ => KeysType::Duplicate,  // 默认
    };
    
    // 解析PARTITION BY
    let partition = if stream.peek_keyword()? == "PARTITION" {
        parse_partition_by(&mut stream)?
    } else {
        None
    };
    
    // 解析DISTRIBUTED BY
    let distribution = if stream.peek_keyword()? == "DISTRIBUTED" {
        parse_distribution_by(&mut stream)?
    } else {
        None
    };
    
    // 解析PROPERTIES
    let properties = if stream.peek_keyword()? == "PROPERTIES" {
        parse_properties(&mut stream)?
    } else {
        vec![]
    };
    
    Ok(DorisExtensions {
        keys_type,
        partition,
        distribution,
        properties,
    })
}

fn parse_distribution_by(stream: &mut TokenStream) -> Result<DistributionDef, ParseError> {
    stream.expect_keywords(&["DISTRIBUTED", "BY"])?;
    
    match stream.peek_keyword()? {
        "HASH" => {
            stream.expect_keyword("HASH")?;
            stream.expect_token("(")?;
            let columns = stream.parse_identifier_list()?;
            stream.expect_token(")")?;
            stream.expect_keyword("BUCKETS")?;
            let buckets = stream.parse_number()?;
            Ok(DistributionDef::Hash { columns, buckets })
        }
        _ => Err(ParseError::UnsupportedDistributionType),
    }
}

fn parse_partition_by(stream: &mut TokenStream) -> Result<PartitionDef, ParseError> {
    stream.expect_keywords(&["PARTITION", "BY"])?;
    
    match stream.peek_keyword()? {
        "RANGE" => parse_range_partition(stream),
        "LIST" => parse_list_partition(stream),
        _ => Err(ParseError::UnsupportedPartitionType),
    }
}
```

**验收标准:**
```rust
#[test]
fn test_doris_create_table() {
    let sql = "CREATE TABLE t (
        id INT,
        name STRING
    ) AGGREGATE KEY(id)
    PARTITION BY RANGE(id)
    DISTRIBUTED BY HASH(id) BUCKETS 10
    PROPERTIES('replication_num'='3')";
    
    let extensions = parse_doris_create_table_extensions(sql)?;
    assert_eq!(extensions.keys_type, KeysType::Aggregate);
    assert!(extensions.partition.is_some());
    assert!(extensions.distribution.is_some());
}
```

---

#### Day 13-14: 替换旧Parser

**任务清单:**
- [ ] 在roris-server中切换到新Parser
- [ ] 运行全量测试
- [ ] 对比新旧Parser错误率
- [ ] 性能测试（解析速度）

**代码修改:**
```rust
// crates/roris-server/src/main.rs
use fe_sql_parser::datafusion_parser::RorisParser;

fn handle_query(sql: &str) -> Result<QueryResult, Error> {
    // 使用新Parser
    let plan = RorisParser::parse(sql)?;
    let typed_stmt = Analyzer::analyze(&plan, &catalog)?;
    
    // 执行查询
    execute_plan(typed_stmt)
}
```

**验收标准:**
```bash
cargo test --workspace  # 所有测试通过
cargo build --release   # 编译成功
./target/release/roris-fe  # 启动成功

# 对比SQL错误率：
# 旧Parser：~5000 errors（roadmap统计）
# 新Parser：<500 errors（目标）
```

---

## 📊 成果验收

### 定量指标

| 指标 | 当前（旧Parser） | 目标（新Parser） | 提升 |
|------|----------------|-----------------|------|
| **SQL语法错误数** | ~5000 | <500 | 90%减少 |
| **CTE支持** | ⚠️ 部分 | ✅ 完整 | 100% |
| **窗口函数支持** | ⚠️ 部分 | ✅ 完整 | 100% |
| **子查询支持** | ⚠️ 部分 | ✅ 完整 | 100% |
| **错误提示质量** | ❌ 不友好 | ✅ 详细位置 | 极大改善 |
| **代码维护成本** | ❌ 高（3000+行） | ✅ 低（500行） | 80%减少 |

### 定性指标

- ✅ SQL解析更稳定（错误率降低）
- ✅ 支持更多SQL特性（CTE、窗口函数、子查询）
- ✅ 错误提示更友好（开发效率提升）
- ✅ 维护成本更低（使用Datafusion现成框架）
- ✅ Arrow生态集成（为后续创新做准备）

---

## 🔗 依赖关系

### 前置依赖
- ✅ Catalog模块（需要能查询表信息）
- ✅ Types模块（Arrow Schema定义）

### 后续依赖
- ✅ Planner模块（使用新Parser生成的LogicalPlan）
- ✅ Execution模块（使用Arrow数据格式）

---

## 📁 涉及文件

### 新建文件
```
crates/fe-sql-parser/src/
├── datafusion_parser.rs     # 新Parser主文件（~200行）
├── analyzer.rs              # 语义分析器（~300行）
├── doris_extensions.rs      # Doris扩展语法（~200行）
├── symbol_table.rs          # 作用域管理（~100行）
└── token_stream.rs          # Token流处理（~150行）

crates/fe-catalog/src/
└── schema_provider.rs       # Datafusion接口适配（~100行）

tests/integration/
└── test_datafusion_parser.rs # 集成测试（~200行）
```

### 修改文件
```
crates/fe-sql-parser/src/
├── lib.rs                   # 导出新模块
├── parser.rs                # 保留为legacy，逐步废弃
└── ast.rs                   # 扩展AST定义（添加TypedStatement）

crates/roris-server/src/
└── main.rs                  # 切换到新Parser

Cargo.toml                   # 添加datafusion依赖
```

---

## ⚠️ 风险和应对

### 风险1: Datafusion API变化
- **应对**: 锁定Datafusion版本（42），定期升级
- **应对**: 抽象接口层，减少直接依赖

### 风险2: Doris特有语法不完全兼容
- **应对**: 先支持核心语法（DISTRIBUTED/PARTITION），其他延后
- **应对**: 使用扩展层，不侵入Datafusion核心

### 风险3: 性能下降（Datafusion Parser开销）
- **应对**: 测试对比，确保性能可接受
- **应对**: 优化Catalog查询（缓存表信息）

### 风险4: 旧代码迁移困难
- **应对**: 渐进式迁移，保留旧Parser作为备份
- **应对**: 充分测试，确保功能一致

---

## 💡 技术创新点

### 1. Arrow生态集成
- ✅ 使用Arrow Schema（列式数据格式）
- ✅ 与Datafusion深度集成（现代查询引擎）
- ✅ 为后续SIMD优化、异步执行做准备

### 2. 语义分析增强
- ✅ 列引用解析（SymbolTable作用域管理）
- ✅ 类型推导（表达式类型检查）
- ✅ 错误诊断（详细错误位置）

### 3. 可扩展架构
- ✅ 扩展层设计（不侵入Datafusion）
- ✅ 支持更多Doris语法（按需添加）
- ✅ 降低维护成本（使用现成框架）

---

## 📅 时间表

```
Week 1: Datafusion集成
  Day 1-2: 添加依赖和基础框架
  Day 3-5: 适配Schema Provider
  Day 6-7: 测试验证

Week 2: 语义分析 + Doris扩展
  Day 8-10: 实现Analyzer
  Day 11-12: Doris扩展语法
  Day 13-14: 替换旧Parser

总计: 14天（2周）
```

---

## 🎯 下一步行动

### 立即开始（Day 1）

```bash
# 1. 添加Datafusion依赖
cd ~/code/RorisDB
cargo add datafusion --version 42

# 2. 创建新Parser文件
vim crates/fe-sql-parser/src/datafusion_parser.rs

# 3. 编写基础框架
# （参考上面的代码框架）

# 4. 编译验证
cargo build -p fe-sql-parser
```

---

## 🔗 相关文档

- [SQL Parser集成方案详细文档](../SQL_PARSER_INTEGRATION_PLAN.md)
- [Rust原生创新方案](../RUST_NATIVE_INNOVATION_PLAN.md)
- [Datafusion官方文档](https://docs.datafusion.apache.org/)
- [Arrow格式规范](https://arrow.apache.org/)

---

## 📝 备注

**为什么选择Datafusion而不是继续改进sqlparser-rs？**

1. ✅ Datafusion是sqlparser-rs的增强版，功能更完整
2. ✅ Datafusion自带语义分析（LogicalPlan生成）
3. ✅ Datafusion活跃维护，社区支持好
4. ✅ Datafusion是Apache Arrow生态，利于后续创新
5. ✅ 使用现成框架，维护成本低

**这是最高优先级任务，立即开始！**