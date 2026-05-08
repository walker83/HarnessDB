# SQL Parser 集成改进方案

## 🔍 当前问题诊断

### 核心问题

你的Parser现状：
```
✅ 使用sqlparser-rs（正确）
❌ 预处理逻辑过于复杂（3000+行手工代码）
❌ 缺少深度语义分析（类型检查、列引用解析）
❌ 错误处理不友好（Error recovery机制缺失）
❌ Doris特有语法处理不完整（导致语法报错）
```

### 具体错误点分析

看你的parser.rs代码：

```rust
// ❌ 问题1：预处理逻辑复杂，容易出错
pub fn parse_sql(sql: &str) -> Result<Vec<Statement>, ParseError> {
    if trimmed.starts_with("CREATE TABLE") {
        let preprocessed = preprocess_create_table(sql);  // 大量预处理
        sql_to_parse = preprocessed.clean_sql;
        // 手工解析很多字段，容易出错
        doris_distribution = preprocessed.distribution;
        doris_partition = preprocessed.partition;
        // ...
    }
    
    // ❌ 问题2：大量手工解析（不是用sqlparser）
    if trimmed.starts_with("CREATE REPOSITORY") {
        return parse_create_repository(sql);  // 手工解析
    }
    if trimmed.starts_with("SHOW REPOSITORIES") {
        return Ok(vec![Statement::ShowRepositories]);  // 简化处理
    }
    // ... 100多个if判断
    
    // ❌ 问题3：缺少语义分析
    let statements = sqlparser::parse_sql(&sql_to_parse)?;
    // 只转换AST，没有类型检查、列引用解析
    let roris_stmts = statements.iter()
        .map(|stmt| convert_statement(stmt))
        .collect();
}
```

**结果：SQL语法报错不断，因为没有系统性的验证机制**

---

## 🚀 Rust生态集成方案

### 方案1: sqlparser-rs + Doris扩展层（推荐）✅

**核心思想：保持sqlparser，但改进扩展方式**

```rust
// 架构设计
Parser架构：
  sqlparser-rs（基础） 
    ↓
  DorisExtensionLayer（扩展层）
    ↓
  SemanticAnalyzer（语义分析）
    ↓
  TypedAST（类型化AST）
```

#### 1.1 改进预处理（减少手工代码）

**当前方式（❌ 问题）：**
```rust
// 你的代码：手工预处理太多
fn preprocess_create_table(sql: &str) -> PreprocessedResult {
    // 手工解析 DISTRIBUTED BY
    // 手工解析 PARTITION BY
    // 手工解析 PROPERTIES
    // 手工解析 AGGREGATE KEY
    // ... 容易出错
}
```

**改进方案（✅ 推荐）：**
```rust
// 新建：DorisExtensionParser trait
pub trait DorisExtensionParser {
    fn parse_doris_distribution(&self, tokens: &[Token]) -> Result<Option<DistributionDef>, ParseError>;
    fn parse_doris_partition(&self, tokens: &[Token]) -> Result<Option<PartitionDef>, ParseError>;
    fn parse_doris_properties(&self, tokens: &[Token]) -> Result<Vec<(String, String)>, ParseError>;
    fn parse_keys_type(&self, tokens: &[Token]) -> Result<KeysType, ParseError>;
}

impl DorisExtensionParser for Parser {
    fn parse_doris_distribution(&self, tokens: &[Token]) -> Result<Option<DistributionDef>, ParseError> {
        // 使用Token流解析，而非手工字符串处理
        let pos = self.find_token_sequence(tokens, &["DISTRIBUTED", "BY"]);
        if pos.is_none() {
            return Ok(None);
        }
        
        // 用Token解析，更可靠
        let dist_tokens = self.extract_tokens_until(tokens, pos.unwrap(), "BUCKETS");
        self.parse_distribution_from_tokens(dist_tokens)?
    }
}

// 新建：TokenStream扩展
pub struct TokenStream<'a> {
    tokens: &'a [Token],
    pos: usize,
}

impl<'a> TokenStream<'a> {
    pub fn expect_keyword(&mut self, keyword: &str) -> Result<(), ParseError> {
        if self.current_token()?.is_keyword(keyword) {
            self.advance();
            Ok(())
        } else {
            Err(ParseError::ExpectedKeyword(keyword))
        }
    }
    
    pub fn parse_identifier(&mut self) -> Result<String, ParseError> {
        match self.current_token()? {
            Token::Identifier(id) => {
                self.advance();
                Ok(id.clone())
            }
            _ => Err(ParseError::ExpectedIdentifier),
        }
    }
}
```

