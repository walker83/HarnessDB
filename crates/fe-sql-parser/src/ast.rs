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
    // Batch 1 additions
    AlterDatabase(AlterDatabaseStmt),
    ShowCreateDatabase(String),
    DropView(DropViewStmt),
    AlterView(AlterViewStmt),
    ShowCreateView(String, String),
    
    // Batch 3: Data export statements
    ExportTable(ExportTableStmt),
    CancelExport(CancelExportStmt),
    ShowExport(ShowExportStmt),
    
    // Batch 4: UDF function management
    CreateFunction(CreateFunctionStmt),
    DropFunction(DropFunctionStmt),
    ShowFunctions(Option<String>),
    ShowCreateFunction(String, String),
    DescFunction(String, String),
    
    // Batch 4: Statistics management
    AnalyzeTable(AnalyzeTableStmt),
    AlterStats(AlterStatsStmt),
    DropStats(DropStatsStmt),
    DropAnalyzeJob(DropAnalyzeJobStmt),
    KillAnalyzeJob(KillAnalyzeJobStmt),
    ShowAnalyze(ShowAnalyzeStmt),
    ShowStats(ShowStatsStmt),
    ShowTableStats(ShowTableStatsStmt),
    
    // Batch 4: Job management
    CreateJob(CreateJobStmt),
    DropJob(DropJobStmt),
    PauseJob(PauseJobStmt),
    ResumeJob(ResumeJobStmt),
    CancelTask(CancelTaskStmt),
    
    // Batch 4: Plugin management
    InstallPlugin(InstallPluginStmt),
    UninstallPlugin(UninstallPluginStmt),
    ShowPlugins,
    
    // Batch 4: Recycle bin management
    RecoverDatabase(RecoverDatabaseStmt),
    RecoverTable(RecoverTableStmt),
    RecoverPartition(RecoverPartitionStmt),
    DropCatalogRecycleBin(DropCatalogRecycleBinStmt),
    ShowCatalogRecycleBin,
    
    // Batch 4: Data governance
    CreateSqlBlockRule(CreateSqlBlockRuleStmt),
    AlterSqlBlockRule(AlterSqlBlockRuleStmt),
    DropSqlBlockRule(DropSqlBlockRuleStmt),
    ShowSqlBlockRule(ShowSqlBlockRuleStmt),
    CreateRowPolicy(CreateRowPolicyStmt),
    DropRowPolicy(DropRowPolicyStmt),
    ShowRowPolicy(ShowRowPolicyStmt),
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
    RenameColumn { old_name: String, new_name: String },
    SetComment(String),
    SetProperty(Vec<(String, String)>),
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

// Batch 1: New statement types

