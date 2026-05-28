//! Hologres SQL dialect translator.
//!
//! Translates Hologres (Alibaba Cloud PostgreSQL-compatible) SQL to
//! RorisDB-compatible SQL by:
//! - Stripping Hologres-specific DDL clauses (WITH table properties, etc.)
//! - Converting CALL set_table_property to no-op
//! - Stripping PARTITION BY LIST and PARTITION OF
//! - Handling pg_catalog table references
//! - Mapping types (TEXT -> STRING, TIMESTAMPTZ -> TIMESTAMP, etc.)
//! - Stripping ON CONFLICT from INSERT statements
//! - Converting EXPLAIN ANALYZE to EXPLAIN
//! - Reporting errors for unsupported features

use regex::Regex;

use crate::{DialectTranslator, TranslateResult};

/// Hologres SQL dialect translator.
pub struct HologresTranslator;

impl HologresTranslator {
    /// Create a new HologresTranslator.
    pub fn new() -> Self {
        Self
    }
}

impl Default for HologresTranslator {
    fn default() -> Self {
        Self::new()
    }
}

/// Find the position of a matching closing parenthesis, starting from `start`.
/// `start` should point to the opening `(` character.
/// Returns `None` if no matching parenthesis is found (unbalanced).
fn find_matching_paren(s: &str, start: usize) -> Option<usize> {
    let bytes = s.as_bytes();
    if start >= bytes.len() || bytes[start] != b'(' {
        return None;
    }
    let mut depth: u32 = 1;
    let mut i = start + 1;
    while i < bytes.len() && depth > 0 {
        match bytes[i] {
            b'(' => depth += 1,
            b')' => depth -= 1,
            _ => {}
        }
        if depth > 0 {
            i += 1;
        }
    }
    if depth == 0 { Some(i) } else { None }
}

/// Extract content within matching parentheses.
/// Returns the content (without the outer parens) and the end position.
fn extract_paren_content(s: &str, start: usize) -> Option<(&str, usize)> {
    let end = find_matching_paren(s, start)?;
    Some((&s[start + 1..end], end))
}

// ── No-op Detection ────────────────────────────────────────────────────

/// Check if the SQL is a CALL set_table_property statement (no-op).
fn is_set_table_property(sql: &str) -> bool {
    let re = Regex::new(r"(?i)^\s*CALL\s+set_table_property\s*\(").unwrap();
    re.is_match(sql)
}

/// Check if the SQL is a CREATE TABLE ... PARTITION OF statement (no-op).
fn is_create_partition_of(sql: &str) -> bool {
    let re = Regex::new(r"(?i)^\s*CREATE\s+TABLE\s+.+?\s+PARTITION\s+OF\s+").unwrap();
    re.is_match(sql)
}

/// Check if the SQL is a CREATE EXTENSION statement (no-op).
fn is_create_extension(sql: &str) -> bool {
    let re = Regex::new(r"(?i)^\s*CREATE\s+EXTENSION\b").unwrap();
    re.is_match(sql)
}

/// Check if the SQL is a LISTEN or NOTIFY statement (error).
fn is_listen_notify(sql: &str) -> bool {
    let listen_re = Regex::new(r"(?i)^\s*LISTEN\b").unwrap();
    let notify_re = Regex::new(r"(?i)^\s*NOTIFY\b").unwrap();
    listen_re.is_match(sql) || notify_re.is_match(sql)
}

// ── Unsupported Feature Detection ──────────────────────────────────────

/// Check for unsupported features and return an error result if found.
fn check_unsupported(sql: &str) -> Option<TranslateResult> {
    let trimmed = sql.trim();

    // CREATE TRIGGER
    if Regex::new(r"(?i)^\s*CREATE\s+TRIGGER\b").unwrap().is_match(trimmed) {
        return Some(TranslateResult::error("Hologres does not support triggers"));
    }

    // CREATE OR REPLACE FUNCTION ... LANGUAGE plpgsql
    if Regex::new(r"(?i)^\s*CREATE\s+(OR\s+REPLACE\s+)?FUNCTION\b").unwrap().is_match(trimmed) {
        return Some(TranslateResult::error(
            "CREATE FUNCTION is not supported by RorisDB in Phase 1",
        ));
    }

    // CREATE DOMAIN
    if Regex::new(r"(?i)^\s*CREATE\s+DOMAIN\b").unwrap().is_match(trimmed) {
        return Some(TranslateResult::error("CREATE DOMAIN is not supported by RorisDB"));
    }

    // WITH RECURSIVE
    if Regex::new(r"(?i)^\s*WITH\s+RECURSIVE\b").unwrap().is_match(trimmed) {
        return Some(TranslateResult::error(
            "Hologres does not support recursive CTE with RorisDB backend",
        ));
    }

    // SELECT ... FOR UPDATE
    if Regex::new(r"(?i)\bFOR\s+UPDATE\b").unwrap().is_match(trimmed) {
        return Some(TranslateResult::error(
            "Hologres does not support row-level locking (FOR UPDATE)",
        ));
    }

    // DISTINCT ON (col)
    if Regex::new(r"(?i)\bDISTINCT\s+ON\s*\(").unwrap().is_match(trimmed) {
        return Some(TranslateResult::error(
            "DISTINCT ON is not supported by RorisDB",
        ));
    }

    // LISTEN / NOTIFY
    if is_listen_notify(trimmed) {
        return Some(TranslateResult::error(
            "LISTEN/NOTIFY is not supported by RorisDB",
        ));
    }

    // JSON / JSONB type in DDL context
    if Regex::new(r"(?i)^\s*CREATE\s+TABLE\b").unwrap().is_match(trimmed) {
        let json_type_re = Regex::new(r"(?i)\bJSONB?\b").unwrap();
        if json_type_re.is_match(trimmed) {
            return Some(TranslateResult::error(
                "JSON/JSONB type is not supported in Phase 1",
            ));
        }

        // Array types: TEXT[], INT[], BIGINT[], etc. in column definitions
        let array_type_re = Regex::new(r"(?i)\b\w+\[\s*\]").unwrap();
        if array_type_re.is_match(trimmed) {
            return Some(TranslateResult::error(
                "Array types are not supported in Phase 1",
            ));
        }
    }

    None
}

