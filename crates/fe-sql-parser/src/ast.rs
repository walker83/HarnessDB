#[derive(Debug, Clone)]
pub enum Statement {
    Query(QueryStmt),
    Insert(InsertStmt),
    Update(UpdateStmt),
    Delete(DeleteStmt),
    CreateDatabase(CreateDatabaseStmt),
    CreateTable(CreateTableStmt),
    CreateView { database: Option<String>, name: String, if_not_exists: bool, query: String, columns: Vec<String> },
    CreateMaterializedView(CreateMaterializedViewStmt),
    DropDatabase(DropDatabaseStmt),
    DropTable(DropTableStmt),
    DropMaterializedView(DropMaterializedViewStmt),
    AlterMaterializedView(AlterMaterializedViewStmt),
    RefreshMaterializedView(RefreshMaterializedViewStmt),
    AlterTable(AlterTableStmt),
    TruncateTable { database: Option<String>, table: String, if_exists: bool },
    ShowDatabases,
    ShowTables(Option<String>),
    ShowCreateTable(String, String),
    Describe(String, String),
    ShowColumns(Option<String>, Option<String>),
    Explain(ExplainStmt),
    UseDatabase(String),
    SetVariable(SetVariableStmt),
    Union(UnionStmt),
    CreateRepository(CreateRepositoryStmt),
    DropRepository(DropRepositoryStmt),
    ShowRepositories,
    BackupDatabase(BackupDatabaseStmt),
    RestoreDatabase(RestoreDatabaseStmt),
    ShowUsers,
    CreateUser(CreateUserStmt),
    DropUser(DropUserStmt),
    CreateCatalog(CreateCatalogStmt),
    DropCatalog(DropCatalogStmt),
    ShowCatalogs,
    RefreshCatalog(RefreshCatalogStmt),
    ExportTable(ExportTableStmt),
    ShowDelete { database: Option<String>, table: Option<String> },
    ShowLastInsert,
    BrokerLoad(BrokerLoadStmt),
    RoutineLoad(RoutineLoadStmt),
    MysqlLoad(MysqlLoadStmt),
}

#[derive(Debug, Clone)]
pub struct UpdateStmt {
    pub table: String,
    pub set_clauses: Vec<SetClause>,
    pub selection: Option<Expr>,
}

#[derive(Debug, Clone)]
pub struct SetClause {
    pub column: String,
    pub value: Expr,
}

#[derive(Debug, Clone)]
pub struct DeleteStmt {
    pub table: String,
    pub selection: Option<Expr>,
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
    pub with: Option<Cte>,
}

#[derive(Debug, Clone)]
pub struct Cte {
    pub name: String,
    pub columns: Vec<String>,
    pub query: Box<QueryStmt>,
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
    pub is_overwrite: bool,
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

#[derive(Debug, Clone)]
pub struct CreateRepositoryStmt {
    pub name: String,
    pub repo_type: RepositoryType,
    pub properties: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub enum RepositoryType {
    Local,
    S3,
    Hdfs,
}

#[derive(Debug, Clone)]
pub struct DropRepositoryStmt {
    pub name: String,
    pub if_exists: bool,
}

#[derive(Debug, Clone)]
pub struct BackupDatabaseStmt {
    pub database: String,
    pub repository: String,
    pub backup_name: String,
    pub properties: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct RestoreDatabaseStmt {
    pub database: String,
    pub repository: String,
    pub backup_name: String,
    pub properties: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct CreateMaterializedViewStmt {
    pub database: Option<String>,
    pub name: String,
    pub if_not_exists: bool,
    pub query: String,
    pub columns: Vec<String>,
    pub refresh: Option<RefreshClause>,
}

#[derive(Debug, Clone)]
pub struct RefreshClause {
    pub r#type: RefreshType,
    pub concurrency: Option<u32>,
}

#[derive(Debug, Clone, Copy)]
pub enum RefreshType {
    Complete,
    Fast,
}

#[derive(Debug, Clone)]
pub struct DropMaterializedViewStmt {
    pub database: Option<String>,
    pub name: String,
    pub if_exists: bool,
}

#[derive(Debug, Clone)]
pub struct AlterMaterializedViewStmt {
    pub database: Option<String>,
    pub name: String,
    pub operation: AlterMaterializedViewOperation,
}

#[derive(Debug, Clone)]
pub enum AlterMaterializedViewOperation {
    PauseRefresh,
    ResumeRefresh,
    Rename(String),
}

#[derive(Debug, Clone)]
pub struct RefreshMaterializedViewStmt {
    pub database: Option<String>,
    pub name: String,
    pub refresh_type: RefreshType,
}

#[derive(Debug, Clone)]
pub struct CreateUserStmt {
    pub username: String,
    pub hostname: Option<String>,
    pub auth_plugin: String,
    pub password: Option<String>,
    pub identified_by_password: bool,
    pub roles: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct DropUserStmt {
    pub username: String,
    pub hostname: Option<String>,
    pub if_exists: bool,
}

#[derive(Debug, Clone)]
pub struct CreateCatalogStmt {
    pub name: String,
    pub catalog_type: String,
    pub properties: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct DropCatalogStmt {
    pub name: String,
    pub if_exists: bool,
}

#[derive(Debug, Clone)]
pub struct RefreshCatalogStmt {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct ExportTableStmt {
    pub table: String,
    pub database: Option<String>,
    pub file_path: String,
    pub format: String,
    pub properties: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct BrokerLoadStmt {
    pub table: String,
    pub database: Option<String>,
    pub file_path: String,
    pub broker_name: String,
    pub properties: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct RoutineLoadStmt {
    pub table: String,
    pub database: Option<String>,
    pub job_name: String,
    pub kafka_broker_list: Option<String>,
    pub kafka_topic: Option<String>,
    pub properties: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct MysqlLoadStmt {
    pub table: String,
    pub database: Option<String>,
    pub file_path: String,
    pub properties: Vec<(String, String)>,
}
