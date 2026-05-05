use crate::ast::*;
use crate::error::ParseError;

pub fn parse_sql(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let trimmed = sql.trim().to_uppercase();

    if trimmed.starts_with("CREATE REPOSITORY") {
        return parse_create_repository(sql);
    }
    if trimmed.starts_with("DROP REPOSITORY") {
        return parse_drop_repository(sql);
    }
    if trimmed.starts_with("SHOW REPOSITORIES") {
        return Ok(vec![Statement::ShowRepositories]);
    }
    if trimmed.starts_with("BACKUP DATABASE") {
        return parse_backup_database(sql);
    }
    if trimmed.starts_with("RESTORE DATABASE") {
        return parse_restore_database(sql);
    }
    if trimmed.starts_with("CREATE MATERIALIZED VIEW") {
        return parse_create_materialized_view(sql);
    }
    if trimmed.starts_with("DROP MATERIALIZED VIEW") {
        return parse_drop_materialized_view(sql);
    }
    if trimmed.starts_with("ALTER MATERIALIZED VIEW") {
        return parse_alter_materialized_view(sql);
    }
    if trimmed.starts_with("REFRESH MATERIALIZED VIEW") {
        return parse_refresh_materialized_view(sql);
    }
    if trimmed.starts_with("CREATE USER") || trimmed.starts_with("CREATE USER IF NOT EXISTS") {
        return parse_create_user(sql);
    }
    if trimmed.starts_with("DROP USER") {
        return parse_drop_user(sql);
    }
    if trimmed.starts_with("SHOW USERS") || trimmed == "SHOW USERS" {
        return Ok(vec![Statement::ShowUsers]);
    }
    if trimmed.starts_with("CREATE CATALOG") {
        return parse_create_catalog(sql);
    }
    if trimmed.starts_with("DROP CATALOG") {
        return parse_drop_catalog(sql);
    }
    if trimmed.starts_with("SHOW CATALOGS") {
        return Ok(vec![Statement::ShowCatalogs]);
    }
    if trimmed.starts_with("REFRESH CATALOG") {
        return parse_refresh_catalog(sql);
    }

    let dialect = sqlparser::dialect::MySqlDialect {};
    let statements = sqlparser::parser::Parser::parse_sql(&dialect, sql)
        .map_err(|e| ParseError::SyntaxError {
            position: 0,
            message: e.to_string(),
        })?;

    statements
        .into_iter()
        .map(convert_statement)
        .collect()
}

fn parse_create_repository(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let sql = sql.trim();
    let after_create = sql.strip_prefix("CREATE REPOSITORY")
        .ok_or_else(|| ParseError::SyntaxError {
            position: 0,
            message: "Expected CREATE REPOSITORY".to_string(),
        })?
        .trim();

    let (name, rest) = extract_identifier(after_create)
        .ok_or_else(|| ParseError::SyntaxError {
            position: 0,
            message: "Expected repository name".to_string(),
        })?;

    let rest = rest.trim();
    let mut repo_type = RepositoryType::Local;
    let mut properties = vec![];

    if rest.starts_with("WITH") {
        let after_with = rest.strip_prefix("WITH").unwrap().trim();
        if after_with.starts_with("S3") || after_with.starts_with("s3") {
            repo_type = RepositoryType::S3;
            let after_s3 = after_with
                .strip_prefix("S3")
                .or_else(|| after_with.strip_prefix("s3"))
                .unwrap_or("")
                .trim();
            properties = parse_properties(after_s3);
        } else if after_with.starts_with("HDFS") || after_with.starts_with("hdfs") {
            repo_type = RepositoryType::Hdfs;
            let after_hdfs = after_with
                .strip_prefix("HDFS")
                .or_else(|| after_with.strip_prefix("hdfs"))
                .unwrap_or("")
                .trim();
            properties = parse_properties(after_hdfs);
        } else {
            properties = parse_properties(after_with);
        }
    }

    Ok(vec![Statement::CreateRepository(CreateRepositoryStmt {
        name: name.to_string(),
        repo_type,
        properties,
    })])
}

fn parse_drop_repository(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let sql = sql.trim();
    let after_drop = sql.strip_prefix("DROP REPOSITORY")
        .ok_or_else(|| ParseError::SyntaxError {
            position: 0,
            message: "Expected DROP REPOSITORY".to_string(),
        })?
        .trim();

    let if_exists = after_drop.starts_with("IF EXISTS");
    let name_part = if if_exists {
        after_drop.strip_prefix("IF EXISTS").unwrap().trim()
    } else {
        after_drop
    };

    let (name, _) = extract_identifier(name_part)
        .ok_or_else(|| ParseError::SyntaxError {
            position: 0,
            message: "Expected repository name".to_string(),
        })?;

    Ok(vec![Statement::DropRepository(DropRepositoryStmt {
        name: name.to_string(),
        if_exists,
    })])
}

fn parse_backup_database(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let sql = sql.trim();
    let after_backup = sql.strip_prefix("BACKUP DATABASE")
        .ok_or_else(|| ParseError::SyntaxError {
            position: 0,
            message: "Expected BACKUP DATABASE".to_string(),
        })?
        .trim();

    let (database, rest) = extract_identifier(after_backup)
        .ok_or_else(|| ParseError::SyntaxError {
            position: 0,
            message: "Expected database name".to_string(),
        })?;

    let rest = rest.trim();
    let rest = rest.strip_prefix("TO")
        .ok_or_else(|| ParseError::SyntaxError {
            position: 0,
            message: "Expected TO".to_string(),
        })?
        .trim();

    let (repository, rest) = extract_identifier(rest)
        .ok_or_else(|| ParseError::SyntaxError {
            position: 0,
            message: "Expected repository name".to_string(),
        })?;

    let rest = rest.trim();
    let backup_name = if rest.starts_with("BACKUP") {
        let after_backup = rest.strip_prefix("BACKUP").unwrap().trim();
        let (name, _) = extract_identifier(after_backup)
            .ok_or_else(|| ParseError::SyntaxError {
                position: 0,
                message: "Expected backup name".to_string(),
            })?;
        name.to_string()
    } else {
        format!("{}_{}", database, chrono_lite_timestamp())
    };

    let properties = parse_properties(rest);

    Ok(vec![Statement::BackupDatabase(BackupDatabaseStmt {
        database: database.to_string(),
        repository: repository.to_string(),
        backup_name,
        properties,
    })])
}

