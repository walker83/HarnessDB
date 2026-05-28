//! MaxCompute (ODPS) SQL dialect translator.
//!
//! Translates MaxCompute SQL to RorisDB-compatible SQL by:
//! - Stripping MaxCompute-specific DDL clauses (PARTITIONED BY, LIFECYCLE, STORED AS, etc.)
//! - Converting INSERT OVERWRITE to INSERT INTO
//! - Removing MAPJOIN/SKEWJOIN optimizer hints
//! - Converting DISTRIBUTE BY + SORT BY to ORDER BY
//! - Handling SET/SETPROJECT as no-ops
//! - Mapping types (STRING(n) -> VARCHAR(n))
//! - Reporting errors for unsupported features

use regex::Regex;

use crate::{DialectTranslator, TranslateResult};

/// MaxCompute SQL dialect translator.
pub struct MaxComputeTranslator;

impl MaxComputeTranslator {
    /// Create a new MaxComputeTranslator.
    pub fn new() -> Self {
        Self
    }
}

impl Default for MaxComputeTranslator {
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

// ── DDL Transformations ────────────────────────────────────────────────

/// Strip `STORED AS ...` from the tail of a CREATE TABLE statement.
fn strip_stored_as(sql: &str) -> (String, Vec<String>) {
    let mut warnings = Vec::new();
    let re = Regex::new(r"(?i)\s+STORED\s+AS\s+\w+").unwrap();
    let result = re.replace_all(sql, |caps: &regex::Captures| {
        let cap = caps.get(0).map(|m| m.as_str()).unwrap_or("");
        warnings.push(format!("STORED AS clause stripped: '{}'", cap.trim()));
        ""
    });
    (result.to_string(), warnings)
}

/// Strip `LIFECYCLE N` from the tail.
fn strip_lifecycle(sql: &str) -> (String, Vec<String>) {
    let mut warnings = Vec::new();
    let re = Regex::new(r"(?i)\s+LIFECYCLE\s+\d+").unwrap();
    let result = re.replace_all(sql, |caps: &regex::Captures| {
        let cap = caps.get(0).map(|m| m.as_str()).unwrap_or("");
        warnings.push(format!("LIFECYCLE clause stripped: '{}', not enforced", cap.trim()));
        ""
    });
    (result.to_string(), warnings)
}

/// Strip `CLUSTERED BY (...) INTO N BUCKETS` from the tail.
fn strip_clustered_by(sql: &str) -> (String, Vec<String>) {
    let mut warnings = Vec::new();
    let s = sql.trim_end();

    // Match: CLUSTERED BY (...) [INTO N BUCKETS] [ONLY]
    let re = Regex::new(r"(?i)\s+CLUSTERED\s+BY\s*\(").unwrap();
    let mut result = s.to_string();
    while let Some(cap) = re.find(&result) {
        let start = cap.start();
        let paren_start = cap.end() - 1; // position of '('
        if let Some((_content, paren_end)) = extract_paren_content(&result, paren_start) {
            let tail_after_paren = &result[paren_end + 1..];
            // Check for INTO N BUCKETS
            let bucket_re = Regex::new(r"(?i)^\s+INTO\s+\d+\s+BUCKETS\b").unwrap();
            let end_pos = if let Some(bucket_match) = bucket_re.find(tail_after_paren) {
                paren_end + 1 + bucket_match.end()
            } else {
                paren_end + 1
            };
            warnings.push(format!(
                "CLUSTERED BY clause stripped: '{}'",
                result[start..end_pos].trim()
            ));
            result = format!("{}{}", &result[..start], &result[end_pos..]);
        } else {
            // Unbalanced, can't strip safely
            break;
        }
    }

    (result, warnings)
}

/// Strip `TBLPROPERTIES (...)` from the tail.
fn strip_tblproperties(sql: &str) -> (String, Vec<String>) {
    let mut warnings = Vec::new();
    let s = sql.trim_end();

    let re = Regex::new(r"(?i)\s+TBLPROPERTIES\s*\(").unwrap();
    let mut result = s.to_string();
    while let Some(cap) = re.find(&result) {
        let start = cap.start();
        let paren_start = cap.end() - 1; // position of '('
        if let Some((_content, paren_end)) = extract_paren_content(&result, paren_start) {
            warnings.push(format!(
                "TBLPROPERTIES clause stripped: '{}'",
                result[start..paren_end + 1].trim()
            ));
            result = format!("{}{}", &result[..start], &result[paren_end + 1..]);
        } else {
            break;
        }
    }

    (result, warnings)
}

/// Extract columns from a `PARTITIONED BY (col1 TYPE, col2 TYPE, ...)` clause.
/// Returns the column definition string and the end position after the closing paren.
fn extract_partition_columns(sql: &str, start: usize) -> Option<(String, usize)> {
    let paren_start = sql[start..].find('(')? + start;
    let (content, paren_end) = extract_paren_content(sql, paren_start)?;

    // Parse individual column definitions separated by commas, respecting nested parens
    let cols = split_by_commas_outside_parens(content);

    if cols.is_empty() {
        return None;
    }

    let partition_cols: Vec<String> = cols
        .iter()
        .map(|c| c.trim().to_string())
        .collect();

    Some((partition_cols.join(", "), paren_end))
}

/// Split a SQL fragment by commas that are not inside parentheses.
fn split_by_commas_outside_parens(s: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth: u32 = 0;
    let mut last = 0;

    for (i, ch) in s.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                parts.push(&s[last..i]);
                last = i + 1;
            }
            _ => {}
        }
    }
    if last <= s.len() {
        parts.push(&s[last..]);
    }
    parts
}

