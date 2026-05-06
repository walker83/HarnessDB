use fe_sql_parser::{parse_sql, Statement};

#[test]
fn test_dml_compatibility() {
    // INSERT ... SET syntax (MySQL compatibility)
    let result = parse_sql("INSERT INTO test_table SET id = 1, name = 'test'");
    assert!(result.is_ok(), "INSERT ... SET failed: {:?}", result);
    let stmt = result.unwrap().into_iter().next().unwrap();
    if let Statement::Insert(insert) = stmt {
        assert_eq!(insert.table, "test_table");
        assert_eq!(insert.columns, vec!["id", "name"]);
        assert!(!insert.values.is_empty());
    } else {
        panic!("Expected Insert statement");
    }

    // INSERT ... SET with multiple columns
    let result2 = parse_sql("INSERT INTO t SET a = 1, b = 2, c = 3");
    assert!(result2.is_ok(), "INSERT ... SET multiple cols failed: {:?}", result2);

    // INSERT ... SET with expressions (tests comma handling inside function calls)
    let result3 = parse_sql("INSERT INTO t SET id = 1 + 1, name = CONCAT('a', 'b')");
    assert!(result3.is_ok(), "INSERT ... SET with expressions failed: {:?}", result3);
}

#[test]
fn test_batch_3_export_statements() {
    assert!(parse_sql("EXPORT TABLE test_db.test_table TO '/tmp/export' PROPERTIES (\"format\"=\"csv\")").is_ok());
    assert!(parse_sql("CANCEL EXPORT export_123").is_ok());
    assert!(parse_sql("SHOW EXPORT").is_ok());
}

#[test]
fn test_batch_4_udf_statements() {
    assert!(parse_sql("CREATE FUNCTION my_udf(INT, INT) RETURNS INT PROPERTIES (\"type\"=\"UDF\")").is_ok());
    assert!(parse_sql("DROP FUNCTION my_udf(INT, INT)").is_ok());
    assert!(parse_sql("DROP FUNCTION IF EXISTS my_udf").is_ok());
    assert!(parse_sql("SHOW FUNCTIONS").is_ok());
    assert!(parse_sql("SHOW FUNCTIONS LIKE 'my_%'").is_ok());
    assert!(parse_sql("SHOW CREATE FUNCTION test_db.my_udf").is_ok());
    assert!(parse_sql("DESC FUNCTION test_db.my_udf").is_ok());
    assert!(parse_sql("DESCRIBE FUNCTION my_udf").is_ok());
}

#[test]
fn test_batch_4_statistics_statements() {
    assert!(parse_sql("ANALYZE TABLE test_db.test_table").is_ok());
    assert!(parse_sql("ANALYZE TABLE test_db.test_table WITH SAMPLE RATE 0.1").is_ok());
    assert!(parse_sql("ANALYZE TABLE test_table UPDATE COLUMNS(col1, col2)").is_ok());
    assert!(parse_sql("ALTER STATS test_db.test_table PROPERTIES (\"enable\"=\"true\")").is_ok());
    assert!(parse_sql("DROP STATS test_db.test_table").is_ok());
    assert!(parse_sql("DROP STATS test_table COLUMNS(col1)").is_ok());
    assert!(parse_sql("DROP ANALYZE JOB job_123").is_ok());
    assert!(parse_sql("KILL ANALYZE JOB job_123").is_ok());
    assert!(parse_sql("SHOW ANALYZE").is_ok());
    assert!(parse_sql("SHOW ANALYZE JOB job_123").is_ok());
    assert!(parse_sql("SHOW STATS test_db.test_table").is_ok());
    assert!(parse_sql("SHOW TABLE STATS test_table").is_ok());
    assert!(parse_sql("SHOW TABLE STATISTICS test_table").is_ok());
}