fn parse_restore_database(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let sql = sql.trim();
    let after_restore = sql.strip_prefix("RESTORE DATABASE")
        .ok_or_else(|| ParseError::SyntaxError {
            position: 0,
            message: "Expected RESTORE DATABASE".to_string(),
        })?
        .trim();

    let (database, rest) = extract_identifier(after_restore)
        .ok_or_else(|| ParseError::SyntaxError {
            position: 0,
            message: "Expected database name".to_string(),
        })?;

    let rest = rest.trim();
    let rest = rest.strip_prefix("FROM")
        .ok_or_else(|| ParseError::SyntaxError {
            position: 0,
            message: "Expected FROM".to_string(),
        })?
        .trim();

    let (repository, rest) = extract_identifier(rest)
        .ok_or_else(|| ParseError::SyntaxError {
            position: 0,
            message: "Expected repository name".to_string(),
        })?;

    let rest = rest.trim();
    let rest = rest.strip_prefix("BACKUP")
        .ok_or_else(|| ParseError::SyntaxError {
            position: 0,
            message: "Expected BACKUP".to_string(),
        })?
        .trim();

    let (backup_name, _) = extract_identifier(rest)
        .ok_or_else(|| ParseError::SyntaxError {
            position: 0,
            message: "Expected backup name".to_string(),
        })?;

    Ok(vec![Statement::RestoreDatabase(RestoreDatabaseStmt {
        database: database.to_string(),
        repository: repository.to_string(),
        backup_name: backup_name.to_string(),
        properties: vec![],
    })])
}

fn parse_create_materialized_view(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let sql = sql.trim();
    let after_create = sql.strip_prefix("CREATE MATERIALIZED VIEW")
        .ok_or_else(|| ParseError::SyntaxError {
            position: 0,
            message: "Expected CREATE MATERIALIZED VIEW".to_string(),
        })?
        .trim();

    let if_not_exists = after_create.starts_with("IF NOT EXISTS");
    let rest = if if_not_exists {
        after_create.strip_prefix("IF NOT EXISTS").unwrap().trim()
    } else {
        after_create
    };

    let (name, rest) = extract_identifier(rest)
        .ok_or_else(|| ParseError::SyntaxError {
            position: 0,
            message: "Expected materialized view name".to_string(),
        })?;

    let rest = rest.trim();
    let name_str = name.to_string();
    let parts: Vec<&str> = name_str.split('.').collect();
    let (database, view_name) = if parts.len() == 2 {
        (Some(parts[0].to_string()), parts[1].to_string())
    } else {
        (None, name_str)
    };

    let mut columns = Vec::new();
    let mut query = String::new();
    let mut refresh = None;

    if rest.starts_with('(') {
        if let Some(end) = rest.find(')') {
            let cols_str = &rest[1..end];
            for col in cols_str.split(',') {
                columns.push(col.trim().to_string());
            }
            query = rest[end + 1..].trim().to_string();
        }
    } else {
        let as_pos = rest.to_uppercase().find(" AS ");
        if let Some(pos) = as_pos {
            let after_as = &rest[pos + 4..];
            query = after_as.trim().to_string();
        }
    }

    if query.to_uppercase().contains("REFRESH") {
        let refresh_pos = query.to_uppercase().find("REFRESH").unwrap();
        let before_refresh = query[..refresh_pos].trim();
        if !before_refresh.is_empty() && before_refresh != "AS" {
            query = before_refresh.to_string();
        }
        let refresh_start = query[refresh_pos..].find("COMPLETE")
            .or_else(|| query[refresh_pos..].find("FAST"))
            .map(|p| p + refresh_pos);

        if let Some(pos) = refresh_start {
            let refresh_str = &query[pos..];
            if refresh_str.to_uppercase().starts_with("COMPLETE") {
                refresh = Some(RefreshClause {
                    r#type: RefreshType::Complete,
                    concurrency: None,
                });
            } else if refresh_str.to_uppercase().starts_with("FAST") {
                refresh = Some(RefreshClause {
                    r#type: RefreshType::Fast,
                    concurrency: None,
                });
            }
        }
    }

    Ok(vec![Statement::CreateMaterializedView(CreateMaterializedViewStmt {
        database,
        name: view_name,
        if_not_exists,
        query: query.replace("AS ", "").replace("as ", "").trim().to_string(),
        columns,
        refresh,
    })])
}

fn parse_drop_materialized_view(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let sql = sql.trim();
    let after_drop = sql.strip_prefix("DROP MATERIALIZED VIEW")
        .ok_or_else(|| ParseError::SyntaxError {
            position: 0,
            message: "Expected DROP MATERIALIZED VIEW".to_string(),
        })?
        .trim();

    let if_exists = after_drop.starts_with("IF EXISTS");
    let rest = if if_exists {
        after_drop.strip_prefix("IF EXISTS").unwrap().trim()
    } else {
        after_drop
    };

    let (name, _) = extract_identifier(rest)
        .ok_or_else(|| ParseError::SyntaxError {
            position: 0,
            message: "Expected materialized view name".to_string(),
        })?;

    let name_str = name.to_string();
    let parts: Vec<&str> = name_str.split('.').collect();
    let (database, view_name) = if parts.len() == 2 {
        (Some(parts[0].to_string()), parts[1].to_string())
    } else {
        (None, name_str)
    };

    Ok(vec![Statement::DropMaterializedView(DropMaterializedViewStmt {
        database,
        name: view_name,
        if_exists,
    })])
}