// ── DDL Transformations ────────────────────────────────────────────────

/// Strip the WITH (orientation='column', ...) clause from CREATE TABLE.
fn strip_with_clause(sql: &str) -> (String, Vec<String>) {
    let mut warnings = Vec::new();
    let trimmed = sql.trim();

    let create_re = Regex::new(r"(?i)^\s*CREATE\s+TABLE\b").unwrap();
    if !create_re.is_match(trimmed) {
        return (trimmed.to_string(), Vec::new());
    }

    // Find the closing paren of column definitions, then look for WITH clause
    // Column defs start at the first '(' after the table name
    let after_create = create_re.find(trimmed).unwrap().end();
    let col_def_start = find_col_def_start(trimmed, after_create);
    let col_def_start = match col_def_start {
        Some(pos) => pos,
        None => return (trimmed.to_string(), Vec::new()),
    };

    let (_col_defs, col_def_end) = match extract_paren_content(trimmed, col_def_start) {
        Some(result) => result,
        None => return (trimmed.to_string(), Vec::new()),
    };

    let tail = &trimmed[col_def_end + 1..];

    // Look for WITH (...) clause
    let with_re = Regex::new(r"(?i)^\s*WITH\s*\(").unwrap();
    if let Some(with_match) = with_re.find(tail) {
        let with_start = with_match.start();
        // with_match covers "WITH (", so the '(' is at with_match.end()-1
        let paren_global = col_def_end + 1 + with_match.end() - 1;

        if let Some((_content, paren_end)) = extract_paren_content(trimmed, paren_global) {
            // Everything from the start of WITH to paren_end should be stripped
            let with_start_global = col_def_end + 1 + with_start;
            warnings.push(format!(
                "WITH table properties clause stripped: '{}'",
                &trimmed[with_start_global..paren_end + 1]
            ));
            let result = format!(
                "{}{}",
                &trimmed[..with_start_global],
                &trimmed[paren_end + 1..]
            );
            return (result.trim().to_string(), warnings);
        }
    }

    (trimmed.to_string(), warnings)
}

/// Find the position of the opening parenthesis for column definitions.
fn find_col_def_start(sql: &str, after_create: usize) -> Option<usize> {
    let rest = &sql[after_create..];
    let bytes = rest.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'(' {
            return Some(after_create + i);
        }
        if bytes[i] == b'`' {
            i += 1;
            while i < bytes.len() && bytes[i] != b'`' {
                i += 1;
            }
            i += 1;
        } else if bytes[i] == b'"' {
            i += 1;
            while i < bytes.len() && bytes[i] != b'"' {
                i += 1;
            }
            i += 1;
        } else {
            i += 1;
        }
    }
    None
}

/// Strip PARTITION BY LIST (...) and any subsequent partition definitions from CREATE TABLE.
fn strip_partition_by_list(sql: &str) -> (String, Vec<String>) {
    let mut warnings = Vec::new();
    let trimmed = sql.trim();

    let re = Regex::new(r"(?i)\s+PARTITION\s+BY\s+LIST\s*\(").unwrap();
    if let Some(part_match) = re.find(trimmed) {
        let paren_pos = part_match.start() + part_match.len() - 1; // position of '('
        if let Some((_content, paren_end)) = extract_paren_content(trimmed, paren_pos) {
            // After stripping PARTITION BY LIST (...), also strip any following
            // parenthesized partition definitions like (PARTITION p1 VALUES IN (...))
            let after = trimmed[paren_end + 1..].trim_start();
            let mut final_end = paren_end + 1;

            // Check if what follows starts with '(' (partition value definitions)
            if after.starts_with('(') {
                let global_paren_start = trimmed.len() - after.len();
                if let Some((_content, sub_paren_end)) = extract_paren_content(trimmed, global_paren_start) {
                    warnings.push(format!(
                        "PARTITION value definitions stripped: '{}'",
                        &trimmed[global_paren_start..sub_paren_end + 1]
                    ));
                    final_end = sub_paren_end + 1;
                }
            }

            warnings.push(format!(
                "PARTITION BY LIST clause stripped: '{}'",
                &trimmed[part_match.start()..final_end]
            ));
            let result = format!(
                "{}{}",
                &trimmed[..part_match.start()],
                &trimmed[final_end..]
            );
            return (result.trim().to_string(), warnings);
        }
    }

    (trimmed.to_string(), warnings)
}

