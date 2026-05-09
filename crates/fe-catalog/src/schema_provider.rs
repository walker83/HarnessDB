//! Schema provider implementation for SQL parser integration.
//!
//! This module provides adapters that connect RorisDB's CatalogManager
//! to a SchemaProvider interface compatible with SQL parsers.

use std::sync::Arc;

use crate::catalog::CatalogManager;
use crate::table::Table;
use types::{Field, Schema};

/// Table type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableType {
    /// A regular physical table
    Base,
    /// A virtual view defined by a query
    View,
    /// A temporary table
    Temporary,
}

/// Filter pushdown support
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterPushDown {
    /// The provider cannot support filter pushdown
    Unsupported,
    /// The provider can do selection pushdown
    Selection,
    /// The provider can do all filter pushdown
    Exact,
}

/// Expression type for filter expressions - simplified version
#[derive(Debug, Clone)]
pub struct Expression;

impl Expression {
    pub fn eq(&self, _other: &Expression) -> Expression {
        Expression
    }
    pub fn not_eq(&self, _other: &Expression) -> Expression {
        Expression
    }
    pub fn lt(&self, _other: &Expression) -> Expression {
        Expression
    }
    pub fn lt_eq(&self, _other: &Expression) -> Expression {
        Expression
    }
    pub fn gt(&self, _other: &Expression) -> Expression {
        Expression
    }
    pub fn gt_eq(&self, _other: &Expression) -> Expression {
        Expression
    }
    pub fn and(&self, _other: &Expression) -> Expression {
        Expression
    }
    pub fn or(&self, _other: &Expression) -> Expression {
        Expression
    }
    pub fn not(&self) -> Expression {
        Expression
    }
    pub fn is_null(&self) -> Expression {
        Expression
    }
    pub fn is_not_null(&self) -> Expression {
        Expression
    }
    pub fn in_list(&self, _list: Vec<Expression>) -> Expression {
        Expression
    }
}

/// Table provider trait - similar to datafusion::catalog::TableProvider
pub trait TableProvider: Send + Sync {
    /// Returns the table schema
    fn schema(&self) -> &Schema;

    /// Returns the table type
    fn table_type(&self) -> TableType;

    /// Scan the table to get a stream of record batches
    /// For now, returns NotImplemented as actual scan requires execution layer
    fn scan(
        &self,
        _projection: Option<&[usize]>,
        _filters: &[Expression],
        _limit: Option<usize>,
    ) -> Result<Arc<dyn TableProvider>, String> {
        Err("Scan not implemented - requires execution plan integration".to_string())
    }

    /// Returns support for filter pushdown
    fn supports_filter_pushdown(&self, _filter: &Expression) -> Result<FilterPushDown, String> {
        Ok(FilterPushDown::Unsupported)
    }
}

/// Schema provider trait - similar to datafusion::catalog::SchemaProvider
pub trait SchemaProvider {
    /// Returns the table with the given name, or None if it doesn't exist
    fn table(&self, name: &str) -> Option<Arc<dyn TableProvider>>;

    /// Returns all table names in this schema
    fn table_names(&self) -> Vec<String>;
}

/// RorisSchemaProvider implements SchemaProvider trait.
/// It provides table metadata to the SQL parser for query planning.
pub struct RorisSchemaProvider {
    catalog: Arc<CatalogManager>,
}

impl RorisSchemaProvider {
    pub fn new(catalog: Arc<CatalogManager>) -> Self {
        Self { catalog }
    }
}

impl SchemaProvider for RorisSchemaProvider {
    fn table(&self, name: &str) -> Option<Arc<dyn TableProvider>> {
        // Search all databases for the table
        for db_name in self.catalog.list_databases() {
            if let Some(table) = self.catalog.get_table(&db_name, name) {
                return Some(Arc::new(RorisTableProvider::new(table)));
            }
        }
        None
    }

    fn table_names(&self) -> Vec<String> {
        let mut names = Vec::new();
        for db_name in self.catalog.list_databases() {
            if let Some(tables) = self.catalog.list_tables(&db_name) {
                names.extend(tables);
            }
        }
        names
    }
}