#[test]
fn test_batch_4_job_statements() {
    assert!(parse_sql("CREATE JOB my_job ON SCHEDULE CRON('0 0 * * *') EXECUTE 'SELECT 1'").is_ok());
    assert!(parse_sql("DROP JOB my_job").is_ok());
    assert!(parse_sql("DROP JOB IF EXISTS my_job").is_ok());
    assert!(parse_sql("PAUSE JOB my_job").is_ok());
    assert!(parse_sql("RESUME JOB my_job").is_ok());
    assert!(parse_sql("CANCEL TASK task_123").is_ok());
}

#[test]
fn test_batch_4_plugin_statements() {
    assert!(parse_sql("INSTALL PLUGIN audit_plugin FROM '/tmp/plugin.so'").is_ok());
    assert!(parse_sql("UNINSTALL PLUGIN audit_plugin").is_ok());
    assert!(parse_sql("SHOW PLUGINS").is_ok());
}

#[test]
fn test_batch_4_recycle_bin_statements() {
    assert!(parse_sql("RECOVER DATABASE dropped_db").is_ok());
    assert!(parse_sql("RECOVER TABLE test_db dropped_table").is_ok());
    assert!(parse_sql("RECOVER PARTITION test_db test_table dropped_partition").is_ok());
    assert!(parse_sql("DROP CATALOG RECYCLE BIN").is_ok());
    assert!(parse_sql("DROP CATALOG RECYCLE BIN WHERE Type='Database'").is_ok());
    assert!(parse_sql("SHOW CATALOG RECYCLE BIN").is_ok());
}

#[test]
fn test_batch_4_data_governance_statements() {
    assert!(parse_sql("CREATE SQL_BLOCK_RULE rule1 PROPERTIES (\"sql\"=\"SELECT\")").is_ok());
    assert!(parse_sql("ALTER SQL_BLOCK_RULE rule1 PROPERTIES (\"sql\"=\"SELECT 1\")").is_ok());
    assert!(parse_sql("DROP SQL_BLOCK_RULE rule1").is_ok());
    assert!(parse_sql("DROP SQL_BLOCK_RULE IF EXISTS rule1").is_ok());
    assert!(parse_sql("SHOW SQL_BLOCK_RULE").is_ok());
    assert!(parse_sql("SHOW SQL_BLOCK_RULE FOR rule1").is_ok());
    
    assert!(parse_sql("CREATE ROW POLICY policy1 ON test_db.test_table AS PERMIT USING 'id > 0'").is_ok());
    assert!(parse_sql("CREATE ROW POLICY policy1 ON test_table AS RESTRICT USING 'id < 100'").is_ok());
    assert!(parse_sql("DROP ROW POLICY policy1 ON test_db.test_table").is_ok());
    assert!(parse_sql("DROP ROW POLICY IF EXISTS policy1 ON test_table").is_ok());
    assert!(parse_sql("SHOW ROW POLICY").is_ok());
    assert!(parse_sql("SHOW ROW POLICY FOR policy1 ON test_db.test_table").is_ok());
}
#[test]
fn test_insert_overwrite_partition_parsing() {
    // Test INSERT OVERWRITE PARTITION parsing
    let sql = "INSERT OVERWRITE insert_test_overwrite_part PARTITION(p202401) VALUES (4, '2024-01-20', 'OverwritePart1', 400)";
    match parse_sql(sql) {
        Ok(stmts) => {
            println!("Parsed {} statements", stmts.len());
            for stmt in &stmts {
                if let Statement::Insert(insert) = stmt {
                    println!("  is_overwrite: {}", insert.is_overwrite);
                    println!("  columns: {:?}", insert.columns);
                    println!("  values: {:?}", insert.values);
                }
            }
        }
        Err(e) => {
            panic!("Parse error: {:?}", e);
        }
    }
}