// ── DML Transformations ────────────────────────────────────────────────

/// Strip ON CONFLICT clause from INSERT statements.
fn strip_on_conflict(sql: &str) -> (String, Vec<String>) {
    let mut warnings = Vec::new();
    let trimmed = sql.trim();

    let insert_re = Regex::new(r"(?i)^\s*INSERT\b").unwrap();
    if !insert_re.is_match(trimmed) {
        return (trimmed.to_string(), Vec::new());
    }

    // Find ON CONFLICT after the INSERT part
    let on_conflict_re = Regex::new(r"(?i)\s+ON\s+CONFLICT\b").unwrap();
    if let Some(m) = on_conflict_re.find(trimmed) {
        warnings.push(format!(
            "ON CONFLICT clause stripped: '{}'",
            &trimmed[m.start()..]
        ));
        let result = format!("{}{}", &trimmed[..m.start()], "");
        return (result.trim().to_string(), warnings);
    }

    (trimmed.to_string(), warnings)
}

/// Handle EXPLAIN ANALYZE -> EXPLAIN.
fn strip_explain_analyze(sql: &str) -> (String, Vec<String>) {
    let mut warnings = Vec::new();
    let trimmed = sql.trim();

    let re = Regex::new(r"(?i)^\s*EXPLAIN\s+ANALYZE\b").unwrap();
    if re.is_match(trimmed) {
        warnings.push("EXPLAIN ANALYZE simplified to EXPLAIN".to_string());
        let result = re.replace(trimmed, "EXPLAIN");
        return (result.to_string(), warnings);
    }

    (trimmed.to_string(), warnings)
}

// ── Type Mapping ───────────────────────────────────────────────────────

/// Map Hologres types to RorisDB types in column definitions.
fn map_types(sql: &str) -> String {
    let s = sql;

    // Order matters: more specific patterns first

    // TIMESTAMPTZ -> TIMESTAMP
    let re = Regex::new(r"(?i)\bTIMESTAMPTZ\b").unwrap();
    let s = re.replace_all(s, "TIMESTAMP");

    // BIGSERIAL -> BIGINT (must come before SERIAL)
    let re = Regex::new(r"(?i)\bBIGSERIAL\b").unwrap();
    let s = re.replace_all(&s, "BIGINT");

    // SERIAL -> INT
    let re = Regex::new(r"(?i)\bSERIAL\b").unwrap();
    let s = re.replace_all(&s, "INT");

    // TEXT -> STRING
    let re = Regex::new(r"(?i)\bTEXT\b").unwrap();
    let s = re.replace_all(&s, "STRING");

    // BYTEA -> BLOB
    let re = Regex::new(r"(?i)\bBYTEA\b").unwrap();
    let s = re.replace_all(&s, "BLOB");

    s.to_string()
}

// ── pg_catalog Translation ─────────────────────────────────────────────

