//! DataFusion Parser Integration for RorisDB
//!
//! This module provides the RorisParser which wraps SQL parsing capabilities.
//! When DataFusion is available in the workspace, this will use DFParser for
//! enhanced SQL support. Currently structured as a placeholder.
//!
//! TODO: Re-add DataFusion dependencies once version compatibility is resolved:
//! - datafusion-sql = "42"
//! - datafusion-common = "42"
//! - datafusion-expr = "42"

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("SQL parsing failed: {0}")]
    Sql(String),
}

/// RorisParser wraps the SQL parser for use in RorisDB
///
/// This is a placeholder that uses the existing sqlparser-rs based approach.
/// Once DataFusion dependencies are properly integrated into the workspace,
/// this will delegate to DataFusion's DFParser for enhanced SQL support.
pub struct RorisParser;

impl RorisParser {
    pub fn new() -> Self {
        Self
    }

    /// Parse SQL text into statements
    ///
    /// Currently delegates to sqlparser-rs via the crate's parse_sql function.
    /// TODO: Switch to DataFusion's DFParser when dependencies are available.
    pub fn parse(sql: &str) -> Result<Vec<sqlparser::ast::Statement>, ParseError> {
        // This would use DFParser::parse_sql when DataFusion is integrated
        // For now, we return an error indicating DataFusion is not yet available
        Err(ParseError::Sql(
            "DataFusion parser not yet integrated. Use parse_sql from crate instead.".to_string()
        ))
    }
}

impl Default for RorisParser {
    fn default() -> Self {
        Self::new()
    }
}