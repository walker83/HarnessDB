use common::ProcedureError;
use tsql_parser::ast::*;
use mysql_protocol::server::{ColumnDef, ColumnType, QueryResult};
use crate::context::ExecutionContext;
use crate::interpreter::{TsqlExecutionResult, TsqlInterpreter};

pub fn execute_system_proc(
    ctx: &mut ExecutionContext,
    name: &str,
    _params: &[ExecuteParam],
    _interp: &TsqlInterpreter,
) -> Result<TsqlExecutionResult, ProcedureError> {
    let upper = name.to_uppercase();
    match upper.as_str() {
        "SP_HELP" => exec_sp_help(ctx),
        "SP_WHO" => exec_sp_who(ctx),
        "SP_HELPDB" => exec_sp_helpdb(ctx),
        "SP_TABLES" => exec_sp_tables(ctx),
        "SP_COLUMNS" => exec_sp_columns(ctx),
        "SP_DATABASES" => exec_sp_databases(ctx),
        "SP_SPACEUSED" => exec_sp_spaceused(ctx),
        "SP_SERVER_INFO" => exec_sp_server_info(ctx),
        "SP_VERSION" => exec_sp_version(ctx),
        _ => Ok(TsqlExecutionResult::Ok),
    }
}

pub fn execute_system_proc_stmt(
    ctx: &mut ExecutionContext,
    sp: &SystemProcStmt,
) -> Result<TsqlExecutionResult, ProcedureError> {
    match sp {
        SystemProcStmt::SpHelp { .. } => exec_sp_help(ctx),
        SystemProcStmt::SpWho { .. } => exec_sp_who(ctx),
        SystemProcStmt::SpHelpDb { .. } => exec_sp_helpdb(ctx),
        SystemProcStmt::SpTables { .. } => exec_sp_tables(ctx),
        SystemProcStmt::SpColumns { .. } => exec_sp_columns(ctx),
        SystemProcStmt::SpDatabases => exec_sp_databases(ctx),
        SystemProcStmt::SpSpaceUsed { .. } => exec_sp_spaceused(ctx),
        SystemProcStmt::SpServerInfo => exec_sp_server_info(ctx),
        SystemProcStmt::SpVersion => exec_sp_version(ctx),
        _ => Ok(TsqlExecutionResult::Ok),
    }
}

fn exec_sp_help(ctx: &ExecutionContext) -> Result<TsqlExecutionResult, ProcedureError> {
    let cols = vec![
        ColumnDef { name: "Name".into(), col_type: ColumnType::String },
        ColumnDef { name: "Owner".into(), col_type: ColumnType::String },
        ColumnDef { name: "Object_type".into(), col_type: ColumnType::String },
    ];
    let tables = ctx.catalog.list_tables(&ctx.current_database).unwrap_or_default();
    let mut rows: Vec<Vec<Option<String>>> = tables.iter().map(|t| {
        vec![Some(t.clone()), Some("dbo".into()), Some("user table".into())]
    }).collect();

    if let Ok(procs) = ctx.catalog.list_procedures(&ctx.current_database) {
        for p in procs {
            rows.push(vec![Some(p.name.clone()), Some("dbo".into()), Some("stored procedure".into())]);
        }
    }

    Ok(TsqlExecutionResult::ResultSet(QueryResult::with_rows(cols, rows)))
}

fn exec_sp_who(ctx: &ExecutionContext) -> Result<TsqlExecutionResult, ProcedureError> {
    let cols = vec![
        ColumnDef { name: "spid".into(), col_type: ColumnType::Int },
        ColumnDef { name: "status".into(), col_type: ColumnType::String },
        ColumnDef { name: "loginame".into(), col_type: ColumnType::String },
        ColumnDef { name: "hostname".into(), col_type: ColumnType::String },
        ColumnDef { name: "blk".into(), col_type: ColumnType::String },
        ColumnDef { name: "dbname".into(), col_type: ColumnType::String },
        ColumnDef { name: "cmd".into(), col_type: ColumnType::String },
    ];
    let rows = vec![vec![
        Some(ctx.conn_id.to_string()),
        Some("running".into()),
        Some("sa".into()),
        Some("localhost".into()),
        Some(" ".into()),
        Some(ctx.current_database.clone()),
        Some("AWAITING COMMAND".into()),
    ]];
    Ok(TsqlExecutionResult::ResultSet(QueryResult::with_rows(cols, rows)))
}

fn exec_sp_helpdb(ctx: &ExecutionContext) -> Result<TsqlExecutionResult, ProcedureError> {
    let cols = vec![
        ColumnDef { name: "name".into(), col_type: ColumnType::String },
        ColumnDef { name: "db_size".into(), col_type: ColumnType::String },
        ColumnDef { name: "owner".into(), col_type: ColumnType::String },
        ColumnDef { name: "dbid".into(), col_type: ColumnType::Int },
        ColumnDef { name: "created".into(), col_type: ColumnType::String },
        ColumnDef { name: "status".into(), col_type: ColumnType::String },
    ];
    let dbs = ctx.catalog.list_databases();
    let rows: Vec<Vec<Option<String>>> = dbs.iter().enumerate().map(|(i, db)| {
        vec![Some(db.clone()), Some("10 MB".into()), Some("sa".into()), Some((i + 1).to_string()), Some("2024-01-01".into()), Some("online".into())]
    }).collect();
    Ok(TsqlExecutionResult::ResultSet(QueryResult::with_rows(cols, rows)))
}

