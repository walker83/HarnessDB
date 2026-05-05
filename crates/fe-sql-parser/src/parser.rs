use crate::ast::*;
use crate::error::ParseError;

pub fn parse_sql(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let trimmed = sql.trim().to_uppercase();

    // Pre-process Doris-specific CREATE TABLE extensions before sqlparser
    let sql_to_parse;
    let mut doris_distribution: Option<DistributionDef> = None;
    let mut doris_partition: Option<PartitionDef> = None;
    let mut doris_properties: Vec<(String, String)> = vec![];

    if trimmed.starts_with("CREATE TABLE") {
        let (clean, dist, part, props) = preprocess_create_table(sql);
        sql_to_parse = clean;
        doris_distribution = dist;
        doris_partition = part;
        doris_properties = props;
    } else {
        sql_to_parse = sql.to_string();
    }

    if trimmed.starts_with("CREATE REPOSITORY") {
        return parse_create_repository(sql);
    }
    if trimmed.starts_with("DROP REPOSITORY") {
        return parse_drop_repository(sql);
    }
    if trimmed.starts_with("SHOW REPOSITORIES") {
        return Ok(vec![Statement::ShowRepositories]);
    }
    if trimmed.starts_with("SHOW CREATE DATABASE") {
        return parse_show_create_database(sql);
    }
    if trimmed.starts_with("SHOW CREATE VIEW") {
        return parse_show_create_view(sql);
    }
    if trimmed.starts_with("SHOW PARTITIONS") {
        return parse_show_partitions(sql);
    }
    if trimmed.starts_with("SHOW TABLE STATUS") {
        return parse_show_table_status(sql);
    }
    if trimmed.starts_with("SHOW VARIABLES") {
        return parse_show_variables(sql);
    }
    if trimmed.starts_with("SHOW PROCESSLIST") || trimmed == "SHOW PROCESSLIST" {
        return parse_show_processlist(sql);
    }
    if trimmed.starts_with("SHOW INDEX") || trimmed.starts_with("SHOW KEYS") {
        return parse_show_index(sql);
    }
    if trimmed.starts_with("SHOW ALTER TABLE") {
        return parse_show_alter_table(sql);
    }
    if trimmed.starts_with("SHOW BACKENDS") || trimmed == "SHOW BACKENDS" {
        return Ok(vec![Statement::ShowBackends]);
    }
    if trimmed.starts_with("SHOW FRONTENDS") || trimmed == "SHOW FRONTENDS" {
        return Ok(vec![Statement::ShowFrontends]);
    }
    if trimmed.starts_with("SHOW DYNAMIC PARTITION") || trimmed.starts_with("SHOW DYNAMIC PARTITION TABLES") {
        return Ok(vec![Statement::ShowDynamicPartitionTables]);
    }
    if trimmed.starts_with("SHOW VIEW") {
        return parse_show_view(sql);
    }
    if trimmed.starts_with("SHOW CREATE MATERIALIZED VIEW") {
        return parse_show_create_materialized_view(sql);
    }
    if trimmed.starts_with("SHOW TABLE ID") || trimmed == "SHOW TABLE ID" {
        return Ok(vec![Statement::ShowTableId]);
    }
    if trimmed.starts_with("SHOW PARTITION ID") || trimmed == "SHOW PARTITION ID" {
        return Ok(vec![Statement::ShowPartitionId]);
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

    // Batch 2 DDL: CREATE INDEX, DROP INDEX, CANCEL ALTER TABLE, ALTER COLOCATE GROUP, ALTER DATABASE, DROP VIEW, ALTER VIEW
    if trimmed.starts_with("CREATE INDEX") {
        return parse_create_index(sql);
    }
    if trimmed.starts_with("DROP INDEX") {
        return parse_drop_index(sql);
    }
    if trimmed.starts_with("CANCEL ALTER TABLE") {
        return parse_cancel_alter_table(sql);
    }
    if trimmed.starts_with("ALTER COLOCATE GROUP") {
        return parse_alter_colocate_group(sql);
    }
    if trimmed.starts_with("ALTER DATABASE") {
        return parse_alter_database(sql);
    }
    if trimmed.starts_with("DROP VIEW") {
        return parse_drop_view(sql);
    }
    if trimmed.starts_with("ALTER VIEW") {
        return parse_alter_view(sql);
    }

    // Doris-specific ALTER TABLE extensions
    if trimmed.starts_with("ALTER TABLE") {
        let upper = trimmed.clone();
        if upper.contains("RENAME COLUMN")
            || upper.contains("SET PROPERTIES")
            || upper.contains("COMMENT")
            || upper.contains("ADD PARTITION")
            || upper.contains("DROP PARTITION")
            || upper.contains("ADD ROLLUP")
            || upper.contains("DROP ROLLUP")
            || upper.contains("REPLACE WITH")
            || upper.contains("ADD GENERATED COLUMN")
        {
            return parse_alter_table_doris(sql);
        }
    }

    // Batch 3/4 statements
    if trimmed.starts_with("EXPORT TABLE") {
        return parse_export_table(sql);
    }
    if trimmed.starts_with("CANCEL EXPORT") {
        return parse_cancel_export(sql);
    }
    if trimmed == "SHOW EXPORT" || trimmed == "SHOW EXPORT;" {
        return Ok(vec![Statement::ShowExport]);
    }
    if trimmed.starts_with("CREATE FUNCTION") {
        return parse_create_function(sql);
    }
    if trimmed.starts_with("DROP FUNCTION") {
        return parse_drop_function(sql);
    }
    if trimmed.starts_with("SHOW CREATE FUNCTION") {
        return parse_show_create_function(sql);
    }
    if trimmed.starts_with("SHOW FUNCTIONS") {
        return parse_show_functions(sql);
    }
    if trimmed.starts_with("DESC FUNCTION") || trimmed.starts_with("DESCRIBE FUNCTION") {
        return parse_desc_function(sql);
    }
    if trimmed.starts_with("ANALYZE TABLE") {
        return parse_analyze_table(sql);
    }
    if trimmed.starts_with("DROP STATS") {
        return parse_drop_stats(sql);
    }
    if trimmed.starts_with("DROP ANALYZE JOB") {
        let after = sql.trim().strip_prefix("DROP ANALYZE JOB").unwrap().trim();
        let (name, _) = extract_identifier(after).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected job ID".to_string() })?;
        return Ok(vec![Statement::DropJob(name.to_string())]);
    }
    if trimmed.starts_with("KILL ANALYZE") {
        return parse_kill_analyze_job(sql);
    }
    if trimmed.starts_with("ALTER STATS") {
        return parse_alter_stats(sql);
    }
    if trimmed.starts_with("SHOW ANALYZE") {
        return parse_show_analyze(sql);
    }
    if trimmed.starts_with("SHOW STATS ") || trimmed == "SHOW STATS" {
        return parse_show_stats(sql);
    }
    if trimmed.starts_with("SHOW TABLE STATS") || trimmed.starts_with("SHOW TABLE STATISTICS") {
        return parse_show_table_stats(sql);
    }
    if trimmed.starts_with("CREATE JOB") {
        return parse_create_job(sql);
    }
    if trimmed.starts_with("DROP JOB") {
        return parse_drop_job(sql);
    }
    if trimmed.starts_with("PAUSE JOB") {
        return parse_pause_job(sql);
    }
    if trimmed.starts_with("RESUME JOB") {
        return parse_resume_job(sql);
    }
    if trimmed.starts_with("CANCEL TASK") {
        return parse_cancel_task(sql);
    }
    if trimmed.starts_with("INSTALL PLUGIN") {
        return parse_install_plugin(sql);
    }
    if trimmed.starts_with("UNINSTALL PLUGIN") {
        return parse_uninstall_plugin(sql);
    }
    if trimmed == "SHOW PLUGINS" {
        return Ok(vec![Statement::ShowPlugins]);
    }
    if trimmed.starts_with("RECOVER DATABASE") {
        return parse_recover_database(sql);
    }
    if trimmed.starts_with("RECOVER TABLE") {
        return parse_recover_table(sql);
    }
    if trimmed.starts_with("RECOVER PARTITION") {
        return parse_recover_partition(sql);
    }
    if trimmed.starts_with("DROP CATALOG RECYCLE") {
        return parse_drop_catalog_recycle_bin(sql);
    }
    if trimmed.starts_with("SHOW CATALOG RECYCLE") {
        return Ok(vec![Statement::ShowCatalogRecycleBin]);
    }
    if trimmed.starts_with("CREATE SQL_BLOCK_RULE") {
        return parse_create_sql_block_rule(sql);
    }
    if trimmed.starts_with("ALTER SQL_BLOCK_RULE") {
        return parse_alter_sql_block_rule(sql);
    }
    if trimmed.starts_with("DROP SQL_BLOCK_RULE") {
        return parse_drop_sql_block_rule(sql);
    }
    if trimmed.starts_with("SHOW SQL_BLOCK_RULE") {
        return parse_show_sql_block_rule(sql);
    }
    if trimmed.starts_with("CREATE ROW POLICY") {
        return parse_create_row_policy(sql);
    }
    if trimmed.starts_with("DROP ROW POLICY") {
        return parse_drop_row_policy(sql);
    }
    if trimmed.starts_with("SHOW ROW POLICY") {
        return parse_show_row_policy(sql);
    }

    let dialect = sqlparser::dialect::MySqlDialect {};
    let statements = sqlparser::parser::Parser::parse_sql(&dialect, &sql_to_parse)
        .map_err(|e| ParseError::SyntaxError {
            position: 0,
            message: e.to_string(),
        })?;

    statements
        .into_iter()
        .map(|s| {
            let mut converted = convert_statement(s)?;
            // Attach Doris extensions to CreateTable
            if let Statement::CreateTable(ref mut ct) = converted {
                if doris_distribution.is_some() {
                    ct.distribution = doris_distribution.take();
                }
                if doris_partition.is_some() {
                    ct.partition = doris_partition.take();
                }
                if !doris_properties.is_empty() {
                    ct.properties = doris_properties.clone();
                }
            }
            Ok(converted)
        })
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

fn parse_show_create_database(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let sql = sql.trim();
    let after_show = sql
        .strip_prefix("SHOW CREATE DATABASE")
        .ok_or_else(|| ParseError::SyntaxError {
            position: 0,
            message: "Expected SHOW CREATE DATABASE".to_string(),
        })?
        .trim();

    let (db_name, _) = extract_identifier(after_show)
        .ok_or_else(|| ParseError::SyntaxError {
            position: 0,
            message: "Expected database name".to_string(),
        })?;

    Ok(vec![Statement::ShowCreateDatabase(db_name.to_string())])
}

fn parse_show_create_view(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let sql = sql.trim();
    let after_show = sql
        .strip_prefix("SHOW CREATE VIEW")
        .ok_or_else(|| ParseError::SyntaxError {
            position: 0,
            message: "Expected SHOW CREATE VIEW".to_string(),
        })?
        .trim();

    let (name, _) = extract_identifier(after_show)
        .ok_or_else(|| ParseError::SyntaxError {
            position: 0,
            message: "Expected view name".to_string(),
        })?;

    let name_str = name.to_string();
    let parts: Vec<&str> = name_str.split('.').collect();
    let (db, view_name) = if parts.len() == 2 {
        (parts[0].to_string(), parts[1].to_string())
    } else {
        (String::new(), name_str)
    };

    Ok(vec![Statement::ShowCreateView(db, view_name)])
}

fn parse_show_partitions(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let sql = sql.trim();
    let after_show = sql
        .strip_prefix("SHOW PARTITIONS")
        .ok_or_else(|| ParseError::SyntaxError {
            position: 0,
            message: "Expected SHOW PARTITIONS".to_string(),
        })?
        .trim();

    let from_pos = after_show.to_uppercase().find(" FROM ");
    let table_name = if let Some(pos) = from_pos {
        let after_from = after_show[pos + 6..].trim();
        let (name, _) = extract_identifier(after_from)
            .ok_or_else(|| ParseError::SyntaxError {
                position: 0,
                message: "Expected table name".to_string(),
            })?;
        name.to_string()
    } else {
        return Err(ParseError::SyntaxError {
            position: 0,
            message: "Expected FROM clause".to_string(),
        });
    };

    let name_str = table_name.to_string();
    let parts: Vec<&str> = name_str.split('.').collect();
    let (db, table) = if parts.len() == 2 {
        (parts[0].to_string(), parts[1].to_string())
    } else {
        (String::new(), name_str)
    };

    Ok(vec![Statement::ShowPartitions(db, table)])
}

fn parse_show_table_status(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let sql = sql.trim();
    let after_show = sql
        .strip_prefix("SHOW TABLE STATUS")
        .ok_or_else(|| ParseError::SyntaxError {
            position: 0,
            message: "Expected SHOW TABLE STATUS".to_string(),
        })?
        .trim();

    let mut db_name = None;
    if after_show.to_uppercase().starts_with("FROM ") {
        let after_from = after_show[5..].trim();
        let (name, _) = extract_identifier(after_from)
            .ok_or_else(|| ParseError::SyntaxError {
                position: 0,
                message: "Expected database name".to_string(),
            })?;
        db_name = Some(name.to_string());
    }

    Ok(vec![Statement::ShowTableStatus(db_name)])
}

fn parse_show_variables(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let sql_upper = sql.trim().to_uppercase();
    let after_show = sql_upper
        .strip_prefix("SHOW VARIABLES")
        .ok_or_else(|| ParseError::SyntaxError {
            position: 0,
            message: "Expected SHOW VARIABLES".to_string(),
        })?
        .trim();

    let mut global = false;
    let mut pattern = None;

    if after_show.starts_with("GLOBAL ") {
        global = true;
        let rest = after_show.strip_prefix("GLOBAL ").unwrap().trim();
        if rest.starts_with("LIKE ") {
            pattern = Some(rest[5..].trim().trim_matches('\'').to_string());
        } else {
            pattern = Some(rest.to_string());
        }
    } else if after_show.starts_with("SESSION ") {
        let rest = after_show.strip_prefix("SESSION ").unwrap().trim();
        if rest.starts_with("LIKE ") {
            pattern = Some(rest[5..].trim().trim_matches('\'').to_string());
        } else {
            pattern = Some(rest.to_string());
        }
    } else if after_show.starts_with("LIKE ") {
        pattern = Some(after_show[5..].trim().trim_matches('\'').to_string());
    } else if !after_show.is_empty() {
        pattern = Some(after_show.to_string());
    }

    Ok(vec![Statement::ShowVariables { global, pattern }])
}

fn parse_show_processlist(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let sql_upper = sql.trim().to_uppercase();
    let after_show = sql_upper
        .strip_prefix("SHOW PROCESSLIST")
        .ok_or_else(|| ParseError::SyntaxError {
            position: 0,
            message: "Expected SHOW PROCESSLIST".to_string(),
        })?
        .trim();

    let full = after_show.contains("FULL");

    Ok(vec![Statement::ShowProcesslist(full)])
}

fn parse_show_index(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let sql_upper = sql.trim().to_uppercase();
    let after_show = sql_upper
        .strip_prefix("SHOW INDEX")
        .ok_or_else(|| ParseError::SyntaxError {
            position: 0,
            message: "Expected SHOW INDEX".to_string(),
        })?
        .trim();

    let from_pos = after_show.find("FROM");
    let (db, table_name) = if let Some(pos) = from_pos {
        let after_from = after_show[pos + 4..].trim();
        let (name, _) = extract_identifier(after_from)
            .ok_or_else(|| ParseError::SyntaxError {
                position: 0,
                message: "Expected table name".to_string(),
            })?;
        let name_str = name.to_string().to_lowercase();
        let parts: Vec<&str> = name_str.split('.').collect();
        if parts.len() == 2 {
            (parts[0].to_string(), parts[1].to_string())
        } else {
            (String::new(), name_str)
        }
    } else {
        return Err(ParseError::SyntaxError {
            position: 0,
            message: "Expected FROM clause".to_string(),
        });
    };

    Ok(vec![Statement::ShowIndex(db, table_name)])
}

fn parse_show_alter_table(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let sql = sql.trim();
    let after_show = sql
        .strip_prefix("SHOW ALTER TABLE")
        .ok_or_else(|| ParseError::SyntaxError {
            position: 0,
            message: "Expected SHOW ALTER TABLE".to_string(),
        })?
        .trim();

    let mut db_name = None;
    if after_show.to_uppercase().starts_with("FROM ") {
        let after_from = after_show[5..].trim();
        let (name, _) = extract_identifier(after_from)
            .ok_or_else(|| ParseError::SyntaxError {
                position: 0,
                message: "Expected database name".to_string(),
            })?;
        db_name = Some(name.to_string());
    }

    Ok(vec![Statement::ShowAlterTable(db_name)])
}

fn parse_show_view(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let sql = sql.trim();
    let after_show = sql
        .strip_prefix("SHOW VIEW")
        .ok_or_else(|| ParseError::SyntaxError {
            position: 0,
            message: "Expected SHOW VIEW".to_string(),
        })?
        .trim();

    let from_pos = after_show.to_uppercase().find(" FROM ");
    let table_name = if let Some(pos) = from_pos {
        let after_from = after_show[pos + 6..].trim();
        let (name, _) = extract_identifier(after_from)
            .ok_or_else(|| ParseError::SyntaxError {
                position: 0,
                message: "Expected table name".to_string(),
            })?;
        name.to_string()
    } else {
        return Err(ParseError::SyntaxError {
            position: 0,
            message: "Expected FROM clause".to_string(),
        });
    };

    let name_str = table_name.to_string();
    let parts: Vec<&str> = name_str.split('.').collect();
    let (db, view_name) = if parts.len() == 2 {
        (parts[0].to_string(), parts[1].to_string())
    } else {
        (String::new(), name_str)
    };

    Ok(vec![Statement::ShowView(db, view_name)])
}

fn parse_show_create_materialized_view(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let sql = sql.trim();
    let after_show = sql
        .strip_prefix("SHOW CREATE MATERIALIZED VIEW")
        .ok_or_else(|| ParseError::SyntaxError {
            position: 0,
            message: "Expected SHOW CREATE MATERIALIZED VIEW".to_string(),
        })?
        .trim();

    let (name, _) = extract_identifier(after_show)
        .ok_or_else(|| ParseError::SyntaxError {
            position: 0,
            message: "Expected materialized view name".to_string(),
        })?;

    Ok(vec![Statement::ShowCreateMaterializedView(name.to_string())])
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

fn preprocess_create_table(sql: &str) -> (String, Option<DistributionDef>, Option<PartitionDef>, Vec<(String, String)>) {
    let sql_upper = sql.to_uppercase();
    let mut clean_sql = sql.to_string();
    let mut distribution: Option<DistributionDef> = None;
    let mut partition: Option<PartitionDef> = None;
    let mut properties: Vec<(String, String)> = vec![];

    // Extract DISTRIBUTED BY HASH(col1, col2) BUCKETS N
    if let Some(dist_pos) = sql_upper.find("DISTRIBUTED BY") {
        let dist_clause = &sql[dist_pos..];
        let dist_upper = dist_clause.to_uppercase();

        if let Some(hash_pos) = dist_upper.find("HASH") {
            let after_hash = dist_clause[hash_pos + 4..].trim_start();
            if after_hash.starts_with('(') {
                if let Some(end_paren) = after_hash.find(')') {
                    let cols_str = &after_hash[1..end_paren];
                    let columns: Vec<String> = cols_str.split(',')
                        .map(|s| s.trim().to_string())
                        .collect();
                    let remaining = after_hash[end_paren + 1..].trim();
                    let buckets = if remaining.to_uppercase().starts_with("BUCKETS") {
                        remaining[7..].trim().split_whitespace().next()
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(1)
                    } else {
                        1
                    };
                    distribution = Some(DistributionDef {
                        dist_type: "HASH".to_string(),
                        columns,
                        buckets,
                    });
                }
            }
        }

        clean_sql = sql[..dist_pos].trim().to_string();
    }

    // Extract PROPERTIES (...) from clean_sql
    if let Some(prop_pos) = clean_sql.to_uppercase().rfind("PROPERTIES") {
        let props_part = &clean_sql[prop_pos..];
        properties = parse_properties(props_part);
        clean_sql = clean_sql[..prop_pos].trim().to_string();
    }

    // Extract PARTITION BY RANGE/LIST/HASH (...) from clean_sql
    if let Some(part_pos) = clean_sql.to_uppercase().find("PARTITION BY") {
        clean_sql = clean_sql[..part_pos].trim().to_string();
        // Partition parsing can be enhanced later
        partition = None;
    }

    (clean_sql, distribution, partition, properties)
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
            object_type,
            names,
            if_exists,
            ..
        } => {
            let name = names.first().map(|n| n.to_string()).unwrap_or_default();
            match object_type {
                sqlparser::ast::ObjectType::Database => {
                    Ok(Statement::DropDatabase(DropDatabaseStmt {
                        name,
                        if_exists,
                    }))
                }
                _ => {
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
            }
        }
        sqlparser::ast::Statement::ShowDatabases { .. } => {
            Ok(Statement::ShowDatabases)
        }
        sqlparser::ast::Statement::CreateDatabase { db_name, if_not_exists, .. } => {
            Ok(Statement::CreateDatabase(CreateDatabaseStmt {
                name: db_name.to_string(),
                if_not_exists,
                properties: vec![],
            }))
        }
        sqlparser::ast::Statement::ShowTables { show_options, .. } => {
            let db_name = show_options
                .show_in
                .and_then(|si| si.parent_name)
                .map(|n| n.to_string());
            Ok(Statement::ShowTables(db_name))
        }
        sqlparser::ast::Statement::Use(use_expr) => {
            let db_name = match use_expr {
                sqlparser::ast::Use::Database(name) => name.to_string(),
                sqlparser::ast::Use::Object(name) => name.to_string(),
                sqlparser::ast::Use::Schema(name) => name.to_string(),
                other => other.to_string(),
            };
            Ok(Statement::UseDatabase(db_name))
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
        sqlparser::ast::Expr::IsNull(expr) => Expr::IsNull {
            expr: Box::new(convert_expr(*expr)),
            negated: false,
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

    let _if_not_exists = sql.to_uppercase().contains("IF NOT EXISTS");

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

// ---- Batch 2: DDL parsers ----

fn parse_create_index(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let sql = sql.trim();
    let after_create = sql.strip_prefix("CREATE INDEX")
        .ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected CREATE INDEX".to_string() })?
        .trim();
    let (index_name, rest) = extract_identifier(after_create)
        .ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected index name".to_string() })?;
    let rest = rest.trim().strip_prefix("ON")
        .ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected ON".to_string() })?
        .trim();
    let (table_name, rest) = extract_identifier(rest)
        .ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected table name".to_string() })?;
    let name_str = table_name.to_string();
    let parts: Vec<&str> = name_str.split('.').collect();
    let (database, table) = if parts.len() == 2 { (Some(parts[0].to_string()), parts[1].to_string()) } else { (None, name_str) };
    let rest = rest.trim();
    let mut columns = vec![];
    let mut index_type = None;
    let mut properties = vec![];
    if rest.starts_with('(') {
        if let Some(end_paren) = rest.find(')') {
            columns = rest[1..end_paren].split(',').map(|c| c.trim().to_string()).collect();
            let after_paren = rest[end_paren + 1..].trim();
            if after_paren.to_uppercase().starts_with("USING") {
                let after_using = after_paren.trim_start_matches(|c: char| c.is_ascii_alphabetic() || c == ' ').trim();
                if let (itype, _) = extract_identifier(after_using).map(|(t, r)| (Some(t.to_string()), r)).unwrap_or((None, "")) {
                    index_type = itype;
                }
            }
            if after_paren.to_uppercase().contains("PROPERTIES") {
                let prop_start = after_paren.to_uppercase().find("PROPERTIES").unwrap();
                properties = parse_properties(&after_paren[prop_start..]);
            }
        }
    }
    Ok(vec![Statement::CreateIndex(CreateIndexStmt { index_name: index_name.to_string(), database, table, columns, index_type, properties })])
}

fn parse_drop_index(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let sql = sql.trim();
    let after_drop = sql.strip_prefix("DROP INDEX")
        .ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected DROP INDEX".to_string() })?
        .trim();
    let if_exists = after_drop.to_uppercase().starts_with("IF EXISTS");
    let rest = if if_exists { after_drop.trim_start_matches(|c: char| c.is_ascii_alphabetic() || c == ' ').trim() } else { after_drop };
    let (index_name, rest) = extract_identifier(rest).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected index name".to_string() })?;
    let rest = rest.trim().strip_prefix("ON").ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected ON".to_string() })?.trim();
    let (table_name, _) = extract_identifier(rest).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected table name".to_string() })?;
    let name_str = table_name.to_string();
    let parts: Vec<&str> = name_str.split('.').collect();
    let (database, table) = if parts.len() == 2 { (Some(parts[0].to_string()), parts[1].to_string()) } else { (None, name_str) };
    Ok(vec![Statement::DropIndex(DropIndexStmt { index_name: index_name.to_string(), database, table, if_exists })])
}

fn parse_cancel_alter_table(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let after = sql.trim().strip_prefix("CANCEL ALTER TABLE")
        .ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected CANCEL ALTER TABLE".to_string() })?
        .trim();
    let (database, rest) = if after.to_uppercase().starts_with("FROM") {
        // Find "FROM" position in original string (case-insensitive)
        let after_upper = after.to_uppercase();
        let from_pos = after_upper.find("FROM").unwrap();
        let after_from = after[from_pos + 4..].trim();
        let (db, remaining) = extract_identifier(after_from).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected database name".to_string() })?;
        (Some(db.to_string()), remaining.trim())
    } else { (None, after) };
    // When database is specified in FROM clause, the next identifier is the table name
    // (not db.table - that would only come from the table name itself containing a dot)
    let (db_from_clause, table_name) = if database.is_some() {
        // Database already extracted from FROM clause - remaining is just the table
        // Skip leading dot if present (FROM sql_test.t1 means db=sql_test, table=t1)
        let rest = rest.trim_start_matches('.');
        let (name, _) = extract_identifier(rest).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected table name".to_string() })?;
        (database, name.to_string())
    } else {
        // No FROM clause - might be db.table or just table
        let (name, remaining) = extract_identifier(rest).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected table name".to_string() })?;
        let name_str = name.to_string();
        let remaining = remaining.trim();
        if remaining.starts_with('.') {
            // Fully qualified db.table
            let after_dot = remaining[1..].trim();
            let (table_part, _) = extract_identifier(after_dot).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected table name".to_string() })?;
            (Some(name_str), table_part.to_string())
        } else {
            (None, name_str)
        }
    };
    Ok(vec![Statement::CancelAlterTable(CancelAlterTableStmt { database: db_from_clause, table: table_name })])
}

fn parse_alter_colocate_group(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let after = sql.trim().strip_prefix("ALTER COLOCATE GROUP")
        .ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected ALTER COLOCATE GROUP".to_string() })?
        .trim();
    let (group_name, rest) = extract_identifier(after).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected group name".to_string() })?;
    let rest = rest.trim();
    let rest_upper = rest.to_uppercase();
    let operation = if rest_upper.starts_with("ADD TABLE") {
        let after_add = rest.trim_start_matches(|c: char| c.is_ascii_alphabetic() || c == ' ').trim();
        let (table_name, _) = extract_identifier(after_add).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected table name".to_string() })?;
        let name_str = table_name.to_string();
        let parts: Vec<&str> = name_str.split('.').collect();
        let (database, table) = if parts.len() == 2 { (Some(parts[0].to_string()), parts[1].to_string()) } else { (None, name_str) };
        ColocateGroupOperation::AddTable { database, table }
    } else if rest_upper.starts_with("REMOVE TABLE") {
        let after_rm = rest.trim_start_matches(|c: char| c.is_ascii_alphabetic() || c == ' ').trim();
        let (table_name, _) = extract_identifier(after_rm).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected table name".to_string() })?;
        let name_str = table_name.to_string();
        let parts: Vec<&str> = name_str.split('.').collect();
        let (database, table) = if parts.len() == 2 { (Some(parts[0].to_string()), parts[1].to_string()) } else { (None, name_str) };
        ColocateGroupOperation::RemoveTable { database, table }
    } else if rest_upper.starts_with("SET PROPERTIES") {
        let after_set = rest.trim_start_matches(|c: char| c.is_ascii_alphabetic() || c == ' ').trim();
        ColocateGroupOperation::SetProperty(parse_properties(&format!("PROPERTIES {}", after_set)))
    } else {
        return Err(ParseError::SyntaxError { position: 0, message: format!("Unknown ALTER COLOCATE GROUP operation: {}", rest) });
    };
    Ok(vec![Statement::AlterColocateGroup(AlterColocateGroupStmt { group_name: group_name.to_string(), operation })])
}

fn parse_alter_database(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let after = sql.trim().strip_prefix("ALTER DATABASE")
        .ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected ALTER DATABASE".to_string() })?
        .trim();
    let (name, rest) = extract_identifier(after).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected database name".to_string() })?;
    let rest = rest.trim();
    let properties = if rest.to_uppercase().starts_with("SET PROPERTIES") {
        let after_set = rest.trim_start_matches(|c: char| c.is_ascii_alphabetic() || c == ' ').trim();
        parse_properties(&format!("PROPERTIES {}", after_set))
    } else { vec![] };
    Ok(vec![Statement::AlterDatabase(AlterDatabaseStmt { name: name.to_string(), properties })])
}

fn parse_drop_view(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let after = sql.trim().strip_prefix("DROP VIEW")
        .ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected DROP VIEW".to_string() })?
        .trim();
    let if_exists = after.to_uppercase().starts_with("IF EXISTS");
    let rest = if if_exists { after.trim_start_matches(|c: char| c.is_ascii_alphabetic() || c == ' ').trim() } else { after };
    let (name, _) = extract_identifier(rest).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected view name".to_string() })?;
    let name_str = name.to_string();
    let parts: Vec<&str> = name_str.split('.').collect();
    let (database, view_name) = if parts.len() == 2 { (Some(parts[0].to_string()), parts[1].to_string()) } else { (None, name_str) };
    Ok(vec![Statement::DropView(DropViewStmt { database, name: view_name, if_exists })])
}

fn parse_alter_view(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let after = sql.trim().strip_prefix("ALTER VIEW")
        .ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected ALTER VIEW".to_string() })?
        .trim();
    let (name, rest) = extract_identifier(after).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected view name".to_string() })?;
    let name_str = name.to_string();
    let parts: Vec<&str> = name_str.split('.').collect();
    let (database, view_name) = if parts.len() == 2 { (Some(parts[0].to_string()), parts[1].to_string()) } else { (None, name_str) };
    let rest = rest.trim();
    let query = if rest.to_uppercase().starts_with("AS ") { rest[3..].trim().to_string() } else { rest.to_string() };
    Ok(vec![Statement::AlterView(AlterViewStmt { database, name: view_name, query })])
}

fn parse_alter_table_doris(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let after = sql.trim().strip_prefix("ALTER TABLE")
        .ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected ALTER TABLE".to_string() })?
        .trim();
    let (name, rest) = extract_identifier(after).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected table name".to_string() })?;
    let name_str = name.to_string();
    let parts: Vec<&str> = name_str.split('.').collect();
    let (database, table_name) = if parts.len() == 2 { (Some(parts[0].to_string()), parts[1].to_string()) } else { (None, name_str) };
    let rest = rest.trim();
    let rest_upper = rest.to_uppercase();

    let operation = if rest_upper.starts_with("RENAME COLUMN") {
        let after_rename = rest.trim_start_matches(|c: char| !c.is_whitespace()).trim().trim_start_matches("COLUMN").trim();
        let (old_name, remaining) = extract_identifier(after_rename).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected old column name".to_string() })?;
        let remaining = remaining.trim().strip_prefix("TO").or_else(|| remaining.trim().strip_prefix("to")).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected TO".to_string() })?.trim();
        let (new_name, _) = extract_identifier(remaining).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected new column name".to_string() })?;
        AlterOperation::RenameColumn { old_name: old_name.to_string(), new_name: new_name.to_string() }
    } else if rest_upper.starts_with("ADD PARTITION") {
        let after_add = rest.trim_start_matches(|c: char| c.is_ascii_alphabetic() || c == ' ').trim();
        let (part_name, remaining) = extract_identifier(after_add).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected partition name".to_string() })?;
        let remaining = remaining.trim();
        let mut values_less_than = vec![];
        let mut properties = vec![];
        if remaining.to_uppercase().starts_with("VALUES LESS THAN") {
            let after_vals = remaining.trim_start_matches(|c: char| c.is_ascii_alphabetic() || c == ' ').trim();
            if after_vals.starts_with('(') {
                if let Some(end) = after_vals.find(')') {
                    values_less_than = after_vals[1..end].split(',').map(|v| v.trim().trim_matches('\'').trim_matches('"').to_string()).collect();
                    let after_paren = after_vals[end + 1..].trim();
                    if after_paren.to_uppercase().starts_with("PROPERTIES") {
                        properties = parse_properties(after_paren);
                    }
                }
            }
        }
        AlterOperation::AddPartition { partition_name: part_name.to_string(), values_less_than, properties }
    } else if rest_upper.starts_with("DROP PARTITION") {
        let after_drop = rest.trim_start_matches(|c: char| c.is_ascii_alphabetic() || c == ' ').trim();
        let if_exists = after_drop.to_uppercase().starts_with("IF EXISTS");
        let name_part = if if_exists { after_drop.trim_start_matches(|c: char| c.is_ascii_alphabetic() || c == ' ').trim() } else { after_drop };
        let (part_name, remaining) = extract_identifier(name_part).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected partition name".to_string() })?;
        let force = remaining.trim().to_uppercase() == "FORCE";
        AlterOperation::DropPartition { partition_name: part_name.to_string(), if_exists, force }
    } else if rest_upper.starts_with("ADD ROLLUP") {
        let after_add = rest.trim_start_matches(|c: char| c.is_ascii_alphabetic() || c == ' ').trim();
        let (rollup_name, remaining) = extract_identifier(after_add).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected rollup name".to_string() })?;
        let remaining = remaining.trim();
        let mut columns = vec![];
        let mut properties = vec![];
        if remaining.starts_with('(') {
            if let Some(end) = remaining.find(')') {
                columns = remaining[1..end].split(',').map(|c| c.trim().to_string()).collect();
                let after_paren = remaining[end + 1..].trim();
                if after_paren.to_uppercase().starts_with("PROPERTIES") { properties = parse_properties(after_paren); }
            }
        }
        AlterOperation::AddRollup { rollup_name: rollup_name.to_string(), columns, properties }
    } else if rest_upper.starts_with("DROP ROLLUP") {
        let after_drop = rest.trim_start_matches(|c: char| c.is_ascii_alphabetic() || c == ' ').trim();
        let if_exists = after_drop.to_uppercase().starts_with("IF EXISTS");
        let name_part = if if_exists { after_drop.trim_start_matches(|c: char| c.is_ascii_alphabetic() || c == ' ').trim() } else { after_drop };
        let (rollup_name, _) = extract_identifier(name_part).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected rollup name".to_string() })?;
        AlterOperation::DropRollup { rollup_name: rollup_name.to_string(), if_exists }
    } else if rest_upper.starts_with("REPLACE WITH") {
        let after_replace = rest.trim_start_matches(|c: char| c.is_ascii_alphabetic() || c == ' ').trim();
        let after_table = if after_replace.to_uppercase().starts_with("TABLE") { after_replace.trim_start_matches(|c: char| c.is_ascii_alphabetic() || c == ' ').trim() } else { after_replace };
        let (old_table, remaining) = extract_identifier(after_table).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected table name".to_string() })?;
        let swap = remaining.trim().to_uppercase().starts_with("SWAP");
        let properties = if remaining.to_uppercase().contains("PROPERTIES") { let s = remaining.to_uppercase().find("PROPERTIES").unwrap(); parse_properties(&remaining[s..]) } else { vec![] };
        AlterOperation::Replace { old_table: old_table.to_string(), swap, properties }
    } else if rest_upper.starts_with("ADD GENERATED COLUMN") {
        let after_add = rest.trim_start_matches(|c: char| c.is_ascii_alphabetic() || c == ' ').trim();
        let (col_name, remaining) = extract_identifier(after_add).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected column name".to_string() })?;
        let remaining = remaining.trim();
        let (data_type, remaining) = extract_identifier(remaining).map(|(dt, r)| (dt.to_string(), r)).unwrap_or_else(|| ("STRING".to_string(), ""));
        let comment = if remaining.trim().to_uppercase().starts_with("COMMENT") {
            remaining.trim().trim_start_matches(|c: char| c.is_ascii_alphabetic() || c == ' ').trim().trim_matches('\'').trim_matches('"').to_string()
        } else { String::new() };
        AlterOperation::AddGeneratedColumn(ColumnDef {
            name: col_name.to_string(), data_type, nullable: true, default_value: None, agg_type: None,
            comment: if comment.is_empty() { None } else { Some(comment) },
        })
    } else if rest_upper.starts_with("COMMENT") {
        let comment_part = rest.trim_start_matches(|c: char| c.is_ascii_alphabetic() || c == ' ').trim();
        AlterOperation::SetComment(comment_part.trim_matches('\'').trim_matches('"').to_string())
    } else if rest_upper.starts_with("SET PROPERTIES") {
        let after_set = rest.trim_start_matches(|c: char| c.is_ascii_alphabetic() || c == ' ').trim();
        AlterOperation::SetProperty(parse_properties(&format!("PROPERTIES {}", after_set)))
    } else {
        return Err(ParseError::SyntaxError { position: 0, message: format!("Unknown ALTER TABLE operation: {}", rest) });
    };
    Ok(vec![Statement::AlterTable(AlterTableStmt { database, table: table_name, operations: vec![operation] })])
}

// ---- Batch 3/4 parsers ----

fn parse_export_table(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let after = sql.trim().strip_prefix("EXPORT TABLE")
        .ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected EXPORT TABLE".to_string() })?
        .trim();
    // Extract qualified table name (db.table or just table)
    let after_upper = after.to_uppercase();
    let to_pos = after_upper.find(" TO ").ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected TO".to_string() })?;
    let table_part = after[..to_pos].trim();
    let rest = after[to_pos + 4..].trim();
    let parts: Vec<&str> = table_part.split('.').collect();
    let (database, table) = if parts.len() == 2 { (Some(parts[0].to_string()), parts[1].to_string()) } else { (None, table_part.to_string()) };
    let path = rest.trim_start_matches(|c: char| c != ' ').trim_start().split_whitespace().next().unwrap_or("").trim_matches('\'').trim_matches('"').to_string();
    let mut properties = vec![];
    let rest_upper = rest.to_uppercase();
    if rest_upper.contains("PROPERTIES") {
        let idx = rest_upper.find("PROPERTIES").unwrap();
        properties = parse_properties(&rest[idx..]);
    }
    Ok(vec![Statement::ExportTable(ExportTableStmt { database, table, path, properties })])
}

fn parse_cancel_export(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let after = sql.trim().strip_prefix("CANCEL EXPORT")
        .ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected CANCEL EXPORT".to_string() })?
        .trim();
    let (name, _) = extract_identifier(after).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected export ID".to_string() })?;
    Ok(vec![Statement::CancelExport(name.to_string())])
}

fn parse_create_function(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let after = sql.trim().strip_prefix("CREATE FUNCTION")
        .ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected CREATE FUNCTION".to_string() })?
        .trim();
    let if_not_exists = after.to_uppercase().starts_with("IF NOT EXISTS");
    let rest = if if_not_exists { after.trim_start_matches(|c: char| c.is_ascii_alphabetic() || c == ' ').trim() } else { after };
    let (name, rest) = extract_identifier(rest).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected function name".to_string() })?;
    let rest = rest.trim();
    let mut args = vec![];
    let mut returns = None;
    let mut properties = vec![];
    if rest.starts_with('(') {
        if let Some(end) = rest.find(')') {
            args = rest[1..end].split(',').map(|a| a.trim().to_string()).collect();
            let after_paren = rest[end + 1..].trim();
            let after_upper = after_paren.to_uppercase();
            if after_upper.starts_with("RETURNS") {
                let after_returns = after_paren.trim_start_matches(|c: char| c.is_ascii_alphabetic() || c == ' ').trim();
                if let (r, _) = extract_identifier(after_returns).map(|(t, r)| (Some(t.to_string()), r)).unwrap_or((None, "")) { returns = r; }
            }
            if after_paren.to_uppercase().contains("PROPERTIES") {
                let idx = after_paren.to_uppercase().find("PROPERTIES").unwrap();
                properties = parse_properties(&after_paren[idx..]);
            }
        }
    }
    Ok(vec![Statement::CreateFunction(CreateFunctionStmt { name: name.to_string(), args, returns, properties, if_not_exists })])
}

fn parse_drop_function(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let after = sql.trim().strip_prefix("DROP FUNCTION")
        .ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected DROP FUNCTION".to_string() })?
        .trim();
    let if_exists = after.to_uppercase().starts_with("IF EXISTS");
    let rest = if if_exists { after.trim_start_matches(|c: char| c.is_ascii_alphabetic() || c == ' ').trim() } else { after };
    let (name, rest) = extract_identifier(rest).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected function name".to_string() })?;
    let mut args = vec![];
    let rest = rest.trim();
    if rest.starts_with('(') {
        if let Some(end) = rest.find(')') { args = rest[1..end].split(',').map(|a| a.trim().to_string()).collect(); }
    }
    Ok(vec![Statement::DropFunction(DropFunctionStmt { name: name.to_string(), args, if_exists })])
}

fn parse_show_create_function(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let after = sql.trim().strip_prefix("SHOW CREATE FUNCTION")
        .ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected SHOW CREATE FUNCTION".to_string() })?
        .trim();
    let (name, _) = extract_identifier(after).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected function name".to_string() })?;
    Ok(vec![Statement::ShowCreateFunction(name.to_string())])
}

fn parse_desc_function(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let after = sql.trim();
    let after = if after.to_uppercase().starts_with("DESCRIBE FUNCTION") { after.trim_start_matches(|c: char| c.is_ascii_alphabetic() || c == ' ').trim() }
    else { after.strip_prefix("DESC FUNCTION").unwrap().trim() };
    let (name, _) = extract_identifier(after).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected function name".to_string() })?;
    Ok(vec![Statement::DescribeFunction(name.to_string())])
}

fn parse_analyze_table(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let after = sql.trim().strip_prefix("ANALYZE TABLE")
        .ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected ANALYZE TABLE".to_string() })?
        .trim();
    let (table_name, rest) = extract_identifier(after).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected table name".to_string() })?;
    let name_str = table_name.to_string();
    let parts: Vec<&str> = name_str.split('.').collect();
    let (database, table) = if parts.len() == 2 { (Some(parts[0].to_string()), parts[1].to_string()) } else { (None, name_str) };
    let rest = rest.trim();
    let mut columns = vec![];
    let mut sample_rate = None;
    let rest_upper = rest.to_uppercase();
    if rest_upper.contains("UPDATE COLUMNS") {
        let idx = rest_upper.find("UPDATE COLUMNS").unwrap();
        let after_cols = rest[idx + 14..].trim();
        if after_cols.starts_with('(') { if let Some(end) = after_cols.find(')') { columns = after_cols[1..end].split(',').map(|c| c.trim().to_string()).collect(); } }
    }
    if rest_upper.contains("SAMPLE RATE") {
        let idx = rest_upper.find("SAMPLE RATE").unwrap();
        let rate_str = rest[idx + 11..].trim().split_whitespace().next().unwrap_or("0.1");
        sample_rate = rate_str.parse().ok();
    }
    Ok(vec![Statement::AnalyzeTable(AnalyzeTableStmt { database, table, columns, sample_rate })])
}

fn parse_drop_stats(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let after = sql.trim().strip_prefix("DROP STATS")
        .ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected DROP STATS".to_string() })?
        .trim();
    let (table_name, rest) = extract_identifier(after).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected table name".to_string() })?;
    let name_str = table_name.to_string();
    let parts: Vec<&str> = name_str.split('.').collect();
    let (database, table) = if parts.len() == 2 { (Some(parts[0].to_string()), parts[1].to_string()) } else { (None, name_str) };
    let mut columns = vec![];
    if rest.trim().to_uppercase().starts_with("COLUMNS") {
        let after_cols = rest.trim().trim_start_matches(|c: char| c.is_ascii_alphabetic() || c == ' ').trim();
        if after_cols.starts_with('(') { if let Some(end) = after_cols.find(')') { columns = after_cols[1..end].split(',').map(|c| c.trim().to_string()).collect(); } }
    }
    Ok(vec![Statement::DropStats(DropStatsStmt { database, table, columns })])
}

fn parse_kill_analyze_job(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let after = sql.trim().strip_prefix("KILL ANALYZE JOB")
        .ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected KILL ANALYZE JOB".to_string() })?
        .trim();
    let (name, _) = extract_identifier(after).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected job ID".to_string() })?;
    Ok(vec![Statement::KillAnalyzeJob(name.to_string())])
}

fn parse_alter_stats(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let after = sql.trim().strip_prefix("ALTER STATS")
        .ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected ALTER STATS".to_string() })?
        .trim();
    let (table_name, rest) = extract_identifier(after).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected table name".to_string() })?;
    let rest = rest.trim();
    let props = if rest.to_uppercase().starts_with("PROPERTIES") { parse_properties(rest) } else { vec![] };
    Ok(vec![Statement::AlterStats(table_name.to_string(), props)])
}

fn parse_show_analyze(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let after = sql.trim().strip_prefix("SHOW ANALYZE")
        .ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected SHOW ANALYZE".to_string() })?
        .trim();
    if after.is_empty() || after == ";" { return Ok(vec![Statement::ShowAnalyze(None)]); }
    let (name, _) = extract_identifier(after).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected job ID".to_string() })?;
    let job_id = if name.to_uppercase() == "JOB" {
        let remaining = after.trim_start_matches(|c: char| c.is_ascii_alphabetic() || c == ' ').trim();
        let (id, _) = extract_identifier(remaining).unwrap_or((&after, ""));
        Some(id.to_string())
    } else { Some(name.to_string()) };
    Ok(vec![Statement::ShowAnalyze(job_id)])
}

fn parse_show_stats(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let after = sql.trim().strip_prefix("SHOW STATS")
        .ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected SHOW STATS".to_string() })?
        .trim();
    let (name, _) = extract_identifier(after).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected table name".to_string() })?;
    Ok(vec![Statement::ShowStats(name.to_string())])
}

fn parse_show_table_stats(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let after = sql.trim();
    let after = if after.to_uppercase().starts_with("SHOW TABLE STATISTICS") { after.trim_start_matches(|c: char| c.is_ascii_alphabetic() || c == ' ').trim() }
    else { after.strip_prefix("SHOW TABLE STATS").unwrap().trim().trim_start_matches(|c: char| c.is_ascii_alphabetic() || c == ' ').trim() };
    let (name, _) = extract_identifier(after).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected table name".to_string() })?;
    Ok(vec![Statement::ShowTableStats(name.to_string())])
}

fn parse_create_job(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let after = sql.trim().strip_prefix("CREATE JOB")
        .ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected CREATE JOB".to_string() })?
        .trim();
    let (name, rest) = extract_identifier(after).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected job name".to_string() })?;
    let rest = rest.trim();
    let schedule = if rest.to_uppercase().starts_with("ON SCHEDULE") {
        let after_schedule = rest.trim_start_matches(|c: char| c.is_ascii_alphabetic() || c == ' ').trim();
        let cron_start = after_schedule.find("CRON").map(|i| i + 4).unwrap_or(0);
        if cron_start > 0 {
            let after_cron = after_schedule[cron_start..].trim();
            let inner = if after_cron.starts_with('(') { if let Some(end) = after_cron.find(')') { after_cron[1..end].to_string() } else { after_cron.to_string() } } else { after_cron.split_whitespace().next().unwrap_or("").to_string() };
            inner.trim_matches('\'').trim_matches('"').to_string()
        } else { after_schedule.to_string() }
    } else { String::new() };
    let execute = if rest.to_uppercase().contains("EXECUTE") {
        let idx = rest.to_uppercase().find("EXECUTE").unwrap();
        let after_exec = rest[idx + 7..].trim();
        after_exec.trim_matches('\'').trim_matches('"').to_string()
    } else { String::new() };
    Ok(vec![Statement::CreateJob(CreateJobStmt { name: name.to_string(), schedule, execute })])
}

fn parse_drop_job(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let after = sql.trim().strip_prefix("DROP JOB")
        .ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected DROP JOB".to_string() })?
        .trim();
    let if_exists = after.to_uppercase().starts_with("IF EXISTS");
    let rest = if if_exists { after.trim_start_matches(|c: char| c.is_ascii_alphabetic() || c == ' ').trim() } else { after };
    let (name, _) = extract_identifier(rest).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected job name".to_string() })?;
    let _ = if_exists;
    Ok(vec![Statement::DropJob(name.to_string())])
}

fn parse_pause_job(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let after = sql.trim().strip_prefix("PAUSE JOB")
        .ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected PAUSE JOB".to_string() })?
        .trim();
    let (name, _) = extract_identifier(after).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected job name".to_string() })?;
    Ok(vec![Statement::PauseJob(name.to_string())])
}

fn parse_resume_job(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let after = sql.trim().strip_prefix("RESUME JOB")
        .ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected RESUME JOB".to_string() })?
        .trim();
    let (name, _) = extract_identifier(after).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected job name".to_string() })?;
    Ok(vec![Statement::ResumeJob(name.to_string())])
}

fn parse_cancel_task(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let after = sql.trim().strip_prefix("CANCEL TASK")
        .ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected CANCEL TASK".to_string() })?
        .trim();
    let (name, _) = extract_identifier(after).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected task ID".to_string() })?;
    Ok(vec![Statement::CancelTask(name.to_string())])
}

fn parse_install_plugin(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let after = sql.trim().strip_prefix("INSTALL PLUGIN")
        .ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected INSTALL PLUGIN".to_string() })?
        .trim();
    let (name, rest) = extract_identifier(after).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected plugin name".to_string() })?;
    let rest = rest.trim().strip_prefix("FROM").ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected FROM".to_string() })?.trim();
    let source = rest.trim_matches('\'').trim_matches('"').to_string();
    Ok(vec![Statement::InstallPlugin(InstallPluginStmt { name: name.to_string(), source })])
}

fn parse_uninstall_plugin(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let after = sql.trim().strip_prefix("UNINSTALL PLUGIN")
        .ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected UNINSTALL PLUGIN".to_string() })?
        .trim();
    let (name, _) = extract_identifier(after).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected plugin name".to_string() })?;
    Ok(vec![Statement::UninstallPlugin(name.to_string())])
}

fn parse_recover_database(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let after = sql.trim().strip_prefix("RECOVER DATABASE")
        .ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected RECOVER DATABASE".to_string() })?
        .trim();
    let (name, _) = extract_identifier(after).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected database name".to_string() })?;
    Ok(vec![Statement::RecoverDatabase(name.to_string())])
}

fn parse_recover_table(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let after = sql.trim().strip_prefix("RECOVER TABLE")
        .ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected RECOVER TABLE".to_string() })?
        .trim();
    let (db_or_table, rest) = extract_identifier(after).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected name".to_string() })?;
    let rest = rest.trim();
    let (database, table) = if let Some((t2, _)) = extract_identifier(rest) { (db_or_table.to_string(), t2.to_string()) } else { (String::new(), db_or_table.to_string()) };
    Ok(vec![Statement::RecoverTable { database, table }])
}

fn parse_recover_partition(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let after = sql.trim().strip_prefix("RECOVER PARTITION")
        .ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected RECOVER PARTITION".to_string() })?
        .trim();
    let (n1, rest) = extract_identifier(after).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected name".to_string() })?;
    let rest = rest.trim();
    let (n2, rest) = extract_identifier(rest).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected name".to_string() })?;
    let rest = rest.trim();
    let (n3, _) = extract_identifier(rest).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected name".to_string() })?;
    Ok(vec![Statement::RecoverPartition { database: n1.to_string(), table: n2.to_string(), partition: n3.to_string() }])
}

fn parse_drop_catalog_recycle_bin(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let after = sql.trim().strip_prefix("DROP CATALOG RECYCLE BIN")
        .ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected DROP CATALOG RECYCLE BIN".to_string() })?
        .trim();
    let filter = if after.to_uppercase().starts_with("WHERE") {
        Some(after.trim_start_matches(|c: char| c.is_ascii_alphabetic() || c == ' ').trim().to_string())
    } else { None };
    Ok(vec![Statement::DropCatalogRecycleBin(filter)])
}

fn parse_create_sql_block_rule(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let after = sql.trim().strip_prefix("CREATE SQL_BLOCK_RULE")
        .ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected CREATE SQL_BLOCK_RULE".to_string() })?
        .trim();
    let (name, rest) = extract_identifier(after).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected rule name".to_string() })?;
    let props = if rest.trim().to_uppercase().starts_with("PROPERTIES") { parse_properties(rest.trim()) } else { vec![] };
    Ok(vec![Statement::CreateSqlBlockRule(CreateSqlBlockRuleStmt { name: name.to_string(), properties: props })])
}

fn parse_alter_sql_block_rule(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let after = sql.trim().strip_prefix("ALTER SQL_BLOCK_RULE")
        .ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected ALTER SQL_BLOCK_RULE".to_string() })?
        .trim();
    let (name, rest) = extract_identifier(after).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected rule name".to_string() })?;
    let props = if rest.trim().to_uppercase().starts_with("PROPERTIES") { parse_properties(rest.trim()) } else { vec![] };
    Ok(vec![Statement::AlterSqlBlockRule(name.to_string(), props)])
}

fn parse_drop_sql_block_rule(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let after = sql.trim().strip_prefix("DROP SQL_BLOCK_RULE")
        .ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected DROP SQL_BLOCK_RULE".to_string() })?
        .trim();
    let if_exists = after.to_uppercase().starts_with("IF EXISTS");
    let rest = if if_exists { after.trim_start_matches(|c: char| c.is_ascii_alphabetic() || c == ' ').trim() } else { after };
    let (name, _) = extract_identifier(rest).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected rule name".to_string() })?;
    let _ = if_exists;
    Ok(vec![Statement::DropSqlBlockRule(name.to_string())])
}

fn parse_show_sql_block_rule(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let after = sql.trim().strip_prefix("SHOW SQL_BLOCK_RULE")
        .ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected SHOW SQL_BLOCK_RULE".to_string() })?
        .trim();
    let filter = if after.is_empty() || after == ";" { None }
    else if after.to_uppercase().starts_with("FOR") {
        let after_for = after.trim_start_matches(|c: char| c.is_ascii_alphabetic() || c == ' ').trim();
        let (name, _) = extract_identifier(after_for).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected rule name".to_string() })?;
        Some(name.to_string())
    } else { Some(after.to_string()) };
    Ok(vec![Statement::ShowSqlBlockRule(filter)])
}

fn parse_create_row_policy(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let after = sql.trim().strip_prefix("CREATE ROW POLICY")
        .ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected CREATE ROW POLICY".to_string() })?
        .trim();
    let (name, rest) = extract_identifier(after).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected policy name".to_string() })?;
    let rest = rest.trim();
    let rest_upper = rest.to_uppercase();
    let after_on = if rest_upper.starts_with("ON") { rest.trim_start_matches(|c: char| c.is_ascii_alphabetic() || c == ' ').trim() } else { rest };
    let (table_name, rest) = extract_identifier(after_on).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected table name".to_string() })?;
    let name_str = table_name.to_string();
    let parts: Vec<&str> = name_str.split('.').collect();
    let (database, table) = if parts.len() == 2 { (Some(parts[0].to_string()), parts[1].to_string()) } else { (None, name_str) };
    let rest = rest.trim();
    let rest_upper = rest.to_uppercase();
    let after_as = if rest_upper.starts_with("AS") { rest.trim_start_matches(|c: char| c.is_ascii_alphabetic() || c == ' ').trim() } else { rest };
    let (policy_type, rest) = extract_identifier(after_as).unwrap_or(("PERMIT", ""));
    let rest = if rest.is_empty() { "" } else { rest.trim() };
    let using_expr = if rest.to_uppercase().starts_with("USING") {
        rest.trim_start_matches(|c: char| c.is_ascii_alphabetic() || c == ' ').trim().trim_matches('\'').trim_matches('"').to_string()
    } else { String::new() };
    Ok(vec![Statement::CreateRowPolicy(CreateRowPolicyStmt {
        name: name.to_string(), database, table, policy_type: policy_type.to_string(), using_expr,
    })])
}

fn parse_drop_row_policy(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let after = sql.trim().strip_prefix("DROP ROW POLICY")
        .ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected DROP ROW POLICY".to_string() })?
        .trim();
    let if_exists = after.to_uppercase().starts_with("IF EXISTS");
    let rest = if if_exists { after.trim_start_matches(|c: char| c.is_ascii_alphabetic() || c == ' ').trim() } else { after };
    let (name, rest) = extract_identifier(rest).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected policy name".to_string() })?;
    let rest = rest.trim();
    let (database, table) = if rest.to_uppercase().starts_with("ON") {
        let after_on = rest.trim_start_matches(|c: char| c.is_ascii_alphabetic() || c == ' ').trim();
        let (table_name, _) = extract_identifier(after_on).ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected table name".to_string() })?;
        let name_str = table_name.to_string();
        let parts: Vec<&str> = name_str.split('.').collect();
        if parts.len() == 2 { (Some(parts[0].to_string()), parts[1].to_string()) } else { (None, name_str) }
    } else { (None, String::new()) };
    let _ = if_exists;
    Ok(vec![Statement::DropRowPolicy { name: name.to_string(), database, table }])
}

fn parse_show_row_policy(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let after = sql.trim().strip_prefix("SHOW ROW POLICY")
        .ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected SHOW ROW POLICY".to_string() })?
        .trim();
    let filter = if after.is_empty() || after == ";" { None } else { Some(after.to_string()) };
    Ok(vec![Statement::ShowRowPolicy(filter)])
}

fn parse_show_functions(sql: &str) -> Result<Vec<Statement>, ParseError> {
    let after = sql.trim().strip_prefix("SHOW FUNCTIONS")
        .ok_or_else(|| ParseError::SyntaxError { position: 0, message: "Expected SHOW FUNCTIONS".to_string() })?
        .trim();
    let pattern = if after.is_empty() || after == ";" {
        None
    } else if after.to_uppercase().starts_with("LIKE") {
        let after_like = after.trim_start_matches(|c: char| c.is_ascii_alphabetic() || c == ' ').trim();
        Some(after_like.trim_matches('\'').trim_matches('"').to_string())
    } else {
        Some(after.to_string())
    };
    Ok(vec![Statement::ShowFunctions(pattern)])
}
