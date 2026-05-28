use regex::Regex;
use tracing::warn;

/// Translate MaxCompute SQL to RorisDB-compatible SQL.
///
/// Returns `(translated_sql, is_noop)` where `is_noop` means the statement
/// should be silently ignored (like SET, SETPROJECT, ADD JAR, ADD FILE).
pub fn translate_mc_sql(sql: &str) -> (String, bool) {
    let trimmed = sql.trim().trim_end_matches(';');
    let trimmed = trimmed.trim();

    if trimmed.is_empty() {
        return (String::new(), true);
    }

    // Check for no-op statements
    if is_noop_statement(trimmed) {
        return (String::new(), true);
    }

    let mut result = trimmed.to_string();

    // Apply translations in order
    result = strip_set_statements(&result);
    result = strip_mapjoin_hints(&result);
    result = strip_skewjoin_hints(&result);
    result = translate_insert_overwrite(&result);
    result = strip_partitioned_by(&result);
    result = strip_lifecycle(&result);
    result = strip_stored_as(&result);
    result = strip_clustered_by(&result);
    result = strip_tblproperties(&result);
    result = translate_distribute_sort_by(&result);
    result = strip_partition_clause_in_insert(&result);

    let result = result.trim().to_string();

    if result.is_empty() {
        (String::new(), true)
    } else {
        (result, false)
    }
}

/// Returns true if the statement is a MaxCompute no-op that should be silently ignored.
fn is_noop_statement(sql: &str) -> bool {
    let sql = sql.trim();
    if sql.is_empty() {
        return true;
    }

    // Case-insensitive check for no-op statements
    let upper = sql.to_uppercase();

    // SET statement (anything starting with SET that's not a DML/DML statement)
    if upper.starts_with("SET ") || upper.starts_with("SET\t") || upper == "SET" {
        return true;
    }

    // SETPROJECT statement
    if upper.starts_with("SETPROJECT ") || upper.starts_with("SETPROJECT\t") || upper == "SETPROJECT" {
        return true;
    }

    // ADD JAR / ADD FILE statements
    if upper.starts_with("ADD JAR ") || upper.starts_with("ADD JAR\t") {
        return true;
    }
    if upper.starts_with("ADD FILE ") || upper.starts_with("ADD FILE\t") {
        return true;
    }

    // ALTER TABLE statement with lifecycle (MaxCompute-specific, no-op for now)
    if upper.starts_with("ALTER TABLE") {
        warn!("ALTER TABLE statement is not supported in RorisDB, ignoring: {}", sql);
        return true;
    }

    false
}

/// Remove standalone SET statements that set MaxCompute-specific properties.
/// If the SET is embedded in a larger SQL, just remove the SET line/part.
fn strip_set_statements(sql: &str) -> String {
    if sql.trim().to_uppercase().starts_with("SET ") || sql.trim().to_uppercase() == "SET" {
        // For standalone SET statements, return empty
        if is_noop_statement(sql.trim()) {
            return String::new();
        }

        // Check if this is a SET statement as part of a larger block
        // We handle simple SET statements that might be embedded
        let re = Regex::new(r"(?i)\bSET\s+\S+\s*=\s*\S+\s*;?\s*").unwrap();
        return re.replace_all(sql, "").to_string();
    }

    // Remove embedded SET statements (lines starting with SET)
    let re = Regex::new(r"(?im)^\s*SET\s+.*?(?:;?\s*)$").unwrap();
    re.replace_all(sql, "").to_string()
}

/// Remove `/*+ MAPJOIN(alias1, alias2, ...) */` hints from SQL.
fn strip_mapjoin_hints(sql: &str) -> String {
    let re = Regex::new(r"(?is)/\*\+\s*MAPJOIN\s*\([^)]*\)\s*\*/").unwrap();
    let result = re.replace_all(sql, "").to_string();

    // Also handle `/*+ MAPJOIN(...) */` with possible nested parens via a simpler approach:
    // For complex cases where MAPJOIN may contain nested parens, use a different pattern
    if result.contains("MAPJOIN") {
        // Deep scan for MAPJOIN hints that weren't caught by the simple regex
        let re_deep = Regex::new(r"(?is)/\*\+\s*MAPJOIN\s*\(").unwrap();
        if re_deep.is_match(&result) {
            return strip_hint_block(&result, "MAPJOIN");
        }
    }

    result
}