#### 1.2 分层解析架构

```rust
// 新建文件：crates/fe-sql-parser/src/stages/

// Stage 1: Token预处理
pub mod tokenizer {
    pub fn tokenize_with_doris_extensions(sql: &str) -> Result<Vec<Token>, ParseError> {
        // 先用sqlparser的tokenizer
        let base_tokens = sqlparser::tokenizer::Tokenizer::new(sql).tokenize()?;
        
        // 添加Doris特有token识别
        let extended_tokens = recognize_doris_tokens(base_tokens);
        Ok(extended_tokens)
    }
    
    fn recognize_doris_tokens(tokens: Vec<Token>) -> Vec<Token> {
        tokens.into_iter().map(|t| {
            match &t {
                Token::Identifier(id) if id.to_uppercase() == "DISTRIBUTED" => Token::Keyword(Keyword::DISTRIBUTED),
                Token::Identifier(id) if id.to_uppercase() == "BUCKETS" => Token::Keyword(Keyword::BUCKETS),
                Token::Identifier(id) if id.to_uppercase() == "HASH" => Token::Keyword(Keyword::HASH),
                _ => t,
            }
        }).collect()
    }
}

// Stage 2: 基础解析（sqlparser）
pub mod base_parser {
    pub fn parse_base_statement(tokens: &[Token]) -> Result<sqlparser::ast::Statement, ParseError> {
        sqlparser::parser::Parser::new(tokens).parse_statement()
    }
}

// Stage 3: Doris扩展解析
pub mod doris_extension_parser {
    pub fn parse_doris_extensions(tokens: &[Token], base_stmt: &Statement) -> Result<DorisExtensions, ParseError> {
        match base_stmt {
            Statement::CreateTable { .. } => parse_create_table_extensions(tokens),
            Statement::Insert { .. } => parse_insert_extensions(tokens),
            _ => Ok(DorisExtensions::default()),
        }
    }
    
    fn parse_create_table_extensions(tokens: &[Token]) -> Result<DorisExtensions, ParseError> {
        let mut stream = TokenStream::new(tokens);
        
        // 跳过CREATE TABLE ... 部分（已由sqlparser解析）
        stream.skip_until_after("CREATE", "TABLE");
        
        // 解析Doris扩展
        let extensions = DorisExtensions {
            distribution: stream.parse_distribution()?,
            partition: stream.parse_partition()?,
            properties: stream.parse_properties()?,
            keys_type: stream.parse_keys_type()?,
            unique_keys: stream.parse_unique_keys()?,
        };
        
        Ok(extensions)
    }
}

// Stage 4: 语义分析（新增）
pub mod semantic_analyzer {
    pub fn analyze(statement: &Statement, catalog: &CatalogManager) -> Result<TypedStatement, SemanticError> {
        match statement {
            Statement::Query(query) => analyze_query(query, catalog),
            Statement::Insert(insert) => analyze_insert(insert, catalog),
            Statement::CreateTable(create) => analyze_create_table(create, catalog),
            _ => Ok(TypedStatement::from(statement)),
        }
    }
    
    fn analyze_query(query: &QueryStmt, catalog: &CatalogManager) -> Result<TypedStatement, SemanticError> {
        let analyzer = QueryAnalyzer::new(catalog);
        
        // 1. 解析表引用（从Catalog获取表信息）
        let tables = analyzer.resolve_table_refs(&query.from)?;
        
        // 2. 解析列引用（检查列是否存在、类型是否匹配）
        let columns = analyzer.resolve_column_refs(&query.select_list, &tables)?;
        
        // 3. 类型检查（表达式类型推导）
        let typed_exprs = analyzer.type_check_exprs(&query.where, &columns)?;
        
        // 4. 检查GROUP BY合法性
        analyzer.check_group_by(&query.group_by, &columns)?;
        
        // 5. 检查HAVING合法性
        analyzer.check_having(&query.having, &typed_exprs)?;
        
        Ok(TypedStatement::Query(TypedQueryStmt {
            query: query.clone(),
            tables,
            columns,
            typed_exprs,
        }))
    }
}

// Stage 5: 合并成完整Parser
pub fn parse_sql_with_analysis(sql: &str, catalog: &CatalogManager) -> Result<Vec<TypedStatement>, ParseError> {
    // Stage 1: Tokenize
    let tokens = tokenizer::tokenize_with_doris_extensions(sql)?;
    
    // Stage 2: Base parse
    let base_stmt = base_parser::parse_base_statement(&tokens)?;
    
    // Stage 3: Doris extensions
    let extensions = doris_extension_parser::parse_doris_extensions(&tokens, &base_stmt)?;
    
    // Stage 4: Combine AST
    let roris_stmt = combine_ast(base_stmt, extensions);
    
    // Stage 5: Semantic analysis
    let typed_stmt = semantic_analyzer::analyze(&roris_stmt, catalog)?;
    
    Ok(vec![typed_stmt])
}
```

