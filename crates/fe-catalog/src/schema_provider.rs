//! SchemaProvider for DataFusion integration
//!
//! This module provides RorisSchemaProvider and RorisTableProvider which implement
//! DataFusion's SchemaProvider and TableProvider traits. These allow DataFusion's
//! SQL parser to query table metadata from RorisDB's catalog.
//!
//! Note: The actual DataFusion integration requires matching versions across the workspace.
//! This module is structured to be integrated once the version alignment is resolved.

use std::sync::Arc;

use crate::{CatalogManager, Table};

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

/// RorisTableProvider wraps a RorisDB Table for DataFusion integration.
/// Currently a placeholder - full integration requires Arrow schema mapping.
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
}