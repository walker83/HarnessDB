//! SchemaProvider for DataFusion integration
//!
//! This module provides RorisSchemaProvider and RorisTableProvider which implement
//! DataFusion's SchemaProvider and TableProvider traits. These allow DataFusion's
//! SQL parser to query table metadata from RorisDB's catalog.

use std::sync::Arc;

use arrow::datatypes::{DataType as ArrowDataType, Field, Schema as ArrowSchema, SchemaRef};
use datafusion_common::DataFusionError;
use datafusion_expr::{Expr, PhysicalPlan};
use datafusion_sql::{
    planner::PlannerState,
    schema::{SchemaProvider, TableProvider},
};

use crate::{CatalogManager, Table, TableColumn};

/// RorisSchemaProvider implements DataFusion's SchemaProvider trait
/// to expose RorisDB's catalog to DataFusion's SQL parser.
#[derive(Debug)]
pub struct RorisSchemaProvider {
    catalog: Arc<CatalogManager>,
}

impl RorisSchemaProvider {
    pub fn new(catalog: Arc<CatalogManager>) -> Self {
        Self { catalog }
    }

    /// Get a table by name from the catalog
    pub fn get_table(&self, name: &str) -> Option<Table> {
        for db_name in self.catalog.list_databases() {
            if let Some(table) = self.catalog.get_table(&db_name, name) {
                return Some(table);
            }
        }
        None
    }

    /// List all table names across all databases
    pub fn table_names(&self) -> Vec<String> {
        self.catalog
            .list_databases()
            .into_iter()
            .flat_map(|db| self.catalog.list_tables(&db).unwrap_or_default())
            .collect()
    }

    /// Check if a table exists
    pub fn table_exists(&self, name: &str) -> bool {
        self.get_table(name).is_some()
    }
}

impl SchemaProvider for RorisSchemaProvider {
    fn get_table(&self, name: &str) -> Option<Arc<dyn TableProvider>> {
        self.get_table(name).map(|t| Arc::new(RorisTableProvider::new(t)) as Arc<dyn TableProvider>)
    }

    fn table_names(&self) -> Vec<String> {
        RorisSchemaProvider::table_names(self)
    }

    fn table_exists(&self, name: &str) -> bool {
        self.table_exists(name)
    }
}

/// RorisTableProvider wraps a RorisDB Table for DataFusion integration.
#[derive(Debug)]
pub struct RorisTableProvider {
    table: Table,
}

impl RorisTableProvider {
    pub fn new(table: Table) -> Self {
        Self { table }
    }

    /// Get the underlying table
    pub fn table(&self) -> &Table {
        &self.table
    }

    /// Convert RorisDB DataType to Arrow DataType
    fn roris_type_to_arrow(data_type: &types::DataType) -> ArrowDataType {
        use types::DataType;
        match data_type {
            DataType::Null => ArrowDataType::Null,
            DataType::Boolean => ArrowDataType::Boolean,
            DataType::Int8 => ArrowDataType::Int8,
            DataType::Int16 => ArrowDataType::Int16,
            DataType::Int32 => ArrowDataType::Int32,
            DataType::Int64 => ArrowDataType::Int64,
            DataType::Int128 => ArrowDataType::Int64, // Arrow doesn't have Int128, map to Int64
            DataType::Float32 => ArrowDataType::Float32,
            DataType::Float64 => ArrowDataType::Float64,
            DataType::Decimal(_) => ArrowDataType::Float64, // Approximate with Float64
            DataType::Date => ArrowDataType::Date32,
            DataType::DateTime => ArrowDataType::Date64,
            DataType::Varchar(_) | DataType::Char(_) | DataType::String => ArrowDataType::Utf8,
            DataType::Binary => ArrowDataType::Binary,
            DataType::Array(inner) => ArrowDataType::List(Box::new(arrow::datatypes::Field::new(
                "item",
                Self::roris_type_to_arrow(inner),
                true, // arrays are nullable
            ))),
            DataType::Map(_, _) => ArrowDataType::Binary, // Map not directly supported
            DataType::Struct(fields) => ArrowDataType::Struct(
                fields
                    .iter()
                    .map(|f| {
                        arrow::datatypes::Field::new(
                            &f.name,
                            Self::roris_type_to_arrow(&f.data_type),
                            f.nullable,
                        )
                    })
                    .collect(),
            ),
            DataType::Json => ArrowDataType::Utf8, // Represent JSON as string
            DataType::Float32Vector(_) => ArrowDataType::Binary, // Vector type as binary
        }
    }

    /// Convert TableColumn to Arrow Field
    fn column_to_field(column: &TableColumn) -> Field {
        Field::new(
            &column.name,
            Self::roris_type_to_arrow(&column.data_type),
            column.nullable,
        )
    }
}

impl TableProvider for RorisTableProvider {
    /// Returns the schema for this table, with each column mapped to Arrow DataType.
    fn schema(&self) -> SchemaRef {
        let fields: Vec<Field> = self.table.columns.iter().map(Self::column_to_field).collect();
        ArrowSchema::new(fields).into()
    }

    /// Create a physical scan plan for this table.
    ///
    /// Currently returns an error indicating full execution engine integration is needed.
    /// The scan requires connecting to the BE storage layer to read actual table data.
    fn scan(
        &self,
        _state: &PlannerState,
        _pushdown_filter: Option<Expr>,
        _pushdown_projection: Option<&[usize]>,
        _limit: Option<usize>,
    ) -> Result<Result<PhysicalPlan, DataFusionError>, DataFusionError> {
        Err(DataFusionError::Internal(
            "RorisTableProvider::scan() requires execution engine integration. \
             The full scan implementation needs to connect to the BE storage layer \
             to read actual table data from tablets.".to_string(),
        ))
    }
}