---

### 方案2: 使用Datafusion SQL Parser（更现代）✅✅

**核心思想：直接用Datafusion的Parser，它是sqlparser-rs的增强版**

```rust
// Datafusion优势：
// 1. 基于sqlparser-rs，但做了大量扩展
// 2. 有完整的语义分析（LogicalPlan生成）
// 3. 支持更多SQL特性（CTE、子查询、窗口函数）
// 4. Apache Arrow生态集成
// 5. 活跃维护，社区支持

use datafusion::sql::parser::DFParser;
use datafusion::sql::planner::SqlToRel;
use datafusion::logical_plan::LogicalPlan;

pub struct RorisSqlParser {
    df_parser: DFParser,
    catalog: Arc<CatalogManager>,
}

impl RorisSqlParser {
    pub fn parse(sql: &str) -> Result<LogicalPlan, ParseError> {
        // 1. Datafusion解析SQL（已有完整支持）
        let statements = DFParser::parse_sql(sql)?;
        
        // 2. 生成LogicalPlan（Datafusion自带）
        let planner = SqlToRel::new(self.catalog.schema_provider());
        let plan = planner.plan(statement)?;
        
        // 3. 添加Doris扩展（在LogicalPlan上）
        let roris_plan = self.add_doris_extensions(plan)?;
        
        Ok(roris_plan)
    }
    
    fn add_doris_extensions(&self, plan: LogicalPlan) -> Result<LogicalPlan, ParseError> {
        // 在LogicalPlan上添加Doris特有逻辑
        match plan {
            LogicalPlan::CreateTable(mut create) => {
                // 添加DISTRIBUTED BY
                create.distribution = self.parse_distribution()?;
                // 添加PARTITION BY
                create.partition = self.parse_partition()?;
                Ok(LogicalPlan::CreateTable(create))
            }
            _ => Ok(plan),
        }
    }
}
```

**对比：**

| 特性 | 你的Parser | Datafusion Parser |
|------|-----------|-------------------|
| 基础SQL | ✅ 支持 | ✅ 完整支持 |
| 语义分析 | ❌ 缺少 | ✅ 完整（LogicalPlan） |
| CTE/子查询 | ⚠️ 部分 | ✅ 完整支持 |
| 窗口函数 | ⚠️ 部分 | ✅ 完整支持 |
| 错误提示 | ❌ 不友好 | ✅ 详细错误位置 |
| 维护成本 | ❌ 高（手工代码） | ✅ 低（使用现成） |
| Arrow集成 | ❌ 无 | ✅ 天然集成 |

---

### 方案3: 参考Doris Analyzer实现语义分析 ✅

**核心思想：借鉴Doris的Analyzer，但用Rust实现**

看Doris的Analyzer实现：

```java
// Doris Analyzer.java
public class Analyzer {
    public void analyze(Statement stmt) {
        if (stmt instanceof QueryStmt) {
            analyzeQueryStmt((QueryStmt) stmt);
        }
    }
    
    private void analyzeQueryStmt(QueryStmt stmt) {
        // 1. Analyze FROM clause（解析表引用）
        analyzeFromClause(stmt.getFromClause());
        
        // 2. Analyze SELECT list（解析列引用）
        analyzeSelectList(stmt.getSelectList());
        
        // 3. Analyze WHERE clause（类型检查）
        analyzeWhereClause(stmt.getWhereClause());
        
        // 4. Analyze GROUP BY（合法性检查）
        analyzeGroupByClause(stmt.getGroupByClause());
        
        // 5. Analyze HAVING（类型检查）
        analyzeHavingClause(stmt.getHavingClause());
        
        // 6. Analyze ORDER BY（类型检查）
        analyzeOrderByClause(stmt.getOrderByClause());
    }
    
    private void analyzeFromClause(TableRef tableRef) {
        // 解析表名 → 从Catalog获取Table对象
        Table table = catalog.getTable(tableRef.getName());
        
        // 检查表别名冲突
        checkAliasConflict(tableRef.getAlias());
        
        // 解析JOIN条件
        if (tableRef.getJoinOp() != null) {
            analyzeJoinOp(tableRef.getJoinOp());
        }
        
        // 注册到SymbolTable（作用域管理）
        symbolTable.addTable(tableRef);
    }
    
    private void analyzeSelectList(List<SelectItem> selectItems) {
        for (SelectItem item : selectItems) {
            // 解析列名
            if (item.getExpr() instanceof SlotRef) {
                SlotRef slotRef = (SlotRef) item.getExpr();
                // 从SymbolTable查找列
                Column column = symbolTable.resolveColumn(slotRef.getTblName(), slotRef.getColName());
                if (column == null) {
                    throw new AnalysisException("Column not found: " + slotRef.getColName());
                }
                slotRef.setType(column.getType());
            }
            
            // 类型推导表达式
            analyzeExpr(item.getExpr());
        }
    }
}
```