/// Remove `/*+ SKEWJOIN(...) */` hints from SQL.
fn strip_skewjoin_hints(sql: &str) -> String {
    let re = Regex::new(r"(?is)/\*\+\s*SKEWJOIN\s*\([^)]*\)\s*\*/").unwrap();
    let result = re.replace_all(sql, "").to_string();

    // Handle nested parens for SKEWJOIN
    if result.contains("SKEWJOIN") {
        let re_deep = Regex::new(r"(?is)/\*\+\s*SKEWJOIN\s*\(").unwrap();
        if re_deep.is_match(&result) {
            return strip_hint_block(&result, "SKEWJOIN");
        }
    }

    result
}

/// Strip a hint block `/*+ HINTNAME(...) */` handling nested parentheses.
fn strip_hint_block(sql: &str, hint_name: &str) -> String {
    let pattern = format!(r"(?is)/\*\+\s*{}\s*\(", hint_name);
    let re = Regex::new(&pattern).unwrap();

    // Collect all match positions and remove ranges
    let mut result = sql.to_string();
    loop {
        if let Some(mat) = re.find(&result) {
            let start = mat.start();
            // Find `*/` after the opening
            let close = result[start..].find("*/");
            if let Some(close_pos) = close {
                let end = start + close_pos + 2;
                result.replace_range(start..end, "");
            } else {
                break;
            }
        } else {
            break;
        }
    }
    result
}

/// Convert `INSERT OVERWRITE [TABLE] t ...` to `INSERT INTO t ...`.
fn translate_insert_overwrite(sql: &str) -> String {
    // `INSERT OVERWRITE TABLE t ...` -> `INSERT INTO t ...`
    let re1 = Regex::new(r"(?i)\bINSERT\s+OVERWRITE\s+TABLE\b").unwrap();
    let result = re1.replace_all(sql, "INSERT INTO").to_string();

    // `INSERT OVERWRITE t ...` -> `INSERT INTO t ...`
    let re2 = Regex::new(r"(?i)\bINSERT\s+OVERWRITE\b").unwrap();
    let result = re2.replace_all(&result, "INSERT INTO").to_string();

    // Log a warning for the translation
    if result != sql {
        warn!("Translated INSERT OVERWRITE to INSERT INTO: {} -> {}", sql, result);
    }

    result
}

/// Find `PARTITIONED BY (...)` and extract partition columns into the table schema.
/// Handles nested parentheses within the partition column definitions.
fn strip_partitioned_by(sql: &str) -> String {
    let re = Regex::new(r"(?i)\bPARTITIONED\s+BY\s*\(").unwrap();

    if let Some(mat) = re.find(sql) {
        let open_pos = sql[mat.start()..].find('(').unwrap() + mat.start();

        if let Some(close_pos) = find_matching_paren(sql, open_pos) {
            // Extract partition columns content (excluding outer parens)
            let partition_content = &sql[open_pos + 1..close_pos];
            let partition_cols = partition_content.trim();

            // Find the CREATE TABLE columns definition
            // We need to find the columns block `(col_defs)` in CREATE TABLE
            let create_re = Regex::new(r"(?i)\bCREATE\s+TABLE\b").unwrap();
            if let Some(_create_mat) = create_re.find(sql) {
                // Find the first outermost `(` after CREATE TABLE (before PARTITIONED BY)
                let before_partition = &sql[..mat.start()];
                if let Some(columns_open) = before_partition.rfind('(') {
                    // This should be the columns definition opening paren
                    let columns_close = find_matching_paren(sql, columns_open);
                    if let Some(cols_close) = columns_close {
                        if cols_close < open_pos {
                            // Insert partition columns before the closing `)` of the columns definition
                            let before_cols = &sql[..cols_close];
                            let after_partition = &sql[close_pos + 1..];
                            // Re-add the closing ) that was consumed by before_cols
                            let combined = format!(
                                "{}, {}){}",
                                before_cols,
                                partition_cols,
                                after_partition
                            );
                            return strip_partitioned_by(&combined);
                        }
                    }
                }
            }

            // Fallback: just remove PARTITIONED BY (...) entirely
            let before = &sql[..mat.start()];
            let after = &sql[close_pos + 1..];
            return format!("{}{}", before, after.trim_start());
        }
    }

    sql.to_string()
}