fn exec_sp_tables(ctx: &ExecutionContext) -> Result<TsqlExecutionResult, ProcedureError> {
    let cols = vec![
        ColumnDef { name: "TABLE_QUALIFIER".into(), col_type: ColumnType::String },
        ColumnDef { name: "TABLE_OWNER".into(), col_type: ColumnType::String },
        ColumnDef { name: "TABLE_NAME".into(), col_type: ColumnType::String },
        ColumnDef { name: "TABLE_TYPE".into(), col_type: ColumnType::String },
        ColumnDef { name: "REMARKS".into(), col_type: ColumnType::String },
    ];
    let tables = ctx.catalog.list_tables(&ctx.current_database).unwrap_or_default();
    let rows: Vec<Vec<Option<String>>> = tables.iter().map(|t| {
        vec![Some(ctx.current_database.clone()), Some("dbo".into()), Some(t.clone()), Some("TABLE".into()), None]
    }).collect();
    Ok(TsqlExecutionResult::ResultSet(QueryResult::with_rows(cols, rows)))
}

fn exec_sp_columns(_ctx: &ExecutionContext) -> Result<TsqlExecutionResult, ProcedureError> {
    let cols = vec![
        ColumnDef { name: "TABLE_NAME".into(), col_type: ColumnType::String },
        ColumnDef { name: "COLUMN_NAME".into(), col_type: ColumnType::String },
        ColumnDef { name: "TYPE_NAME".into(), col_type: ColumnType::String },
        ColumnDef { name: "PRECISION".into(), col_type: ColumnType::Int },
        ColumnDef { name: "LENGTH".into(), col_type: ColumnType::Int },
        ColumnDef { name: "NULLABLE".into(), col_type: ColumnType::Int },
    ];
    let rows: Vec<Vec<Option<String>>> = Vec::new();
    Ok(TsqlExecutionResult::ResultSet(QueryResult::with_rows(cols, rows)))
}

fn exec_sp_databases(ctx: &ExecutionContext) -> Result<TsqlExecutionResult, ProcedureError> {
    let cols = vec![
        ColumnDef { name: "DATABASE_NAME".into(), col_type: ColumnType::String },
        ColumnDef { name: "DATABASE_SIZE".into(), col_type: ColumnType::Int },
        ColumnDef { name: "REMARKS".into(), col_type: ColumnType::String },
    ];
    let dbs = ctx.catalog.list_databases();
    let rows: Vec<Vec<Option<String>>> = dbs.iter().map(|db| {
        vec![Some(db.clone()), Some(10240.to_string()), None]
    }).collect();
    Ok(TsqlExecutionResult::ResultSet(QueryResult::with_rows(cols, rows)))
}

fn exec_sp_spaceused(ctx: &ExecutionContext) -> Result<TsqlExecutionResult, ProcedureError> {
    let cols = vec![
        ColumnDef { name: "name".into(), col_type: ColumnType::String },
        ColumnDef { name: "rows".into(), col_type: ColumnType::String },
        ColumnDef { name: "reserved".into(), col_type: ColumnType::String },
        ColumnDef { name: "data".into(), col_type: ColumnType::String },
        ColumnDef { name: "index_size".into(), col_type: ColumnType::String },
        ColumnDef { name: "unused".into(), col_type: ColumnType::String },
    ];
    let rows = vec![vec![
        Some(ctx.current_database.clone()),
        Some("0".into()),
        Some("0 KB".into()),
        Some("0 KB".into()),
        Some("0 KB".into()),
        Some("0 KB".into()),
    ]];
    Ok(TsqlExecutionResult::ResultSet(QueryResult::with_rows(cols, rows)))
}

fn exec_sp_server_info(_ctx: &ExecutionContext) -> Result<TsqlExecutionResult, ProcedureError> {
    let cols = vec![
        ColumnDef { name: "attribute_id".into(), col_type: ColumnType::Int },
        ColumnDef { name: "attribute_name".into(), col_type: ColumnType::String },
        ColumnDef { name: "attribute_value".into(), col_type: ColumnType::String },
    ];
    let rows = vec![
        vec![Some("1".into()), Some("DBMS_NAME".into()), Some("HarnessDB".into())],
        vec![Some("2".into()), Some("DBMS_VERSION".into()), Some("16.0".into())],
        vec![Some("10".into()), Some("OWNER_TERM".into()), Some("owner".into())],
        vec![Some("11".into()), Some("TABLE_TERM".into()), Some("table".into())],
        vec![Some("12".into()), Some("COLUMN_TERM".into()), Some("column".into())],
    ];
    Ok(TsqlExecutionResult::ResultSet(QueryResult::with_rows(cols, rows)))
}

fn exec_sp_version(_ctx: &ExecutionContext) -> Result<TsqlExecutionResult, ProcedureError> {
    let cols = vec![
        ColumnDef { name: "Attribute".into(), col_type: ColumnType::String },
        ColumnDef { name: "Value".into(), col_type: ColumnType::String },
    ];
    let rows = vec![
        vec![Some("Sybase Server".into()), Some("HarnessDB 16.0".into())],
        vec![Some("Sybase Server Directory".into()), Some("/opt/harnessdb".into())],
    ];
    Ok(TsqlExecutionResult::ResultSet(QueryResult::with_rows(cols, rows)))
}