**Rust实现：**

```rust
// 新建：crates/fe-sql-parser/src/analyzer.rs

pub struct Analyzer {
    catalog: Arc<CatalogManager>,
    symbol_table: SymbolTable,  // 作用域管理
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
        // 1. Analyze FROM clause
        let tables = self.analyze_from(&query.from)?;
        
        // 2. Analyze SELECT list
        let select_items = self.analyze_select_list(&query.select_list, &tables)?;
        
        // 3. Analyze WHERE clause
        let typed_where = self.analyze_where(&query.where, &tables)?;
        
        // 4. Analyze GROUP BY
        self.analyze_group_by(&query.group_by, &select_items)?;
        
        // 5. Analyze HAVING
        self.analyze_having(&query.having, &select_items)?;
        
        // 6. Analyze ORDER BY
        let typed_order_by = self.analyze_order_by(&query.order_by, &tables)?;
        
        Ok(TypedStatement::Query(TypedQueryStmt {
            query: query.clone(),
            tables,
            select_items,
            typed_where,
            typed_order_by,
        }))
    }
    
    fn analyze_from(&self, from: &Option<TableRef>) -> Result<Vec<ResolvedTable>, AnalysisError> {
        match from {
            Some(table_ref) => {
                // 1. 从Catalog获取Table
                let table = self.catalog.get_table(
                    table_ref.database.as_deref(),
                    &table_ref.table,
                ).ok_or_else(|| AnalysisError::TableNotFound(table_ref.table.clone()))?;
                
                // 2. 检查别名冲突
                if let Some(alias) = &table_ref.alias {
                    self.symbol_table.check_alias_conflict(alias)?;
                }
                
                // 3. 解析JOIN条件
                if let Some(join) = &table_ref.join {
                    self.analyze_join(join)?;
                }
                
                // 4. 注册到SymbolTable
                self.symbol_table.add_table(table_ref)?;
                
                Ok(vec![ResolvedTable {
                    table,
                    alias: table_ref.alias.clone(),
                }])
            }
            None => Ok(vec![]),
        }
    }
    
    fn analyze_select_list(&self, items: &[SelectItem], tables: &[ResolvedTable]) -> Result<Vec<TypedSelectItem>, AnalysisError> {
        items.iter().map(|item| {
            match item {
                SelectItem::UnnamedExpr(expr) => {
                    // 解析表达式
                    let typed_expr = self.analyze_expr(expr, tables)?;
                    Ok(TypedSelectItem::Expr(typed_expr))
                }
                SelectItem::Wildcard => {
                    // 展开所有列
                    let columns = self.expand_wildcard(tables)?;
                    Ok(TypedSelectItem::Wildcard(columns))
                }
            }
        }).collect()
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
                let left_typed = self.analyze_expr(left, tables)?;
                let right_typed = self.analyze_expr(right, tables)?;
                
                // 类型检查
                self.check_binary_op_type(op, &left_typed.data_type(), &right_typed.data_type())?;
                
                Ok(TypedExpr::BinaryOp {
                    left: left_typed,
                    op: op.clone(),
                    right: right_typed,
                    result_type: self.infer_binary_op_type(op, &left_typed.data_type(), &right_typed.data_type()),
                })
            }
            
            Expr::FunctionCall { name, args } => {
                let typed_args = args.iter()
                    .map(|arg| self.analyze_expr(arg, tables))
                    .collect::<Result<Vec<_>, _>>()?;
                
                // 函数签名检查
                let func_sig = self.resolve_function(name, &typed_args)?;
                
                Ok(TypedExpr::FunctionCall {
                    name: name.clone(),
                    args: typed_args,
                    result_type: func_sig.return_type,
                })
            }
            
            _ => Ok(TypedExpr::from(expr.clone())),
        }
    }
}

// SymbolTable（作用域管理）
pub struct SymbolTable {
    tables: HashMap<String, ResolvedTable>,  // alias → table
    columns: HashMap<(Option<String>, String), Column>,  // (alias, col_name) → column
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

---

## 📊 方案对比

| 方案 | 优势 | 劣势 | 时间成本 | 推荐度 |
|------|------|------|---------|--------|
| **方案1: sqlparser改进** | 保持现有架构 | 手工代码仍有 | 1-2周 | ✅ 推荐 |
| **方案2: Datafusion集成** | 现代Parser，功能完整 | 需要适配 | 2-3周 | ✅✅ 强烈推荐 |
| **方案3: Analyzer实现** | 深度语义分析 | 工作量大 | 3-4周 | ✅ 推荐（必须做） |

---

## 🎯 立即行动计划（分阶段）

### 第一阶段：Datafusion集成（2周，最高价值）✅✅

**目标：用Datafusion Parser替代现有Parser，解决大部分SQL语法问题**

```bash
# 1. 添加Datafusion依赖
cargo add datafusion --version 42