/// Remove `LIFECYCLE N` clause.
fn strip_lifecycle(sql: &str) -> String {
    let re = Regex::new(r"(?i)\bLIFECYCLE\s+\d+\b").unwrap();
    let result = re.replace_all(sql, "").to_string();

    if result != sql {
        warn!("Stripped LIFECYCLE clause from: {}", sql);
    }

    result
}

/// Remove `STORED AS format` clause.
fn strip_stored_as(sql: &str) -> String {
    // Match STORED AS followed by optional format name (ORC, PARQUET, TEXTFILE, SEQUENCEFILE, RCFILE, AVRO, JSONFILE, etc.)
    let re = Regex::new(r"(?i)\bSTORED\s+AS\s+\w+\b").unwrap();
    let result = re.replace_all(sql, "").to_string();

    if result != sql {
        warn!("Stripped STORED AS clause from: {}", sql);
    }

    result
}

/// Remove `CLUSTERED BY (...) [SORTED BY (...)] INTO N BUCKETS` clause.
fn strip_clustered_by(sql: &str) -> String {
    let re = Regex::new(r"(?i)\bCLUSTERED\s+BY\s*\(").unwrap();

    if let Some(mat) = re.find(sql) {
        let open_pos = sql[mat.start()..].find('(').unwrap() + mat.start();
        if let Some(close_pos) = find_matching_paren(sql, open_pos) {
            // Remove CLUSTERED BY (...) and optional SORTED BY (...)
            let after_paren = &sql[close_pos + 1..];

            // Check for SORTED BY (...)
            let sorted_re = Regex::new(r"(?i)^\s*\bSORTED\s+BY\s*\(").unwrap();
            let (end_pos, sorted_removed) = if let Some(sorted_mat) = sorted_re.find(after_paren) {
                let sorted_open = after_paren[sorted_mat.start()..].find('(').unwrap() + sorted_mat.start();
                if let Some(sorted_close) = find_matching_paren(after_paren, sorted_open) {
                    // sorted_close is position within after_paren, so end position in sql is:
                    // close_pos + 1 (start of after_paren) + sorted_close + 1 (past the ')')
                    (close_pos + 1 + sorted_close + 1, true)
                } else {
                    (close_pos + 1, false)
                }
            } else {
                (close_pos + 1, false)
            };

            let after_all = if sorted_removed {
                &sql[end_pos..]
            } else {
                &sql[close_pos + 1..]
            };

            // Check for INTO N BUCKETS
            let into_re = Regex::new(r"(?i)^\s*\bINTO\s+\d+\s+BUCKETS?\b").unwrap();
            let after_buckets = into_re.replace(after_all, "").to_string();

            let before = &sql[..mat.start()];
            return format!("{}{}", before, after_buckets);
        }
    }

    sql.to_string()
}

/// Remove `TBLPROPERTIES (...)` clause.
fn strip_tblproperties(sql: &str) -> String {
    let re = Regex::new(r"(?i)\bTBLPROPERTIES\s*").unwrap();

    if let Some(mat) = re.find(sql) {
        // Find the opening paren after the keyword
        let rest = &sql[mat.end()..];
        let open_idx = rest.find('(');

        if let Some(oi) = open_idx {
            let open_pos = mat.end() + oi;
            if let Some(close_pos) = find_matching_paren(sql, open_pos) {
                // Remove TBLPROPERTIES keyword and its paren block
                let before = &sql[..mat.start()];
                let after = &sql[close_pos + 1..];
                let result = format!("{}{}", before, after.trim_start());

                if result != sql {
                    warn!("Stripped TBLPROPERTIES clause from: {}", sql);
                }

                return result;
            }
        }
    }

    sql.to_string()
}

