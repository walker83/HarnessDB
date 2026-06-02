//! MaxCompute Tunnel Schema types.
//!
//! Defines the schema representation used by Tunnel sessions, with conversion
//! between MaxCompute (ODPS) type names and the MySQL types used internally.

/// A single column in a Tunnel schema.
#[derive(Debug, Clone)]
pub struct TunnelColumn {
    pub name: String,
    /// MaxCompute type name (e.g. "BIGINT", "STRING", "DOUBLE")
    pub odps_type: String,
    pub nullable: bool,
    pub comment: Option<String>,
}

/// A complete Tunnel schema (columns + optional partition keys).
#[derive(Debug, Clone)]
pub struct TunnelSchema {
    pub columns: Vec<TunnelColumn>,
    pub partition_keys: Vec<TunnelColumn>,
}

impl TunnelSchema {
    /// Create an empty schema.
    pub fn empty() -> Self {
        Self {
            columns: Vec::new(),
            partition_keys: Vec::new(),
        }
    }

    /// All columns including partition keys.
    pub fn all_columns(&self) -> Vec<&TunnelColumn> {
        self.columns.iter().chain(self.partition_keys.iter()).collect()
    }

    /// Column count for serialization (all columns).
    pub fn column_count(&self) -> usize {
        self.columns.len() + self.partition_keys.len()
    }
}

impl Default for TunnelSchema {
    fn default() -> Self {
        Self::empty()
    }
}

// ============================================================================
// Type mapping
// ============================================================================

/// Map a MySQL type name (from DESCRIBE) to a MaxCompute type name.
pub fn mysql_to_odps_type(mysql_type: &str) -> &str {
    match mysql_type.to_uppercase().as_str() {
        "BIGINT" | "BIGINT UNSIGNED" => "BIGINT",
        "INT" | "INTEGER" | "MEDIUMINT" => "INT",
        "SMALLINT" => "SMALLINT",
        "TINYINT" => "TINYINT",
        "VARCHAR" | "TEXT" | "CHAR" | "MEDIUMTEXT" | "LONGTEXT" => "STRING",
        "FLOAT" => "FLOAT",
        "DOUBLE" | "REAL" => "DOUBLE",
        "DECIMAL" | "NUMERIC" => "DECIMAL",
        "BOOLEAN" | "BOOL" => "BOOLEAN",
        "DATETIME" | "TIMESTAMP" => "DATETIME",
        "DATE" => "DATE",
        "BLOB" | "VARBINARY" | "BINARY" => "BINARY",
        // Default fallback
        other => {
            tracing::debug!("Unknown MySQL type for ODPS mapping: {}, falling back to STRING", other);
            "STRING"
        }
    }
}