fn parse_alter_materialized_view(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let sql = sql.trim();
    let after_alter = sql.strip_prefix("ALTER MATERIALIZED VIEW")
        .ok_or_else(|| ParseError::SyntaxError {
            position: 0,
            message: "Expected ALTER MATERIALIZED VIEW".to_string(),
        })?
        .trim();

    let (name, rest) = extract_identifier(after_alter)
        .ok_or_else(|| ParseError::SyntaxError {
            position: 0,
            message: "Expected materialized view name".to_string(),
        })?;

    let name_str = name.to_string();
    let parts: Vec<&str> = name_str.split('.').collect();
    let (database, view_name) = if parts.len() == 2 {
        (Some(parts[0].to_string()), parts[1].to_string())
    } else {
        (None, name_str)
    };

    let rest = rest.trim();
    let operation = if rest.to_uppercase().starts_with("PAUSE REFRESH") {
        AlterMaterializedViewOperation::PauseRefresh
    } else if rest.to_uppercase().starts_with("RESUME REFRESH") {
        AlterMaterializedViewOperation::ResumeRefresh
    } else if rest.to_uppercase().starts_with("RENAME TO ") {
        let new_name = rest.strip_prefix("RENAME TO ").unwrap().trim();
        AlterMaterializedViewOperation::Rename(new_name.to_string())
    } else {
        return Err(ParseError::SyntaxError {
            position: 0,
            message: format!("Unknown ALTER MATERIALIZED VIEW operation: {}", rest),
        });
    };

    Ok(vec![Statement::AlterMaterializedView(AlterMaterializedViewStmt {
        database,
        name: view_name,
        operation,
    })])
}

fn parse_refresh_materialized_view(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let sql = sql.trim();
    let after_refresh = sql.strip_prefix("REFRESH MATERIALIZED VIEW")
        .ok_or_else(|| ParseError::SyntaxError {
            position: 0,
            message: "Expected REFRESH MATERIALIZED VIEW".to_string(),
        })?
        .trim();

    let (name, rest) = extract_identifier(after_refresh)
        .ok_or_else(|| ParseError::SyntaxError {
            position: 0,
            message: "Expected materialized view name".to_string(),
        })?;

    let name_str = name.to_string();
    let parts: Vec<&str> = name_str.split('.').collect();
    let (database, view_name) = if parts.len() == 2 {
        (Some(parts[0].to_string()), parts[1].to_string())
    } else {
        (None, name_str)
    };

    let refresh_type = if rest.trim().to_uppercase().starts_with("COMPLETE") {
        RefreshType::Complete
    } else {
        RefreshType::Fast
    };

    Ok(vec![Statement::RefreshMaterializedView(RefreshMaterializedViewStmt {
        database,
        name: view_name,
        refresh_type,
    })])
}

fn extract_identifier(s: &str) -> Option<(&str, &str)> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    if s.starts_with('"') || s.starts_with('\'') {
        let quote = s.chars().next().unwrap();
        let rest = &s[1..];
        let end = rest.find(quote).unwrap_or(0);
        let identifier = &rest[..end];
        let remaining = &rest[end + 1..];
        Some((identifier, remaining))
    } else {
        let end = s.find(|c: char| !c.is_alphanumeric() && c != '_')
            .unwrap_or(s.len());
        if end == 0 {
            return None;
        }
        let identifier = &s[..end];
        let remaining = &s[end..];
        Some((identifier, remaining))
    }
}

fn parse_properties(s: &str) -> Vec<(String, String)> {
    let s = s.trim();
    if !s.starts_with("PROPERTIES") {
        return vec![];
    }

    let props_str = s.strip_prefix("PROPERTIES").unwrap().trim();
    if !props_str.starts_with('(') || !props_str.ends_with(')') {
        return vec![];
    }

    let content = &props_str[1..props_str.len() - 1];
    let mut props = vec![];

    for pair in content.split(',') {
        let pair = pair.trim();
        if let Some((key, value)) = pair.split_once('=') {
            let key = key.trim().trim_matches('"').trim_matches('\'');
            let value = value.trim().trim_matches('"').trim_matches('\'');
            props.push((key.to_string(), value.to_string()));
        }
    }

    props
}

fn chrono_lite_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", duration.as_secs())
}

