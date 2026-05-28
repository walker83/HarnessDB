//! SQL dialect translators for RorisDB database chameleon.
//!
//! This crate provides SQL translation between different database dialects
//! and RorisDB's internal SQL format. Each translator handles the specific
//! syntax differences of a target database.
//!
//! # Supported Dialects
//! - MaxCompute: Strip PARTITIONED BY, LIFECYCLE, STORED AS, MAPJOIN hints
//! - Hologres: Handle WITH table properties, set_table_property, PG type mapping

pub mod hologres;
pub mod maxcompute;

pub use hologres::HologresTranslator;
pub use maxcompute::MaxComputeTranslator;

/// Trait for SQL dialect translators.
pub trait DialectTranslator {
    /// Translate SQL from the target dialect to RorisDB-compatible SQL.
    fn translate(&self, sql: &str) -> TranslateResult;

    /// Return the name of this dialect.
    fn dialect_name(&self) -> &str;

    /// Return a list of features not supported by this dialect.
    fn unsupported_features(&self) -> &[&str];
}

/// Result of SQL translation.
#[derive(Debug, Clone)]
pub struct TranslateResult {
    /// The translated SQL string.
    pub sql: String,
    /// Whether the translation was successful.
    pub success: bool,
    /// Error message if translation failed.
    pub error: Option<String>,
    /// Warnings generated during translation.
    pub warnings: Vec<String>,
}

impl TranslateResult {
    pub fn ok(sql: String) -> Self {
        Self {
            sql,
            success: true,
            error: None,
            warnings: Vec::new(),
        }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            sql: String::new(),
            success: false,
            error: Some(msg.into()),
            warnings: Vec::new(),
        }
    }

    pub fn with_warning(mut self, warning: impl Into<String>) -> Self {
        self.warnings.push(warning.into());
        self
    }

    /// Add multiple warnings to the result.
    pub fn with_warnings(mut self, warnings: Vec<String>) -> Self {
        self.warnings.extend(warnings);
        self
    }
}

// ── Shared Utility Functions ─────────────────────────────────────────────

/// Find the position of a matching closing parenthesis, starting from `start`.
/// `start` should point to the opening `(` character.
/// Returns `None` if no matching parenthesis is found (unbalanced).
pub(crate) fn find_matching_paren(s: &str, start: usize) -> Option<usize> {
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

/// Replace string literal contents (`'...'`) with placeholders (`__STRLITn__`)
/// to prevent regex operations from matching keywords inside string literals.
///
/// Handles escaped single quotes (`''`) inside strings.
/// Returns the masked SQL and a vector of original string literal contents.
pub(crate) fn mask_string_literals(sql: &str) -> (String, Vec<String>) {
    let mut result = String::with_capacity(sql.len());
    let mut strings = Vec::new();
    let bytes = sql.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'\'' {
            // Start of string literal
            let start = i;
            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'\'' {
                    // Could be end of string or escaped quote ('')
                    if i + 1 < bytes.len() && bytes[i + 1] == b'\'' {
                        // Escaped quote - skip both
                        i += 2;
                    } else {
                        break; // End of string literal
                    }
                } else {
                    i += 1;
                }
            }
            if i < bytes.len() {
                i += 1; // Skip closing quote
            }
            strings.push(sql[start..i].to_string());
            result.push_str(&format!("__STRLIT{}__", strings.len() - 1));
        } else {
            result.push(bytes[i] as char);
            i += 1;
        }
    }

    (result, strings)
}

/// Restore original string literals that were masked by `mask_string_literals`.
pub(crate) fn restore_string_literals(sql: &str, strings: &[String]) -> String {
    let mut result = sql.to_string();
    // Restore in reverse order to avoid placeholder overlap issues
    for (i, s) in strings.iter().enumerate().rev() {
        result = result.replace(&format!("__STRLIT{}__", i), s);
    }
    result
}

/// Split a SQL fragment by commas that are not inside parentheses.
pub(crate) fn split_by_commas_outside_parens(s: &str) -> Vec<&str> {
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