/// Map a MaxCompute type name to the MySQL type string used internally.
pub fn odps_to_mysql_type(odps_type: &str) -> &str {
    match odps_type.to_uppercase().as_str() {
        "BIGINT" => "BIGINT",
        "INT" => "INT",
        "SMALLINT" => "SMALLINT",
        "TINYINT" => "TINYINT",
        "STRING" | "VARCHAR" | "CHAR" | "TEXT" => "VARCHAR",
        "FLOAT" => "FLOAT",
        "DOUBLE" | "REAL" => "DOUBLE",
        "DECIMAL" | "NUMERIC" => "DECIMAL",
        "BOOLEAN" | "BOOL" => "BOOLEAN",
        "DATETIME" | "TIMESTAMP" => "DATETIME",
        "DATE" => "DATE",
        "BINARY" | "BLOB" | "VARBINARY" => "BLOB",
        // Complex types — passed through as-is
        "ARRAY" | "MAP" | "STRUCT" | "JSON" => odps_type,
        other => {
            tracing::debug!("Unknown ODPS type, falling back to STRING: {}", other);
            "VARCHAR"
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_empty() {
        let s = TunnelSchema::empty();
        assert!(s.columns.is_empty());
        assert!(s.partition_keys.is_empty());
        assert_eq!(s.column_count(), 0);
        assert!(s.all_columns().is_empty());
    }

    #[test]
    fn test_schema_with_columns() {
        let mut s = TunnelSchema::empty();
        s.columns.push(TunnelColumn {
            name: "id".into(),
            odps_type: "BIGINT".into(),
            nullable: false,
            comment: None,
        });
        s.columns.push(TunnelColumn {
            name: "name".into(),
            odps_type: "STRING".into(),
            nullable: true,
            comment: Some("user name".into()),
        });
        s.partition_keys.push(TunnelColumn {
            name: "ds".into(),
            odps_type: "STRING".into(),
            nullable: true,
            comment: None,
        });
        assert_eq!(s.column_count(), 3);
        assert_eq!(s.all_columns().len(), 3);
    }

    #[test]
    fn test_mysql_to_odps_numeric_types() {
        assert_eq!(mysql_to_odps_type("BIGINT"), "BIGINT");
        assert_eq!(mysql_to_odps_type("INT"), "INT");
        assert_eq!(mysql_to_odps_type("SMALLINT"), "SMALLINT");
        assert_eq!(mysql_to_odps_type("TINYINT"), "TINYINT");
        assert_eq!(mysql_to_odps_type("INTEGER"), "INT");
        assert_eq!(mysql_to_odps_type("MEDIUMINT"), "INT");
    }

    #[test]
    fn test_mysql_to_odps_string_types() {
        assert_eq!(mysql_to_odps_type("VARCHAR"), "STRING");
        assert_eq!(mysql_to_odps_type("TEXT"), "STRING");
        assert_eq!(mysql_to_odps_type("CHAR"), "STRING");
        assert_eq!(mysql_to_odps_type("MEDIUMTEXT"), "STRING");
        assert_eq!(mysql_to_odps_type("LONGTEXT"), "STRING");
    }

    #[test]
    fn test_mysql_to_odps_float_double() {
        assert_eq!(mysql_to_odps_type("FLOAT"), "FLOAT");
        assert_eq!(mysql_to_odps_type("DOUBLE"), "DOUBLE");
        assert_eq!(mysql_to_odps_type("REAL"), "DOUBLE");
    }

    #[test]
    fn test_mysql_to_odps_datetime() {
        assert_eq!(mysql_to_odps_type("DATETIME"), "DATETIME");
        assert_eq!(mysql_to_odps_type("TIMESTAMP"), "DATETIME");
        assert_eq!(mysql_to_odps_type("DATE"), "DATE");
    }

    #[test]
    fn test_mysql_to_odps_fallback() {
        assert_eq!(mysql_to_odps_type("GEOMETRY"), "STRING");
        assert_eq!(mysql_to_odps_type("UNKNOWN_TYPE"), "STRING");
    }

    #[test]
    fn test_odps_to_mysql_numeric_types() {
        assert_eq!(odps_to_mysql_type("BIGINT"), "BIGINT");
        assert_eq!(odps_to_mysql_type("INT"), "INT");
        assert_eq!(odps_to_mysql_type("SMALLINT"), "SMALLINT");
        assert_eq!(odps_to_mysql_type("TINYINT"), "TINYINT");
    }

    #[test]
    fn test_odps_to_mysql_string_types() {
        assert_eq!(odps_to_mysql_type("STRING"), "VARCHAR");
        assert_eq!(odps_to_mysql_type("VARCHAR"), "VARCHAR");
        assert_eq!(odps_to_mysql_type("CHAR"), "VARCHAR");
        assert_eq!(odps_to_mysql_type("TEXT"), "VARCHAR");
    }

    #[test]
    fn test_odps_to_mysql_datetime() {
        assert_eq!(odps_to_mysql_type("DATETIME"), "DATETIME");
        assert_eq!(odps_to_mysql_type("TIMESTAMP"), "DATETIME");
        assert_eq!(odps_to_mysql_type("DATE"), "DATE");
    }

    #[test]
    fn test_odps_to_mysql_complex_types() {
        assert_eq!(odps_to_mysql_type("ARRAY"), "ARRAY");
        assert_eq!(odps_to_mysql_type("MAP"), "MAP");
        assert_eq!(odps_to_mysql_type("STRUCT"), "STRUCT");
        assert_eq!(odps_to_mysql_type("JSON"), "JSON");
    }

    #[test]
    fn test_odps_to_mysql_fallback() {
        assert_eq!(odps_to_mysql_type("UNKNOWN"), "VARCHAR");
    }

    #[test]
    fn test_schema_default() {
        let s = TunnelSchema::default();
        assert!(s.columns.is_empty());
    }

    #[test]
    fn test_type_mapping_case_insensitive() {
        assert_eq!(mysql_to_odps_type("bigint"), "BIGINT");
        assert_eq!(mysql_to_odps_type("Bigint"), "BIGINT");
        assert_eq!(odps_to_mysql_type("string"), "VARCHAR");
        assert_eq!(odps_to_mysql_type("String"), "VARCHAR");
    }
}
