//! Semantic Analyzer for RorisDB SQL
//!
//! This module provides semantic analysis capabilities including:
//! - Symbol table for column resolution
//! - Type checking
//! - Column reference resolution
//!
//! Note: DataFusion integration is pending version compatibility resolution.

use std::collections::HashMap;
use thiserror::Error;

/// Analysis errors
#[derive(Error, Debug)]
pub enum AnalysisError {
    #[error("Column not found: {0}")]
    ColumnNotFound(String),
    #[error("Table not found: {0}")]
    TableNotFound(String),
    #[error("Type mismatch: {0}")]
    TypeMismatch(String),
    #[error("Ambiguous column: {0}")]
    AmbiguousColumn(String),
}

/// A column with its resolved type
#[derive(Clone, Debug)]
pub struct ResolvedColumn {
    pub name: String,
    pub table: Option<String>,
    pub data_type: String,  // Type name as string until Arrow types are available
}

/// Symbol table for tracking column references during query analysis
pub struct SymbolTable {
    columns: HashMap<(Option<String>, String), ResolvedColumn>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            columns: HashMap::new(),
        }
    }

    /// Add a column to the symbol table
    pub fn add_column(&mut self, table: Option<String>, column: ResolvedColumn) {
        self.columns.insert((table, column.name.clone()), column);
    }

    /// Resolve a column reference by name and optional table alias
    pub fn resolve_column(
        &self,
        table_alias: &Option<String>,
        col_name: &str,
    ) -> Result<ResolvedColumn, AnalysisError> {
        // If table alias provided, look up with that first
        if let Some(alias) = table_alias {
            if let Some(col) = self
                .columns
                .get(&(Some(alias.clone()), col_name.to_string()))
            {
                return Ok(col.clone());
            }
        }

        // Otherwise search all tables for this column name
        for ((_t, name), col) in &self.columns {
            if name == col_name {
                return Ok(col.clone());
            }
        }

        Err(AnalysisError::ColumnNotFound(col_name.to_string()))
    }

    /// Get the number of columns in the symbol table
    pub fn len(&self) -> usize {
        self.columns.len()
    }

    /// Check if symbol table is empty
    pub fn is_empty(&self) -> bool {
        self.columns.is_empty()
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

/// Typed expression with type information
#[derive(Clone, Debug)]
pub struct TypedExpr {
    pub expr: String,  // Serialized expression until DataFusion is integrated
    pub data_type: String,
}

/// Placeholder for SELECT statement analysis
pub struct Select;

impl Select {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Select {
    fn default() -> Self {
        Self::new()
    }
}

/// Placeholder for typed SELECT result
pub struct TypedSelect;

impl TypedSelect {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TypedSelect {
    fn default() -> Self {
        Self::new()
    }
}

/// Analyzer for semantic analysis of SQL statements
pub struct Analyzer {
    symbol_table: SymbolTable,
}

impl Analyzer {
    pub fn new() -> Self {
        Self {
            symbol_table: SymbolTable::new(),
        }
    }

    /// Analyze a SELECT statement
    ///
    /// Steps:
    /// 1. Analyze FROM clause to populate symbol table with table columns
    /// 2. Analyze SELECT list using symbol table
    /// 3. Analyze WHERE clause
    /// 4. Return typed select with all type information
    pub fn analyze_select(&mut self, _select: &Select) -> Result<TypedSelect, AnalysisError> {
        // TODO: Implement full analysis when DataFusion is integrated
        // For now, return empty result
        Ok(TypedSelect::new())
    }

    /// Get a mutable reference to the symbol table for building
    pub fn symbol_table_mut(&mut self) -> &mut SymbolTable {
        &mut self.symbol_table
    }

    /// Get a reference to the symbol table
    pub fn symbol_table(&self) -> &SymbolTable {
        &self.symbol_table
    }
}

impl Default for Analyzer {
    fn default() -> Self {
        Self::new()
    }
}