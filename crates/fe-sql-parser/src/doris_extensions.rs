use sqlparser::ast::Statement;
use thiserror::Error;

/// Keys type for table creation
#[derive(Debug, Clone, PartialEq)]
pub enum KeysType {
    Aggregate,
    Unique,
    Primary,
    Duplicate,
}

/// Distribution definition
#[derive(Debug, Clone)]
pub struct DistributionDef {
    pub kind: DistributionKind,
    pub buckets: usize,
}

#[derive(Debug, Clone)]
pub enum DistributionKind {
    Hash(Vec<String>),
    Random,
}

/// Partition definition
#[derive(Debug, Clone)]
pub struct PartitionDef {
    pub kind: PartitionKind,
    pub columns: Vec<String>,
    // Partition bounds would go here
}

#[derive(Debug, Clone)]
pub enum PartitionKind {
    Range,
    List,
}

/// Doris extensions extracted from CREATE TABLE
#[derive(Debug, Clone)]
pub struct DorisExtensions {
    pub keys_type: KeysType,
    pub partition: Option<PartitionDef>,
    pub distribution: Option<DistributionDef>,
    pub properties: Vec<(String, String)>,
}

#[derive(Error, Debug)]
pub enum DorisExtensionError {
    #[error("Unsupported distribution type: {0}")]
    UnsupportedDistribution(String),
    #[error("Unsupported partition type: {0}")]
    UnsupportedPartition(String),
    #[error("Missing keyword: {0}")]
    MissingKeyword(String),
}

/// Parse Doris-specific extensions from a CREATE TABLE statement
pub fn parse_doris_extensions(stmt: &Statement) -> Result<DorisExtensions, DorisExtensionError> {
    match stmt {
        Statement::CreateTable(_create_table) => {
            // Look through the SQL text or parse additional tokens for Doris extensions
            // For now, return a default/empty extensions object
            Ok(DorisExtensions {
                keys_type: KeysType::Duplicate,
                partition: None,
                distribution: None,
                properties: vec![],
            })
        }
        _ => Ok(DorisExtensions {
            keys_type: KeysType::Duplicate,
            partition: None,
            distribution: None,
            properties: vec![],
        }),
    }
}