//! RocksDB-based metadata storage for RorisDB.
//!
//! This crate provides a RocksDB backend for storing:
//! - Catalog metadata (databases, tables)
//! - Tablet metadata (schemas, rowsets)
//! - Edit log (WAL for catalog changes)
//!
//! # Column Families
//!
//! Three column families are used:
//! - `catalog`: Database and table metadata, atomic ID counter
//! - `tablet`: Tablet schemas, rowset metadata, segment counters
//! - `edit_log`: Write-ahead log entries for recovery
//!
//! # Data Types
//!
//! This crate defines its own data types that mirror those in fe-catalog and be-storage
//! to avoid cyclic dependencies. Types can be converted between crates when needed.

mod meta_store;
mod catalog_store;

pub use meta_store::{MetaStore, RocksStoreError, Result};
pub use catalog_store::{CatalogStore, Database, Table, TableColumn, KeysType};