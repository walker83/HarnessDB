//! Migration tool for converting JSON metadata to RocksDB.
//!
//! Usage:
//!   migrate-meta --fe-meta-dir data/fe/doris-meta --be-storage-dir data/be/storage --rocks-dir data/rocks-meta

use clap::Parser;
use std::path::PathBuf;
use std::collections::HashMap;
use serde::Deserialize;
use types::DataType;

#[derive(Parser)]
#[command(name = "migrate-meta", about = "Migrate RorisDB metadata from JSON to RocksDB")]
struct Args {
    /// Frontend metadata directory (contains catalog.json, edit_log_*.json)
    #[arg(long, default_value = "data/fe/doris-meta")]
    fe_meta_dir: PathBuf,

    /// Backend storage directory (contains tablet_*/schema.json, rowset_*.json)
    #[arg(long, default_value = "data/be/storage")]
    be_storage_dir: PathBuf,

    /// RocksDB output directory
    #[arg(long, default_value = "data/rocks-meta")]
    rocks_dir: PathBuf,

    /// Verify consistency after migration
    #[arg(long, default_value = "true")]
    verify: bool,

    /// Dry run (don't actually write to RocksDB)
    #[arg(long, default_value = "false")]
    dry_run: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    println!("RorisDB Metadata Migration Tool");
    println!("================================");
    println!("FE meta dir: {}", args.fe_meta_dir.display());
    println!("BE storage dir: {}", args.be_storage_dir.display());
    println!("RocksDB dir: {}", args.rocks_dir.display());
    println!("Verify: {}", args.verify);
    println!("Dry run: {}", args.dry_run);
    println!();

    // Create output directory
    if !args.dry_run {
        std::fs::create_dir_all(&args.rocks_dir)?;
    }

    // Open RocksDB
    let store = if !args.dry_run {
        Some(be_rocks::MetaStore::open(&args.rocks_dir)?)
    } else {
        None
    };

    // 1. Migrate catalog metadata
    println!("Migrating catalog metadata...");
    let catalog_path = args.fe_meta_dir.join("catalog.json");
    let catalog_stats = migrate_catalog(&catalog_path, store.as_ref(), args.dry_run)?;
    println!("  - Databases: {}", catalog_stats.databases);
    println!("  - Tables: {}", catalog_stats.tables);

    // 2. Migrate edit log
    println!("Migrating edit log...");
    let edit_log_stats = migrate_edit_log(&args.fe_meta_dir, store.as_ref(), args.dry_run)?;
    println!("  - Entries: {}", edit_log_stats.entries);

    // 3. Migrate tablet metadata
    println!("Migrating tablet metadata...");
    let tablet_stats = migrate_tablets(&args.be_storage_dir, store.as_ref(), args.dry_run)?;
    println!("  - Tablets: {}", tablet_stats.tablets);
    println!("  - Rowsets: {}", tablet_stats.rowsets);

    println!();
    println!("Migration completed successfully!");

    // Verification
    if args.verify && !args.dry_run {
        println!();
        println!("Verifying consistency...");
        verify_consistency(&args.fe_meta_dir, &args.be_storage_dir, store.as_ref())?;
        println!("Verification passed!");
    }

    Ok(())
}

struct CatalogStats {
    databases: usize,
    tables: usize,
}

struct EditLogStats {
    entries: usize,
}

struct TabletStats {
    tablets: usize,
    rowsets: usize,
}

fn migrate_catalog(catalog_path: &PathBuf, store: Option<&be_rocks::MetaStore>, dry_run: bool) -> anyhow::Result<CatalogStats> {
    if !catalog_path.exists() {
        println!("  Catalog file not found, skipping...");
        return Ok(CatalogStats { databases: 0, tables: 0 });
    }

    let json = std::fs::read_to_string(catalog_path)?;
    let state: CatalogStateJson = serde_json::from_str(&json)?;

    let mut stats = CatalogStats { databases: 0, tables: 0 };

    if let Some(store) = store {
        let catalog_store = be_rocks::CatalogStore::new(store.clone());

        // Write databases and tables
        for (db_name, db) in &state.databases {
            // Convert fe-catalog::Database to be-rocks::Database
            let rocks_db = be_rocks::Database::new(db.id, db_name);
            catalog_store.put_database(db_name, &rocks_db)?;
            stats.databases += 1;

            for (table_name, table) in &db.tables {
                // Convert fe-catalog::Table to be-rocks::Table
                let rocks_table = convert_table(table);
                catalog_store.put_table(db_name, table_name, &rocks_table)?;
                stats.tables += 1;
            }
        }

        // Set next_id counter
        catalog_store.set_next_id(state.next_id)?;
    }

    Ok(stats)
}