/// Translate pg_catalog queries to RorisDB equivalents.
fn translate_pg_catalog(sql: &str) -> (String, Vec<String>) {
    let mut warnings = Vec::new();
    let trimmed = sql.trim();

    // Check if this is a pg_catalog query
    let pg_catalog_re = Regex::new(r"(?i)\bpg_catalog\b").unwrap();
    let pg_tables_re = Regex::new(r"(?i)\bpg_tables\b").unwrap();
    let pg_class_re = Regex::new(r"(?i)\bpg_class\b").unwrap();
    let pg_namespace_re = Regex::new(r"(?i)\bpg_namespace\b").unwrap();
    let pg_indexes_re = Regex::new(r"(?i)\bpg_indexes\b").unwrap();
    let pg_views_re = Regex::new(r"(?i)\bpg_views\b").unwrap();
    let pg_type_re = Regex::new(r"(?i)\bpg_type\b").unwrap();
    let pg_database_re = Regex::new(r"(?i)\bpg_database\b").unwrap();
    let pg_roles_re = Regex::new(r"(?i)\bpg_roles\b").unwrap();

    if !pg_catalog_re.is_match(trimmed)
        && !pg_tables_re.is_match(trimmed)
        && !pg_class_re.is_match(trimmed)
        && !pg_namespace_re.is_match(trimmed)
        && !pg_indexes_re.is_match(trimmed)
        && !pg_views_re.is_match(trimmed)
        && !pg_type_re.is_match(trimmed)
        && !pg_database_re.is_match(trimmed)
        && !pg_roles_re.is_match(trimmed)
    {
        // Not a pg_catalog query
        return (trimmed.to_string(), Vec::new());
    }

    // Handle simple pg_tables queries
    if pg_tables_re.is_match(trimmed) {
        let simple_select_re = Regex::new(r"(?i)^\s*SELECT\s+\*\s+FROM\s+pg_tables\s*;?\s*$").unwrap();
        if simple_select_re.is_match(trimmed) {
            warnings.push("pg_tables query translated to SHOW TABLES".to_string());
            return ("SHOW TABLES".to_string(), warnings);
        }

        // More complex pg_tables queries - try to map to information_schema
        warnings.push("pg_tables query translated to information_schema.tables query".to_string());
        let result = pg_tables_re.replace(trimmed, "information_schema.tables");
        return (result.to_string(), warnings);
    }

    // Handle simple pg_class queries
    if pg_class_re.is_match(trimmed) {
        let simple_select_re =
            Regex::new(r"(?i)^\s*SELECT\s+\*\s+FROM\s+pg_class\s*;?\s*$").unwrap();
        if simple_select_re.is_match(trimmed) {
            warnings.push(
                "pg_class query translated to information_schema.tables query".to_string(),
            );
            return (
                "SELECT * FROM information_schema.tables".to_string(),
                warnings,
            );
        }

        warnings.push("pg_class query translated to information_schema query".to_string());
        let result = pg_class_re.replace(trimmed, "information_schema.tables");
        return (result.to_string(), warnings);
    }

    // Handle other pg_catalog tables
    warnings.push(
        "pg_catalog query translated to information_schema equivalent (approximate)".to_string(),
    );

    // Map common pg_catalog tables to information_schema
    // For pg_type, pg_database, pg_roles - these don't have exact equivalents
    // We'll do our best with information_schema
    let result = trimmed.to_string()
        .replace("pg_catalog.", "information_schema.")
        .replace("pg_tables", "information_schema.tables")
        .replace("pg_class", "information_schema.tables")
        .replace("pg_namespace", "information_schema.schemata")
        .replace("pg_indexes", "information_schema.table_constraints")
        .replace("pg_views", "information_schema.views")
        .replace("pg_type", "information_schema.columns")
        .replace("pg_database", "information_schema.schemata")
        .replace("pg_roles", "information_schema.table_privileges")
        .replace("PG_CATALOG.", "INFORMATION_SCHEMA.");

    (result, warnings)
}

// ── Main Translation Logic ─────────────────────────────────────────────

impl DialectTranslator for HologresTranslator {
    fn translate(&self, sql: &str) -> TranslateResult {
        let trimmed = sql.trim();

        // Empty SQL
        if trimmed.is_empty() {
            return TranslateResult::ok(String::new());
        }

        // Remove trailing semicolons for processing
        let cleaned = trimmed.trim_end_matches(';').trim();

        // Check for no-op patterns
        if is_set_table_property(cleaned) {
            return TranslateResult::ok(String::new())
                .with_warning("CALL set_table_property is a no-op in RorisDB");
        }

        if is_create_partition_of(cleaned) {
            return TranslateResult::ok(String::new())
                .with_warning("CREATE TABLE ... PARTITION OF is ignored in RorisDB");
        }

        if is_create_extension(cleaned) {
            return TranslateResult::ok(String::new())
                .with_warning("CREATE EXTENSION is ignored in RorisDB");
        }

        // Check for unsupported features
        if let Some(err) = check_unsupported(cleaned) {
            return err;
        }

        let mut warnings: Vec<String> = Vec::new();
        let mut result = cleaned.to_string();

        // Step 1: Handle EXPLAIN ANALYZE -> EXPLAIN
        {
            let (r, w) = strip_explain_analyze(&result);
            result = r;
            warnings.extend(w);
        }

        // Step 2: Translate pg_catalog queries
        {
            let (r, w) = translate_pg_catalog(&result);
            result = r;
            warnings.extend(w);
        }

        // Step 3: Strip ON CONFLICT from INSERT
        {
            let (r, w) = strip_on_conflict(&result);
            result = r;
            warnings.extend(w);
        }

        // Step 4: Handle CREATE TABLE transformations
        if Regex::new(r"(?i)^\s*CREATE\s+TABLE\b").unwrap().is_match(&result) {
            // Strip WITH clause
            {
                let (r, w) = strip_with_clause(&result);
                result = r;
                warnings.extend(w);
            }

            // Strip PARTITION BY LIST
            {
                let (r, w) = strip_partition_by_list(&result);
                result = r;
                warnings.extend(w);
            }
        }

        // Step 5: Map types
        result = map_types(&result);

        // Step 6: Clean up extra whitespace
        result = result.trim().to_string();
        let multi_space = Regex::new(r"\s{2,}").unwrap();
        result = multi_space.replace_all(&result, " ").to_string();

        TranslateResult::ok(result).with_warnings(warnings)
    }

    fn dialect_name(&self) -> &str {
        "hologres"
    }