/// RorisTableProvider implements TableProvider trait.
/// It wraps a RorisDB Table and provides the necessary metadata for query execution.
pub struct RorisTableProvider {
    table: Table,
    schema: Schema,
}

impl RorisTableProvider {
    pub fn new(table: Table) -> Self {
        let schema = Self::to_schema(&table);
        Self { table, schema }
    }

    /// Convert RorisDB Table to Schema
    fn to_schema(table: &Table) -> Schema {
        let fields: Vec<Field> = table
            .columns
            .iter()
            .map(|col| Field {
                name: col.name.clone(),
                data_type: col.data_type.clone(),
                nullable: col.nullable,
            })
            .collect();

        Schema::new(fields)
    }

    /// Get the underlying table reference
    pub fn table(&self) -> &Table {
        &self.table
    }
}

impl TableProvider for RorisTableProvider {
    fn schema(&self) -> &Schema {
        &self.schema
    }

    fn table_type(&self) -> TableType {
        if self.table.view_definition.is_some() {
            TableType::View
        } else {
            TableType::Base
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::table::{KeysType, TableColumn};
    use types::DataType;
    use std::collections::HashMap;

    fn create_test_table() -> Table {
        Table {
            id: 1,
            name: "test_table".to_string(),
            database: "test_db".to_string(),
            columns: vec![
                TableColumn {
                    name: "id".to_string(),
                    data_type: DataType::Int64,
                    nullable: false,
                    default_value: None,
                    agg_type: None,
                    comment: "Primary key".to_string(),
                },
                TableColumn {
                    name: "name".to_string(),
                    data_type: DataType::String,
                    nullable: true,
                    default_value: None,
                    agg_type: None,
                    comment: "User name".to_string(),
                },
                TableColumn {
                    name: "age".to_string(),
                    data_type: DataType::Int32,
                    nullable: false,
                    default_value: Some("18".to_string()),
                    agg_type: None,
                    comment: "User age".to_string(),
                },
            ],
            keys_type: KeysType::Duplicate,
            unique_keys: vec![],
            partition_info: None,
            distribution_info: None,
            replication_num: 1,
            properties: HashMap::new(),
            row_count: 1000,
            data_size: 10000,
            stats: None,
            view_definition: None,
        }
    }

    #[test]
    fn test_roris_table_provider_schema() {
        let table = create_test_table();
        let provider = RorisTableProvider::new(table);

        let schema = provider.schema();
        assert_eq!(schema.num_fields(), 3);
        assert_eq!(schema.field(0).unwrap().name, "id");
        assert_eq!(schema.field(1).unwrap().name, "name");
        assert_eq!(schema.field(2).unwrap().name, "age");
    }

    #[test]
    fn test_roris_table_provider_table_type() {
        let table = create_test_table();
        let provider = RorisTableProvider::new(table);

        assert_eq!(provider.table_type(), TableType::Base);
    }

    #[test]
    fn test_roris_schema_provider_table_lookup() {
        let catalog = Arc::new(CatalogManager::new());
        catalog.create_database("test_db").unwrap();
        catalog
            .create_table("test_db", create_test_table())
            .unwrap();

        let schema_provider = RorisSchemaProvider::new(catalog);

        // Should find the table
        let table_provider = schema_provider.table("test_table");
        assert!(table_provider.is_some());

        // Should not find non-existent table
        let table_provider = schema_provider.table("nonexistent");
        assert!(table_provider.is_none());
    }

    #[test]
    fn test_roris_schema_provider_table_names() {
        let catalog = Arc::new(CatalogManager::new());
        catalog.create_database("test_db").unwrap();
        catalog
            .create_table("test_db", create_test_table())
            .unwrap();

        let schema_provider = RorisSchemaProvider::new(catalog);
        let names = schema_provider.table_names();

        assert!(names.contains(&"test_table".to_string()));
    }

    #[test]
    fn test_roris_table_provider_access_table() {
        let table = create_test_table();
        let provider = RorisTableProvider::new(table);

        let table_ref = provider.table();
        assert_eq!(table_ref.name, "test_table");
        assert_eq!(table_ref.database, "test_db");
    }
}