/// Translate `DISTRIBUTE BY col1 SORT BY col2` to `ORDER BY col2`.
/// If only `DISTRIBUTE BY` is present, remove it.
fn translate_distribute_sort_by(sql: &str) -> String {
    let result = sql.to_string();

    // Pattern: DISTRIBUTE BY ... SORT BY col -> ORDER BY col
    let re = Regex::new(r"(?i)\bDISTRIBUTE\s+BY\s+(.+?)\bSORT\s+BY\s+(\S+(?:\s*,\s*\S+)*)\b").unwrap();

    if re.is_match(&result) {
        let translated = re.replace_all(&result, |caps: &regex::Captures| {
            let sort_cols = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            format!("ORDER BY {}", sort_cols)
        });
        let translated_str = translated.to_string();

        // Remove any remaining standalone DISTRIBUTE BY clauses
        let re_dist_only = Regex::new(r"(?i)(?:^|\s)\bDISTRIBUTE\s+BY\s+\S+(?:\s*,\s*\S+)*(?:\s|$)").unwrap();
        let final_result = re_dist_only.replace_all(&translated_str, " ").to_string();

        if final_result != sql {
            warn!("Translated DISTRIBUTE BY / SORT BY to ORDER BY: {} -> {}", sql, final_result);
        }

        // Also handle standalone DISTRIBUTE BY without SORT BY
        // Fall through to handle remaining cases
        let result2 = final_result;

        // Standalone DISTRIBUTE BY (without SORT BY)
        let re_dist_alone = Regex::new(r"(?i)\bDISTRIBUTE\s+BY\s+\S+(?:\s*,\s*\S+)*$").unwrap();
        let result2 = re_dist_alone.replace_all(&result2, "").to_string();

        // DISTRIBUTE BY in middle of statement (not at end)
        let re_dist_mid = Regex::new(r"(?i)\bDISTRIBUTE\s+BY\s+\S+(?:\s*,\s*\S+)*\s+").unwrap();
        let result2 = re_dist_mid.replace_all(&result2, "").to_string();

        return result2;
    }

    // Handle standalone DISTRIBUTE BY (without SORT BY)
    let re_dist_only = Regex::new(r"(?i)\bDISTRIBUTE\s+BY\s+\S+(?:\s*,\s*\S+)*(?:\s|$)").unwrap();
    if re_dist_only.is_match(&result) {
        let result2 = re_dist_only.replace_all(&result, " ").to_string();
        warn!("Removed DISTRIBUTE BY clause from: {}", sql);
        return result2;
    }

    result
}

/// Remove `PARTITION(...)` clause from INSERT statements.
fn strip_partition_clause_in_insert(sql: &str) -> String {
    let re = Regex::new(r"(?i)(INSERT\s+(?:INTO|OVERWRITE)\s+\S+)\s+PARTITION\s*\(").unwrap();

    if let Some(mat) = re.find(sql) {
        // Find the start of PARTITION(
        let partition_re = Regex::new(r"(?i)\bPARTITION\s*\(").unwrap();
        if let Some(pmat) = partition_re.find(&sql[mat.start()..]) {
            let partition_start = mat.start() + pmat.start();
            let open_idx = sql[partition_start..].find('(').unwrap();
            let open_pos = partition_start + open_idx;

            if let Some(close_pos) = find_matching_paren(sql, open_pos) {
                let before = &sql[..partition_start];
                let after = &sql[close_pos + 1..];
                let result = format!("{}{}", before, after.trim_start());

                if result != sql {
                    warn!("Stripped PARTITION clause from INSERT: {} -> {}", sql, result);
                }

                return result;
            }
        }
    }

    sql.to_string()
}

