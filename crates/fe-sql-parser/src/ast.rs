#[derive(Debug, Clone)]
pub enum Statement {
    Query(QueryStmt),
    Insert(InsertStmt),
    CreateDatabase(CreateDatabaseStmt),
    CreateTable(CreateTableStmt),
    DropDatabase(DropDatabaseStmt),
    DropTable(DropTableStmt),
    AlterTable(AlterTableStmt),
    ShowDatabases,
    ShowTables(Option<String>),
    ShowCreateTable(String, String),
    Describe(String, String),  // (database, table)
    ShowColumns(Option<String>, Option<String>),  // (database, table)
    Explain(ExplainStmt),
    UseDatabase(String),
    SetVariable(SetVariableStmt),
    Union(UnionStmt),
}

#[derive(Debug, Clone)]
pub struct QueryStmt {
    pub select_list: Vec<SelectItem>,
    pub from: Option<TableRef>,
    pub r#where: Option<Expr>,
    pub group_by: Vec<Expr>,
    pub having: Option<Expr>,
    pub order_by: Vec<OrderByItem>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct UnionStmt {
    pub op: UnionOperator,
    pub all: bool,
    pub left: Box<QueryStmt>,
    pub right: Box<QueryStmt>,
}

#[derive(Debug, Clone, Copy)]
pub enum UnionOperator {
    Union,
    Except,
    Intersect,
}

#[derive(Debug, Clone)]
pub struct SelectItem {
    pub expr: Expr,
    pub alias: Option<String>,
}

#[derive(Debug, Clone)]
pub enum TableRef {
    Table { name: String, alias: Option<String> },
    Join { left: Box<TableRef>, right: Box<TableRef>, r#type: JoinType, condition: Option<Expr> },
    Subquery { query: Box<QueryStmt>, alias: String },
}

#[derive(Debug, Clone, Copy)]
pub enum JoinType {
    Inner,
    LeftOuter,
    RightOuter,
    FullOuter,
    Cross,
}

#[derive(Debug, Clone)]
pub struct InsertStmt {
    pub table: String,
    pub columns: Vec<String>,
    pub values: Vec<Vec<Expr>>,
    pub query: Option<QueryStmt>,
}

#[derive(Debug, Clone)]
pub struct CreateDatabaseStmt {
    pub name: String,
    pub if_not_exists: bool,
    pub properties: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct CreateTableStmt {
    pub database: Option<String>,
    pub name: String,
    pub if_not_exists: bool,
    pub columns: Vec<ColumnDef>,
    pub keys_type: KeysType,
    pub partition: Option<PartitionDef>,
    pub distribution: Option<DistributionDef>,
    pub properties: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct ColumnDef {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub default_value: Option<Expr>,
    pub agg_type: Option<String>,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum KeysType {
    Duplicate,
    Aggregate,
    Unique,
    Primary,
}

#[derive(Debug, Clone)]
pub struct PartitionDef {
    pub partition_type: String,
    pub columns: Vec<String>,
    pub ranges: Vec<PartitionRange>,
}

#[derive(Debug, Clone)]
pub struct PartitionRange {
    pub name: String,
    pub start: String,
    pub end: String,
}

#[derive(Debug, Clone)]
pub struct DistributionDef {
    pub dist_type: String,
    pub columns: Vec<String>,
    pub buckets: usize,
}

#[derive(Debug, Clone)]
pub struct DropDatabaseStmt {
    pub name: String,
    pub if_exists: bool,
}

#[derive(Debug, Clone)]
pub struct DropTableStmt {
    pub database: Option<String>,
    pub name: String,
    pub if_exists: bool,
}

#[derive(Debug, Clone)]
pub struct AlterTableStmt {
    pub database: Option<String>,
    pub table: String,
    pub operations: Vec<AlterOperation>,
}

#[derive(Debug, Clone)]
pub enum AlterOperation {
    AddColumn(ColumnDef),
    DropColumn(String),
    ModifyColumn(ColumnDef),
    RenameTable(String),
}

#[derive(Debug, Clone)]
pub struct ExplainStmt {
    pub statement: Box<Statement>,
    pub verbose: bool,
}

#[derive(Debug, Clone)]
pub struct SetVariableStmt {
    pub variable: String,
    pub value: Expr,
    pub is_global: bool,
}

#[derive(Debug, Clone)]
pub struct OrderByItem {
    pub expr: Expr,
    pub ascending: bool,
    pub nulls_first: bool,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Literal(LiteralValue),
    ColumnRef { table: Option<String>, column: String },
    BinaryOp { left: Box<Expr>, op: BinaryOp, right: Box<Expr> },
    UnaryOp { op: UnaryOp, expr: Box<Expr> },
    FunctionCall { name: String, args: Vec<Expr>, distinct: bool },
    Between { expr: Box<Expr>, low: Box<Expr>, high: Box<Expr>, negated: bool },
    InList { expr: Box<Expr>, list: Vec<Expr>, negated: bool },
    InSubquery { expr: Box<Expr>, query: Box<QueryStmt>, negated: bool },
    Exists(Box<QueryStmt>),
    Subquery(Box<QueryStmt>),
    IsNull { expr: Box<Expr>, negated: bool },
    Like { expr: Box<Expr>, pattern: Box<Expr>, negated: bool },
    Cast { expr: Box<Expr>, target_type: String },
    Wildcard,
}

#[derive(Debug, Clone)]
pub enum LiteralValue {
    Null,
    Boolean(bool),
    Int64(i64),
    Float64(f64),
    String(String),
    Date(String),
}

#[derive(Debug, Clone, Copy)]
pub enum BinaryOp {
    Eq, NotEq, Lt, LtEq, Gt, GtEq,
    And, Or,
    Plus, Minus, Multiply, Divide, Modulo,
    Like, NotLike,
    In, NotIn,
}

#[derive(Debug, Clone, Copy)]
pub enum UnaryOp {
    Not,
    Negate,
}