#[test]
fn test_insert_overwrite_partition_details() {
    // Test the exact SQL from the issue
    let sql = "INSERT OVERWRITE insert_test_overwrite_part PARTITION(p202401) VALUES (4, '2024-01-20', 'OverwritePart1', 400)";
    match parse_sql(sql) {
        Ok(stmts) => {
            println!("\nParsing: {}", sql);
            for stmt in &stmts {
                if let Statement::Insert(insert) = stmt {
                    println!("  table: {}", insert.table);
                    println!("  columns: {:?}", insert.columns);
                    println!("  values count: {}", insert.values.len());
                    for (i, row) in insert.values.iter().enumerate() {
                        println!("  row {}: {:?}", i, row);
                    }
                    println!("  is_overwrite: {}", insert.is_overwrite);
                    println!("  query: {:?}", insert.query);
                }
            }
        }
        Err(e) => {
            panic!("Parse error: {:?}", e);
        }
    }

    // Test without PARTITION clause
    let sql2 = "INSERT OVERWRITE insert_test_overwrite_part VALUES (4, '2024-01-20', 'OverwritePart1', 400)";
    match parse_sql(sql2) {
        Ok(stmts) => {
            println!("\nParsing: {}", sql2);
            for stmt in &stmts {
                if let Statement::Insert(insert) = stmt {
                    println!("  table: {}", insert.table);
                    println!("  columns: {:?}", insert.columns);
                    println!("  values count: {}", insert.values.len());
                    for (i, row) in insert.values.iter().enumerate() {
                        println!("  row {}: {:?}", i, row);
                    }
                    println!("  is_overwrite: {}", insert.is_overwrite);
                }
            }
        }
        Err(e) => {
            panic!("Parse error: {:?}", e);
        }
    }
}

#[test]
fn test_insert_overwrite_partition_debug() {
    use fe_sql_parser::{parse_sql, Statement};
    let sql = "INSERT OVERWRITE insert_test_overwrite_part PARTITION(p202401) VALUES (4, '2024-01-20', 'OverwritePart1', 400)";
    match parse_sql(sql) {
        Ok(stmts) => {
            println!("Parsed statements: {:#?}", stmts);
            for stmt in stmts {
                if let Statement::Insert(insert) = stmt {
                    println!("is_overwrite: {}, columns: {:?}, values: {:?}", insert.is_overwrite, insert.columns, insert.values);
                }
            }
        }
        Err(e) => {
            panic!("Error: {:?}", e);
        }
    }
}

#[test]
fn test_values_function_in_upsert() {
    // Test the exact pattern from ON DUPLICATE KEY UPDATE
    let sql = "INSERT INTO upsert_test_seq (seq_id, seq_name, seq_version, seq_data) VALUES (1, 'SeqItem1', 2, 'version1_data_v2') ON DUPLICATE KEY UPDATE seq_name = VALUES(seq_name), seq_version = VALUES(seq_version), seq_data = VALUES(seq_data)";
    match parse_sql(sql) {
        Ok(stmts) => {
            println!("\nParsing: INSERT ... ON DUPLICATE KEY UPDATE");
            for stmt in &stmts {
                if let Statement::Insert(insert) = stmt {
                    println!("  table: {}", insert.table);
                    println!("  columns: {:?}", insert.columns);
                    println!("  on_duplicate_key_update count: {}", insert.on_duplicate_key_update.len());
                    for (i, update) in insert.on_duplicate_key_update.iter().enumerate() {
                        println!("  update[{}]: column={}, value={:?}", i, update.column, update.value);
                    }
                }
            }
        }
        Err(e) => {
            panic!("Parse error: {:?}", e);
        }
    }
}

#[test]
fn test_insert_overwrite_with_partition_parsing() {
    // Test the exact SQL from 01_insert_operations.sql line 325
    let sql = "INSERT OVERWRITE insert_test_overwrite_part PARTITION(p202401) VALUES (4, '2024-01-20', 'OverwritePart1', 400)";
    match parse_sql(sql) {
        Ok(stmts) => {
            println!("\nParsing: {}", sql);
            for stmt in &stmts {
                if let Statement::Insert(insert) = stmt {
                    println!("  table: {}", insert.table);
                    println!("  columns: {:?}", insert.columns);
                    println!("  values: {:?}", insert.values);
                    println!("  is_overwrite: {}", insert.is_overwrite);
                    println!("  partition: {:?}", insert.partition);
                }
            }
        }
        Err(e) => {
            panic!("Parse error: {:?}", e);
        }
    }
}