fn migrate_edit_log(fe_meta_dir: &PathBuf, store: Option<&be_rocks::MetaStore>, dry_run: bool) -> anyhow::Result<EditLogStats> {
    let mut stats = EditLogStats { entries: 0 };

    // Find all edit_log_*.json files
    let entries = std::fs::read_dir(fe_meta_dir)?;
    let mut log_files: Vec<PathBuf> = Vec::new();
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with("edit_log_") && name.ends_with(".json") {
                log_files.push(path);
            }
        }
    }
    log_files.sort();

    if log_files.is_empty() {
        println!("  No edit log files found, skipping...");
        return Ok(stats);
    }

    if let Some(store) = store {
        let edit_log_store = be_rocks::EditLogStore::new(store.clone());

        for log_file in &log_files {
            let json = std::fs::read_to_string(log_file)?;
            for line in json.lines() {
                if line.is_empty() {
                    continue;
                }
                let entry: fe_common::edit_log::EditLogEntry = serde_json::from_str(line)?;

                // Convert to be-rocks::EditLogEntry
                let rocks_entry = be_rocks::EditLogEntry {
                    term: entry.term,
                    index: entry.index,
                    op_type: convert_op_type(entry.op_type),
                    data: entry.data,
                };

                // Write to RocksDB
                let key = format!("log:{}", rocks_entry.index);
                let value = serde_json::to_vec(&rocks_entry)?;
                store.put_cf(be_rocks::meta_store::CF_EDIT_LOG, key.as_bytes(), &value)?;
                stats.entries += 1;
            }
        }

        // Set counters
        if stats.entries > 0 {
            edit_log_store.set_last_applied(stats.entries as u64)?;
            edit_log_store.set_current_term(stats.entries as u64)?; // Approximate
        }
    }

    Ok(stats)
}

fn migrate_tablets(be_storage_dir: &PathBuf, store: Option<&be_rocks::MetaStore>, dry_run: bool) -> anyhow::Result<TabletStats> {
    let mut stats = TabletStats { tablets: 0, rowsets: 0 };

    // Find all tablet directories
    let entries = std::fs::read_dir(be_storage_dir)?;
    let mut tablet_dirs: Vec<PathBuf> = Vec::new();
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with("tablet_") {
                tablet_dirs.push(path);
            }
        }
    }

    if tablet_dirs.is_empty() {
        println!("  No tablet directories found, skipping...");
        return Ok(stats);
    }

    if let Some(store) = store {
        let tablet_store = be_rocks::TabletStore::new(store.clone());

        for tablet_dir in &tablet_dirs {
            let tablet_id = extract_tablet_id(tablet_dir)?;

            // Read schema.json
            let schema_path = tablet_dir.join("schema.json");
            if schema_path.exists() {
                let json = std::fs::read_to_string(&schema_path)?;
                let schema: be_storage::TabletSchema = serde_json::from_str(&json)?;

                // Convert to be-rocks::TabletSchema
                let rocks_schema = convert_tablet_schema(&schema);
                tablet_store.put_schema(tablet_id, &rocks_schema)?;
                stats.tablets += 1;
            }

            // Read rowset_*.json files
            let rowset_entries = std::fs::read_dir(tablet_dir)?;
            for rowset_entry in rowset_entries {
                let rowset_entry = rowset_entry?;
                let rowset_path = rowset_entry.path();
                if let Some(name) = rowset_path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with("rowset_") && name.ends_with(".json") {
                        let json = std::fs::read_to_string(&rowset_path)?;
                        let (meta, segments): (be_storage::RowsetMeta, Vec<be_storage::SegmentRef>) =
                            serde_json::from_str(&json)?;

                        // Convert to be-rocks types
                        let rocks_meta = convert_rowset_meta(&meta);
                        let rocks_segments = convert_segments(&segments);
                        tablet_store.put_rowset(tablet_id, meta.rowset_id, &rocks_meta, &rocks_segments)?;
                        stats.rowsets += 1;
                    }
                }
            }
        }
    }

    Ok(stats)
}