#[derive(Debug, Clone)]
pub struct AlterDatabaseStmt {
    pub name: String,
    pub properties: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct DropViewStmt {
    pub database: Option<String>,
    pub name: String,
    pub if_exists: bool,
}

#[derive(Debug, Clone)]
pub struct AlterViewStmt {
    pub database: Option<String>,
    pub name: String,
    pub query: String,
}

// Batch 3: Data export structures

#[derive(Debug, Clone)]
pub struct ExportTableStmt {
    pub database: Option<String>,
    pub table: String,
    pub path: String,
    pub properties: Vec<(String, String)>,
    pub columns: Option<Vec<String>>,
    pub where_clause: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CancelExportStmt {
    pub export_id: String,
}

#[derive(Debug, Clone)]
pub struct ShowExportStmt {
    pub export_id: Option<String>,
    pub state: Option<String>,
}

// Batch 4: UDF function management structures

#[derive(Debug, Clone)]
pub struct CreateFunctionStmt {
    pub name: String,
    pub function_type: String,
    pub input_types: Vec<String>,
    pub return_type: String,
    pub properties: Vec<(String, String)>,
    pub library_path: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DropFunctionStmt {
    pub name: String,
    pub input_types: Option<Vec<String>>,
    pub if_exists: bool,
}

// Batch 4: Statistics management structures

#[derive(Debug, Clone)]
pub struct AnalyzeTableStmt {
    pub database: Option<String>,
    pub table: String,
    pub columns: Option<Vec<String>>,
    pub analyze_type: AnalyzeType,
    pub sample_rate: Option<f64>,
    pub async_mode: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum AnalyzeType {
    Full,
    Sample,
}

#[derive(Debug, Clone)]
pub struct AlterStatsStmt {
    pub database: Option<String>,
    pub table: String,
    pub properties: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct DropStatsStmt {
    pub database: Option<String>,
    pub table: String,
    pub columns: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct DropAnalyzeJobStmt {
    pub job_id: String,
}

#[derive(Debug, Clone)]
pub struct KillAnalyzeJobStmt {
    pub job_id: String,
}

#[derive(Debug, Clone)]
pub struct ShowAnalyzeStmt {
    pub job_id: Option<String>,
    pub state: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ShowStatsStmt {
    pub database: Option<String>,
    pub table: String,
}

#[derive(Debug, Clone)]
pub struct ShowTableStatsStmt {
    pub database: Option<String>,
    pub table: String,
}

// Batch 4: Job management structures

#[derive(Debug, Clone)]
pub struct CreateJobStmt {
    pub name: String,
    pub database: Option<String>,
    pub schedule: String,
    pub job_type: JobType,
    pub definition: String,
    pub properties: Vec<(String, String)>,
}

#[derive(Debug, Clone, Copy)]
pub enum JobType {
    Cron,
    OneTime,
}

#[derive(Debug, Clone)]
pub struct DropJobStmt {
    pub name: String,
    pub database: Option<String>,
    pub if_exists: bool,
}

#[derive(Debug, Clone)]
pub struct PauseJobStmt {
    pub name: String,
    pub database: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResumeJobStmt {
    pub name: String,
    pub database: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CancelTaskStmt {
    pub task_id: String,
}

// Batch 4: Plugin management structures

#[derive(Debug, Clone)]
pub struct InstallPluginStmt {
    pub plugin_name: String,
    pub plugin_path: String,
    pub properties: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct UninstallPluginStmt {
    pub plugin_name: String,
}

// Batch 4: Recycle bin management structures

#[derive(Debug, Clone)]
pub struct RecoverDatabaseStmt {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct RecoverTableStmt {
    pub database: String,
    pub table: String,
}

#[derive(Debug, Clone)]
pub struct RecoverPartitionStmt {
    pub database: String,
    pub table: String,
    pub partition: String,
}

#[derive(Debug, Clone)]
pub struct DropCatalogRecycleBinStmt {
    pub type_filter: Option<String>,
    pub name_filter: Option<String>,
}

// Batch 4: Data governance structures

#[derive(Debug, Clone)]
pub struct CreateSqlBlockRuleStmt {
    pub name: String,
    pub properties: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct AlterSqlBlockRuleStmt {
    pub name: String,
    pub properties: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct DropSqlBlockRuleStmt {
    pub name: String,
    pub if_exists: bool,
}

#[derive(Debug, Clone)]
pub struct ShowSqlBlockRuleStmt {
    pub name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CreateRowPolicyStmt {
    pub name: String,
    pub database: String,
    pub table: String,
    pub policy_type: RowPolicyType,
    pub filter: String,
    pub properties: Vec<(String, String)>,
}

#[derive(Debug, Clone, Copy)]
pub enum RowPolicyType {
    Permit,
    Restrict,
}

#[derive(Debug, Clone)]
pub struct DropRowPolicyStmt {
    pub name: String,
    pub database: String,
    pub table: String,
    pub if_exists: bool,
}

#[derive(Debug, Clone)]
pub struct ShowRowPolicyStmt {
    pub name: Option<String>,
    pub database: Option<String>,
    pub table: Option<String>,
}