    fn unsupported_features(&self) -> &[&str] {
        &[
            "CREATE TRIGGER (Hologres does not support triggers)",
            "CREATE FUNCTION (not supported in Phase 1)",
            "CREATE DOMAIN (not supported)",
            "WITH RECURSIVE (recursive CTE not supported)",
            "SELECT ... FOR UPDATE (row-level locking not supported)",
            "DISTINCT ON (not supported)",
            "JSON / JSONB types (not supported in Phase 1)",
            "Array types INT[], TEXT[], etc. (not supported in Phase 1)",
            "LISTEN / NOTIFY (not supported)",
            "CALL set_table_property (silently ignored)",
            "CREATE EXTENSION (silently ignored)",
            "CREATE TABLE ... PARTITION OF (silently ignored)",
            "WITH table properties (silently stripped)",
            "PARTITION BY LIST (silently stripped)",
            "INSERT ... ON CONFLICT (ON CONFLICT stripped)",
            "EXPLAIN ANALYZE (simplified to EXPLAIN)",
            "pg_catalog queries (translated to information_schema approximately)",
        ]
    }
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn translator() -> HologresTranslator {
        HologresTranslator::new()
    }

    fn assert_translated(sql: &str, expected: &str) {
        let result = translator().translate(sql);
        assert!(result.success, "Translation failed: {:?}", result.error);
        assert_eq!(
            result.sql.trim(),
            expected.trim(),
            "SQL mismatch.\nInput:    {}\nExpected: {}\nGot:      {}",
            sql,
            expected,
            result.sql
        );
    }

    fn assert_error(sql: &str, expected_msg: &str) {
        let result = translator().translate(sql);
        assert!(!result.success, "Expected error but got success for: {}", sql);
        let err = result.error.as_ref().unwrap();
        assert!(
            err.contains(expected_msg),
            "Error message mismatch.\nExpected contains: {}\nGot: {}",
            expected_msg,
            err
        );
    }

    fn assert_noop(sql: &str) {
        let result = translator().translate(sql);
        assert!(
            result.success,
            "Expected no-op success but got error: {:?}",
            result.error
        );
        assert!(
            result.sql.is_empty() || result.sql.trim().is_empty(),
            "Expected empty/no-op SQL but got: '{}'",
            result.sql
        );
    }

    // ── find_matching_paren tests ──

    #[test]
    fn test_find_matching_paren_simple() {
        assert_eq!(find_matching_paren("(hello)", 0), Some(6));
    }

    #[test]
    fn test_find_matching_paren_nested() {
        assert_eq!(find_matching_paren("(a (b (c)) d)", 0), Some(12));
    }

    #[test]
    fn test_find_matching_paren_unbalanced() {
        assert_eq!(find_matching_paren("(a(b)", 0), None);
    }

    // ── Basic CREATE TABLE ──

    #[test]
    fn test_create_table_basic() {
        assert_translated(
            "CREATE TABLE t (col1 TEXT, col2 BIGINT)",
            "CREATE TABLE t (col1 STRING, col2 BIGINT)",
        );
    }

    #[test]
    fn test_create_table_if_not_exists() {
        assert_translated(
            "CREATE TABLE IF NOT EXISTS t (col1 TEXT)",
            "CREATE TABLE IF NOT EXISTS t (col1 STRING)",
        );
    }

    // ── CREATE TABLE with WITH clause ──

    #[test]
    fn test_create_table_with_clause() {
        let result = translator().translate(
            "CREATE TABLE t (col1 TEXT, col2 BIGINT) WITH (orientation='column', distribution_key='col1')",
        );
        assert!(result.success);
        assert_eq!(result.sql, "CREATE TABLE t (col1 STRING, col2 BIGINT)");
        assert!(
            result.warnings.iter().any(|w| w.contains("WITH")),
            "Expected warning about WITH clause, got: {:?}",
            result.warnings
        );
    }

    #[test]
    fn test_create_table_with_clause_bitmap() {
        let result = translator().translate(
            "CREATE TABLE t (col1 TEXT) WITH (orientation='column', bitmap_columns='col1')",
        );
        assert!(result.success);
        assert_eq!(result.sql, "CREATE TABLE t (col1 STRING)");
    }

    // ── CALL set_table_property (no-op) ──

    #[test]
    fn test_call_set_table_property() {
        assert_noop("CALL set_table_property('my_table', 'orientation', 'column')");
    }

    #[test]
    fn test_call_set_table_property_case_insensitive() {
        assert_noop("call SET_TABLE_PROPERTY('my_table', 'time_to_live_in_days', '90')");
    }

    // ── CREATE TABLE with PARTITION BY LIST ──

    #[test]
    fn test_create_table_partition_by_list() {
        let result = translator().translate(
            "CREATE TABLE t (col1 TEXT, col2 BIGINT) PARTITION BY LIST (col1)",
        );
        assert!(result.success);
        assert_eq!(result.sql, "CREATE TABLE t (col1 STRING, col2 BIGINT)");
        assert!(
            result.warnings.iter().any(|w| w.contains("PARTITION BY LIST")),
            "Expected warning about PARTITION BY LIST, got: {:?}",
            result.warnings
        );
    }

    #[test]
    fn test_create_table_partition_by_list_values() {
        let result = translator().translate(
            "CREATE TABLE t (col1 TEXT) PARTITION BY LIST (col1) (PARTITION p1 VALUES IN ('a', 'b'))",
        );
        assert!(result.success);
        // The PARTITION BY LIST and its sub-clause should be stripped
        assert_eq!(result.sql, "CREATE TABLE t (col1 STRING)");
    }

    // ── CREATE TABLE ... PARTITION OF (no-op) ──

