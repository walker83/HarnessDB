use fe_sql_parser::parse_sql;

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