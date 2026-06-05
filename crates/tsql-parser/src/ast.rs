//! T-SQL Abstract Syntax Tree definitions for SAP ASE 16 compatibility.
//!
//! This module defines the complete AST for the T-SQL dialect including
//! stored procedures, control flow, cursors, error handling, and all
//! SAP ASE-specific syntax extensions.

use serde::{Deserialize, Serialize};

// ============================================================================
// Top-level statement
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TsqlStatement {
    // ── DDL ──
    CreateTable(TsqlCreateTable),
    AlterTable(TsqlAlterTable),
    DropTable(TsqlDropTable),
    TruncateTable(TsqlTruncateTable),
    CreateDatabase(TsqlCreateDatabase),
    DropDatabase(TsqlDropDatabase),
    CreateIndex(TsqlCreateIndex),
    DropIndex(TsqlDropIndex),
    CreateView(TsqlCreateView),
    DropView(TsqlDropView),

    // ── DML ──
    Select(TsqlSelect),
    Insert(TsqlInsert),
    Update(TsqlUpdate),
    Delete(TsqlDelete),
    Merge(TsqlMerge),

    // ── Stored Procedures ──
    CreateProcedure(CreateProcedureStmt),
    AlterProcedure(AlterProcedureStmt),
    DropProcedure(DropProcedureStmt),
    Execute(ExecuteStmt),

    // ── Control Flow ──
    BeginEnd(Vec<TsqlStatement>),
    IfElse {
        condition: TsqlExpr,
        then_body: Vec<TsqlStatement>,
        else_body: Option<Vec<TsqlStatement>>,
    },
    While {
        condition: TsqlExpr,
        body: Vec<TsqlStatement>,
    },
    Return(Option<TsqlExpr>),
    Goto(String),
    Label(String),
    WaitFor(WaitForType),
    Break,
    Continue,

    // ── Variables ──
    Declare(DeclareStmt),
    SetVariable(SetVariableStmt),
    SelectIntoVars(SelectIntoVarsStmt),
    Print(TsqlExpr),

    // ── Cursors ──
    DeclareCursor(DeclareCursorStmt),
    OpenCursor(String),
    FetchCursor(FetchCursorStmt),
    CloseCursor(String),
    DeallocateCursor(String),

    // ── Error Handling ──
    TryCatch(TryCatchStmt),
    Raiserror(RaiserrorStmt),
    Throw {
        error_number: Option<i32>,
        message: TsqlExpr,
        state: Option<i32>,
    },

    // ── Transactions ──
    BeginTransaction(Option<String>),
    CommitTransaction(Option<String>),
    RollbackTransaction(Option<String>),
    SaveTransaction(String),

    // ── Temp Tables ──
    CreateTempTable(TsqlCreateTable),

    // ── System Procedures ──
    SystemProcedure(SystemProcStmt),

    // ── Text/Image Operations ──
    ReadText {
        table: String,
        column: String,
        offset: TsqlExpr,
        size: TsqlExpr,
    },
    WriteText {
        table: String,
        column: String,
        offset: Option<TsqlExpr>,
        data: TsqlExpr,
    },
    UpdateText {
        table: String,
        column: String,
        offset: TsqlExpr,
        size: TsqlExpr,
        data: TsqlExpr,
    },

    // ── Utility ──
    UseDatabase(String),
    SetOption(String, TsqlExpr),
    Compute {
        exprs: Vec<TsqlExpr>,
        by: Vec<TsqlExpr>,
    },
    Batch(Vec<TsqlStatement>),
    Passthrough(String),
    NoOp,
}

