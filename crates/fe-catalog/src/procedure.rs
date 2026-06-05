//! Stored procedure metadata for T-SQL compatibility.

use serde::{Deserialize, Serialize};

/// Metadata for a stored procedure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredProcedure {
    pub id: u64,
    pub name: String,
    pub database: String,
    pub owner: String,
    pub create_time: i64,
    pub alter_time: i64,
    /// Original T-SQL source text of CREATE PROCEDURE.
    pub source_sql: String,
    pub params: Vec<ProcedureParamMeta>,
    pub is_recompiled: bool,
    pub is_encrypted: bool,
}

/// Parameter metadata for a stored procedure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcedureParamMeta {
    pub name: String,
    pub data_type: String,
    pub direction: String, // "IN", "OUT", "INOUT"
    pub default_value: Option<String>,
}
