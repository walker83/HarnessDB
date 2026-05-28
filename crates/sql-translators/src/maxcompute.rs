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
    crate::find_matching_paren(s, start)
}

/// Extract content within matching parentheses.
/// Returns the content (without the outer parens) and the end position.
fn extract_paren_content(s: &str, start: usize) -> Option<(&str, usize)> {
    let end = find_matching_paren(s, start)?;
    Some((&s[start + 1..end], end))
}

// ── DDL Transformations ────────────────────────────────────────────────

/// Strip `STORED AS ...` from the tail of a CREATE TABLE statement.
/// Handles both simple forms (`STORED AS ORC`, `STORED AS PARQUET`) and
/// extended forms (`STORED AS INPUTFORMAT '...' OUTPUTFORMAT '...'`, `STORED AS BY '...'`).
/// The extended form regex is applied on unmasked SQL since the quotes in the
/// INPUTFORMAT/BY patterns are SQL syntax, not user data. The simple form is
/// applied after string literal masking to avoid matching inside strings.
fn strip_stored_as(sql: &str) -> (String, Vec<String>) {
    let mut warnings = Vec::new();

    // Apply extended form regex on unmasked SQL. The patterns
    // `STORED AS INPUTFORMAT '...' OUTPUTFORMAT '...'` and
    // `STORED AS BY '...'` are very specific and extremely unlikely
    // to appear inside user string literals.
    let extended_re = Regex::new(
        r"(?i)\s+STORED\s+AS\s+(INPUTFORMAT\s+'[^']*'\s+OUTPUTFORMAT\s+'[^']*'|BY\s+'[^']*')",
    )
    .unwrap();
    let after_extended = extended_re.replace_all(sql, |caps: &regex::Captures| {
        let cap = caps.get(0).map(|m| m.as_str()).unwrap_or("");
        warnings.push(format!("STORED AS clause stripped: '{}'", cap.trim()));
        ""
    });

    // Now mask string literals for the simple form
    let (masked, original_strings) = crate::mask_string_literals(&after_extended);

    // Handle simple form: STORED AS <word>
    let simple_re = Regex::new(r"(?i)\s+STORED\s+AS\s+\w+").unwrap();
    let result_masked = simple_re.replace_all(&masked, |caps: &regex::Captures| {
        let cap = caps.get(0).map(|m| m.as_str()).unwrap_or("");
        warnings.push(format!("STORED AS clause stripped: '{}'", cap.trim()));
        ""
    });

    let result = crate::restore_string_literals(&result_masked, &original_strings);
    (result.to_string(), warnings)
}

/// Strip `LIFECYCLE N` from the tail.
/// Uses string literal masking to avoid matching inside string values.
fn strip_lifecycle(sql: &str) -> (String, Vec<String>) {
    let mut warnings = Vec::new();
    let (masked, original_strings) = crate::mask_string_literals(sql);
    let re = Regex::new(r"(?i)\s+LIFECYCLE\s+\d+").unwrap();
    let result_masked = re.replace_all(&masked, |caps: &regex::Captures| {
        let cap = caps.get(0).map(|m| m.as_str()).unwrap_or("");
        warnings.push(format!("LIFECYCLE clause stripped: '{}', not enforced", cap.trim()));
        ""
    });
    let result = crate::restore_string_literals(&result_masked, &original_strings);
    (result.to_string(), warnings)
}