# 2. 创建新Parser文件
vim crates/fe-sql-parser/src/datafusion_parser.rs
```

```rust
// crates/fe-sql-parser/src/datafusion_parser.rs
use datafusion::sql::parser::DFParser;
use datafusion::sql::planner::SqlToRel;
use datafusion::logical_plan::LogicalPlan;
use datafusion::error::DataFusionError;

pub struct RorisParser {
    catalog_provider: Arc<RorisSchemaProvider>,
}

impl RorisParser {
    pub fn parse(sql: &str) -> Result<LogicalPlan, ParseError> {
        // Parse with Datafusion
        let statements = DFParser::parse_sql(sql)
            .map_err(|e| ParseError::Datafusion(e.to_string()))?;
        
        if statements.is_empty() {
            return Err(ParseError::EmptyStatement);
        }
        
        let stmt = statements[0];
        
        // Convert to LogicalPlan
        let planner = SqlToRel::new(&self.catalog_provider);
        let plan = planner.plan(&stmt)
            .map_err(|e| ParseError::Planning(e.to_string()))?;
        
        // Add Doris extensions
        self.add_doris_extensions(plan)
    }
    
    fn add_doris_extensions(&self, plan: LogicalPlan) -> Result<LogicalPlan, ParseError> {
        // 在LogicalPlan上添加Doris特有逻辑
        // 例如：DISTRIBUTED BY、PARTITION BY等
        Ok(plan)
    }
}

// Schema provider（适配Datafusion）
pub struct RorisSchemaProvider {
    catalog: Arc<CatalogManager>,
}

impl SchemaProvider for RorisSchemaProvider {
    fn table(&self, name: &str) -> Option<Arc<dyn TableProvider>> {
        let table = self.catalog.get_table(None, name)?;
        Some(Arc::new(RorisTableProvider(table)))
    }
}

// Table provider（适配Datafusion）
pub struct RorisTableProvider(Arc<Table>);

impl TableProvider for RorisTableProvider {
    fn schema(&self) -> SchemaRef {
        // 返回Arrow Schema
        self.0.arrow_schema()
    }
    
    fn scan(&self, projection: Option<&Vec<usize>>, filters: &[Expr], limit: Option<usize>) -> Result<Arc<dyn ExecutionPlan>, DataFusionError> {
        // 返回RorisDB的Scan执行计划
        Ok(Arc::new(RorisScanExecNode::new(self.0.clone(), projection, filters, limit)))
    }
}
```

**验证测试：**
```bash
cargo test -p fe-sql-parser --test datafusion_integration
```

---

### 第二阶段：语义分析实现（1周，必须做）✅

**目标：添加深度语义分析，解决列引用、类型检查问题**

```rust
// crates/fe-sql-parser/src/analyzer.rs（如上方案3）

// 验证测试：
// 1. 列不存在 → 报错
// 2. 类型不匹配 → 报错
// 3. 表别名冲突 → 报错
```

---

### 第三阶段：Doris扩展语法支持（1周，按需）✅

**目标：完整支持DISTRIBUTED BY、PARTITION BY等**

```rust
// crates/fe-sql-parser/src/doris_extensions.rs