/// Find the matching closing parenthesis starting from an opening one.
/// Returns the index of the closing paren, or None if not found.
fn find_matching_paren(s: &str, open_pos: usize) -> Option<usize> {
    let chars: Vec<char> = s.chars().collect();
    if open_pos >= chars.len() || chars[open_pos] != '(' {
        return None;
    }
    let mut depth = 1usize;
    let mut i = open_pos + 1;
    while i < chars.len() && depth > 0 {
        match chars[i] {
            '(' => depth += 1,
            ')' => depth -= 1,
            _ => {}
        }
        i += 1;
    }
    if depth == 0 { Some(i - 1) } else { None }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== Basic CREATE TABLE =====

    #[test]
    fn test_basic_create_table_unchanged() {
        let sql = "CREATE TABLE t (id BIGINT, name STRING)";
        let (result, noop) = translate_mc_sql(sql);
        assert!(!noop, "basic CREATE TABLE should not be noop");
        assert_eq!(result, sql);
    }

    // ===== CREATE TABLE with PARTITIONED BY =====

    #[test]
    fn test_create_table_partitioned_by_single() {
        let sql = "CREATE TABLE t (id BIGINT) PARTITIONED BY (ds STRING)";
        let (result, noop) = translate_mc_sql(sql);
        assert!(!noop);
        assert_eq!(result, "CREATE TABLE t (id BIGINT, ds STRING)");
    }

    #[test]
    fn test_create_table_partitioned_by_multiple() {
        let sql = "CREATE TABLE t (id BIGINT, name STRING) PARTITIONED BY (ds STRING, region STRING)";
        let (result, noop) = translate_mc_sql(sql);
        assert!(!noop);
        assert_eq!(result, "CREATE TABLE t (id BIGINT, name STRING, ds STRING, region STRING)");
    }

    #[test]
    fn test_create_table_partitioned_by_case_insensitive() {
        let sql = "create table t (id bigint) partitioned by (ds string)";
        let (result, noop) = translate_mc_sql(sql);
        assert!(!noop);
        assert_eq!(result, "create table t (id bigint, ds string)");
    }

    #[test]
    fn test_create_table_partitioned_by_nested_parens() {
        // Partition column with default value containing parens
        let sql = "CREATE TABLE t (id BIGINT) PARTITIONED BY (ds STRING COMMENT 'partition (date)')";
        let (result, noop) = translate_mc_sql(sql);
        assert!(!noop);
        assert_eq!(result, "CREATE TABLE t (id BIGINT, ds STRING COMMENT 'partition (date)')");
    }

    // ===== CREATE TABLE with LIFECYCLE =====

    #[test]
    fn test_create_table_with_lifecycle() {
        let sql = "CREATE TABLE t (id BIGINT) LIFECYCLE 365";
        let (result, noop) = translate_mc_sql(sql);
        assert!(!noop);
        assert_eq!(result, "CREATE TABLE t (id BIGINT)");
    }

    #[test]
    fn test_create_table_with_lifecycle_case_insensitive() {
        let sql = "create table t (id bigint) lifecycle 365";
        let (result, noop) = translate_mc_sql(sql);
        assert!(!noop);
        assert_eq!(result, "create table t (id bigint)");
    }

    // ===== CREATE TABLE with STORED AS =====

    #[test]
    fn test_create_table_with_stored_as_orc() {
        let sql = "CREATE TABLE t (id BIGINT) STORED AS ORC";
        let (result, noop) = translate_mc_sql(sql);
        assert!(!noop);
        assert_eq!(result, "CREATE TABLE t (id BIGINT)");
    }

    #[test]
    fn test_create_table_with_stored_as_parquet() {
        let sql = "CREATE TABLE t (id BIGINT) STORED AS PARQUET";
        let (result, noop) = translate_mc_sql(sql);
        assert!(!noop);
        assert_eq!(result, "CREATE TABLE t (id BIGINT)");
    }

    // ===== CREATE TABLE with multiple clauses =====

    #[test]
    fn test_create_table_multiple_clauses() {
        let sql = "CREATE TABLE t (id BIGINT, name STRING) PARTITIONED BY (ds STRING) STORED AS ORC LIFECYCLE 365";
        let (result, noop) = translate_mc_sql(sql);
        assert!(!noop);
        // After processing: PARTITIONED BY adds ds to columns, STORED AS and LIFECYCLE are stripped
        assert_eq!(result, "CREATE TABLE t (id BIGINT, name STRING, ds STRING)");
    }

    // ===== INSERT OVERWRITE =====

    #[test]
    fn test_insert_overwrite_table() {
        let sql = "INSERT OVERWRITE TABLE t SELECT * FROM s";
        let (result, noop) = translate_mc_sql(sql);
        assert!(!noop);
        assert_eq!(result, "INSERT INTO t SELECT * FROM s");
    }

    #[test]
    fn test_insert_overwrite_without_table_keyword() {
        let sql = "INSERT OVERWRITE t VALUES (1, 'a')";
        let (result, noop) = translate_mc_sql(sql);
        assert!(!noop);
        assert_eq!(result, "INSERT INTO t VALUES (1, 'a')");
    }

    #[test]
    fn test_insert_overwrite_case_insensitive() {
        let sql = "insert overwrite table t select * from s";
        let (result, noop) = translate_mc_sql(sql);
        assert!(!noop);
        assert_eq!(result, "INSERT INTO t select * from s");
    }

    // ===== INSERT INTO with PARTITION clause =====

    #[test]
    fn test_insert_into_with_partition() {
        let sql = "INSERT INTO t PARTITION(ds='2024') VALUES (1, 'a')";
        let (result, noop) = translate_mc_sql(sql);
        assert!(!noop);
        assert_eq!(result, "INSERT INTO t VALUES (1, 'a')");
    }

    #[test]
    fn test_insert_overwrite_with_partition() {
        let sql = "INSERT OVERWRITE TABLE t PARTITION(ds='2024') SELECT * FROM s";
        let (result, noop) = translate_mc_sql(sql);
        assert!(!noop);
        // After overwrite->into and partition stripping
        assert_eq!(result, "INSERT INTO t SELECT * FROM s");
    }

    // ===== MAPJOIN hints =====

    #[test]
    fn test_select_mapjoin_hint() {
        let sql = "SELECT /*+ MAPJOIN(b) */ a.id, b.name FROM a JOIN b ON a.id = b.id";
        let (result, noop) = translate_mc_sql(sql);
        assert!(!noop);
        assert_eq!(result, "SELECT  a.id, b.name FROM a JOIN b ON a.id = b.id");
    }

    #[test]
    fn test_select_mapjoin_hint_no_spaces() {
        let sql = "SELECT /*+MAPJOIN(b)*/ a.id FROM a JOIN b ON a.id = b.id";
        let (result, noop) = translate_mc_sql(sql);
        assert!(!noop);
        assert_eq!(result, "SELECT  a.id FROM a JOIN b ON a.id = b.id");
    }

    #[test]
    fn test_select_multiple_hints() {
        let sql = "SELECT /*+ MAPJOIN(b) */ /*+ SKEWJOIN(c) */ a.id FROM a JOIN b JOIN c";
        let (result, noop) = translate_mc_sql(sql);
        assert!(!noop);
        assert_eq!(result, "SELECT   a.id FROM a JOIN b JOIN c");
    }

    // ===== SET / SETPROJECT noop =====

    #[test]
    fn test_set_statement_noop() {
        let sql = "SET odps.sql.allow.fullscan=true";
        let (result, noop) = translate_mc_sql(sql);
        assert!(noop);
        assert_eq!(result, "");
    }

    #[test]
    fn test_setproject_statement_noop() {
        let sql = "SETPROJECT myproject odps.sql.allow.fullscan=true";
        let (result, noop) = translate_mc_sql(sql);
        assert!(noop);
        assert_eq!(result, "");
    }

    #[test]
    fn test_add_jar_statement_noop() {
        let sql = "ADD JAR /path/to/udf.jar";
        let (result, noop) = translate_mc_sql(sql);
        assert!(noop);
        assert_eq!(result, "");
    }

    #[test]
    fn test_add_file_statement_noop() {
        let sql = "ADD FILE /path/to/resource.txt";
        let (result, noop) = translate_mc_sql(sql);
        assert!(noop);
        assert_eq!(result, "");
    }

    // ===== DISTRIBUTE BY / SORT BY =====

    #[test]
    fn test_distribute_by_sort_by() {
        let sql = "SELECT * FROM t DISTRIBUTE BY id SORT BY name";
        let (result, noop) = translate_mc_sql(sql);
        assert!(!noop);
        assert_eq!(result, "SELECT * FROM t ORDER BY name");
    }

    #[test]
    fn test_distribute_by_only() {
        let sql = "SELECT * FROM t DISTRIBUTE BY id";
        let (result, noop) = translate_mc_sql(sql);
        assert!(!noop);
        assert_eq!(result, "SELECT * FROM t");
    }

    // ===== CLUSTERED BY / SORTED BY =====

    #[test]
    fn test_clustered_by_sorted_by_into_buckets() {
        let sql = "CREATE TABLE t (id BIGINT, name STRING) CLUSTERED BY (id) SORTED BY (name) INTO 100 BUCKETS";
        let (result, noop) = translate_mc_sql(sql);
        assert!(!noop);
        assert_eq!(result, "CREATE TABLE t (id BIGINT, name STRING)");
    }

    #[test]
    fn test_clustered_by_only() {
        let sql = "CREATE TABLE t (id BIGINT) CLUSTERED BY (id) INTO 10 BUCKETS";
        let (result, noop) = translate_mc_sql(sql);
        assert!(!noop);
        assert_eq!(result, "CREATE TABLE t (id BIGINT)");
    }

    // ===== TBLPROPERTIES =====

    #[test]
    fn test_tblproperties() {
        let sql = r#"CREATE TABLE t (id BIGINT) TBLPROPERTIES ('comment'='test')"#;
        let (result, noop) = translate_mc_sql(sql);
        assert!(!noop);
        assert_eq!(result, "CREATE TABLE t (id BIGINT)");
    }

    #[test]
    fn test_tblproperties_with_nested_parens() {
        let sql = r#"CREATE TABLE t (id BIGINT) TBLPROPERTIES ('nested'='value(with)parens')"#;
        let (result, noop) = translate_mc_sql(sql);
        assert!(!noop);
        assert_eq!(result, "CREATE TABLE t (id BIGINT)");
    }

    // ===== Complex CREATE TABLE with all clauses =====

    #[test]
    fn test_complex_create_table_all_clauses() {
        let sql = "CREATE TABLE t (id BIGINT, name STRING) PARTITIONED BY (ds STRING) CLUSTERED BY (id) SORTED BY (name) INTO 100 BUCKETS STORED AS ORC LIFECYCLE 365 TBLPROPERTIES ('comment'='test')";
        let (result, noop) = translate_mc_sql(sql);
        assert!(!noop);
        assert_eq!(result, "CREATE TABLE t (id BIGINT, name STRING, ds STRING)");
    }

    // ===== Edge cases =====

    #[test]
    fn test_empty_sql() {
        let (result, noop) = translate_mc_sql("");
        assert!(noop);
        assert_eq!(result, "");
    }

    #[test]
    fn test_whitespace_only() {
        let (result, noop) = translate_mc_sql("   \t  \n  ");
        assert!(noop);
        assert_eq!(result, "");
    }

    #[test]
    fn test_sql_with_trailing_semicolon() {
        let sql = "CREATE TABLE t (id BIGINT, name STRING);";
        let (result, noop) = translate_mc_sql(sql);
        assert!(!noop);
        // Semicolons are stripped during processing, but the result should match the SQL without them
        assert_eq!(result, "CREATE TABLE t (id BIGINT, name STRING)");
    }

    #[test]
    fn test_select_preserved() {
        let sql = "SELECT * FROM t WHERE id = 1";
        let (result, noop) = translate_mc_sql(sql);
        assert!(!noop);
        assert_eq!(result, sql);
    }

    #[test]
    fn test_alter_table_statement_noop() {
        let sql = "ALTER TABLE t SET LIFECYCLE 365";
        let (result, noop) = translate_mc_sql(sql);
        assert!(noop);
        assert_eq!(result, "");
    }

    #[test]
    fn test_create_table_without_partitioned_by() {
        let sql = "CREATE TABLE t (id BIGINT, name STRING, ds STRING) STORED AS PARQUET LIFECYCLE 365";
        let (result, noop) = translate_mc_sql(sql);
        assert!(!noop);
        assert_eq!(result, "CREATE TABLE t (id BIGINT, name STRING, ds STRING)");
    }

    #[test]
    fn test_insert_into_normal_preserved() {
        let sql = "INSERT INTO t VALUES (1, 'a')";
        let (result, noop) = translate_mc_sql(sql);
        assert!(!noop);
        assert_eq!(result, sql);
    }

    #[test]
    fn test_select_mapjoin_skewjoin_hints() {
        let sql = "SELECT /*+ MAPJOIN(b) */ /*+ SKEWJOIN(c) */ a.id, b.name, c.val FROM a JOIN b JOIN c WHERE a.id = b.id AND b.id = c.id";
        let (result, noop) = translate_mc_sql(sql);
        assert!(!noop);
        assert_eq!(result, "SELECT   a.id, b.name, c.val FROM a JOIN b JOIN c WHERE a.id = b.id AND b.id = c.id");
    }

    #[test]
    fn test_multiple_distribute_by() {
        let sql = "SELECT * FROM t DISTRIBUTE BY id, name SORT BY age, city";
        let (result, noop) = translate_mc_sql(sql);
        assert!(!noop);
        assert_eq!(result, "SELECT * FROM t ORDER BY age, city");
    }

    #[test]
    fn test_strip_tblproperties_with_empty_parens() {
        let sql = "CREATE TABLE t (id BIGINT) TBLPROPERTIES ()";
        let (result, noop) = translate_mc_sql(sql);
        assert!(!noop);
        assert_eq!(result, "CREATE TABLE t (id BIGINT)");
    }

    #[test]
    fn test_lifecycle_after_stored_as() {
        let sql = "CREATE TABLE t (id BIGINT) STORED AS ORC LIFECYCLE 365";
        let (result, noop) = translate_mc_sql(sql);
        assert!(!noop);
        assert_eq!(result, "CREATE TABLE t (id BIGINT)");
    }

    #[test]
    fn test_insert_overwrite_multiple_times() {
        let sql = "INSERT OVERWRITE TABLE t1 SELECT * FROM t2; INSERT OVERWRITE TABLE t3 SELECT * FROM t4";
        // The translator operates on the full input
        let (result, noop) = translate_mc_sql(sql);
        assert!(!noop);
        assert!(result.contains("INSERT INTO t1"));
        assert!(result.contains("INSERT INTO t3"));
    }

    #[test]
    fn test_partitioned_by_complex_column_defs() {
        let sql = "CREATE TABLE t (id BIGINT) PARTITIONED BY (dt STRING COMMENT 'date field', region STRING COMMENT 'geo')";
        let (result, noop) = translate_mc_sql(sql);
        assert!(!noop);
        assert_eq!(
            result,
            "CREATE TABLE t (id BIGINT, dt STRING COMMENT 'date field', region STRING COMMENT 'geo')"
        );
    }

    #[test]
    fn test_partition_clause_with_nested_parens() {
        let sql = "INSERT INTO t PARTITION(ds='2024', category='(special)') VALUES (1)";
        let (result, noop) = translate_mc_sql(sql);
        assert!(!noop);
        assert_eq!(result, "INSERT INTO t VALUES (1)");
    }

    #[test]
    fn test_strip_mapjoin_and_clustered_by() {
        let sql = "SELECT /*+ MAPJOIN(b) */ a.id FROM a JOIN b";
        let (result, noop) = translate_mc_sql(sql);
        assert!(!noop);
        assert_eq!(result, "SELECT  a.id FROM a JOIN b");
    }

    #[test]
    fn test_plain_comment_preserved() {
        // Comments that are not hints should be preserved
        let sql = "SELECT * FROM t -- this is a comment";
        let (result, noop) = translate_mc_sql(sql);
        assert!(!noop);
        assert_eq!(result, "SELECT * FROM t -- this is a comment");
    }
}