    #[test]
    fn test_create_partition_of() {
        assert_noop(
            "CREATE TABLE child PARTITION OF parent FOR VALUES IN ('2024-01-01')",
        );
    }

    #[test]
    fn test_create_partition_of_with_default() {
        assert_noop(
            "CREATE TABLE child_default PARTITION OF parent DEFAULT",
        );
    }

    // ── CREATE EXTENSION (no-op) ──

    #[test]
    fn test_create_extension() {
        assert_noop("CREATE EXTENSION postgis");
    }

    #[test]
    fn test_create_extension_if_not_exists() {
        assert_noop("CREATE EXTENSION IF NOT EXISTS postgis");
    }

    // ── Type Mapping ──

    #[test]
    fn test_text_to_string() {
        assert_translated(
            "CREATE TABLE t (col1 TEXT)",
            "CREATE TABLE t (col1 STRING)",
        );
    }

    #[test]
    fn test_timestamptz_to_timestamp() {
        assert_translated(
            "CREATE TABLE t (col1 TIMESTAMPTZ)",
            "CREATE TABLE t (col1 TIMESTAMP)",
        );
    }

    #[test]
    fn test_serial_to_int() {
        assert_translated(
            "CREATE TABLE t (col1 SERIAL)",
            "CREATE TABLE t (col1 INT)",
        );
    }

    #[test]
    fn test_bigserial_to_bigint() {
        assert_translated(
            "CREATE TABLE t (col1 BIGSERIAL)",
            "CREATE TABLE t (col1 BIGINT)",
        );
    }

    #[test]
    fn test_bytea_to_blob() {
        assert_translated(
            "CREATE TABLE t (col1 BYTEA)",
            "CREATE TABLE t (col1 BLOB)",
        );
    }

    #[test]
    fn test_all_type_mappings() {
        assert_translated(
            "CREATE TABLE t (a TEXT, b TIMESTAMPTZ, c SERIAL, d BIGSERIAL, e BYTEA)",
            "CREATE TABLE t (a STRING, b TIMESTAMP, c INT, d BIGINT, e BLOB)",
        );
    }

    #[test]
    fn test_timestamp_no_change() {
        assert_translated(
            "CREATE TABLE t (col1 TIMESTAMP)",
            "CREATE TABLE t (col1 TIMESTAMP)",
        );
    }

    #[test]
    fn test_int_no_change() {
        assert_translated(
            "CREATE TABLE t (col1 INT, col2 BIGINT, col3 SMALLINT, col4 TINYINT)",
            "CREATE TABLE t (col1 INT, col2 BIGINT, col3 SMALLINT, col4 TINYINT)",
        );
    }

    #[test]
    fn test_boolean_no_change() {
        assert_translated(
            "CREATE TABLE t (col1 BOOLEAN)",
            "CREATE TABLE t (col1 BOOLEAN)",
        );
    }

    // ── JSON/JSONB/Array type errors ──

    #[test]
    fn test_json_type_error() {
        assert_error(
            "CREATE TABLE t (col1 JSON)",
            "JSON/JSONB type",
        );
    }

    #[test]
    fn test_jsonb_type_error() {
        assert_error(
            "CREATE TABLE t (col1 JSONB)",
            "JSON/JSONB type",
        );
    }

    #[test]
    fn test_array_type_error() {
        assert_error(
            "CREATE TABLE t (col1 TEXT[])",
            "Array types",
        );
    }

    #[test]
    fn test_int_array_type_error() {
        assert_error(
            "CREATE TABLE t (col1 INT[])",
            "Array types",
        );
    }

    // ── Unsupported Features (errors) ──

    #[test]
    fn test_create_trigger_error() {
        assert_error(
            "CREATE TRIGGER update_trigger BEFORE UPDATE ON t FOR EACH ROW EXECUTE FUNCTION f()",
            "triggers",
        );
    }

    #[test]
    fn test_create_function_error() {
        assert_error(
            "CREATE OR REPLACE FUNCTION add(a INT, b INT) RETURNS INT LANGUAGE plpgsql AS $$ BEGIN RETURN a + b; END; $$",
            "CREATE FUNCTION",
        );
    }

    #[test]
    fn test_create_domain_error() {
        assert_error(
            "CREATE DOMAIN positive_int AS INT CHECK (VALUE > 0)",
            "CREATE DOMAIN",
        );
    }

    #[test]
    fn test_with_recursive_error() {
        assert_error(
            "WITH RECURSIVE cte AS (SELECT 1 AS n UNION ALL SELECT n + 1 FROM cte WHERE n < 10) SELECT * FROM cte",
            "recursive CTE",
        );
    }

    #[test]
    fn test_for_update_error() {
        assert_error(
            "SELECT * FROM t WHERE id = 1 FOR UPDATE",
            "row-level locking",
        );
    }

    #[test]
    fn test_distinct_on_error() {
        assert_error(
            "SELECT DISTINCT ON (col1) col1, col2 FROM t ORDER BY col1, col2",
            "DISTINCT ON",
        );
    }

    #[test]
    fn test_listen_error() {
        assert_error("LISTEN my_channel", "LISTEN/NOTIFY");
    }