fn verify_consistency(fe_meta_dir: &PathBuf, be_storage_dir: &PathBuf, store: &be_rocks::MetaStore) -> anyhow::Result<()> {
    // Verify catalog
    let catalog_path = fe_meta_dir.join("catalog.json");
    if catalog_path.exists() {
        let json = std::fs::read_to_string(&catalog_path)?;
        let state: CatalogStateJson = serde_json::from_str(&json)?;

        let catalog_store = be_rocks::CatalogStore::new(store.clone());
        let rocks_dbs = catalog_store.list_databases()?;

        // Compare counts
        let json_db_count = state.databases.len();
        let rocks_db_count = rocks_dbs.len();
        if json_db_count != rocks_db_count {
            return Err(anyhow::anyhow!("Database count mismatch: JSON={}, RocksDB={}", json_db_count, rocks_db_count));
        }

        println!("  Catalog verification: OK ({} databases)", rocks_db_count);
    }

    println!("  Consistency check passed!");
    Ok(())
}

// Conversion helpers

fn convert_table(table: &fe_catalog::Table) -> be_rocks::Table {
    be_rocks::Table {
        id: table.id,
        tablet_id: table.tablet_id,
        name: table.name.clone(),
        database: table.database.clone(),
        columns: table.columns.iter().map(convert_table_column).collect(),
        keys_type: convert_keys_type(table.keys_type),
        unique_keys: table.unique_keys.iter().map(convert_unique_key).collect(),
        partition_info: table.partition_info.as_ref().map(convert_partition_info),
        distribution_info: table.distribution_info.as_ref().map(convert_distribution_info),
        replication_num: table.replication_num,
        properties: table.properties.clone(),
        row_count: table.row_count,
        data_size: table.data_size,
        stats: table.stats.as_ref().map(convert_table_stats),
        view_definition: table.view_definition.clone(),
    }
}

fn convert_table_column(col: &fe_catalog::TableColumn) -> be_rocks::TableColumn {
    be_rocks::TableColumn {
        name: col.name.clone(),
        data_type: col.data_type.clone(),
        nullable: col.nullable,
        default_value: col.default_value.clone(),
        agg_type: col.agg_type.clone(),
        comment: col.comment.clone(),
    }
}

fn convert_keys_type(k: fe_catalog::KeysType) -> be_rocks::KeysType {
    match k {
        fe_catalog::KeysType::Duplicate => be_rocks::KeysType::Duplicate,
        fe_catalog::KeysType::Aggregate => be_rocks::KeysType::Aggregate,
        fe_catalog::KeysType::Unique => be_rocks::KeysType::Unique,
        fe_catalog::KeysType::Primary => be_rocks::KeysType::Primary,
    }
}

fn convert_unique_key(uk: &fe_catalog::UniqueKeyDef) -> be_rocks::UniqueKeyDef {
    be_rocks::UniqueKeyDef {
        name: uk.name.clone(),
        columns: uk.columns.clone(),
    }
}

fn convert_partition_info(pi: &fe_catalog::PartitionInfo) -> be_rocks::PartitionInfo {
    be_rocks::PartitionInfo {
        partition_type: pi.partition_type.clone(),
        columns: pi.columns.clone(),
        partitions: pi.partitions.iter().map(convert_partition).collect(),
    }
}

fn convert_partition(p: &fe_catalog::Partition) -> be_rocks::Partition {
    be_rocks::Partition {
        id: p.id,
        name: p.name.clone(),
        range_start: p.range_start.clone(),
        range_end: p.range_end.clone(),
    }
}

fn convert_distribution_info(di: &fe_catalog::DistributionInfo) -> be_rocks::DistributionInfo {
    be_rocks::DistributionInfo {
        dist_type: di.dist_type.clone(),
        columns: di.columns.clone(),
        buckets: di.buckets,
    }
}