fn convert_statement(
    stmt: sqlparser::ast::Statement,
) -> Result<Statement, ParseError> {
    match stmt {
        sqlparser::ast::Statement::Query(query) => {
            let query_stmt = convert_query(*query)?;
            Ok(Statement::Query(query_stmt))
        }
        sqlparser::ast::Statement::Insert(stmt) => {
            let table_name = stmt.table_name.to_string();
            let cols: Vec<String> = stmt.columns.iter().map(|c| c.value.clone()).collect();
            // Handle VALUES via source query
            let query_opt: Option<QueryStmt> = stmt.source.as_ref().and_then(|q| {
                if let sqlparser::ast::SetExpr::Values(_) = &*q.body {
                    None
                } else {
                    convert_query(*q.clone()).ok()
                }
            });
            let values_list: Vec<Vec<Expr>> = stmt.source.as_ref().and_then(|q| {
                if let sqlparser::ast::SetExpr::Values(values) = &*q.body {
                    Some(values.rows.iter().map(|row| {
                        row.iter().map(|e| convert_expr(e.clone())).collect()
                    }).collect())
                } else {
                    None
                }
            }).unwrap_or_default();
            Ok(Statement::Insert(InsertStmt {
                table: table_name,
                columns: cols,
                values: values_list,
                query: query_opt,
                is_overwrite: stmt.overwrite,
            }))
        }
        sqlparser::ast::Statement::CreateTable(stmt) => {
            let name_str = stmt.name.to_string();
            let parts: Vec<&str> = name_str.split('.').collect();
            let (database, table_name) = if parts.len() == 2 {
                (Some(parts[0].to_string()), parts[1].to_string())
            } else {
                (None, parts.first().map(|s| s.to_string()).unwrap_or_default())
            };
            let col_defs: Vec<ColumnDef> = stmt.columns.iter().map(|c| ColumnDef {
                name: c.name.value.clone(),
                data_type: c.data_type.to_string(),
                nullable: true,
                default_value: None,
                agg_type: None,
                comment: None,
            }).collect();
            Ok(Statement::CreateTable(CreateTableStmt {
                database,
                name: table_name,
                if_not_exists: stmt.if_not_exists,
                columns: col_defs,
                keys_type: KeysType::Duplicate,
                partition: None,
                distribution: None,
                properties: vec![],
            }))
        }
        sqlparser::ast::Statement::Drop {
            names, if_exists, ..
        } => {
            let name = names.first().map(|n| n.to_string()).unwrap_or_default();
            if name.contains('.') {
                let parts: Vec<&str> = name.splitn(2, '.').collect();
                Ok(Statement::DropTable(DropTableStmt {
                    database: Some(parts[0].to_string()),
                    name: parts[1].to_string(),
                    if_exists,
                }))
            } else {
                Ok(Statement::DropTable(DropTableStmt {
                    database: None,
                    name,
                    if_exists,
                }))
            }
        }
        sqlparser::ast::Statement::ExplainTable {
            table_name, ..
        } => {
            let name_str = table_name.to_string();
            let parts: Vec<&str> = name_str.split('.').collect();
            let (db, tbl) = if parts.len() == 2 {
                (parts[0].to_string(), parts[1].to_string())
            } else {
                (String::new(), parts[0].to_string())
            };
            Ok(Statement::Describe(db, tbl))
        }
        sqlparser::ast::Statement::ShowCreate {
            obj_name, ..
        } => {
            let name_str = obj_name.to_string();
            let parts: Vec<&str> = name_str.split('.').collect();
            let (db, tbl) = if parts.len() == 2 {
                (parts[0].to_string(), parts[1].to_string())
            } else {
                (String::new(), parts[0].to_string())
            };
            Ok(Statement::ShowCreateTable(db, tbl))
        }
        sqlparser::ast::Statement::ShowColumns {
            show_options: _, ..
        } => {
            // SHOW COLUMNS FROM table - table name is in the filter
            Ok(Statement::ShowColumns(None, None))
        }
        sqlparser::ast::Statement::Use(db) => {
            Ok(Statement::UseDatabase(db.to_string()))
        }
        sqlparser::ast::Statement::SetVariable {
            variables, value, ..
        } => {
            let var_name = match variables {
                sqlparser::ast::OneOrManyWithParens::One(o) => o.to_string(),
                sqlparser::ast::OneOrManyWithParens::Many(v) => v.first().map(|s: &sqlparser::ast::ObjectName| s.to_string()).unwrap_or_default(),
            };
            let value_expr = value.first().map(|e: &sqlparser::ast::Expr| convert_expr(e.clone())).unwrap_or(Expr::Literal(LiteralValue::Null));
            Ok(Statement::SetVariable(SetVariableStmt {
                variable: var_name,
                value: value_expr,
                is_global: false,
            }))
        }
        sqlparser::ast::Statement::Explain {
            statement, verbose, ..
        } => {
            let inner = convert_statement(*statement)?;
            Ok(Statement::Explain(ExplainStmt {
                statement: Box::new(inner),
                verbose,
            }))
        }
        sqlparser::ast::Statement::Truncate { table_names, .. } => {
            let first_table = table_names.first();
            if let Some(table) = first_table {
                let name_str = table.name.to_string();
                let parts: Vec<&str> = name_str.split('.').collect();
                let (database, table) = if parts.len() == 2 {
                    (Some(parts[0].to_string()), parts[1].to_string())
                } else {
                    (None, parts.first().map(|s| s.to_string()).unwrap_or_default())
                };
                Ok(Statement::TruncateTable {
                    database,
                    table,
                    if_exists: false,
                })
            } else {
                Err(ParseError::SyntaxError {
                    position: 0,
                    message: "TRUNCATE requires at least one table".to_string(),
                })
            }
        }
        sqlparser::ast::Statement::CreateView {
            or_replace: _,
            materialized: _,
            name,
            columns,
            query,
            if_not_exists,
            ..
        } => {
            let name_str = name.to_string();
            let parts: Vec<&str> = name_str.split('.').collect();
            let (database, view_name) = if parts.len() == 2 {
                (Some(parts[0].to_string()), parts[1].to_string())
            } else {
                (None, parts.first().map(|s| s.to_string()).unwrap_or_default())
            };
            let col_names: Vec<String> = columns.iter().map(|c: &sqlparser::ast::ViewColumnDef| c.name.value.clone()).collect();
            Ok(Statement::CreateView {
                database,
                name: view_name,
                if_not_exists,
                query: query.to_string(),
                columns: col_names,
            })
        }
        sqlparser::ast::Statement::Update { table, assignments, from: _, selection, returning: _, or: _ } => {
            let table_name = table.to_string();
            let set_clauses: Vec<SetClause> = assignments.iter().map(|s| {
                let column = match &s.target {
                    sqlparser::ast::AssignmentTarget::ColumnName(name) => name.to_string(),
                    sqlparser::ast::AssignmentTarget::Tuple(_) => String::new(),
                };
                let value = convert_expr(s.value.clone());
                SetClause { column, value }
            }).collect();
            let selection = selection.map(convert_expr);
            Ok(Statement::Update(UpdateStmt {
                table: table_name,
                set_clauses,
                selection,
            }))
        }
        sqlparser::ast::Statement::Delete(delete) => {
            let table_name = delete.tables.first().map(|t: &sqlparser::ast::ObjectName| t.to_string()).unwrap_or_default();
            let selection = delete.selection.map(convert_expr);
            Ok(Statement::Delete(DeleteStmt {
                table: table_name,
                selection,
            }))
        }
        sqlparser::ast::Statement::AlterTable {
            name,
            if_exists: _,
            only: _,
            operations,
            ..
        } => {
            let name_str = name.to_string();
            let parts: Vec<&str> = name_str.split('.').collect();
            let (database, table_name) = if parts.len() == 2 {
                (Some(parts[0].to_string()), parts[1].to_string())
            } else {
                (None, parts.first().map(|s| s.to_string()).unwrap_or_default())
            };
            let alter_ops: Vec<AlterOperation> = operations.into_iter().filter_map(|op| {
                match op {
                    sqlparser::ast::AlterTableOperation::AddColumn { column_def, .. } => {
                        Some(AlterOperation::AddColumn(ColumnDef {
                            name: column_def.name.value.clone(),
                            data_type: column_def.data_type.to_string(),
                            nullable: true,
                            default_value: None,
                            agg_type: None,
                            comment: None,
                        }))
                    }
                    sqlparser::ast::AlterTableOperation::DropColumn { column_name, .. } => {
                        Some(AlterOperation::DropColumn(column_name.value.clone()))
                    }
                    sqlparser::ast::AlterTableOperation::RenameTable { table_name } => {
                        Some(AlterOperation::RenameTable(table_name.to_string()))
                    }
                    sqlparser::ast::AlterTableOperation::ModifyColumn { col_name, data_type, .. } => {
                        Some(AlterOperation::ModifyColumn(ColumnDef {
                            name: col_name.value.clone(),
                            data_type: data_type.to_string(),
                            nullable: true,
                            default_value: None,
                            agg_type: None,
                            comment: None,
                        }))
                    }
                    _ => None,
                }
            }).collect();
            Ok(Statement::AlterTable(AlterTableStmt {
                database,
                table: table_name,
                operations: alter_ops,
            }))
        }
        _ => Err(ParseError::Unsupported(format!(
            "statement type: {:?}",
            stmt
        ))),
    }
}