    #[test]
    fn test_notify_error() {
        assert_error("NOTIFY my_channel, 'hello'", "LISTEN/NOTIFY");
    }

    // ── EXPLAIN ANALYZE → EXPLAIN ──

    #[test]
    fn test_explain_analyze() {
        let result = translator().translate("EXPLAIN ANALYZE SELECT * FROM t");
        assert!(result.success);
        assert_eq!(result.sql, "EXPLAIN SELECT * FROM t");
        assert!(
            result.warnings.iter().any(|w| w.contains("EXPLAIN ANALYZE")),
            "Expected warning about EXPLAIN ANALYZE, got: {:?}",
            result.warnings
        );
    }

    #[test]
    fn test_explain_without_analyze() {
        assert_translated("EXPLAIN SELECT * FROM t", "EXPLAIN SELECT * FROM t");
    }

    // ── INSERT ... ON CONFLICT ──

    #[test]
    fn test_insert_on_conflict_do_nothing() {
        let result = translator().translate(
            "INSERT INTO t VALUES (1, 'a') ON CONFLICT DO NOTHING",
        );
        assert!(result.success);
        assert_eq!(result.sql, "INSERT INTO t VALUES (1, 'a')");
        assert!(
            result.warnings.iter().any(|w| w.contains("ON CONFLICT")),
            "Expected warning about ON CONFLICT, got: {:?}",
            result.warnings
        );
    }

    #[test]
    fn test_insert_on_conflict_do_update() {
        let result = translator().translate(
            "INSERT INTO t VALUES (1, 'a') ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name",
        );
        assert!(result.success);
        assert_eq!(result.sql, "INSERT INTO t VALUES (1, 'a')");
    }

    #[test]
    fn test_insert_without_conflict() {
        assert_translated(
            "INSERT INTO t VALUES (1, 'a')",
            "INSERT INTO t VALUES (1, 'a')",
        );
    }

    // ── pg_catalog Translation ──

    #[test]
    fn test_select_from_pg_tables() {
        let result = translator().translate("SELECT * FROM pg_tables");
        assert!(result.success);
        assert_eq!(result.sql, "SHOW TABLES");
        assert!(
            result.warnings.iter().any(|w| w.contains("pg_tables")),
            "Expected warning about pg_tables, got: {:?}",
            result.warnings
        );
    }

    #[test]
    fn test_select_from_pg_class() {
        let result = translator().translate("SELECT * FROM pg_class");
        assert!(result.success);
        assert_eq!(result.sql, "SELECT * FROM information_schema.tables");
    }

    #[test]
    fn test_select_from_pg_catalog_tables() {
        let result = translator().translate("SELECT * FROM pg_catalog.pg_tables");
        assert!(result.success);
        assert!(result.sql.contains("SHOW TABLES") || result.sql.contains("information_schema"));
    }

    #[test]
    fn test_select_from_pg_catalog_class() {
        let result = translator().translate(
            "SELECT relname FROM pg_catalog.pg_class WHERE relkind = 'r'",
        );
        assert!(result.success);
        assert!(result.sql.contains("information_schema"));
    }

    #[test]
    fn test_select_from_pg_namespace() {
        let result = translator().translate("SELECT nspname FROM pg_namespace");
        assert!(result.success);
        assert!(result.sql.contains("information_schema.schemata"));
    }

    // ── Case Insensitivity ──

    #[test]
    fn test_case_insensitive_create() {
        let result = translator().translate(
            "create table t (col1 text) with (orientation='column')",
        );
        assert!(result.success);
        // Type names are normalized to uppercase by the type mapper
        assert_eq!(result.sql, "create table t (col1 STRING)");
    }

    #[test]
    fn test_case_insensitive_type_mapping() {
        assert_translated(
            "CREATE TABLE t (col1 Text, col2 TeXt)",
            "CREATE TABLE t (col1 STRING, col2 STRING)",
        );
    }

    #[test]
    fn test_case_insensitive_on_conflict() {
        let result = translator().translate(
            "insert into t values (1) on conflict do nothing",
        );
        assert!(result.success);
        assert_eq!(result.sql, "insert into t values (1)");
    }

    // ── Edge Cases ──

    #[test]
    fn test_empty_sql() {
        let result = translator().translate("");
        assert!(result.success);
        assert_eq!(result.sql, "");
    }

    #[test]
    fn test_whitespace_sql() {
        let result = translator().translate("   ");
        assert!(result.success);
        assert_eq!(result.sql, "");
    }

    #[test]
    fn test_simple_select() {
        assert_translated("SELECT * FROM t", "SELECT * FROM t");
    }

    #[test]
    fn test_select_with_where() {
        assert_translated(
            "SELECT col1, col2 FROM t WHERE col1 > 10",
            "SELECT col1, col2 FROM t WHERE col1 > 10",
        );
    }

    #[test]
    fn test_create_database() {
        assert_translated("CREATE DATABASE db", "CREATE DATABASE db");
    }

    #[test]
    fn test_drop_table() {
        assert_translated("DROP TABLE t", "DROP TABLE t");
    }

    #[test]
    fn test_show_tables() {
        assert_translated("SHOW TABLES", "SHOW TABLES");
    }