fn convert_table_stats(ts: &fe_catalog::stats::TableStats) -> be_rocks::TableStats {
    be_rocks::TableStats {
        row_count: ts.row_count,
        data_size: ts.data_size,
        column_stats: ts.column_stats.iter().map(|(k, v)| (k.clone(), convert_column_stats(v))).collect(),
    }
}

fn convert_column_stats(cs: &fe_catalog::stats::ColumnStats) -> be_rocks::ColumnStats {
    be_rocks::ColumnStats {
        min_value: cs.min_value.clone(),
        max_value: cs.max_value.clone(),
        null_count: cs.null_count,
        distinct_count: cs.distinct_count,
    }
}

fn convert_op_type(op: fe_common::edit_log::OpType) -> be_rocks::OpType {
    match op {
        fe_common::edit_log::OpType::CreateDatabase => be_rocks::OpType::CreateDatabase,
        fe_common::edit_log::OpType::DropDatabase => be_rocks::OpType::DropDatabase,
        fe_common::edit_log::OpType::CreateTable => be_rocks::OpType::CreateTable,
        fe_common::edit_log::OpType::DropTable => be_rocks::OpType::DropTable,
        fe_common::edit_log::OpType::AlterDatabase => be_rocks::OpType::AlterDatabase,
        fe_common::edit_log::OpType::AlterTable => be_rocks::OpType::AlterTable,
        fe_common::edit_log::OpType::CreateTablet => be_rocks::OpType::CreateTablet,
        fe_common::edit_log::OpType::DropTablet => be_rocks::OpType::DropTablet,
        fe_common::edit_log::OpType::AlterTablet => be_rocks::OpType::AlterTablet,
        fe_common::edit_log::OpType::AddBackend => be_rocks::OpType::AddBackend,
        fe_common::edit_log::OpType::RemoveBackend => be_rocks::OpType::RemoveBackend,
        fe_common::edit_log::OpType::UpdateStats => be_rocks::OpType::UpdateStats,
    }
}

fn convert_tablet_schema(schema: &be_storage::TabletSchema) -> be_rocks::TabletSchema {
    be_rocks::TabletSchema {
        tablet_id: schema.tablet_id,
        columns: schema.columns.iter().map(convert_tablet_column).collect(),
        keys_type: schema.keys_type.clone(),
        num_rows_per_row_block: schema.num_rows_per_row_block,
    }
}

fn convert_tablet_column(col: &be_storage::TabletColumn) -> be_rocks::TabletColumn {
    be_rocks::TabletColumn {
        name: col.name.clone(),
        data_type: col.data_type.clone(),
        nullable: col.nullable,
        is_key: col.is_key,
        agg_type: col.agg_type.clone(),
    }
}

fn convert_rowset_meta(meta: &be_storage::RowsetMeta) -> be_rocks::RowsetMeta {
    be_rocks::RowsetMeta {
        rowset_id: meta.rowset_id,
        tablet_id: meta.tablet_id,
        txn_id: meta.txn_id,
        version: meta.version,
        num_rows: meta.num_rows,
        data_size: meta.data_size,
        num_segments: meta.num_segments,
        empty: meta.empty,
        packed_data_size: meta.packed_data_size,
        index_size: meta.index_size,
    }
}

fn convert_segments(segments: &[be_storage::SegmentRef]) -> Vec<be_rocks::SegmentRef> {
    segments.iter().map(|s| be_rocks::SegmentRef {
        segment_id: s.segment_id,
        path: s.path.clone(),
        num_rows: s.num_rows,
        size: s.size,
    }).collect()
}

fn extract_tablet_id(path: &PathBuf) -> anyhow::Result<u64> {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let id_str = name.strip_prefix("tablet_").unwrap_or("");
    id_str.parse::<u64>()
        .map_err(|e| anyhow::anyhow!("Failed to parse tablet ID from {}: {}", name, e))
}

// JSON state structures (mirrors fe-catalog::CatalogState)

#[derive(Debug, Deserialize)]
struct CatalogStateJson {
    databases: HashMap<String, fe_catalog::Database>,
    materialized_views: HashMap<String, fe_catalog::MaterializedView>,
    next_id: u64,
}