fn convert_query(
    query: sqlparser::ast::Query,
) -> Result<QueryStmt, ParseError> {
    let cte = query.with.as_ref().and_then(|w| {
        w.cte_tables.first().map(|c| Cte {
            name: c.alias.name.value.clone(),
            columns: vec![],
            query: Box::new(convert_query(*c.query.clone()).unwrap_or_else(|_| QueryStmt {
                select_list: vec![],
                from: None,
                r#where: None,
                group_by: vec![],
                having: None,
                order_by: vec![],
                limit: None,
                offset: None,
                with: None,
            })),
        })
    });

    match *query.body {
        sqlparser::ast::SetExpr::Select(select) => {
            let order_by = query.order_by.map(|ob| ob.exprs).unwrap_or_default();
            let limit = query.limit;
            let offset = query.offset;
            convert_select(*select, order_by, limit, offset, cte)
        }
        sqlparser::ast::SetExpr::SetOperation { op, set_quantifier, left, right } => {
            let left_query = convert_set_expr(*left)?;
            let right_query = convert_set_expr(*right)?;
            let union_op = match op {
                sqlparser::ast::SetOperator::Union => UnionOperator::Union,
                sqlparser::ast::SetOperator::Except => UnionOperator::Except,
                sqlparser::ast::SetOperator::Intersect => UnionOperator::Intersect,
            };
            let _ = (union_op, set_quantifier);
            let _ = right_query;
            let order_by = query.order_by.map(|ob| ob.exprs).unwrap_or_default();
            Ok(QueryStmt {
                select_list: left_query.select_list,
                from: left_query.from,
                r#where: left_query.r#where,
                group_by: left_query.group_by,
                having: left_query.having,
                order_by: order_by.into_iter().map(|o| OrderByItem {
                    expr: convert_expr(o.expr),
                    ascending: o.asc.unwrap_or(true),
                    nulls_first: o.nulls_first.unwrap_or(true),
                }).collect(),
                limit: query.limit.and_then(|l| match l {
                    sqlparser::ast::Expr::Value(sqlparser::ast::Value::Number(n, _)) => n.parse().ok(),
                    _ => None,
                }),
                offset: query.offset.and_then(|o| match o.value {
                    sqlparser::ast::Expr::Value(sqlparser::ast::Value::Number(n, _)) => n.parse().ok(),
                    _ => None,
                }),
                with: cte,
            })
        }
        _ => Err(ParseError::Unsupported("non-SELECT query body".to_string())),
    }
}

fn convert_set_expr(expr: sqlparser::ast::SetExpr) -> Result<QueryStmt, ParseError> {
    match expr {
        sqlparser::ast::SetExpr::Select(select) => convert_select(*select, vec![], None, None, None),
        sqlparser::ast::SetExpr::Query(query) => convert_query(*query),
        _ => Err(ParseError::Unsupported("set operation not supported".to_string())),
    }
}

