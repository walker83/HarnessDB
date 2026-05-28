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