/// Handle the PARTITIONED BY clause and other DDL clauses in CREATE TABLE.
fn process_create_table(sql: &str) -> (String, Vec<String>) {
    let trimmed = sql.trim();

    // Must start with CREATE TABLE
    let create_re = Regex::new(r"(?i)^\s*CREATE\s+TABLE\b").unwrap();
    if !create_re.is_match(trimmed) {
        return (trimmed.to_string(), Vec::new());
    }

    let mut warnings: Vec<String> = Vec::new();

    // Find the first opening paren that starts the column definition block.
    // We need to skip past "CREATE TABLE [IF NOT EXISTS] [db.]name"
    let after_create = create_re.find(trimmed).unwrap().end();
    // The column defs start with the first '(' that is not part of a keyword
    let col_def_start = find_col_def_start(trimmed, after_create);
    let col_def_start = match col_def_start {
        Some(pos) => pos,
        None => return (trimmed.to_string(), Vec::new()),
    };

    // Extract column definitions
    let (_col_defs_raw, col_def_end) = match extract_paren_content(trimmed, col_def_start) {
        Some(result) => result,
        None => return (trimmed.to_string(), Vec::new()),
    };

    // The rest after column definitions
    let tail = &trimmed[col_def_end + 1..];

    // Now check for PARTITIONED BY in the tail
    let partition_re = Regex::new(r"(?i)\s*PARTITIONED\s+BY\s*\(").unwrap();

    if let Some(part_match) = partition_re.find(tail) {
        let part_start = part_match.start();
        // Find the opening paren of PARTITIONED BY
        let paren_start_in_tail = tail[part_start..].find('(').unwrap() + part_start;
        if let Some((partition_cols, part_end)) =
            extract_partition_columns(tail, paren_start_in_tail)
        {
            let part_clause_end = part_end + 1; // +1 to include the closing paren offset from extract_paren_content

            // Now we need the correct position in the original trimmed string.
            // part_end is relative to tail, but we need the position of the closing paren in trimmed.
            let part_clause_end_global = col_def_end + 1 + part_clause_end;

            // Reconstruct: everything before partition, partition columns added
            let after_partition = &trimmed[part_clause_end_global..];

            // Build new column defs with partition columns
            let new_col_defs = &trimmed[col_def_start + 1..col_def_end];
            let partition_cols_trimmed = partition_cols.trim().to_string();
            let new_cols = if new_col_defs.trim().is_empty() {
                partition_cols_trimmed.to_string()
            } else if partition_cols_trimmed.is_empty() {
                new_col_defs.trim().to_string()
            } else {
                format!("{}, {}", new_col_defs.trim(), partition_cols_trimmed)
            };

            let new_sql = format!(
                "{} ({}){}",
                &trimmed[..col_def_start],
                new_cols,
                after_partition
            );

            warnings.push("PARTITIONED BY columns merged into regular column definitions".to_string());

            // Now strip rest of partition related clauses from the tail
            // The tail now doesn't have PARTITIONED BY anymore since we reconstructed
            // But we need to update trimmed to the new SQL

            let mut result = new_sql.trim().to_string();
            // Continue stripping other clauses from the result
            let (r, w) = strip_lifecycle(&result);
            result = r;
            warnings.extend(w);

            let (r2, w2) = strip_stored_as(&result);
            result = r2;
            warnings.extend(w2);

            let (r3, w3) = strip_clustered_by(&result);
            result = r3;
            warnings.extend(w3);

            let (r4, w4) = strip_tblproperties(&result);
            result = r4;
            warnings.extend(w4);

            return (result.trim().to_string(), warnings);
        }
    }

    // No PARTITIONED BY, just strip other clauses
    let mut result = trimmed.to_string();

    // Strip COMMENT on table (keep column comments in the column defs)
    // Actually, let's not strip table COMMENT - keep it if RorisDB supports it
    // The spec says "Handle COMMENT 'text' on columns and tables (keep if RorisDB supports, else strip)"

    let (r1, w1) = strip_lifecycle(&result);
    result = r1;
    warnings.extend(w1);

    let (r2, w2) = strip_stored_as(&result);
    result = r2;
    warnings.extend(w2);

    let (r3, w3) = strip_clustered_by(&result);
    result = r3;
    warnings.extend(w3);

    let (r4, w4) = strip_tblproperties(&result);
    result = r4;
    warnings.extend(w4);

    (result.trim().to_string(), warnings)
}