fn convert_select(
    select: sqlparser::ast::Select,
    order_by: Vec<sqlparser::ast::OrderByExpr>,
    limit: Option<sqlparser::ast::Expr>,
    offset: Option<sqlparser::ast::Offset>,
    cte: Option<Cte>,
) -> Result<QueryStmt, ParseError> {
    let select_list = select.projection.into_iter().map(|item| {
        match item {
            sqlparser::ast::SelectItem::UnnamedExpr(expr) => SelectItem {
                expr: convert_expr(expr),
                alias: None,
            },
            sqlparser::ast::SelectItem::ExprWithAlias { expr, alias } => SelectItem {
                expr: convert_expr(expr),
                alias: Some(alias.value),
            },
            sqlparser::ast::SelectItem::Wildcard(_) => SelectItem {
                expr: Expr::Wildcard,
                alias: None,
            },
            _ => SelectItem {
                expr: Expr::Wildcard,
                alias: None,
            },
        }
    }).collect();

    let from = select.from.into_iter().next().map(convert_table_ref);

    let group_by = match select.group_by {
        sqlparser::ast::GroupByExpr::Expressions(exprs, _) => {
            exprs.into_iter().map(convert_expr).collect()
        }
        _ => vec![],
    };

    let order_by: Vec<OrderByItem> = order_by.into_iter().map(|o| OrderByItem {
        expr: convert_expr(o.expr),
        ascending: o.asc.unwrap_or(true),
        nulls_first: o.nulls_first.unwrap_or(true),
    }).collect();

    let limit = limit.and_then(|l| match l {
        sqlparser::ast::Expr::Value(sqlparser::ast::Value::Number(n, _)) => n.parse().ok(),
        _ => None,
    });

    let offset = offset.and_then(|o| match o.value {
        sqlparser::ast::Expr::Value(sqlparser::ast::Value::Number(n, _)) => n.parse().ok(),
        _ => None,
    });

    Ok(QueryStmt {
        select_list,
        from,
        r#where: select.selection.map(convert_expr),
        group_by,
        having: select.having.map(convert_expr),
        order_by,
        limit,
        offset,
        with: cte,
    })
}

fn extract_join_condition(op: &sqlparser::ast::JoinOperator) -> Option<sqlparser::ast::Expr> {
    use sqlparser::ast::JoinOperator;
    match op {
        JoinOperator::Inner(constraint) => extract_constraint_expr(constraint),
        JoinOperator::LeftOuter(constraint) => extract_constraint_expr(constraint),
        JoinOperator::RightOuter(constraint) => extract_constraint_expr(constraint),
        JoinOperator::FullOuter(constraint) => extract_constraint_expr(constraint),
        _ => None,
    }
}

fn extract_constraint_expr(constraint: &sqlparser::ast::JoinConstraint) -> Option<sqlparser::ast::Expr> {
    match constraint {
        sqlparser::ast::JoinConstraint::On(expr) => Some(expr.clone()),
        _ => None,
    }
}

fn convert_table_ref(t: sqlparser::ast::TableWithJoins) -> TableRef {
    let name = match &t.relation {
        sqlparser::ast::TableFactor::Table { name, alias, .. } => {
            let table_name = name.to_string();
            TableRef::Table {
                name: table_name,
                alias: alias.as_ref().map(|a| a.name.value.clone()),
            }
        }
        sqlparser::ast::TableFactor::Derived { subquery, alias, .. } => {
            let query = convert_query(*subquery.clone()).ok().unwrap();
            return TableRef::Subquery {
                query: Box::new(query),
                alias: alias.as_ref().map(|a| a.name.value.clone()).unwrap_or_default(),
            };
        }
        _ => TableRef::Table { name: "unknown".into(), alias: None },
    };

    t.joins.into_iter().fold(name, |left, join| {
        let right = convert_table_ref_simple(join.relation);
        let condition = extract_join_condition(&join.join_operator);
        TableRef::Join {
            left: Box::new(left),
            right: Box::new(right),
            r#type: match join.join_operator {
                sqlparser::ast::JoinOperator::Inner(_) => JoinType::Inner,
                sqlparser::ast::JoinOperator::LeftOuter(_) => JoinType::LeftOuter,
                sqlparser::ast::JoinOperator::RightOuter(_) => JoinType::RightOuter,
                sqlparser::ast::JoinOperator::FullOuter(_) => JoinType::FullOuter,
                sqlparser::ast::JoinOperator::CrossJoin => JoinType::Cross,
                _ => JoinType::Inner,
            },
            condition: condition.map(convert_expr),
        }
    })
}

fn convert_table_ref_simple(factor: sqlparser::ast::TableFactor) -> TableRef {
    match factor {
        sqlparser::ast::TableFactor::Table { name, alias, .. } => TableRef::Table {
            name: name.to_string(),
            alias: alias.map(|a| a.name.value),
        },
        _ => TableRef::Table { name: "unknown".into(), alias: None },
    }
}

fn convert_function_args(args: sqlparser::ast::FunctionArguments) -> Vec<Expr> {
    match args {
        sqlparser::ast::FunctionArguments::None => vec![],
        sqlparser::ast::FunctionArguments::Subquery(_) => vec![],
        sqlparser::ast::FunctionArguments::List(list) => {
            list.args.into_iter().map(|arg| {
                match arg {
                    sqlparser::ast::FunctionArg::Unnamed(arg_expr) |
                    sqlparser::ast::FunctionArg::Named { arg: arg_expr, .. } => {
                        match arg_expr {
                            sqlparser::ast::FunctionArgExpr::Expr(expr) => convert_expr(expr),
                            _ => Expr::Wildcard,
                        }
                    }
                    _ => Expr::Wildcard,
                }
            }).collect()
        }
    }
}