    #[test]
    fn test_select_with_join() {
        assert_translated(
            "SELECT a.*, b.name FROM t a JOIN t2 b ON a.id = b.id",
            "SELECT a.*, b.name FROM t a JOIN t2 b ON a.id = b.id",
        );
    }

    #[test]
    fn test_select_with_group_by() {
        assert_translated(
            "SELECT col1, COUNT(*) FROM t GROUP BY col1",
            "SELECT col1, COUNT(*) FROM t GROUP BY col1",
        );
    }

    #[test]
    fn test_insert_into_values_direct() {
        assert_translated(
            "INSERT INTO t VALUES (1, 'a'), (2, 'b')",
            "INSERT INTO t VALUES (1, 'a'), (2, 'b')",
        );
    }

    #[test]
    fn test_select_with_subquery() {
        assert_translated(
            "SELECT * FROM (SELECT id, name FROM users WHERE active = 1) sub",
            "SELECT * FROM (SELECT id, name FROM users WHERE active = 1) sub",
        );
    }

    #[test]
    fn test_create_table_with_comment() {
        assert_translated(
            "CREATE TABLE t (col1 TEXT COMMENT 'a column')",
            "CREATE TABLE t (col1 STRING COMMENT 'a column')",
        );
    }

    #[test]
    fn test_decimal_type_no_change() {
        assert_translated(
            "CREATE TABLE t (col1 DECIMAL, col2 DECIMAL(10,2))",
            "CREATE TABLE t (col1 DECIMAL, col2 DECIMAL(10,2))",
        );
    }

    #[test]
    fn test_float_double_no_change() {
        assert_translated(
            "CREATE TABLE t (col1 FLOAT, col2 DOUBLE PRECISION)",
            "CREATE TABLE t (col1 FLOAT, col2 DOUBLE PRECISION)",
        );
    }

    #[test]
    fn test_create_table_with_position_and_not_null() {
        assert_translated(
            "CREATE TABLE t (col1 BIGSERIAL NOT NULL, col2 TEXT)",
            "CREATE TABLE t (col1 BIGINT NOT NULL, col2 STRING)",
        );
    }

    #[test]
    fn test_create_table_with_default() {
        assert_translated(
            "CREATE TABLE t (col1 SERIAL DEFAULT 1, col2 TEXT DEFAULT 'hello')",
            "CREATE TABLE t (col1 INT DEFAULT 1, col2 STRING DEFAULT 'hello')",
        );
    }

    #[test]
    fn test_select_with_text_function() {
        // Ensure TEXT in function names or identifiers isn't replaced
        assert_translated(
            "SELECT text_col, LENGTH(text_col) FROM t",
            "SELECT text_col, LENGTH(text_col) FROM t",
        );
    }

    #[test]
    fn test_alter_table_no_change() {
        assert_translated(
            "ALTER TABLE t ADD COLUMN col1 TEXT",
            "ALTER TABLE t ADD COLUMN col1 STRING",
        );
    }

    #[test]
    fn test_create_table_empty_cols_with_with_clause() {
        let result = translator().translate(
            "CREATE TABLE t (id BIGINT) WITH (orientation='column')",
        );
        assert!(result.success);
        assert_eq!(result.sql, "CREATE TABLE t (id BIGINT)");
    }

    #[test]
    fn test_multiple_type_mappings_in_one_column() {
        // Types like BIGSERIAL might appear as part of types
        assert_translated(
            "CREATE TABLE t (a BIGSERIAL NOT NULL DEFAULT nextval('seq'))",
            "CREATE TABLE t (a BIGINT NOT NULL DEFAULT nextval('seq'))",
        );
    }

    #[test]
    fn test_update_with_text_column() {
        // Non-DDL statement shouldn't be affected by CREATE TABLE checks
        assert_translated(
            "UPDATE t SET col1 = 'hello' WHERE id = 1",
            "UPDATE t SET col1 = 'hello' WHERE id = 1",
        );
    }

    #[test]
    fn test_delete_with_condition() {
        assert_translated(
            "DELETE FROM t WHERE id = 1",
            "DELETE FROM t WHERE id = 1",
        );
    }

    #[test]
    fn test_select_with_cte() {
        // Non-recursive CTE should pass through
        assert_translated(
            "WITH cte AS (SELECT 1 AS n) SELECT * FROM cte",
            "WITH cte AS (SELECT 1 AS n) SELECT * FROM cte",
        );
    }

    #[test]
    fn test_quoted_identifiers() {
        assert_translated(
            r#"CREATE TABLE "my_table" ("col1" TEXT, "col2" BIGINT)"#,
            r#"CREATE TABLE "my_table" ("col1" STRING, "col2" BIGINT)"#,
        );
    }

    #[test]
    fn test_create_table_single_column() {
        assert_translated(
            "CREATE TABLE t (a TEXT)",
            "CREATE TABLE t (a STRING)",
        );
    }

    #[test]
    fn test_semicolon_handling() {
        let result = translator().translate("SELECT * FROM t;");
        assert!(result.success);
        assert_eq!(result.sql, "SELECT * FROM t");
    }
}