/// Find the position of the opening parenthesis for column definitions
/// in a CREATE TABLE statement.
fn find_col_def_start(sql: &str, after_create: usize) -> Option<usize> {
    let rest = &sql[after_create..];
    // Skip past: [IF NOT EXISTS] [db.]table_name
    // Look for first '(' that is not inside a string or comment
    let mut i = 0;
    let bytes = rest.as_bytes();
    while i < bytes.len() {
        if bytes[i] == b'(' {
            return Some(after_create + i);
        }
        // Skip past identifiers (they could contain db.table patterns)
        if bytes[i] == b'`' {
            // Quoted identifier
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

/// Handle INSERT OVERWRITE and INSERT INTO with PARTITION.
fn translate_insert(sql: &str) -> (String, Vec<String>) {
    let mut warnings = Vec::new();
    let trimmed = sql.trim();

    // INSERT OVERWRITE [TABLE] t ... -> INSERT INTO [TABLE] t ...
    let overwrite_re = Regex::new(r"(?i)^(\s*INSERT\s+)OVERWRITE(\s+(?:TABLE\s+)?)").unwrap();
    let after_overwrite = if let Some(caps) = overwrite_re.captures(trimmed) {
        warnings.push("INSERT OVERWRITE converted to INSERT INTO".to_string());
        // Get the full match end position to preserve the rest of the SQL
        let full_match_end = caps.get(0).unwrap().end();
        format!("{}INTO{}{}", &caps[1], &caps[2], &trimmed[full_match_end..])
    } else {
        trimmed.to_string()
    };

    let mut result = after_overwrite.to_string();

    // Handle INSERT TABLE t PARTITION(ds='2024') -> strip PARTITION clause
    // Need to handle the matching paren for PARTITION(...)
    let partition_re = Regex::new(r"(?i)\s+PARTITION\s*\(").unwrap();
    let s = result.clone();
    if let Some(part_match) = partition_re.find(&s) {
        // Find the opening '(' after PARTITION
        let rest = &s[part_match.start()..];
        let paren_pos_in_rest = rest.find('(').unwrap();
        let global_paren_start = part_match.start() + paren_pos_in_rest;

        if let Some((_content, paren_end)) = extract_paren_content(&s, global_paren_start) {
            let part_end_in_s = paren_end + 1; // after closing paren
            warnings.push(format!(
                "PARTITION clause stripped: '{}'",
                &s[part_match.start()..part_end_in_s]
            ));
            result = format!("{}{}", &s[..part_match.start()], &s[part_end_in_s..]);
        }
    }

    (result.trim().to_string(), warnings)
}

/// Strip MAPJOIN and SKEWJOIN optimizer hints from SQL.
fn strip_hints(sql: &str) -> (String, Vec<String>) {
    let mut warnings = Vec::new();
    let mapjoin_re = Regex::new(r"(?i)/\*\+[^*]*(?:MAPJOIN|SKEWJOIN)[^*]*\*/").unwrap();
    let result = mapjoin_re.replace_all(sql, |_caps: &regex::Captures| {
        warnings.push("MAPJOIN/SKEWJOIN hint stripped".to_string());
        ""
    });
    (result.to_string(), warnings)
}

/// Convert `DISTRIBUTE BY col SORT BY col` to `ORDER BY col`.
fn translate_distribute_sort(sql: &str) -> (String, Vec<String>) {
    let mut warnings = Vec::new();
    let distribute_re = Regex::new(r"(?i)\bDISTRIBUTE\s+BY\b").unwrap();
    let sort_re = Regex::new(r"(?i)\bSORT\s+BY\b").unwrap();

    let s = sql;
    if let Some(dist_pos) = distribute_re.find(s) {
        if let Some(sort_pos) = sort_re.find(s) {
            if sort_pos.start() > dist_pos.end() {
                // Extract the SORT BY columns
                let after_sort = &s[sort_pos.end()..];
                // Find end of SORT BY columns: up to next keyword like FROM, WHERE, GROUP, LIMIT, etc.
                let end_keywords = [
                    " FROM ", " WHERE ", " GROUP ", " HAVING ", " ORDER ", " LIMIT ",
                    " UNION ", " INTERSECT ", " EXCEPT ", ";",
                ];
                let mut sort_end = s.len();
                for kw in &end_keywords {
                    if let Some(pos) = after_sort.to_uppercase().find(&kw.to_uppercase()) {
                        let candidate = sort_pos.end() + pos;
                        if candidate < sort_end {
                            sort_end = candidate;
                        }
                    }
                }
                let sort_cols = &s[sort_pos.end()..sort_end].trim();

                // Replace DISTRIBUTE BY ... SORT BY col with ORDER BY col
                warnings.push(
                    "DISTRIBUTE BY + SORT BY converted to ORDER BY".to_string(),
                );

                // The content between DISTRIBUTE BY's columns and SORT BY is the distribute columns.
                let distribute_cols = &s[dist_pos.end()..sort_pos.start()].trim();
                warnings.push(format!(
                    "DISTRIBUTE BY columns '{}' dropped, using SORT BY columns for ORDER BY",
                    distribute_cols
                ));

                let final_sql = format!(
                    "{} ORDER BY {}{}",
                    &s[..dist_pos.start()],
                    sort_cols,
                    &s[sort_end..]
                );
                return (final_sql.trim().to_string(), warnings);
            }
        }
    }

    (s.to_string(), warnings)
}

/// Handle SET/SETPROJECT statements (no-op).
fn is_noop_set_statement(sql: &str) -> bool {
    let trimmed = sql.trim();
    let set_re = Regex::new(r"(?i)^\s*SET\s+odps\.\S+\s*=\s*\S*\s*;?\s*$").unwrap();
    let setproject_re = Regex::new(r"(?i)^\s*SETPROJECT\s+\S+\s*=\s*\S*\s*;?\s*$").unwrap();
    set_re.is_match(trimmed) || setproject_re.is_match(trimmed)
}

/// Check for unsupported features and return an error if found.
fn check_unsupported(sql: &str) -> Option<TranslateResult> {
    let trimmed = sql.trim();

    // SELECT TRANSFORM ... USING 'script'
    let transform_re = Regex::new(r"(?i)\bTRANSFORM\s*\(.+?\)\s+USING\b").unwrap();
    if transform_re.is_match(trimmed) {
        return Some(TranslateResult::error(
            "SELECT TRANSFORM is not supported by RorisDB",
        ));
    }

    // LATERAL VIEW explode(col)
    let lateral_view_re = Regex::new(r"(?i)\bLATERAL\s+VIEW\s+EXPLODE\b").unwrap();
    if lateral_view_re.is_match(trimmed) {
        return Some(TranslateResult::error(
            "LATERAL VIEW EXPLODE is not supported by RorisDB",
        ));
    }

    // SELECT * REPLACE(expr AS col)
    let replace_re = Regex::new(r"(?i)\*\s+REPLACE\s*\(").unwrap();
    if replace_re.is_match(trimmed) {
        return Some(TranslateResult::error(
            "SELECT * REPLACE is not supported by RorisDB",
        ));
    }

    // MERGE INTO
    let merge_re = Regex::new(r"(?i)^\s*MERGE\s+INTO\b").unwrap();
    if merge_re.is_match(trimmed) {
        return Some(TranslateResult::error(
            "MERGE INTO is not supported by RorisDB",
        ));
    }

    // SELECT * EXCEPT(col1, col2) - cannot expand without schema
    let except_re = Regex::new(r"(?i)\*\s+EXCEPT\s*\(").unwrap();
    if except_re.is_match(trimmed) {
        return Some(TranslateResult::error(
            "SELECT * EXCEPT cannot be translated without schema information. \
             Please specify the column list explicitly.",
        ));
    }

    // Complex types in CREATE TABLE: ARRAY<T>, MAP<K,V>, STRUCT<...>
    // Check in DDL context
    if Regex::new(r"(?i)^\s*CREATE\s+TABLE\b").unwrap().is_match(trimmed) {
        let complex_type_re = Regex::new(r"(?i)\b(ARRAY|MAP|STRUCT)\s*[<(\[]").unwrap();
        if complex_type_re.is_match(trimmed) {
            return Some(TranslateResult::error(
                "Complex types (ARRAY, MAP, STRUCT) are not supported by RorisDB in Phase 1",
            ));
        }
    }

    // UPDATE / DELETE on MaxCompute (not supported by MaxCompute either, but handle anyway)
    let update_re = Regex::new(r"(?i)^\s*UPDATE\b").unwrap();
    if update_re.is_match(trimmed) {
        return Some(TranslateResult::error(
            "UPDATE is not supported by MaxCompute dialect",
        ));
    }
    let delete_re = Regex::new(r"(?i)^\s*DELETE\b").unwrap();
    if delete_re.is_match(trimmed) {
        return Some(TranslateResult::error(
            "DELETE is not supported by MaxCompute dialect",
        ));
    }

    None
}

/// Map MaxCompute types to RorisDB types in column definitions.
fn map_types_in_ddl(sql: &str) -> String {
    // STRING(n) -> VARCHAR(n)
    let string_n_re = Regex::new(r"(?i)\bSTRING\s*\(\s*(\d+)\s*\)").unwrap();
    let result = string_n_re.replace_all(sql, |caps: &regex::Captures| {
        let len = caps.get(1).map(|m| m.as_str()).unwrap_or("255");
        format!("VARCHAR({})", len)
    });

    result.to_string()
}

// ── Main Translation Logic ─────────────────────────────────────────────

impl DialectTranslator for MaxComputeTranslator {
    fn translate(&self, sql: &str) -> TranslateResult {
        let trimmed = sql.trim();

        // Empty SQL
        if trimmed.is_empty() {
            return TranslateResult::ok(String::new());
        }

        // Remove trailing semicolons for processing
        let cleaned = trimmed.trim_end_matches(';').trim();

        // Check for no-op SET/SETPROJECT statements
        if is_noop_set_statement(cleaned) {
            return TranslateResult::ok(String::new())
                .with_warning("SET/SETPROJECT statement is a no-op in RorisDB");
        }

        // Check for unsupported features
        if let Some(err) = check_unsupported(cleaned) {
            return err;
        }

        let mut warnings: Vec<String> = Vec::new();
        let mut result = cleaned.to_string();

        // Step 1: Strip optimizer hints
        {
            let (r, w) = strip_hints(&result);
            result = r;
            warnings.extend(w);
        }

        // Step 2: Handle INSERT transformations
        {
            let (r, w) = translate_insert(&result);
            result = r;
            warnings.extend(w);
        }

        // Step 3: Handle CREATE TABLE transformations
        if Regex::new(r"(?i)^\s*CREATE\s+TABLE\b").unwrap().is_match(&result) {
            let (r, w) = process_create_table(&result);
            result = r;
            warnings.extend(w);
        }

        // Step 4: Handle DISTRIBUTE BY + SORT BY -> ORDER BY
        {
            let (r, w) = translate_distribute_sort(&result);
            result = r;
            warnings.extend(w);
        }

        // Step 5: Map types
        result = map_types_in_ddl(&result);

        // Step 6: Clean up extra whitespace
        result = result.trim().to_string();
        let multi_space = Regex::new(r"\s{2,}").unwrap();
        result = multi_space.replace_all(&result, " ").to_string();

        TranslateResult::ok(result).with_warnings(warnings)
    }

    fn dialect_name(&self) -> &str {
        "maxcompute"
    }

    fn unsupported_features(&self) -> &[&str] {
        &[
            "MERGE INTO",
            "SELECT TRANSFORM ... USING 'script'",
            "LATERAL VIEW EXPLODE",
            "SELECT * REPLACE(expr AS col)",
            "Complex types (ARRAY, MAP, STRUCT)",
            "SELECT * EXCEPT(col1, col2) (without schema)",
            "UPDATE / DELETE",
            "MAPJOIN / SKEWJOIN hints (silently stripped)",
            "PARTITIONED BY (converted to regular columns)",
            "LIFECYCLE (silently stripped)",
            "STORED AS (silently stripped)",
            "CLUSTERED BY (silently stripped)",
            "TBLPROPERTIES (silently stripped)",
            "INSERT OVERWRITE (converted to INSERT INTO)",
            "INSERT with PARTITION (partition clause stripped)",
            "DISTRIBUTE BY (converted to ORDER BY via SORT BY)",
            "SET odps.* (no-op)",
            "SETPROJECT (no-op)",
        ]
    }
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn translator() -> MaxComputeTranslator {
        MaxComputeTranslator::new()
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
        assert!(
            result.error.as_ref().unwrap().contains(expected_msg),
            "Error message mismatch.\nExpected contains: {}\nGot: {:?}",
            expected_msg,
            result.error
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
        assert_eq!(find_matching_paren("(a)", 0), Some(2));
    }

    #[test]
    fn test_find_matching_paren_nested() {
        assert_eq!(find_matching_paren("(a (b) c)", 0), Some(8));
    }

    #[test]
    fn test_find_matching_paren_no_match() {
        assert_eq!(find_matching_paren("(a(b)", 0), None);
    }

    #[test]
    fn test_find_matching_paren_not_paren() {
        assert_eq!(find_matching_paren("hello", 0), None);
    }

    // ── Basic CREATE TABLE ──

    #[test]
    fn test_create_table_basic() {
        assert_translated(
            "CREATE TABLE t (col1 STRING, col2 BIGINT)",
            "CREATE TABLE t (col1 STRING, col2 BIGINT)",
        );
    }

    #[test]
    fn test_create_table_with_if_not_exists() {
        assert_translated(
            "CREATE TABLE IF NOT EXISTS t (col1 STRING)",
            "CREATE TABLE IF NOT EXISTS t (col1 STRING)",
        );
    }

    // ── CREATE TABLE with PARTITIONED BY ──

    #[test]
    fn test_create_table_partitioned_by() {
        let result = translator().translate(
            "CREATE TABLE t (col1 STRING) PARTITIONED BY (dt STRING)",
        );
        assert!(result.success, "Failed: {:?}", result.error);
        assert_eq!(result.sql, "CREATE TABLE t (col1 STRING, dt STRING)");
        assert!(result.warnings.iter().any(|w| w.contains("PARTITIONED BY")));
    }

    #[test]
    fn test_create_table_partitioned_by_multiple_cols() {
        let result = translator().translate(
            "CREATE TABLE t (col1 STRING) PARTITIONED BY (dt STRING, hh STRING)",
        );
        assert!(result.success);
        assert_eq!(result.sql, "CREATE TABLE t (col1 STRING, dt STRING, hh STRING)");
    }

    #[test]
    fn test_create_table_partitioned_by_only() {
        let result = translator().translate(
            "CREATE TABLE t (col1 STRING) PARTITIONED BY (dt STRING)",
        );
        assert!(result.success);
        assert_eq!(result.sql, "CREATE TABLE t (col1 STRING, dt STRING)");
    }

    // ── CREATE TABLE with LIFECYCLE ──

    #[test]
    fn test_create_table_lifecycle() {
        let result = translator().translate(
            "CREATE TABLE t (col1 STRING) LIFECYCLE 365",
        );
        assert!(result.success);
        assert_eq!(result.sql, "CREATE TABLE t (col1 STRING)");
        assert!(
            result.warnings.iter().any(|w| w.contains("LIFECYCLE")),
            "Expected warning about LIFECYCLE, got: {:?}",
            result.warnings
        );
    }

    // ── CREATE TABLE with STORED AS ──

    #[test]
    fn test_create_table_stored_as() {
        let result = translator().translate(
            "CREATE TABLE t (col1 STRING) STORED AS ORC",
        );
        assert!(result.success);
        assert_eq!(result.sql, "CREATE TABLE t (col1 STRING)");
        assert!(
            result.warnings.iter().any(|w| w.contains("STORED AS")),
            "Expected warning about STORED AS, got: {:?}",
            result.warnings
        );
    }

    #[test]
    fn test_create_table_stored_as_parquet() {
        let result = translator().translate(
            "CREATE TABLE t (col1 STRING) STORED AS PARQUET",
        );
        assert!(result.success);
        assert_eq!(result.sql, "CREATE TABLE t (col1 STRING)");
    }

    // ── CREATE TABLE with CLUSTERED BY ──

    #[test]
    fn test_create_table_clustered_by() {
        let result = translator().translate(
            "CREATE TABLE t (col1 STRING) CLUSTERED BY (col1) INTO 256 BUCKETS",
        );
        assert!(result.success);
        assert_eq!(result.sql, "CREATE TABLE t (col1 STRING)");
        assert!(
            result.warnings.iter().any(|w| w.contains("CLUSTERED BY")),
            "Expected warning about CLUSTERED BY, got: {:?}",
            result.warnings
        );
    }

    // ── CREATE TABLE with TBLPROPERTIES ──

    #[test]
    fn test_create_table_tblproperties() {
        let result = translator().translate(
            "CREATE TABLE t (col1 STRING) TBLPROPERTIES ('key'='value')",
        );
        assert!(result.success);
        assert_eq!(result.sql, "CREATE TABLE t (col1 STRING)");
        assert!(
            result.warnings.iter().any(|w| w.contains("TBLPROPERTIES")),
            "Expected warning about TBLPROPERTIES, got: {:?}",
            result.warnings
        );
    }

    #[test]
    fn test_create_table_tblproperties_nested_parens() {
        let result = translator().translate(
            "CREATE TABLE t (col1 STRING) TBLPROPERTIES ('k'='(v)')",
        );
        assert!(result.success);
        assert_eq!(result.sql, "CREATE TABLE t (col1 STRING)");
    }

    // ── CREATE TABLE with all clauses ──

    #[test]
    fn test_create_table_all_clauses() {
        let result = translator().translate(
            "CREATE TABLE t (col1 STRING, col2 BIGINT) PARTITIONED BY (dt STRING) \
             LIFECYCLE 365 STORED AS ORC CLUSTERED BY (col1) INTO 256 BUCKETS \
             TBLPROPERTIES ('k'='v')",
        );
        assert!(result.success);
        assert_eq!(result.sql, "CREATE TABLE t (col1 STRING, col2 BIGINT, dt STRING)");
        assert!(
            result.warnings.len() >= 4,
            "Expected at least 4 warnings, got {}: {:?}",
            result.warnings.len(),
            result.warnings
        );
    }

    // ── CREATE TABLE with COMMENT ──

    #[test]
    fn test_create_table_with_comment() {
        // Table-level COMMENT should be preserved (RorisDB may support it)
        assert_translated(
            "CREATE TABLE t (col1 STRING) COMMENT 'a table'",
            "CREATE TABLE t (col1 STRING) COMMENT 'a table'",
        );
    }

    #[test]
    fn test_create_table_column_comment() {
        assert_translated(
            "CREATE TABLE t (col1 STRING COMMENT 'a column')",
            "CREATE TABLE t (col1 STRING COMMENT 'a column')",
        );
    }

    // ── INSERT OVERWRITE → INSERT INTO ──

    #[test]
    fn test_insert_overwrite_table() {
        let result = translator().translate(
            "INSERT OVERWRITE TABLE t VALUES (1, 'a')",
        );
        assert!(result.success);
        assert_eq!(result.sql, "INSERT INTO TABLE t VALUES (1, 'a')");
        assert!(
            result.warnings.iter().any(|w| w.contains("INSERT OVERWRITE")),
            "Expected warning about INSERT OVERWRITE, got: {:?}",
            result.warnings
        );
    }

    #[test]
    fn test_insert_overwrite_select() {
        let result = translator().translate(
            "INSERT OVERWRITE TABLE t SELECT * FROM src",
        );
        assert!(result.success);
        assert_eq!(result.sql, "INSERT INTO TABLE t SELECT * FROM src");
    }

    // ── INSERT with PARTITION ──

    #[test]
    fn test_insert_into_partition() {
        let result = translator().translate(
            "INSERT INTO TABLE t PARTITION (ds='2024') VALUES (1)",
        );
        assert!(result.success);
        assert_eq!(result.sql, "INSERT INTO TABLE t VALUES (1)");
        assert!(
            result.warnings.iter().any(|w| w.contains("PARTITION")),
            "Expected warning about PARTITION, got: {:?}",
            result.warnings
        );
    }

    #[test]
    fn test_insert_overwrite_partition_select() {
        let result = translator().translate(
            "INSERT OVERWRITE TABLE t PARTITION (ds='2024') SELECT * FROM src",
        );
        assert!(result.success);
        assert_eq!(
            result.sql,
            "INSERT INTO TABLE t SELECT * FROM src"
        );
    }

    // ── MERGE INTO (error) ──

    #[test]
    fn test_merge_into_error() {
        assert_error(
            "MERGE INTO target USING source ON target.id = source.id WHEN MATCHED THEN UPDATE SET target.val = source.val",
            "MERGE INTO",
        );
    }

    // ── SELECT with hints ──

    #[test]
    fn test_select_mapjoin_hint() {
        let result = translator().translate(
            "SELECT /*+ MAPJOIN(b) */ a.* FROM t a JOIN small b ON a.id = b.id",
        );
        assert!(result.success);
        assert!(!result.sql.contains("MAPJOIN"));
        assert!(
            result.warnings.iter().any(|w| w.contains("MAPJOIN")),
            "Expected warning about MAPJOIN hint, got: {:?}",
            result.warnings
        );
    }

    #[test]
    fn test_select_skewjoin_hint() {
        let result = translator().translate(
            "SELECT /*+ SKEWJOIN(a) */ * FROM t a",
        );
        assert!(result.success);
        assert!(!result.sql.contains("SKEWJOIN"));
    }

    #[test]
    fn test_select_multiple_hints() {
        let result = translator().translate(
            "SELECT /*+ MAPJOIN(a) SKEWJOIN(b) */ * FROM t",
        );
        assert!(result.success);
        assert!(!result.sql.contains("MAPJOIN") && !result.sql.contains("SKEWJOIN"));
    }

    // ── DISTRIBUTE BY + SORT BY → ORDER BY ──

    #[test]
    fn test_distribute_by_sort_by() {
        let result = translator().translate(
            "SELECT * FROM t DISTRIBUTE BY col1 SORT BY col2",
        );
        assert!(result.success);
        assert_eq!(result.sql, "SELECT * FROM t ORDER BY col2");
        assert!(
            result.warnings.iter().any(|w| w.contains("DISTRIBUTE BY")),
            "Expected warning about DISTRIBUTE BY, got: {:?}",
            result.warnings
        );
    }

    #[test]
    fn test_sort_by_with_clause() {
        let result = translator().translate(
            "SELECT * FROM t DISTRIBUTE BY col1 SORT BY col2 DESC LIMIT 10",
        );
        assert!(result.success);
        assert!(result.sql.contains("ORDER BY col2 DESC LIMIT 10"));
    }

    // ── SET/SETPROJECT (no-op) ──

    #[test]
    fn test_set_odps_statement() {
        assert_noop("SET odps.sql.allow.fullscan=true;");
    }

    #[test]
    fn test_set_odps_without_semicolon() {
        assert_noop("SET odps.sql.allow.fullscan=true");
    }

    #[test]
    fn test_setproject_statement() {
        assert_noop("SETPROJECT my_project=value;");
    }

    // ── Unsupported features ──

    #[test]
    fn test_select_transform_error() {
        assert_error(
            "SELECT TRANSFORM (col) USING 'python script.py' FROM t",
            "SELECT TRANSFORM",
        );
    }

    #[test]
    fn test_lateral_view_explode_error() {
        assert_error(
            "SELECT col, tag FROM t LATERAL VIEW EXPLODE(tags) t AS tag",
            "LATERAL VIEW EXPLODE",
        );
    }

    #[test]
    fn test_select_replace_error() {
        assert_error(
            "SELECT * REPLACE (col1 + 1 AS col1) FROM t",
            "SELECT * REPLACE",
        );
    }

    #[test]
    fn test_select_except_error() {
        assert_error(
            "SELECT * EXCEPT (col1, col2) FROM t",
            "SELECT * EXCEPT",
        );
    }

    #[test]
    fn test_create_table_complex_types() {
        assert_error(
            "CREATE TABLE t (col1 ARRAY<STRING>)",
            "Complex types",
        );
    }

    #[test]
    fn test_update_error() {
        assert_error("UPDATE t SET col1 = 1 WHERE id = 1", "UPDATE");
    }

    #[test]
    fn test_delete_error() {
        assert_error("DELETE FROM t WHERE id = 1", "DELETE");
    }

    // ── Type Mapping ──

    #[test]
    fn test_string_n_to_varchar() {
        assert_translated(
            "CREATE TABLE t (col1 STRING(10))",
            "CREATE TABLE t (col1 VARCHAR(10))",
        );
    }

    #[test]
    fn test_string_n_to_varchar_partitioned() {
        let result = translator().translate(
            "CREATE TABLE t (col1 STRING(10)) PARTITIONED BY (dt STRING(8))",
        );
        assert!(result.success);
        assert_eq!(
            result.sql,
            "CREATE TABLE t (col1 VARCHAR(10), dt VARCHAR(8))"
        );
    }

    #[test]
    fn test_bigint_no_change() {
        assert_translated("CREATE TABLE t (col1 BIGINT)", "CREATE TABLE t (col1 BIGINT)");
    }

    #[test]
    fn test_basic_types_no_change() {
        assert_translated(
            "CREATE TABLE t (a INT, b DOUBLE, c BOOLEAN, d FLOAT, e TINYINT, f SMALLINT)",
            "CREATE TABLE t (a INT, b DOUBLE, c BOOLEAN, d FLOAT, e TINYINT, f SMALLINT)",
        );
    }

    // ── Case Insensitivity ──

    #[test]
    fn test_case_insensitive_create() {
        let result = translator().translate(
            "create table t (col1 string) partitioned by (dt string) lifecycle 365",
        );
        assert!(result.success);
        assert_eq!(result.sql, "create table t (col1 string, dt string)");
    }

    #[test]
    fn test_case_insensitive_insert_overwrite() {
        let result = translator().translate(
            "insert OVERWRITE table t values (1)",
        );
        assert!(result.success);
        assert_eq!(result.sql, "insert INTO table t values (1)");
    }

    #[test]
    fn test_case_insensitive_hints() {
        let result = translator().translate(
            "SELECT /*+ mapjoin(a) */ * FROM t",
        );
        assert!(result.success);
        assert!(!result.sql.contains("mapjoin"));
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
    fn test_create_table_with_nested_parens_in_tblproperties() {
        // TBLPROPERTIES with nested parentheses values
        let result = translator().translate(
            "CREATE TABLE t (col1 STRING) TBLPROPERTIES ('k'='(nested)') LIFECYCLE 30",
        );
        assert!(result.success);
        assert_eq!(result.sql, "CREATE TABLE t (col1 STRING)");
    }

    #[test]
    fn test_multiple_statements_with_semicolons() {
        // Note: Our translator handles one statement at a time
        // The semicolons are just trailing
        let result = translator().translate("SELECT 1;");
        assert!(result.success);
        assert_eq!(result.sql, "SELECT 1");
    }

    #[test]
    fn test_date_type_no_change() {
        assert_translated(
            "CREATE TABLE t (col1 DATE, col2 DATETIME, col3 TIMESTAMP)",
            "CREATE TABLE t (col1 DATE, col2 DATETIME, col3 TIMESTAMP)",
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
    fn test_partitioned_by_no_columns() {
        // Edge case: PARTITIONED BY with no columns shouldn't crash
        let result = translator().translate(
            "CREATE TABLE t (col1 STRING) PARTITIONED BY ()",
        );
        assert!(result.success);
        // Should just strip the PARTITIONED BY clause
        assert_eq!(result.sql, "CREATE TABLE t (col1 STRING)");
    }

    #[test]
    fn test_create_table_partitioned_by_with_comment() {
        // PARTITIONED BY with COMMENT on partition columns
        let result = translator().translate(
            "CREATE TABLE t (col1 STRING) PARTITIONED BY (dt STRING COMMENT 'date')",
        );
        assert!(result.success);
        assert_eq!(
            result.sql,
            "CREATE TABLE t (col1 STRING, dt STRING COMMENT 'date')"
        );
    }

    #[test]
    fn test_create_table_quoted_db_name() {
        let result = translator().translate(
            "CREATE TABLE `db`.`t` (col1 STRING) PARTITIONED BY (dt STRING) LIFECYCLE 365",
        );
        assert!(result.success);
        assert_eq!(result.sql, "CREATE TABLE `db`.`t` (col1 STRING, dt STRING)");
    }

    #[test]
    fn test_insert_overwrite_without_table_keyword() {
        // Some MaxCompute syntax: INSERT OVERWRITE t SELECT ...
        let result = translator().translate(
            "INSERT OVERWRITE t SELECT * FROM src",
        );
        assert!(result.success);
        assert_eq!(result.sql, "INSERT INTO t SELECT * FROM src");
    }

    #[test]
    fn test_string_trailing_spaces_in_hints() {
        let result = translator().translate(
            "SELECT /*+ MAPJOIN(a)  */ * FROM t",
        );
        assert!(result.success);
        assert!(!result.sql.contains("MAPJOIN"));
    }

    #[test]
    fn test_insert_overwrite_table_with_partition_and_strip() {
        let result = translator().translate(
            "INSERT OVERWRITE TABLE t PARTITION (ds='2024', hh='01') SELECT * FROM src WHERE id > 10",
        );
        assert!(result.success);
        assert_eq!(
            result.sql,
            "INSERT INTO TABLE t SELECT * FROM src WHERE id > 10"
        );
    }

    #[test]
    fn test_partitioned_by_type_with_comment_only() {
        let result = translator().translate(
            "CREATE TABLE sales (amount DOUBLE) PARTITIONED BY (region STRING COMMENT 'sales region')",
        );
        assert!(result.success);
        assert_eq!(
            result.sql,
            "CREATE TABLE sales (amount DOUBLE, region STRING COMMENT 'sales region')"
        );
    }

    #[test]
    fn test_create_table_lifecycle_and_tblproperties() {
        let result = translator().translate(
            "CREATE TABLE t (id BIGINT) LIFECYCLE 90 TBLPROPERTIES ('abc'='def')",
        );
        assert!(result.success);
        assert_eq!(result.sql, "CREATE TABLE t (id BIGINT)");
    }

    #[test]
    fn test_select_with_subquery_no_transform() {
        // Should not be affected
        assert_translated(
            "SELECT * FROM (SELECT id, name FROM users WHERE active = 1) sub",
            "SELECT * FROM (SELECT id, name FROM users WHERE active = 1) sub",
        );
    }

    #[test]
    fn test_simple_order_by() {
        // DISTRIBUTE BY ... SORT BY should only affect that specific pattern
        assert_translated(
            "SELECT * FROM t ORDER BY col1",
            "SELECT * FROM t ORDER BY col1",
        );
    }

    #[test]
    fn test_boolean_literal() {
        assert_translated("SELECT true, false", "SELECT true, false");
    }

    #[test]
    fn test_create_table_if_not_exists_complex() {
        assert_translated(
            "CREATE TABLE IF NOT EXISTS db.t (id BIGINT, name STRING) LIFECYCLE 30 STORED AS ORC",
            "CREATE TABLE IF NOT EXISTS db.t (id BIGINT, name STRING)",
        );
    }
}