fn convert_expr(expr: sqlparser::ast::Expr) -> Expr {
    match expr {
        sqlparser::ast::Expr::Value(v) => Expr::Literal(match v {
            sqlparser::ast::Value::Number(n, _) => {
                if n.contains('.') || n.contains('e') || n.contains('E') {
                    LiteralValue::Float64(n.parse().unwrap_or(0.0))
                } else {
                    LiteralValue::Int64(n.parse().unwrap_or(0))
                }
            }
            sqlparser::ast::Value::SingleQuotedString(s) => LiteralValue::String(s),
            sqlparser::ast::Value::DoubleQuotedString(s) => LiteralValue::String(s),
            sqlparser::ast::Value::Boolean(b) => LiteralValue::Boolean(b),
            sqlparser::ast::Value::Null => LiteralValue::Null,
            _ => LiteralValue::Null,
        }),
        sqlparser::ast::Expr::Identifier(id) => Expr::ColumnRef {
            table: None,
            column: id.value,
        },
        sqlparser::ast::Expr::CompoundIdentifier(ids) => {
            let len = ids.len();
            if len == 2 {
                Expr::ColumnRef {
                    table: Some(ids[0].value.clone()),
                    column: ids[1].value.clone(),
                }
            } else {
                Expr::ColumnRef { table: None, column: ids.last().map(|i| i.value.clone()).unwrap_or_default() }
            }
        }
        sqlparser::ast::Expr::BinaryOp { left, op, right } => Expr::BinaryOp {
            left: Box::new(convert_expr(*left)),
            op: convert_binary_op(op),
            right: Box::new(convert_expr(*right)),
        },
        sqlparser::ast::Expr::UnaryOp { op, expr } => Expr::UnaryOp {
            op: match op {
                sqlparser::ast::UnaryOperator::Not => UnaryOp::Not,
                sqlparser::ast::UnaryOperator::Minus => UnaryOp::Negate,
                _ => UnaryOp::Not,
            },
            expr: Box::new(convert_expr(*expr)),
        },
        sqlparser::ast::Expr::Function(fun) => {
            let name = fun.name.to_string();
            let args = convert_function_args(fun.args);
            Expr::FunctionCall { name, args, distinct: false }
        }
        sqlparser::ast::Expr::InList { expr, list, negated } => Expr::InList {
            expr: Box::new(convert_expr(*expr)),
            list: list.into_iter().map(convert_expr).collect(),
            negated,
        },
        sqlparser::ast::Expr::InSubquery { expr, subquery, negated } => Expr::InSubquery {
            expr: Box::new(convert_expr(*expr)),
            query: Box::new(convert_query(*subquery).unwrap_or_else(|_| QueryStmt {
                select_list: vec![],
                from: None,
                r#where: None,
                group_by: vec![],
                having: None,
                order_by: vec![],
                limit: None,
                offset: None,
                with: None,
            })),
            negated,
        },
        sqlparser::ast::Expr::Exists { subquery, negated } => {
            let query = convert_query(*subquery).unwrap_or_else(|_| QueryStmt {
                select_list: vec![],
                from: None,
                r#where: None,
                group_by: vec![],
                having: None,
                order_by: vec![],
                limit: None,
                offset: None,
                with: None,
            });
            if negated {
                Expr::UnaryOp {
                    op: UnaryOp::Not,
                    expr: Box::new(Expr::Exists(Box::new(query))),
                }
            } else {
                Expr::Exists(Box::new(query))
            }
        }
        sqlparser::ast::Expr::Subquery(subquery) => Expr::Subquery(
            Box::new(convert_query(*subquery).unwrap_or_else(|_| QueryStmt {
                select_list: vec![],
                from: None,
                r#where: None,
                group_by: vec![],
                having: None,
                order_by: vec![],
                limit: None,
                offset: None,
                with: None,
            }))
        ),
        sqlparser::ast::Expr::Between { expr, negated, low, high } => Expr::Between {
            expr: Box::new(convert_expr(*expr)),
            low: Box::new(convert_expr(*low)),
            high: Box::new(convert_expr(*high)),
            negated,
        },
        sqlparser::ast::Expr::IsNull { expr, negated } => Expr::IsNull {
            expr: Box::new(convert_expr(*expr)),
            negated,
        },
        _ => Expr::Wildcard,
    }
}

fn convert_binary_op(op: sqlparser::ast::BinaryOperator) -> BinaryOp {
    use sqlparser::ast::BinaryOperator;
    match op {
        BinaryOperator::Eq => BinaryOp::Eq,
        BinaryOperator::NotEq => BinaryOp::NotEq,
        BinaryOperator::Lt => BinaryOp::Lt,
        BinaryOperator::LtEq => BinaryOp::LtEq,
        BinaryOperator::Gt => BinaryOp::Gt,
        BinaryOperator::GtEq => BinaryOp::GtEq,
        BinaryOperator::And => BinaryOp::And,
        BinaryOperator::Or => BinaryOp::Or,
        BinaryOperator::Plus => BinaryOp::Plus,
        BinaryOperator::Minus => BinaryOp::Minus,
        BinaryOperator::Multiply => BinaryOp::Multiply,
        BinaryOperator::Divide => BinaryOp::Divide,
        BinaryOperator::Modulo => BinaryOp::Modulo,
        _ => BinaryOp::Eq,
    }
}