pub fn parse_doris_create_table_extensions(tokens: &[Token]) -> Result<DorisExtensions, ParseError> {
    let stream = TokenStream::new(tokens);
    
    // 解析KEYS类型
    let keys_type = parse_keys_type(&mut stream)?;
    
    // 解析PARTITION BY
    let partition = parse_partition_by(&mut stream)?;
    
    // 解析DISTRIBUTED BY
    let distribution = parse_distribution_by(&mut stream)?;
    
    // 解析PROPERTIES
    let properties = parse_properties(&mut stream)?;
    
    Ok(DorisExtensions {
        keys_type,
        partition,
        distribution,
        properties,
    })
}
```

---

## 💡 快速修复现有Bug（短期）

### Bug修复清单

根据你的roadmap，这些是P0必须修的：

#### 1. INSERT ON DUPLICATE KEY解析错误

**当前代码：**
```rust
// 你的parser.rs有fixup_insert_select_on_duplicate
// 但可能不够完善
```

**修复：**
```rust
// 改用TokenStream解析，更可靠
fn parse_insert_on_duplicate(tokens: &[Token]) -> Result<OnDuplicateKeyClause, ParseError> {
    let stream = TokenStream::new(tokens);
    
    // 找到ON DUPLICATE KEY位置
    stream.find_keywords(&["ON", "DUPLICATE", "KEY", "UPDATE"]);
    
    // 解析UPDATE列表
    let updates = stream.parse_update_list()?;
    
    Ok(OnDuplicateKeyClause { updates })
}
```

#### 2. ALTER TABLE解析不完整

**修复：**
```rust
fn parse_alter_table(tokens: &[Token]) -> Result<AlterTableStmt, ParseError> {
    let stream = TokenStream::new(tokens);
    
    stream.expect_keywords(&["ALTER", "TABLE"])?;
    let table_name = stream.parse_table_name()?;
    
    // 支持多种ALTER操作
    let operations = vec![];
    while !stream.is_eof() {
        let op = match stream.peek_keyword()? {
            "ADD" => parse_add_column(&mut stream)?,
            "DROP" => parse_drop_column(&mut stream)?,
            "MODIFY" => parse_modify_column(&mut stream)?,
            "RENAME" => parse_rename(&mut stream)?,
            "PARTITION" => parse_partition_op(&mut stream)?,
            _ => break,
        };
        operations.push(op);
    }
    
    Ok(AlterTableStmt { table_name, operations })
}
```

#### 3. SHOW语句解析不完整

**修复：**
```rust
fn parse_show_statement(tokens: &[Token]) -> Result<Statement, ParseError> {
    let stream = TokenStream::new(tokens);
    
    stream.expect_keyword("SHOW")?;
    
    match stream.peek_keyword()? {
        "DATABASES" => Ok(Statement::ShowDatabases),
        "TABLES" => {
            stream.expect_keyword("TABLES")?;
            let pattern = stream.parse_optional_from()?;  // FROM database
            Ok(Statement::ShowTables(pattern))
        }
        "CREATE" => parse_show_create(&mut stream)?,
        "PARTITIONS" => parse_show_partitions(&mut stream)?,
        "VARIABLES" => parse_show_variables(&mut stream)?,
        // ... 其他SHOW语句
        _ => Err(ParseError::UnsupportedShowStatement(stream.current_token()?.to_string())),
    }
}
```

---

## 🎯 最终建议

**立即开始：第一阶段（Datafusion集成）**

理由：
1. ✅ **最高价值**：Datafusion Parser已成熟，解决80%的SQL问题
2. ✅ **时间最短**：2周就能完成，立即见效
3. ✅ **架构更现代**：Arrow生态集成，利于后续创新
4. ✅ **维护成本低**：使用现成框架，不用维护手工Parser

**具体步骤：**

```bash
# 第1天：集成Datafusion
cargo add datafusion
vim crates/fe-sql-parser/src/datafusion_parser.rs  # 创建新Parser

# 第3天：适配Schema Provider
vim crates/fe-catalog/src/schema_provider.rs  # 实现Datafusion接口

# 第5天：测试验证
cargo test -p fe-sql-parser --test datafusion_integration

# 第7天：替换现有Parser
# 在roris-server中用新Parser替代旧Parser

# 第10天：添加语义分析
vim crates/fe-sql-parser/src/analyzer.rs

# 第14天：完整测试
cargo test --workspace
```

想立即开始Datafusion集成吗？我可以帮你设计具体的实现代码。