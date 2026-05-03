use std::collections::HashMap;

/// Statistics for a table, used by the cost-based optimizer.
#[derive(Debug, Clone)]
pub struct TableStats {
    pub row_count: u64,
    pub data_size: u64,
    pub column_stats: HashMap<String, ColumnStats>,
}

impl TableStats {
    pub fn empty() -> Self {
        Self {
            row_count: 0,
            data_size: 0,
            column_stats: HashMap::new(),
        }
    }

    pub fn with_row_count(row_count: u64) -> Self {
        Self {
            row_count,
            data_size: 0,
            column_stats: HashMap::new(),
        }
    }

    pub fn estimate_selectivity(&self, column: &str) -> f64 {
        if let Some(col_stats) = self.column_stats.get(column) {
            if self.row_count == 0 {
                return 0.0;
            }
            // Estimate selectivity based on NDV: 1/ndv gives per-value selectivity
            if col_stats.ndv > 0 {
                1.0 / col_stats.ndv as f64
            } else {
                0.1
            }
        } else {
            0.1
        }
    }
}

impl Default for TableStats {
    fn default() -> Self {
        Self::empty()
    }
}

/// Statistics for a single column.
#[derive(Debug, Clone)]
pub struct ColumnStats {
    /// Number of distinct values.
    pub ndv: u64,
    /// Number of null values.
    pub null_count: u64,
    /// Minimum value (as string representation).
    pub min_value: Option<String>,
    /// Maximum value (as string representation).
    pub max_value: Option<String>,
    /// Average width in bytes for variable-length types.
    pub avg_width: u32,
}

impl ColumnStats {
    pub fn unknown() -> Self {
        Self {
            ndv: 0,
            null_count: 0,
            min_value: None,
            max_value: None,
            avg_width: 0,
        }
    }

    pub fn with_ndv(ndv: u64) -> Self {
        Self {
            ndv,
            ..Self::unknown()
        }
    }

    pub fn is_stats_available(&self) -> bool {
        self.ndv > 0
    }
}

/// Provider trait for fetching table statistics.
pub trait StatisticsProvider: Send + Sync {
    fn get_table_stats(&self, database: &str, table: &str) -> Option<TableStats>;
}

/// A simple in-memory statistics provider for testing.
pub struct InMemoryStatsProvider {
    stats: HashMap<String, TableStats>,
}

impl InMemoryStatsProvider {
    pub fn new() -> Self {
        Self {
            stats: HashMap::new(),
        }
    }

    pub fn add_table_stats(&mut self, database: &str, table: &str, stats: TableStats) {
        let key = format!("{}.{}", database, table);
        self.stats.insert(key, stats);
    }
}

impl Default for InMemoryStatsProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl StatisticsProvider for InMemoryStatsProvider {
    fn get_table_stats(&self, database: &str, table: &str) -> Option<TableStats> {
        let key = format!("{}.{}", database, table);
        self.stats.get(&key).cloned()
    }
}