fn parse_create_user(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let sql = sql.trim();

    let if_not_exists = sql.to_uppercase().contains("IF NOT EXISTS");

    let after_create = sql
        .strip_prefix("CREATE USER")
        .or_else(|| sql.strip_prefix("create user"))
        .ok_or_else(|| ParseError::SyntaxError {
            position: 0,
            message: "Expected CREATE USER".to_string(),
        })?
        .trim();

    let (username, rest) = extract_username_hostname(after_create)?;

    let mut auth_plugin = "mysql_native_password".to_string();
    let mut password = None;
    let mut identified_by_password = false;

    let rest_upper = rest.to_uppercase();
    if rest_upper.contains("IDENTIFIED BY") {
        identified_by_password = true;
        if let Some(pwd_part) = rest_upper.split("IDENTIFIED BY").nth(1) {
            let pwd = pwd_part.trim().trim_end_matches('\'').trim_end_matches('"');
            password = Some(pwd.to_string());
        }
        if rest_upper.contains("WITH") {
            if let Some(with_part) = rest_upper.split("WITH").nth(1) {
                let plugin = with_part.trim().split_whitespace().next().unwrap_or("mysql_native_password");
                auth_plugin = plugin.to_string();
            }
        }
    } else if rest_upper.contains("IDENTIFIED WITH") {
        if let Some(with_part) = rest_upper.split("IDENTIFIED WITH").nth(1) {
            let plugin = with_part.trim().split_whitespace().next().unwrap_or("mysql_native_password");
            auth_plugin = plugin.to_string();
        }
    }

    Ok(vec![Statement::CreateUser(CreateUserStmt {
        username,
        hostname: None,
        auth_plugin,
        password,
        identified_by_password,
        roles: vec![],
    })])
}

fn parse_drop_user(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let sql = sql.trim();

    let if_exists = sql.to_uppercase().contains("IF EXISTS");

    let after_drop = sql
        .strip_prefix("DROP USER")
        .or_else(|| sql.strip_prefix("drop user"))
        .ok_or_else(|| ParseError::SyntaxError {
            position: 0,
            message: "Expected DROP USER".to_string(),
        })?
        .trim();

    let (username, _hostname) = extract_username_hostname(after_drop)?;

    Ok(vec![Statement::DropUser(DropUserStmt {
        username,
        hostname: None,
        if_exists,
    })])
}

fn extract_username_hostname(s: &str) -> Result<(String, String), ParseError> {
    let s = s.trim();

    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.is_empty() {
        return Err(ParseError::SyntaxError {
            position: 0,
            message: "Expected username".to_string(),
        });
    }

    let username_hostname = parts[0];
    let remaining = if parts.len() > 1 { parts[1].to_string() } else { String::new() };

    if username_hostname.contains('@') {
        let uparts: Vec<&str> = username_hostname.split('@').collect();
        let username = uparts[0].to_string();
        let hostname = if uparts.len() > 1 { Some(uparts[1].to_string()) } else { None };
        let hostname_str = hostname.unwrap_or_else(|| "%".to_string());
        Ok((username, hostname_str))
    } else {
        Ok((username_hostname.to_string(), remaining))
    }
}

fn parse_create_catalog(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let sql = sql.trim();
    let after_create = sql
        .strip_prefix("CREATE CATALOG")
        .ok_or_else(|| ParseError::SyntaxError {
            position: 0,
            message: "Expected CREATE CATALOG".to_string(),
        })?
        .trim();

    let (name, rest) = extract_identifier(after_create)
        .ok_or_else(|| ParseError::SyntaxError {
            position: 0,
            message: "Expected catalog name".to_string(),
        })?;

    let rest = rest.trim();
    let mut catalog_type = "iceberg".to_string();
    let mut properties = vec![];

    if rest.starts_with("PROPERTIES") || rest.starts_with("WITH") {
        let after_with = if rest.starts_with("PROPERTIES") {
            rest.strip_prefix("PROPERTIES").unwrap().trim()
        } else {
            rest.strip_prefix("WITH").unwrap().trim()
        };

        if after_with.starts_with("TYPE") {
            let type_part = after_with
                .strip_prefix("TYPE")
                .unwrap_or("")
                .trim()
                .trim_start_matches('=')
                .trim();
            let first_word = type_part.split_whitespace().next()
                .unwrap_or(type_part)
                .trim_matches(|c| c == ',' || c == ';');
            catalog_type = first_word.to_lowercase();
        }

        if rest.contains('(') && rest.contains('=') {
            properties = parse_properties(rest);
        }
    }

    Ok(vec![Statement::CreateCatalog(CreateCatalogStmt {
        name: name.to_string(),
        catalog_type,
        properties,
    })])
}

fn parse_drop_catalog(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let sql = sql.trim();
    let if_exists = sql.to_uppercase().contains("IF EXISTS");

    let after_drop = sql
        .strip_prefix("DROP CATALOG")
        .or_else(|| sql.strip_prefix("drop catalog"))
        .ok_or_else(|| ParseError::SyntaxError {
            position: 0,
            message: "Expected DROP CATALOG".to_string(),
        })?
        .trim();

    let (name, _) = extract_identifier(after_drop)
        .ok_or_else(|| ParseError::SyntaxError {
            position: 0,
            message: "Expected catalog name".to_string(),
        })?;

    Ok(vec![Statement::DropCatalog(DropCatalogStmt {
        name: name.to_string(),
        if_exists,
    })])
}

fn parse_refresh_catalog(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let sql = sql.trim();
    let after_refresh = sql
        .strip_prefix("REFRESH CATALOG")
        .ok_or_else(|| ParseError::SyntaxError {
            position: 0,
            message: "Expected REFRESH CATALOG".to_string(),
        })?
        .trim();

    let (name, _) = extract_identifier(after_refresh)
        .ok_or_else(|| ParseError::SyntaxError {
            position: 0,
            message: "Expected catalog name".to_string(),
        })?;

    Ok(vec![Statement::RefreshCatalog(RefreshCatalogStmt {
        name: name.to_string(),
    })])
}