/// Strip `CLUSTERED BY (...) INTO N BUCKETS` from the tail.
/// Uses string literal masking to avoid matching inside string values.
fn strip_clustered_by(sql: &str) -> (String, Vec<String>) {
    let mut warnings = Vec::new();
    let (masked, original_strings) = crate::mask_string_literals(sql);
    let s = masked.trim_end();

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

    let result = crate::restore_string_literals(&result, &original_strings);
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
    crate::split_by_commas_outside_parens(s)
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

    // Check for CTAS (CREATE TABLE ... AS SELECT)
    // This needs to be checked before looking for column definitions
    // because CTAS might have a subquery with parentheses
    let as_select_re = Regex::new(r"(?i)\bAS\s+SELECT\b").unwrap();
    let after_create_rest = &trimmed[after_create..];
    if as_select_re.is_match(after_create_rest) {
        // CTAS - strip MC clauses from the tail
        let mut result = trimmed.to_string();
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
        return (result.trim().to_string(), warnings);
    }

    // The column defs start with the first '(' that is not part of a keyword
    let col_def_start = find_col_def_start(trimmed, after_create);
    let col_def_start = match col_def_start {
        Some(pos) => pos,
        None => {
            // No column definitions - could be CREATE TABLE LIKE or similar
            // Strip MC-specific clauses from the tail
            let mut result = trimmed.to_string();
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
            return (result.trim().to_string(), warnings);
        },
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
/// in a CREATE TABLE statement. Skips SQL comments (block comments `/* ... */`
/// and line comments `-- ...`) as well as quoted identifiers.
fn find_col_def_start(sql: &str, after_create: usize) -> Option<usize> {
    let rest = &sql[after_create..];
    // Skip past: [IF NOT EXISTS] [db.]table_name, SQL comments
    // Look for first '(' that is not inside a string, comment, or quoted identifier
    let mut i = 0;
    let bytes = rest.as_bytes();
    while i < bytes.len() {
        if bytes[i] == b'(' {
            return Some(after_create + i);
        }
        // Skip block comments: /* ... */
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            if i + 1 < bytes.len() {
                i += 2; // Skip */
            }
            continue;
        }
        // Skip line comments: -- ...
        if i + 1 < bytes.len() && bytes[i] == b'-' && bytes[i + 1] == b'-' {
            i += 2;
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            if i < bytes.len() {
                i += 1; // Skip newline
            }
            continue;
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
/// Uses string literal masking to avoid matching keywords inside string values.
fn translate_insert(sql: &str) -> (String, Vec<String>) {
    let mut warnings = Vec::new();
    let trimmed = sql.trim();

    // Mask string literals to avoid matching keywords inside string values
    let (masked, original_strings) = crate::mask_string_literals(trimmed);

    // INSERT OVERWRITE [TABLE] t ... -> INSERT INTO [TABLE] t ...
    let overwrite_re = Regex::new(r"(?i)^(\s*INSERT\s+)OVERWRITE(\s+(?:TABLE\s+)?)").unwrap();
    let after_overwrite = if let Some(caps) = overwrite_re.captures(&masked) {
        warnings.push("INSERT OVERWRITE converted to INSERT INTO".to_string());
        // Get the full match end position to preserve the rest of the SQL
        let full_match_end = caps.get(0).unwrap().end();
        format!("{}INTO{}{}", &caps[1], &caps[2], &masked[full_match_end..])
    } else {
        masked.to_string()
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

    let result = crate::restore_string_literals(&result, &original_strings);
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
/// Uses string literal masking and paren-depth tracking to avoid matching
/// keywords inside subqueries or string values.
fn translate_distribute_sort(sql: &str) -> (String, Vec<String>) {
    let mut warnings = Vec::new();
    let trimmed = sql.trim();

    // Mask string literals to avoid matching keywords inside string values
    let (masked, original_strings) = crate::mask_string_literals(trimmed);

    let distribute_re = Regex::new(r"(?i)\bDISTRIBUTE\s+BY\b").unwrap();
    let sort_re = Regex::new(r"(?i)\bSORT\s+BY\b").unwrap();

    if let Some(dist_pos) = distribute_re.find(&masked) {
        if let Some(sort_pos) = sort_re.find(&masked) {
            if sort_pos.start() > dist_pos.end() {
                // Extract the SORT BY columns
                let after_sort = &masked[sort_pos.end()..];

                // Find end of SORT BY columns with paren-depth tracking
                // so we don't match keywords inside subqueries
                let mut sort_end = masked.len();
                let mut paren_depth: u32 = 0;
                let bytes = after_sort.as_bytes();
                for (i, &b) in bytes.iter().enumerate() {
                    match b {
                        b'(' => paren_depth += 1,
                        b')' => paren_depth = paren_depth.saturating_sub(1),
                        _ => {}
                    }
                    if paren_depth > 0 {
                        continue;
                    }
                    // Check for end keywords (uppercase comparison)
                    if i + 1 < bytes.len() {
                        // We need to check for keywords at this position
                        let remaining = &after_sort[i..];
                        let remaining_upper = remaining.to_uppercase();
                        let end_keywords = [
                            " FROM ", " WHERE ", " GROUP ", " HAVING ", " ORDER ", " LIMIT ",
                            " UNION ", " INTERSECT ", " EXCEPT ", ";",
                        ];
                        for kw in &end_keywords {
                            if remaining_upper.starts_with(kw) {
                                let candidate = sort_pos.end() + i;
                                if candidate < sort_end {
                                    sort_end = candidate;
                                }
                                break;
                            }
                        }
                    }
                    // Also check for standalone semicolon
                    if b == b';' {
                        let candidate = sort_pos.end() + i;
                        if candidate < sort_end {
                            sort_end = candidate;
                        }
                    }
                }

                let sort_cols = &masked[sort_pos.end()..sort_end].trim();

                // Replace DISTRIBUTE BY ... SORT BY col with ORDER BY col
                warnings.push(
                    "DISTRIBUTE BY + SORT BY converted to ORDER BY".to_string(),
                );

                // The content between DISTRIBUTE BY's columns and SORT BY is the distribute columns.
                let distribute_cols = &masked[dist_pos.end()..sort_pos.start()].trim();
                warnings.push(format!(
                    "DISTRIBUTE BY columns '{}' dropped, using SORT BY columns for ORDER BY",
                    distribute_cols
                ));

                let final_sql_masked = format!(
                    "{} ORDER BY {}{}",
                    &masked[..dist_pos.start()],
                    sort_cols,
                    &masked[sort_end..]
                );
                let final_sql = crate::restore_string_literals(&final_sql_masked, &original_strings);
                return (final_sql.trim().to_string(), warnings);
            }
        }
    }

    // No match, restore and return original
    let result = crate::restore_string_literals(&masked, &original_strings);
    (result, warnings)
}

/// Handle SET/SETPROJECT statements (no-op).
fn is_noop_set_statement(sql: &str) -> bool {
    let trimmed = sql.trim();
    let set_re = Regex::new(r"(?i)^\s*SET\s+\S+\s*=\s*\S*\s*;?\s*$").unwrap();
    let setproject_re = Regex::new(r"(?i)^\s*SETPROJECT\s+\S+\s*=\s*\S*\s*;?\s*$").unwrap();
    set_re.is_match(trimmed) || setproject_re.is_match(trimmed)
}

/// Translate LATERAL VIEW explode(col) table_alias AS col_alias
/// to CROSS JOIN UNNEST(col) AS table_alias(col_alias) for DataFusion compatibility.
///
/// Uses string literal masking and extract_paren_content to handle
/// balanced parentheses inside the explode expression.
fn translate_lateral_view(sql: &str) -> (String, Vec<String>) {
    let mut warnings = Vec::new();
    let (masked, original_strings) = crate::mask_string_literals(sql);

    let lateral_view_re = Regex::new(r"(?i)\bLATERAL\s+VIEW\s+EXPLODE\(").unwrap();
    let mut result = masked.to_string();

    loop {
        if let Some(cap) = lateral_view_re.find(&result) {
            let paren_start = cap.end() - 1; // position of '(' after EXPLODE
            if let Some((content, paren_end)) = extract_paren_content(&result, paren_start) {
                let after_paren = &result[paren_end + 1..];
                // After the closing paren: table_alias AS col_alias
                let alias_re = Regex::new(r"(?i)^\s*(\w+)\s+AS\s+(\w+)").unwrap();
                if let Some(alias_caps) = alias_re.captures(after_paren) {
                    let table_alias = alias_caps.get(1).unwrap().as_str();
                    let col_alias = alias_caps.get(2).unwrap().as_str();
                    let alias_end = paren_end + 1 + alias_caps.get(0).unwrap().len();

                    warnings.push(
                        "LATERAL VIEW EXPLODE converted to CROSS JOIN UNNEST".to_string(),
                    );

                    let replacement = format!(
                        " CROSS JOIN UNNEST({}) AS {}({})",
                        content, table_alias, col_alias
                    );

                    result = format!(
                        "{}{}{}",
                        &result[..cap.start()],
                        replacement,
                        &result[alias_end..]
                    );
                } else {
                    // Can't parse alias pattern, skip this match
                    break;
                }
            } else {
                // Unbalanced parens, skip
                break;
            }
        } else {
            break;
        }
    }

    let result = crate::restore_string_literals(&result, &original_strings);
    (result, warnings)
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

    // Complex types (ARRAY, MAP, STRUCT) are now passed through to DataFusion
    // MERGE INTO is now passed through to DataFusion
    // LATERAL VIEW EXPLODE is now translated to CROSS JOIN UNNEST
    None
}

/// Convert `CLUSTER BY col` to `ORDER BY col`.
/// Uses string literal masking and paren-depth tracking to avoid matching
/// keywords inside subqueries or string values.
fn translate_cluster_by(sql: &str) -> (String, Vec<String>) {
    let mut warnings = Vec::new();
    let trimmed = sql.trim();

    // Mask string literals to avoid matching keywords inside string values
    let (masked, original_strings) = crate::mask_string_literals(trimmed);

    let cluster_re = Regex::new(r"(?i)\bCLUSTER\s+BY\b").unwrap();

    if let Some(cluster_pos) = cluster_re.find(&masked) {
        warnings.push("CLUSTER BY converted to ORDER BY".to_string());

        // Extract the CLUSTER BY columns
        let after_cluster = &masked[cluster_pos.end()..];

        // Find end of CLUSTER BY columns with paren-depth tracking
        let mut cluster_end = masked.len();
        let mut paren_depth: u32 = 0;
        let bytes = after_cluster.as_bytes();
        for (i, &b) in bytes.iter().enumerate() {
            match b {
                b'(' => paren_depth += 1,
                b')' => paren_depth = paren_depth.saturating_sub(1),
                _ => {}
            }
            if paren_depth > 0 {
                continue;
            }
            if i + 1 < bytes.len() {
                let remaining = &after_cluster[i..];
                let remaining_upper = remaining.to_uppercase();
                let end_keywords = [
                    " FROM ", " WHERE ", " GROUP ", " HAVING ", " ORDER ", " LIMIT ",
                    " UNION ", " INTERSECT ", " EXCEPT ", ";",
                ];
                for kw in &end_keywords {
                    if remaining_upper.starts_with(kw) {
                        let candidate = cluster_pos.end() + i;
                        if candidate < cluster_end {
                            cluster_end = candidate;
                        }
                        break;
                    }
                }
            }
            if b == b';' {
                let candidate = cluster_pos.end() + i;
                if candidate < cluster_end {
                    cluster_end = candidate;
                }
            }
        }

        let cluster_cols = &masked[cluster_pos.end()..cluster_end].trim();

        let final_sql_masked = format!(
            "{} ORDER BY {}{}",
            &masked[..cluster_pos.start()],
            cluster_cols,
            &masked[cluster_end..]
        );

        let final_sql = crate::restore_string_literals(&final_sql_masked, &original_strings);
        return (final_sql.trim().to_string(), warnings);
    }

    let result = crate::restore_string_literals(&masked, &original_strings);
    (result, warnings)
}

/// Strip `ZORDER BY (...)` clauses for compatibility.
/// ZORDER BY is a MaxCompute data skipping optimization not supported by RorisDB.
fn strip_zorder_by(sql: &str) -> (String, Vec<String>) {
    let mut warnings = Vec::new();
    let (masked, original_strings) = crate::mask_string_literals(sql);
    let mut result = masked.trim_end().to_string();

    // Match ZORDER BY (col1, col2) or ZORDER BY col1, col2
    let re = Regex::new(r"(?i)\s+ZORDER\s+BY\b").unwrap();
    while let Some(cap) = re.find(&result) {
        let start = cap.start();
        let after = &result[cap.end()..];
        let trimmed_after = after.trim_start();
        let mut end_pos = result.len();

        if trimmed_after.starts_with('(') {
            // Parenthesized form: ZORDER BY (col1, col2)
            let paren_offset = after.len() - trimmed_after.len();
            let paren_start = cap.end() + paren_offset;
            if let Some((_content, paren_end)) = extract_paren_content(&result, paren_start) {
                end_pos = paren_end + 1;
            } else {
                break; // unbalanced parens, stop
            }
        } else {
            // Non-parenthesized form: ZORDER BY col1, col2
            let mut paren_depth: u32 = 0;
            let bytes = after.as_bytes();
            for (i, &b) in bytes.iter().enumerate() {
                match b {
                    b'(' => paren_depth += 1,
                    b')' => paren_depth = paren_depth.saturating_sub(1),
                    _ => {}
                }
                if paren_depth > 0 {
                    continue;
                }
                let remaining = &after[i..];
                let remaining_upper = remaining.to_uppercase();
                for kw in &[
                    " FROM ", " WHERE ", " GROUP ", " HAVING ", " ORDER ", " LIMIT ",
                    " UNION ", " INTERSECT ", " EXCEPT ", ";",
                ] {
                    if remaining_upper.starts_with(kw) {
                        end_pos = cap.end() + i;
                        break;
                    }
                }
                if end_pos < result.len() {
                    break;
                }
            }
        }

        warnings.push(format!(
            "ZORDER BY clause stripped: '{}'",
            result[start..end_pos].trim()
        ));
        result = format!("{}{}", &result[..start], &result[end_pos..]);
    }

    let result = crate::restore_string_literals(&result, &original_strings);
    (result, warnings)
}

/// Map MaxCompute types to RorisDB types in column definitions.
/// Uses string literal masking to avoid matching type keywords inside string values.
fn map_types_in_ddl(sql: &str) -> String {
    let (masked, original_strings) = crate::mask_string_literals(sql);

    // STRING(n) -> VARCHAR(n)
    let string_n_re = Regex::new(r"(?i)\bSTRING\s*\(\s*(\d+)\s*\)").unwrap();
    let result_masked = string_n_re.replace_all(&masked, |caps: &regex::Captures| {
        let len = caps.get(1).map(|m| m.as_str()).unwrap_or("255");
        format!("VARCHAR({})", len)
    });

    crate::restore_string_literals(&result_masked, &original_strings)
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

        // Step 4b: Handle CLUSTER BY -> ORDER BY
        {
            let (r, w) = translate_cluster_by(&result);
            result = r;
            warnings.extend(w);
        }

        // Step 4c: Handle ZORDER BY (strip for compatibility)
        {
            let (r, w) = strip_zorder_by(&result);
            result = r;
            warnings.extend(w);
        }

        // Step 4d: Handle LATERAL VIEW EXPLODE -> CROSS JOIN UNNEST
        {
            let (r, w) = translate_lateral_view(&result);
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
            "SELECT TRANSFORM ... USING 'script'",
            "LATERAL VIEW EXPLODE (translated to CROSS JOIN UNNEST)",
            "Complex types (ARRAY, MAP, STRUCT) - passed through to DataFusion",
            "MERGE INTO - passed through to DataFusion",
            "MAPJOIN / SKEWJOIN hints (silently stripped)",
            "PARTITIONED BY (converted to regular columns)",
            "LIFECYCLE (silently stripped)",
            "STORED AS (silently stripped)",
            "CLUSTERED BY (silently stripped)",
            "TBLPROPERTIES (silently stripped)",
            "INSERT OVERWRITE (converted to INSERT INTO)",
            "INSERT with PARTITION (partition clause stripped)",
            "DISTRIBUTE BY (converted to ORDER BY via SORT BY)",
            "CLUSTER BY (converted to ORDER BY)",
            "ZORDER BY (silently stripped)",
            "SET odps.* / project.* / hive.* / spark.* / mapreduce.* (no-op)",
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

    // ── MERGE INTO (passthrough) ──

    #[test]
    fn test_merge_into_passthrough() {
        let result = translator().translate(
            "MERGE INTO target USING source ON target.id = source.id WHEN MATCHED THEN UPDATE SET target.val = source.val",
        );
        assert!(result.success, "MERGE INTO should pass through, got error: {:?}", result.error);
        // The MERGE INTO should be preserved for DataFusion to try
        assert!(result.sql.contains("MERGE INTO"));
        assert!(result.sql.contains("WHEN MATCHED"));
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

    // ── CLUSTER BY → ORDER BY ──

    #[test]
    fn test_cluster_by_to_order_by() {
        let result = translator().translate(
            "SELECT * FROM t CLUSTER BY col1",
        );
        assert!(result.success);
        assert_eq!(result.sql, "SELECT * FROM t ORDER BY col1");
        assert!(
            result.warnings.iter().any(|w| w.contains("CLUSTER BY")),
            "Expected warning about CLUSTER BY, got: {:?}",
            result.warnings
        );
    }

    #[test]
    fn test_cluster_by_multiple_cols() {
        let result = translator().translate(
            "SELECT * FROM t CLUSTER BY col1, col2",
        );
        assert!(result.success);
        assert_eq!(result.sql, "SELECT * FROM t ORDER BY col1, col2");
    }

    #[test]
    fn test_cluster_by_with_clause() {
        let result = translator().translate(
            "SELECT * FROM t CLUSTER BY col1 LIMIT 10",
        );
        assert!(result.success);
        assert_eq!(result.sql, "SELECT * FROM t ORDER BY col1 LIMIT 10");
    }

    // ── ZORDER BY ──

    #[test]
    fn test_zorder_by_stripped() {
        let result = translator().translate(
            "SELECT * FROM t ZORDER BY (col1, col2)",
        );
        assert!(result.success);
        assert_eq!(result.sql, "SELECT * FROM t");
        assert!(
            result.warnings.iter().any(|w| w.contains("ZORDER BY")),
            "Expected warning about ZORDER BY, got: {:?}",
            result.warnings
        );
    }

    #[test]
    fn test_zorder_by_simple() {
        let result = translator().translate(
            "SELECT * FROM t ZORDER BY col1",
        );
        assert!(result.success);
        assert_eq!(result.sql, "SELECT * FROM t");
    }

    #[test]
    fn test_zorder_by_with_select() {
        let result = translator().translate(
            "SELECT * FROM t ZORDER BY (a, b) LIMIT 10",
        );
        assert!(result.success);
        assert_eq!(result.sql, "SELECT * FROM t LIMIT 10");
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

    #[test]
    fn test_set_project_prefix() {
        assert_noop("SET project.name=my_project;");
    }

    #[test]
    fn test_set_hive_prefix() {
        assert_noop("SET hive.exec.dynamic.partition=true;");
    }

    #[test]
    fn test_set_hive_without_semicolon() {
        assert_noop("SET hive.mapred.mode=nonstrict");
    }

    #[test]
    fn test_set_project_without_semicolon() {
        assert_noop("SET project.name=my_project");
    }

    #[test]
    fn test_set_spark_prefix() {
        assert_noop("SET spark.sql.adaptive.enabled=true;");
    }

    #[test]
    fn test_set_spark_without_semicolon() {
        assert_noop("SET spark.sql.adaptive.enabled=true");
    }

    #[test]
    fn test_set_mapreduce_prefix() {
        assert_noop("SET mapreduce.map.memory.mb=4096;");
    }

    #[test]
    fn test_set_mapreduce_without_semicolon() {
        assert_noop("SET mapreduce.reduce.memory.mb=8192");
    }

    #[test]
    fn test_set_generic_equal_value() {
        assert_noop("SET any.arbitrary.key=any_value;");
    }

    #[test]
    fn test_set_generic_no_semicolon() {
        assert_noop("SET some.setting=12345");
    }

    #[test]
    fn test_set_with_empty_value() {
        assert_noop("SET odps.sql.allow.fullscan=");
    }

    // ── CREATE TABLE LIKE ──

    #[test]
    fn test_create_table_like() {
        let result = translator().translate(
            "CREATE TABLE new_table LIKE existing_table",
        );
        assert!(result.success);
        assert_eq!(result.sql, "CREATE TABLE new_table LIKE existing_table");
    }

    #[test]
    fn test_create_table_like_with_suffixes() {
        let result = translator().translate(
            "CREATE TABLE new_table LIKE existing_table LIFECYCLE 30 STORED AS PARQUET",
        );
        assert!(result.success);
        assert_eq!(result.sql, "CREATE TABLE new_table LIKE existing_table");
        assert!(
            result.warnings.iter().any(|w| w.contains("LIFECYCLE")),
            "Expected warning about LIFECYCLE"
        );
        assert!(
            result.warnings.iter().any(|w| w.contains("STORED AS")),
            "Expected warning about STORED AS"
        );
    }

    #[test]
    fn test_create_table_like_if_not_exists() {
        let result = translator().translate(
            "CREATE TABLE IF NOT EXISTS new_table LIKE existing_table TBLPROPERTIES ('k'='v')",
        );
        assert!(result.success);
        assert_eq!(result.sql, "CREATE TABLE IF NOT EXISTS new_table LIKE existing_table");
    }

    // ── CREATE TABLE AS SELECT (CTAS) ──

    #[test]
    fn test_create_table_as_select() {
        let result = translator().translate(
            "CREATE TABLE new_table AS SELECT * FROM src",
        );
        assert!(result.success);
        assert_eq!(result.sql, "CREATE TABLE new_table AS SELECT * FROM src");
    }

    #[test]
    fn test_create_table_as_select_with_if_not_exists() {
        let result = translator().translate(
            "CREATE TABLE IF NOT EXISTS db.new_table AS SELECT id, name FROM src WHERE active = 1",
        );
        assert!(result.success);
        assert_eq!(
            result.sql,
            "CREATE TABLE IF NOT EXISTS db.new_table AS SELECT id, name FROM src WHERE active = 1"
        );
    }

    #[test]
    fn test_ctas_with_mc_clauses() {
        let result = translator().translate(
            "CREATE TABLE new_table AS SELECT * FROM src LIFECYCLE 90 STORED AS ORC",
        );
        assert!(result.success);
        assert_eq!(result.sql, "CREATE TABLE new_table AS SELECT * FROM src");
        assert!(
            result.warnings.iter().any(|w| w.contains("LIFECYCLE")),
            "Expected warning about LIFECYCLE"
        );
        assert!(
            result.warnings.iter().any(|w| w.contains("STORED AS")),
            "Expected warning about STORED AS"
        );
    }

    #[test]
    fn test_ctas_with_subquery_parens() {
        // CTAS with parenthesized subquery should not confuse column def detection
        let result = translator().translate(
            "CREATE TABLE t AS SELECT * FROM (SELECT * FROM src WHERE active = 1) sub",
        );
        assert!(result.success);
        assert_eq!(
            result.sql,
            "CREATE TABLE t AS SELECT * FROM (SELECT * FROM src WHERE active = 1) sub"
        );
    }

    // ── MULTI INSERT ──

    #[test]
    fn test_multi_insert_passthrough() {
        let result = translator().translate(
            "FROM src INSERT INTO t1 SELECT a, b WHERE cond INSERT INTO t2 SELECT a, c WHERE cond2",
        );
        assert!(result.success);
        assert_eq!(
            result.sql,
            "FROM src INSERT INTO t1 SELECT a, b WHERE cond INSERT INTO t2 SELECT a, c WHERE cond2"
        );
    }

    #[test]
    fn test_multi_insert_overwrite_passthrough() {
        let result = translator().translate(
            "FROM src INSERT OVERWRITE TABLE t1 SELECT a, b INSERT OVERWRITE TABLE t2 SELECT a, c",
        );
        assert!(result.success);
        // INSERT OVERWRITE not converted because MULTI INSERT starts with FROM, not INSERT
        // The INSERT OVERWRITE regex uses ^ so it won't match nested INSERTs
        assert_eq!(
            result.sql,
            "FROM src INSERT OVERWRITE TABLE t1 SELECT a, b INSERT OVERWRITE TABLE t2 SELECT a, c"
        );
    }

    // ── Features now passed through to RorisDB ──
    fn test_select_replace_passthrough() {
        // SELECT * REPLACE now passes through to DataFusion
        assert_translated(
            "SELECT * REPLACE (col1 + 1 AS col1) FROM t",
            "SELECT * REPLACE (col1 + 1 AS col1) FROM t",
        );
    }

    #[test]
    fn test_select_except_passthrough() {
        // SELECT * EXCEPT now passes through to DataFusion
        assert_translated(
            "SELECT * EXCEPT (col1, col2) FROM t",
            "SELECT * EXCEPT (col1, col2) FROM t",
        );
    }

    #[test]
    fn test_update_passthrough() {
        // UPDATE now passes through (RorisDB supports it)
        assert_translated(
            "UPDATE t SET col1 = 1 WHERE id = 1",
            "UPDATE t SET col1 = 1 WHERE id = 1",
        );
    }

    #[test]
    fn test_delete_passthrough() {
        // DELETE now passes through (RorisDB supports it)
        assert_translated(
            "DELETE FROM t WHERE id = 1",
            "DELETE FROM t WHERE id = 1",
        );
    }

    #[test]
    fn test_tablesample_passthrough() {
        assert_translated(
            "SELECT * FROM t TABLESAMPLE(10 PERCENT)",
            "SELECT * FROM t TABLESAMPLE(10 PERCENT)",
        );
    }

    #[test]
    fn test_qualify_passthrough() {
        assert_translated(
            "SELECT *, ROW_NUMBER() OVER (PARTITION BY col1 ORDER BY col2) AS rn FROM t QUALIFY rn = 1",
            "SELECT *, ROW_NUMBER() OVER (PARTITION BY col1 ORDER BY col2) AS rn FROM t QUALIFY rn = 1",
        );
    }

    #[test]
    fn test_grouping_sets_passthrough() {
        assert_translated(
            "SELECT col1, col2, COUNT(*) FROM t GROUP BY GROUPING SETS ((col1), (col2))",
            "SELECT col1, col2, COUNT(*) FROM t GROUP BY GROUPING SETS ((col1), (col2))",
        );
    }

    #[test]
    fn test_rollup_passthrough() {
        assert_translated(
            "SELECT col1, col2, COUNT(*) FROM t GROUP BY ROLLUP (col1, col2)",
            "SELECT col1, col2, COUNT(*) FROM t GROUP BY ROLLUP (col1, col2)",
        );
    }

    #[test]
    fn test_cube_passthrough() {
        assert_translated(
            "SELECT col1, col2, COUNT(*) FROM t GROUP BY CUBE (col1, col2)",
            "SELECT col1, col2, COUNT(*) FROM t GROUP BY CUBE (col1, col2)",
        );
    }

    #[test]
    fn test_create_table_complex_types_passthrough() {
        let result = translator().translate(
            "CREATE TABLE t (col1 ARRAY<STRING>)",
        );
        assert!(result.success, "Complex types should pass through, got error: {:?}", result.error);
        assert!(result.sql.contains("ARRAY<STRING>"));
    }

    #[test]
    fn test_create_table_map_type_passthrough() {
        let result = translator().translate(
            "CREATE TABLE t (col1 MAP<STRING, BIGINT>)",
        );
        assert!(result.success, "MAP type should pass through, got error: {:?}", result.error);
        assert!(result.sql.contains("MAP<STRING, BIGINT>"));
    }

    #[test]
    fn test_create_table_struct_type_passthrough() {
        let result = translator().translate(
            "CREATE TABLE t (col1 STRUCT<a:STRING, b:BIGINT>)",
        );
        assert!(result.success, "STRUCT type should pass through, got error: {:?}", result.error);
        assert!(result.sql.contains("STRUCT<a:STRING, b:BIGINT>"));
    }

    // ── LATERAL VIEW EXPLODE -> CROSS JOIN UNNEST ──

    #[test]
    fn test_lateral_view_explode_single() {
        let result = translator().translate(
            "SELECT a, b FROM t LATERAL VIEW explode(col) tmp AS alias",
        );
        assert!(result.success, "LATERAL VIEW should translate, got error: {:?}", result.error);
        assert_eq!(
            result.sql,
            "SELECT a, b FROM t CROSS JOIN UNNEST(col) AS tmp(alias)"
        );
        assert!(
            result.warnings.iter().any(|w| w.contains("LATERAL VIEW")),
            "Expected warning about LATERAL VIEW, got: {:?}",
            result.warnings
        );
    }

    #[test]
    fn test_lateral_view_explode_multiple() {
        let result = translator().translate(
            "SELECT * FROM t LATERAL VIEW explode(col1) t1 AS c1 LATERAL VIEW explode(col2) t2 AS c2",
        );
        assert!(result.success, "Multiple LATERAL VIEW should translate, got error: {:?}", result.error);
        assert_eq!(
            result.sql,
            "SELECT * FROM t CROSS JOIN UNNEST(col1) AS t1(c1) CROSS JOIN UNNEST(col2) AS t2(c2)"
        );
        assert!(
            result.warnings.len() >= 2,
            "Expected at least 2 warnings about LATERAL VIEW, got: {:?}",
            result.warnings
        );
    }

    #[test]
    fn test_lateral_view_explode_complex_expr() {
        let result = translator().translate(
            "SELECT * FROM t LATERAL VIEW EXPLODE(SPLIT(col, ',')) tmp AS part",
        );
        assert!(result.success, "LATERAL VIEW with complex expr should translate, got error: {:?}", result.error);
        assert_eq!(
            result.sql,
            "SELECT * FROM t CROSS JOIN UNNEST(SPLIT(col, ',')) AS tmp(part)"
        );
    }

    #[test]
    fn test_lateral_view_explode_with_where() {
        let result = translator().translate(
            "SELECT * FROM t LATERAL VIEW EXPLODE(col) tmp AS alias WHERE alias > 0",
        );
        assert!(result.success, "LATERAL VIEW with WHERE should translate, got error: {:?}", result.error);
        assert_eq!(
            result.sql,
            "SELECT * FROM t CROSS JOIN UNNEST(col) AS tmp(alias) WHERE alias > 0"
        );
    }

    #[test]
    fn test_lateral_view_explode_case_insensitive() {
        let result = translator().translate(
            "select * from t lateral view explode(col) tmp as alias",
        );
        assert!(result.success, "Case-insensitive LATERAL VIEW should translate, got error: {:?}", result.error);
        assert_eq!(
            result.sql,
            "select * from t CROSS JOIN UNNEST(col) AS tmp(alias)"
        );
    }

    #[test]
    fn test_select_without_lateral_view_unchanged() {
        // Regular SELECT without LATERAL VIEW should be unchanged
        assert_translated(
            "SELECT * FROM t WHERE col1 > 0",
            "SELECT * FROM t WHERE col1 > 0",
        );
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

    // ── New tests for bug fixes ──

    #[test]
    fn test_stored_as_inputformat_outputformat() {
        let result = translator().translate(
            "CREATE TABLE t (col1 STRING) STORED AS INPUTFORMAT 'org.apache.hadoop.mapred.TextInputFormat' OUTPUTFORMAT 'org.apache.hadoop.mapred.TextOutputFormat'",
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
    fn test_stored_as_by_format() {
        let result = translator().translate(
            "CREATE TABLE t (col1 STRING) STORED AS BY 'com.example.CustomStorageHandler'",
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
    fn test_sql_comment_before_column_defs() {
        let result = translator().translate(
            "CREATE TABLE t /* block comment */ (col1 STRING) LIFECYCLE 30",
        );
        assert!(result.success);
        // Comments are preserved in output
        assert_eq!(result.sql, "CREATE TABLE t /* block comment */ (col1 STRING)");
    }

    #[test]
    fn test_sql_line_comment_before_column_defs() {
        let result = translator().translate(
            "CREATE TABLE t -- line comment\n(col1 STRING) LIFECYCLE 30",
        );
        assert!(result.success);
        // Line comments are preserved in output
        assert_eq!(result.sql, "CREATE TABLE t -- line comment\n(col1 STRING)");
    }

    #[test]
    fn test_distribute_sort_with_subquery_in_columns() {
        // DISTRIBUTE BY + SORT BY with a subquery in the SORT columns
        let result = translator().translate(
            "SELECT * FROM t DISTRIBUTE BY col1 SORT BY (SELECT MAX(x) FROM y)",
        );
        assert!(result.success);
        assert!(result.sql.contains("ORDER BY"));
        assert!(result.sql.contains("(SELECT MAX(x) FROM y)"));
    }

    #[test]
    fn test_string_literal_masking_type_mapping() {
        // STRING inside a string literal should not be translated to VARCHAR
        let result = translator().translate(
            "SELECT * FROM t WHERE col1 = 'STRING(10)'",
        );
        assert!(result.success);
        assert_eq!(result.sql, "SELECT * FROM t WHERE col1 = 'STRING(10)'");
    }

    #[test]
    fn test_stored_as_inside_string_literal_not_stripped() {
        // STORED AS inside a string literal must NOT be stripped
        let result = translator().translate(
            "INSERT INTO t VALUES (1, 'this should STORED AS is fine')",
        );
        assert!(result.success);
        assert_eq!(
            result.sql,
            "INSERT INTO t VALUES (1, 'this should STORED AS is fine')"
        );
    }

    // ── Edge case tests ──

    #[test]
    fn test_complex_partitioned_by_with_many_columns() {
        let result = translator().translate(
            "CREATE TABLE t (col1 STRING) PARTITIONED BY (ds STRING, hr STRING, region STRING)",
        );
        assert!(result.success);
        assert_eq!(
            result.sql,
            "CREATE TABLE t (col1 STRING, ds STRING, hr STRING, region STRING)"
        );
        assert!(
            result.warnings.iter().any(|w| w.contains("PARTITIONED BY")),
            "Expected warning about PARTITIONED BY, got: {:?}",
            result.warnings
        );
    }

    #[test]
    fn test_create_table_with_all_mc_types() {
        assert_translated(
            "CREATE TABLE t (a BIGINT, b INT, c SMALLINT, d TINYINT, e STRING, f DOUBLE, g FLOAT, h DECIMAL(10,2), i BOOLEAN, j DATETIME, k DATE, l TIMESTAMP, m BINARY)",
            "CREATE TABLE t (a BIGINT, b INT, c SMALLINT, d TINYINT, e STRING, f DOUBLE, g FLOAT, h DECIMAL(10,2), i BOOLEAN, j DATETIME, k DATE, l TIMESTAMP, m BINARY)",
        );
    }

    #[test]
    fn test_insert_overwrite_preserves_complex_select() {
        let result = translator().translate(
            "INSERT OVERWRITE TABLE t PARTITION(ds='2024') SELECT a, b, c FROM (SELECT x.id, x.name, y.val FROM x JOIN y ON x.id = y.id WHERE x.active = 1) sub",
        );
        assert!(result.success);
        assert_eq!(
            result.sql,
            "INSERT INTO TABLE t SELECT a, b, c FROM (SELECT x.id, x.name, y.val FROM x JOIN y ON x.id = y.id WHERE x.active = 1) sub"
        );
        assert!(
            result.warnings.iter().any(|w| w.contains("INSERT OVERWRITE")),
            "Expected warning about INSERT OVERWRITE, got: {:?}",
            result.warnings
        );
    }

    #[test]
    fn test_set_with_and_without_equals() {
        // SET with equals sign and value -> no-op
        let result1 = translator().translate("SET odps.sql.allow.fullscan=true");
        assert!(result1.success);
        assert!(
            result1.sql.is_empty(),
            "SET with = value should be no-op, got: '{}'",
            result1.sql
        );

        // SET without equals sign and value -> not treated as no-op, passes through as SET statement
        let result2 = translator().translate("SET odps.sql.allow.fullscan");
        assert!(result2.success);
        assert_eq!(
            result2.sql,
            "SET odps.sql.allow.fullscan",
            "SET without value should pass through as SET statement"
        );
    }

    // ── Data Lake Scenario Tests ─────────────────────────────────────────
    //
    // These tests verify end-to-end SQL translation for realistic MaxCompute
    // data lake workloads. They test the full translation pipeline (not just
    // individual sub-operations) on production-like SQL.

    mod data_lake_tests {
        use super::*;

        #[test]
        fn test_data_lake_create_partitioned_table() {
            let result = translator().translate(
                "CREATE TABLE IF NOT EXISTS user_events (\
                     user_id BIGINT COMMENT '用户ID', \
                     event_type STRING COMMENT '事件类型', \
                     event_time DATETIME COMMENT '事件时间', \
                     properties STRING COMMENT '属性JSON' \
                 ) PARTITIONED BY (ds STRING COMMENT '日期分区') \
                 LIFECYCLE 365 \
                 COMMENT '用户事件表'",
            );
            assert!(result.success, "Failed: {:?}", result.error);
            assert_eq!(
                result.sql,
                "CREATE TABLE IF NOT EXISTS user_events (\
                 user_id BIGINT COMMENT '用户ID', \
                 event_type STRING COMMENT '事件类型', \
                 event_time DATETIME COMMENT '事件时间', \
                 properties STRING COMMENT '属性JSON', \
                 ds STRING COMMENT '日期分区') COMMENT '用户事件表'"
            );
            assert!(
                result.warnings.iter().any(|w| w.contains("PARTITIONED BY")),
                "Expected warning about PARTITIONED BY, got: {:?}",
                result.warnings
            );
            assert!(
                result.warnings.iter().any(|w| w.contains("LIFECYCLE")),
                "Expected warning about LIFECYCLE, got: {:?}",
                result.warnings
            );
        }

        #[test]
        fn test_data_lake_insert_overwrite_partition() {
            let result = translator().translate(
                "INSERT OVERWRITE TABLE user_events PARTITION(ds='2024-01-01') \
                 SELECT user_id, event_type, event_time, properties \
                 FROM staging_events \
                 WHERE ds = '2024-01-01'",
            );
            assert!(result.success, "Failed: {:?}", result.error);
            assert_eq!(
                result.sql,
                "INSERT INTO TABLE user_events \
                 SELECT user_id, event_type, event_time, properties \
                 FROM staging_events \
                 WHERE ds = '2024-01-01'"
            );
            assert!(
                result.warnings.iter().any(|w| w.contains("INSERT OVERWRITE")),
                "Expected warning about INSERT OVERWRITE, got: {:?}",
                result.warnings
            );
            assert!(
                result.warnings.iter().any(|w| w.contains("PARTITION")),
                "Expected warning about PARTITION, got: {:?}",
                result.warnings
            );
        }

        #[test]
        fn test_data_lake_complex_query() {
            let result = translator().translate(
                "SELECT /*+ MAPJOIN(b) */ \
                     a.user_id, \
                     a.event_type, \
                     b.user_name \
                 FROM user_events a \
                 JOIN user_dim b ON a.user_id = b.user_id \
                 WHERE a.ds = '2024-01-01' \
                 DISTRIBUTE BY a.user_id \
                 SORT BY a.event_time DESC",
            );
            assert!(result.success, "Failed: {:?}", result.error);
            assert!(!result.sql.contains("MAPJOIN"), "MAPJOIN hint should be stripped");
            assert!(result.sql.contains("ORDER BY"), "Should contain ORDER BY");
            assert!(result.sql.contains("a.event_time DESC"), "Should preserve sort column and direction");
            assert!(
                result.warnings.iter().any(|w| w.contains("MAPJOIN")),
                "Expected warning about MAPJOIN hint, got: {:?}",
                result.warnings
            );
            assert!(
                result.warnings.iter().any(|w| w.contains("DISTRIBUTE BY")),
                "Expected warning about DISTRIBUTE BY, got: {:?}",
                result.warnings
            );
        }

        #[test]
        fn test_data_lake_set_statements() {
            // All SET / SETPROJECT statements should be no-ops
            assert_noop("SET odps.sql.allow.fullscan=true;");
            assert_noop("SET odps.sql.type.system.odps2=true;");
            assert_noop("SET project.name=my_project;");
            assert_noop("SET hive.exec.dynamic.partition=true;");
            assert_noop("SETPROJECT odps.instance.priority=1;");
        }

        #[test]
        fn test_data_lake_etl_workflow() {
            // Simulate a complete ETL workflow translating each statement individually.
            //
            // Step 1: SET statement (no-op)
            {
                let result = translator().translate("SET odps.sql.allow.fullscan=true;");
                assert!(result.success, "SET should be no-op");
                assert!(
                    result.sql.is_empty(),
                    "SET should produce empty SQL"
                );
            }

            // Step 2: CREATE TABLE with PARTITIONED BY and LIFECYCLE
            {
                let result = translator().translate(
                    "CREATE TABLE IF NOT EXISTS etl_results (\
                         id BIGINT COMMENT '主键', \
                         name STRING COMMENT '名称', \
                         created_at DATETIME \
                     ) PARTITIONED BY (ds STRING) \
                     LIFECYCLE 90",
                );
                assert!(result.success, "CREATE TABLE failed: {:?}", result.error);
                assert_eq!(
                    result.sql,
                    "CREATE TABLE IF NOT EXISTS etl_results (\
                     id BIGINT COMMENT '主键', \
                     name STRING COMMENT '名称', \
                     created_at DATETIME, \
                     ds STRING)"
                );
                assert!(
                    result.warnings.iter().any(|w| w.contains("PARTITIONED BY")),
                    "Expected PARTITIONED BY warning"
                );
                assert!(
                    result.warnings.iter().any(|w| w.contains("LIFECYCLE")),
                    "Expected LIFECYCLE warning"
                );
            }

            // Step 3: INSERT OVERWRITE ... PARTITION -> INSERT INTO
            {
                let result = translator().translate(
                    "INSERT OVERWRITE TABLE etl_results PARTITION(ds='2024-06-01') \
                     SELECT id, name, created_at FROM raw_source WHERE ds = '2024-06-01'",
                );
                assert!(result.success, "INSERT failed: {:?}", result.error);
                assert_eq!(
                    result.sql,
                    "INSERT INTO TABLE etl_results \
                     SELECT id, name, created_at FROM raw_source WHERE ds = '2024-06-01'"
                );
                assert!(
                    result.warnings.iter().any(|w| w.contains("INSERT OVERWRITE")),
                    "Expected INSERT OVERWRITE warning"
                );
                assert!(
                    result.warnings.iter().any(|w| w.contains("PARTITION")),
                    "Expected PARTITION warning"
                );
            }

            // Step 4: SELECT with DISTRIBUTE BY + SORT BY -> ORDER BY
            {
                let result = translator().translate(
                    "SELECT * FROM etl_results DISTRIBUTE BY id SORT BY created_at DESC",
                );
                assert!(result.success, "SELECT failed: {:?}", result.error);
                assert_eq!(
                    result.sql,
                    "SELECT * FROM etl_results ORDER BY created_at DESC"
                );
                assert!(
                    result.warnings.iter().any(|w| w.contains("DISTRIBUTE BY")),
                    "Expected DISTRIBUTE BY warning"
                );
            }
        }

        #[test]
        fn test_data_lake_stored_as_formats() {
            let result = translator().translate(
                "CREATE TABLE ext_logs (col1 STRING, col2 BIGINT) \
                 STORED AS INPUTFORMAT 'com.hadoop.mapred.TextInputFormat' \
                 OUTPUTFORMAT 'com.hadoop.mapred.TextOutputFormat' \
                 LIFECYCLE 30",
            );
            assert!(result.success, "Failed: {:?}", result.error);
            assert_eq!(
                result.sql,
                "CREATE TABLE ext_logs (col1 STRING, col2 BIGINT)"
            );
            assert!(
                result.warnings.iter().any(|w| w.contains("STORED AS")),
                "Expected warning about STORED AS, got: {:?}",
                result.warnings
            );
            assert!(
                result.warnings.iter().any(|w| w.contains("LIFECYCLE")),
                "Expected warning about LIFECYCLE, got: {:?}",
                result.warnings
            );
        }
    }
}