// ============================================================================
// DDL Structures
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TsqlCreateTable {
    pub name: String,
    pub database: Option<String>,
    pub columns: Vec<TsqlColumnDef>,
    pub constraints: Vec<TsqlTableConstraint>,
    pub on_clause: Option<String>,
    pub with_clause: Option<Vec<(String, String)>>,
    pub text_image_on: Option<String>,
    pub lock_datapages: Option<bool>,
    pub is_temp: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TsqlColumnDef {
    pub name: String,
    pub data_type: TsqlDataType,
    pub nullable: Option<bool>,
    pub default: Option<TsqlExpr>,
    pub identity: Option<IdentityDef>,
    pub not_for_replication: bool,
    pub constraint: Option<TsqlColumnConstraint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityDef {
    pub seed: i64,
    pub increment: i64,
    pub not_for_replication: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TsqlColumnConstraint {
    PrimaryKey { clustered: Option<bool> },
    Unique { clustered: Option<bool> },
    References { table: String, column: String },
    Check(TsqlExpr),
    Default(TsqlExpr),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TsqlTableConstraint {
    PrimaryKey {
        name: Option<String>,
        columns: Vec<String>,
        clustered: Option<bool>,
    },
    Unique {
        name: Option<String>,
        columns: Vec<String>,
        clustered: Option<bool>,
    },
    ForeignKey {
        name: Option<String>,
        columns: Vec<String>,
        ref_table: String,
        ref_columns: Vec<String>,
    },
    Check {
        name: Option<String>,
        expr: TsqlExpr,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TsqlAlterTable {
    pub name: String,
    pub action: AlterTableAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlterTableAction {
    Add(TsqlColumnDef),
    DropColumn(String),
    AlterColumn(TsqlColumnDef),
    AddConstraint(TsqlTableConstraint),
    DropConstraint(String),
    EnableTrigger(String),
    DisableTrigger(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TsqlDropTable {
    pub name: String,
    pub if_exists: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TsqlTruncateTable {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TsqlCreateDatabase {
    pub name: String,
    pub on_clause: Option<String>,
    pub log_on: Option<String>,
    pub with_clause: Option<Vec<(String, String)>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TsqlDropDatabase {
    pub name: String,
    pub if_exists: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TsqlCreateIndex {
    pub name: String,
    pub table: String,
    pub columns: Vec<TsqlIndexColumn>,
    pub unique: bool,
    pub clustered: Option<bool>,
    pub with_clause: Option<Vec<(String, String)>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TsqlIndexColumn {
    pub name: String,
    pub ascending: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TsqlDropIndex {
    pub name: String,
    pub table: Option<String>,
    pub if_exists: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TsqlCreateView {
    pub name: String,
    pub columns: Vec<String>,
    pub query: Box<TsqlSelect>,
    pub with_check: bool,
    pub with_encryption: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TsqlDropView {
    pub name: String,
    pub if_exists: bool,
}

// ============================================================================
// DML Structures
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TsqlSelect {
    pub distinct: bool,
    pub top: Option<TopClause>,
    pub select_list: Vec<TsqlSelectItem>,
    pub into_table: Option<String>,
    pub from: Option<TsqlTableRef>,
    pub where_clause: Option<TsqlExpr>,
    pub group_by: Vec<TsqlExpr>,
    pub having: Option<TsqlExpr>,
    pub order_by: Vec<TsqlOrderBy>,
    pub compute: Option<ComputeClause>,
    pub for_browse: bool,
    pub option_hints: Vec<String>,
    pub union: Option<Box<TsqlSelect>>,
    pub union_all: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopClause {
    pub count: TsqlExpr,
    pub percent: bool,
    pub with_ties: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeClause {
    pub aggregates: Vec<TsqlExpr>,
    pub by: Vec<TsqlExpr>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TsqlSelectItem {
    pub expr: TsqlExpr,
    pub alias: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TsqlOrderBy {
    pub expr: TsqlExpr,
    pub ascending: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TsqlTableRef {
    Table {
        database: Option<String>,
        schema: Option<String>,
        name: String,
        alias: Option<String>,
        hints: Vec<String>,
    },
    Join {
        left: Box<TsqlTableRef>,
        right: Box<TsqlTableRef>,
        join_type: TsqlJoinType,
        condition: Option<TsqlExpr>,
    },
    Subquery {
        query: Box<TsqlSelect>,
        alias: String,
    },
    DerivedTable {
        query: Box<TsqlSelect>,
        alias: String,
        columns: Vec<String>,
    },
    TempTable {
        name: String,
        alias: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TsqlJoinType {
    Inner,
    LeftOuter,
    RightOuter,
    FullOuter,
    Cross,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TsqlInsert {
    pub table: String,
    pub columns: Vec<String>,
    pub source: InsertSource,
    pub output_clause: Option<Vec<TsqlExpr>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InsertSource {
    Values(Vec<Vec<TsqlExpr>>),
    Select(Box<TsqlSelect>),
    Execute(Box<ExecuteStmt>),
    DefaultValues,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TsqlUpdate {
    pub table: String,
    pub alias: Option<String>,
    pub assignments: Vec<(String, TsqlExpr)>,
    pub from: Option<TsqlTableRef>,
    pub where_clause: Option<TsqlExpr>,
    pub output_clause: Option<Vec<TsqlExpr>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TsqlDelete {
    pub table: String,
    pub alias: Option<String>,
    pub from: Option<TsqlTableRef>,
    pub where_clause: Option<TsqlExpr>,
    pub output_clause: Option<Vec<TsqlExpr>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TsqlMerge {
    pub target: TsqlTableRef,
    pub target_alias: Option<String>,
    pub source: Box<TsqlTableRef>,
    pub source_alias: Option<String>,
    pub on_condition: TsqlExpr,
    pub when_matched: Option<MergeAction>,
    pub when_not_matched: Option<MergeAction>,
    pub when_not_matched_by_source: Option<MergeAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MergeAction {
    Insert { columns: Vec<String>, values: Vec<TsqlExpr> },
    Update { assignments: Vec<(String, TsqlExpr)> },
    Delete,
}

// ============================================================================
// Stored Procedure Structures
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProcedureStmt {
    pub database: Option<String>,
    pub name: String,
    pub params: Vec<ProcedureParam>,
    pub body: Vec<TsqlStatement>,
    pub with_recompile: bool,
    pub with_encryption: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlterProcedureStmt {
    pub database: Option<String>,
    pub name: String,
    pub params: Vec<ProcedureParam>,
    pub body: Vec<TsqlStatement>,
    pub with_recompile: bool,
    pub with_encryption: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DropProcedureStmt {
    pub name: String,
    pub if_exists: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcedureParam {
    pub name: String,
    pub data_type: TsqlDataType,
    pub direction: ParamDirection,
    pub default: Option<TsqlExpr>,
    pub output: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ParamDirection {
    Input,
    Output,
    InOut,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteStmt {
    pub procedure: String,
    pub params: Vec<ExecuteParam>,
    pub return_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecuteParam {
    Positional(TsqlExpr),
    Named {
        name: String,
        value: TsqlExpr,
        output: bool,
    },
}

// ============================================================================
// Variable Structures
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclareStmt {
    pub variables: Vec<VariableDecl>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableDecl {
    pub name: String,
    pub data_type: TsqlDataType,
    pub default: Option<TsqlExpr>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetVariableStmt {
    pub assignments: Vec<(String, TsqlExpr)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectIntoVarsStmt {
    pub assignments: Vec<(String, TsqlExpr)>,
    pub from: Option<TsqlTableRef>,
    pub where_clause: Option<TsqlExpr>,
    pub group_by: Vec<TsqlExpr>,
    pub having: Option<TsqlExpr>,
    pub order_by: Vec<TsqlOrderBy>,
}

// ============================================================================
// Cursor Structures
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclareCursorStmt {
    pub name: String,
    pub scroll_type: CursorScrollType,
    pub sensitivity: CursorSensitivity,
    pub query: Box<TsqlSelect>,
    pub for_read_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum CursorScrollType {
    ForwardOnly,
    Scroll,
    Keyset,
    Dynamic,
    Static,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum CursorSensitivity {
    Unspecified,
    Insensitive,
    Sensitive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchCursorStmt {
    pub cursor_name: String,
    pub fetch_orientation: FetchOrientation,
    pub into_variables: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FetchOrientation {
    Next,
    Prior,
    First,
    Last,
    Absolute(i64),
    Relative(i64),
}

// ============================================================================
// Error Handling Structures
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TryCatchStmt {
    pub try_body: Vec<TsqlStatement>,
    pub catch_body: Vec<TsqlStatement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaiserrorStmt {
    pub message_or_id: TsqlExpr,
    pub severity: TsqlExpr,
    pub state: TsqlExpr,
    pub with_log: bool,
}

// ============================================================================
// System Procedure Structures
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemProcStmt {
    SpHelp { object: Option<String> },
    SpWho { login_name: Option<String> },
    SpHelpDb { database: Option<String> },
    SpTables {
        table_name: Option<String>,
        table_owner: Option<String>,
        table_type: Option<String>,
    },
    SpColumns {
        table_name: String,
        column_name: Option<String>,
        table_owner: Option<String>,
    },
    SpHelpIndex { table: String },
    SpDatabases,
    SpServerInfo,
    SpVersion,
    SpLock,
    SpDepends { object: String },
    SpRename { object: String, new_name: String },
    SpChangeDbOwner { database: String, owner: String },
    SpConfigure { option: Option<String>, value: Option<String> },
    SpSpaceUsed { table: Option<String> },
    SpHelpConstraint { table: String },
    SpHelpKey { table: String },
    SpHelpType,
    SpHelpUser,
    SpPasswd {
        login: String,
        old_passwd: Option<String>,
        new_passwd: Option<String>,
    },
    Custom {
        name: String,
        params: Vec<TsqlExpr>,
    },
}

// ============================================================================
// Expressions
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TsqlExpr {
    Literal(TsqlLiteral),
    ColumnRef {
        database: Option<String>,
        schema: Option<String>,
        table: Option<String>,
        column: String,
    },
    Variable(String),
    SystemVariable(String),
    BinaryOp {
        left: Box<TsqlExpr>,
        op: TsqlBinaryOp,
        right: Box<TsqlExpr>,
    },
    UnaryOp {
        op: TsqlUnaryOp,
        expr: Box<TsqlExpr>,
    },
    FunctionCall {
        name: String,
        args: Vec<TsqlExpr>,
    },
    Cast {
        expr: Box<TsqlExpr>,
        data_type: TsqlDataType,
    },
    Convert {
        data_type: TsqlDataType,
        expr: Box<TsqlExpr>,
        style: Option<Box<TsqlExpr>>,
    },
    CaseWhen {
        operand: Option<Box<TsqlExpr>>,
        when_clauses: Vec<(TsqlExpr, TsqlExpr)>,
        else_expr: Option<Box<TsqlExpr>>,
    },
    IsNull {
        expr: Box<TsqlExpr>,
        replacement: Box<TsqlExpr>,
    },
    Coalesce(Vec<TsqlExpr>),
    NullIf {
        expr: Box<TsqlExpr>,
        other: Box<TsqlExpr>,
    },
    Subquery(Box<TsqlSelect>),
    InList {
        expr: Box<TsqlExpr>,
        list: Vec<TsqlExpr>,
        negated: bool,
    },
    InSubquery {
        expr: Box<TsqlExpr>,
        query: Box<TsqlSelect>,
        negated: bool,
    },
    Between {
        expr: Box<TsqlExpr>,
        low: Box<TsqlExpr>,
        high: Box<TsqlExpr>,
        negated: bool,
    },
    Like {
        expr: Box<TsqlExpr>,
        pattern: Box<TsqlExpr>,
        negated: bool,
        escape: Option<Box<TsqlExpr>>,
    },
    Exists(Box<TsqlSelect>),
    Top {
        count: Box<TsqlExpr>,
        with_ties: bool,
    },
    OldStyleJoin {
        left: Box<TsqlExpr>,
        right: Box<TsqlExpr>,
        join_type: OldStyleJoinType,
    },
    Wildcard,
    TableWildcard {
        table: String,
    },
    GlobalVariable(String),
    CompoundAssign {
        var: String,
        op: TsqlBinaryOp,
        value: Box<TsqlExpr>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TsqlBinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    And,
    Or,
    StringConcat,
    Assign,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TsqlUnaryOp {
    Not,
    Negate,
    BitwiseNot,
    Positive,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum OldStyleJoinType {
    LeftOuter,
    RightOuter,
}

// ============================================================================
// Literals
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TsqlLiteral {
    Null,
    Int(i64),
    Float(f64),
    String(String),
    Binary(Vec<u8>),
    Money(String),
    DateTime(String),
    Bit(bool),
}

// ============================================================================
// T-SQL Data Types
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TsqlDataType {
    Int,
    SmallInt,
    TinyInt,
    BigInt,
    Bit,
    Decimal(Option<u8>, Option<u8>),
    Numeric(Option<u8>, Option<u8>),
    Money,
    SmallMoney,
    Float(Option<u8>),
    Real,
    Char(Option<usize>),
    Varchar(Option<usize>),
    NChar(Option<usize>),
    NVarchar(Option<usize>),
    NText,
    Text,
    Binary(Option<usize>),
    VarBinary(Option<usize>),
    Image,
    Date,
    Time(Option<u8>),
    DateTime,
    SmallDateTime,
    DateTime2(Option<u8>),
    DateTimeOffset(Option<u8>),
    UniqueIdentifier,
    Xml,
    SqlVariant,
    Table,
    CursorType,
    UserDefined(String),
}

impl std::fmt::Display for TsqlDataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Int => write!(f, "INT"),
            Self::SmallInt => write!(f, "SMALLINT"),
            Self::TinyInt => write!(f, "TINYINT"),
            Self::BigInt => write!(f, "BIGINT"),
            Self::Bit => write!(f, "BIT"),
            Self::Decimal(p, s) => match (p, s) {
                (Some(p), Some(s)) => write!(f, "DECIMAL({}, {})", p, s),
                (Some(p), None) => write!(f, "DECIMAL({})", p),
                _ => write!(f, "DECIMAL"),
            },
            Self::Numeric(p, s) => match (p, s) {
                (Some(p), Some(s)) => write!(f, "NUMERIC({}, {})", p, s),
                (Some(p), None) => write!(f, "NUMERIC({})", p),
                _ => write!(f, "NUMERIC"),
            },
            Self::Money => write!(f, "MONEY"),
            Self::SmallMoney => write!(f, "SMALLMONEY"),
            Self::Float(p) => match p {
                Some(p) => write!(f, "FLOAT({})", p),
                None => write!(f, "FLOAT"),
            },
            Self::Real => write!(f, "REAL"),
            Self::Char(n) => match n {
                Some(n) => write!(f, "CHAR({})", n),
                None => write!(f, "CHAR"),
            },
            Self::Varchar(n) => match n {
                Some(n) => write!(f, "VARCHAR({})", n),
                None => write!(f, "VARCHAR"),
            },
            Self::NChar(n) => match n {
                Some(n) => write!(f, "NCHAR({})", n),
                None => write!(f, "NCHAR"),
            },
            Self::NVarchar(n) => match n {
                Some(n) => write!(f, "NVARCHAR({})", n),
                None => write!(f, "NVARCHAR"),
            },
            Self::NText => write!(f, "NTEXT"),
            Self::Text => write!(f, "TEXT"),
            Self::Binary(n) => match n {
                Some(n) => write!(f, "BINARY({})", n),
                None => write!(f, "BINARY"),
            },
            Self::VarBinary(n) => match n {
                Some(n) => write!(f, "VARBINARY({})", n),
                None => write!(f, "VARBINARY"),
            },
            Self::Image => write!(f, "IMAGE"),
            Self::Date => write!(f, "DATE"),
            Self::Time(p) => match p {
                Some(p) => write!(f, "TIME({})", p),
                None => write!(f, "TIME"),
            },
            Self::DateTime => write!(f, "DATETIME"),
            Self::SmallDateTime => write!(f, "SMALLDATETIME"),
            Self::DateTime2(p) => match p {
                Some(p) => write!(f, "DATETIME2({})", p),
                None => write!(f, "DATETIME2"),
            },
            Self::DateTimeOffset(p) => match p {
                Some(p) => write!(f, "DATETIMEOFFSET({})", p),
                None => write!(f, "DATETIMEOFFSET"),
            },
            Self::UniqueIdentifier => write!(f, "UNIQUEIDENTIFIER"),
            Self::Xml => write!(f, "XML"),
            Self::SqlVariant => write!(f, "SQL_VARIANT"),
            Self::Table => write!(f, "TABLE"),
            Self::CursorType => write!(f, "CURSOR"),
            Self::UserDefined(name) => write!(f, "{}", name),
        }
    }
}

// ============================================================================
// WaitFor Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WaitForType {
    Delay(String),
    Time(String),
}
