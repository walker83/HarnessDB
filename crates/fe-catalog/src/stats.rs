use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Statistics for a table, used by the cost-based optimizer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableStats {
    pub row_count: u64,
    pub data_size: u64,
    pub column_stats: HashMap<String, ColumnStats>,
    /// Timestamp (epoch seconds) when stats were last updated.
    pub updated_at: Option<u64>,
}

impl TableStats {
    pub fn empty() -> Self {
        Self {
            row_count: 0,
            data_size: 0,
            column_stats: HashMap::new(),
            updated_at: None,
        }
    }

    pub fn with_row_count(row_count: u64) -> Self {
        Self {
            row_count,
            data_size: 0,
            column_stats: HashMap::new(),
            updated_at: None,
        }
    }

    /// Estimate equality selectivity for a column based on NDV.
    pub fn estimate_selectivity(&self, column: &str) -> f64 {
        if let Some(col_stats) = self.column_stats.get(column) {
            if self.row_count == 0 {
                return 0.0;
            }
            if col_stats.ndv > 0 {
                1.0 / col_stats.ndv as f64
            } else {
                0.1
            }
        } else {
            0.1
        }
    }

    /// Estimate range selectivity for a column using histogram if available.
    pub fn estimate_range_selectivity(&self, column: &str, low: &str, high: &str) -> f64 {
        if let Some(col_stats) = self.column_stats.get(column) {
            if let Some(ref hist) = col_stats.histogram {
                return hist.estimate_range_selectivity(low, high);
            }
            // Fallback: use min/max if available
            if let (Some(min), Some(max)) = (&col_stats.min_value, &col_stats.max_value) {
                return Self::range_selectivity_from_min_max(min, max, low, high);
            }
        }
        // Default range selectivity guess: 1/3
        0.33
    }

    fn range_selectivity_from_min_max(min: &str, max: &str, low: &str, high: &str) -> f64 {
        let min_f: f64 = min.parse().unwrap_or(0.0);
        let max_f: f64 = max.parse().unwrap_or(0.0);
        let low_f: f64 = low.parse().unwrap_or(min_f);
        let high_f: f64 = high.parse().unwrap_or(max_f);
        let range = max_f - min_f;
        if range <= 0.0 {
            return 1.0;
        }
        let selectivity = (high_f - low_f) / range;
        selectivity.clamp(0.0, 1.0)
    }
}

impl Default for TableStats {
    fn default() -> Self {
        Self::empty()
    }
}

/// Statistics for a single column.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// Equi-depth histogram for range selectivity estimation.
    pub histogram: Option<Histogram>,
}

impl ColumnStats {
    pub fn unknown() -> Self {
        Self {
            ndv: 0,
            null_count: 0,
            min_value: None,
            max_value: None,
            avg_width: 0,
            histogram: None,
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

/// Equi-depth histogram bucket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramBucket {
    /// Upper bound value of this bucket (inclusive, as string).
    pub upper_bound: String,
    /// Number of distinct values in this bucket.
    pub distinct_count: u64,
    /// Number of rows in this bucket.
    pub row_count: u64,
}

/// Equi-depth histogram for range query selectivity estimation.
/// All buckets contain approximately the same number of rows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Histogram {
    /// Minimum value across all buckets.
    pub min_value: String,
    /// Maximum value across all buckets.
    pub max_value: String,
    /// Total row count represented by this histogram.
    pub total_rows: u64,
    /// Histogram buckets.
    pub buckets: Vec<HistogramBucket>,
}

impl Histogram {
    pub fn new(min_value: String, max_value: String, total_rows: u64) -> Self {
        Self {
            min_value,
            max_value,
            total_rows,
            buckets: Vec::new(),
        }
    }

    /// Estimate selectivity for a range predicate (col BETWEEN low AND high).
    pub fn estimate_range_selectivity(&self, low: &str, high: &str) -> f64 {
        if self.buckets.is_empty() || self.total_rows == 0 {
            return 0.33;
        }

        let mut matched_rows: u64 = 0;
        for bucket in &self.buckets {
            // A bucket overlaps the range if its upper_bound >= low and bucket_start <= high.
            // Since we don't store lower bounds explicitly, the lower bound of bucket[i]
            // is the upper bound of bucket[i-1] (or self.min_value for i=0).
            let bucket_lower = self.bucket_lower(bucket);
            if Self::compare_values(&bucket.upper_bound, low) >= 0
                && Self::compare_values(&bucket_lower, high) <= 0
            {
                matched_rows += bucket.row_count;
            }
        }

        matched_rows as f64 / self.total_rows as f64
    }

    fn bucket_lower(&self, bucket: &HistogramBucket) -> String {
        // Find the index of this bucket
        for (i, b) in self.buckets.iter().enumerate() {
            if std::ptr::eq(b, bucket) {
                if i == 0 {
                    return self.min_value.clone();
                } else {
                    return self.buckets[i - 1].upper_bound.clone();
                }
            }
        }
        self.min_value.clone()
    }

    /// Simple numeric string comparison. Falls back to lexicographic for non-numeric.
    fn compare_values(a: &str, b: &str) -> i32 {
        let a_f: Result<f64, _> = a.parse();
        let b_f: Result<f64, _> = b.parse();
        match (a_f, b_f) {
            (Ok(a), Ok(b)) => a.partial_cmp(&b).map(|c| c as i32).unwrap_or(0),
            _ => a.cmp(b) as i32,
        }
    }

    /// Build an equi-depth histogram from sorted values.
    /// `values` must be sorted. `num_buckets` is the desired number of buckets.
    pub fn build_from_sorted(values: &[String], num_buckets: usize) -> Option<Self> {
        if values.is_empty() {
            return None;
        }
        let total_rows = values.len() as u64;
        let min_value = values.first().unwrap().clone();
        let max_value = values.last().unwrap().clone();

        let rows_per_bucket = (total_rows as usize + num_buckets - 1) / num_buckets;
        let mut buckets = Vec::new();

        let mut i = 0;
        while i < values.len() {
            let end = (i + rows_per_bucket).min(values.len());
            let bucket_values = &values[i..end];
            let upper_bound = bucket_values.last().unwrap().clone();

            // Count distinct values in bucket
            let mut distinct = 0u64;
            let mut prev: Option<&str> = None;
            for v in bucket_values {
                if prev != Some(v.as_str()) {
                    distinct += 1;
                }
                prev = Some(v.as_str());
            }

            buckets.push(HistogramBucket {
                upper_bound,
                distinct_count: distinct,
                row_count: bucket_values.len() as u64,
            });
            i = end;
        }

        Some(Self {
            min_value,
            max_value,
            total_rows,
            buckets,
        })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_stats_selectivity() {
        let mut stats = TableStats::with_row_count(1000);
        stats
            .column_stats
            .insert("col1".into(), ColumnStats::with_ndv(100));
        let sel = stats.estimate_selectivity("col1");
        assert!((sel - 0.01).abs() < 0.001);
    }

    #[test]
    fn test_table_stats_range_selectivity_histogram() {
        let mut col = ColumnStats::unknown();
        col.histogram = Some(Histogram {
            min_value: "0".into(),
            max_value: "100".into(),
            total_rows: 1000,
            buckets: vec![
                HistogramBucket {
                    upper_bound: "25".into(),
                    distinct_count: 25,
                    row_count: 250,
                },
                HistogramBucket {
                    upper_bound: "50".into(),
                    distinct_count: 25,
                    row_count: 250,
                },
                HistogramBucket {
                    upper_bound: "75".into(),
                    distinct_count: 25,
                    row_count: 250,
                },
                HistogramBucket {
                    upper_bound: "100".into(),
                    distinct_count: 25,
                    row_count: 250,
                },
            ],
        });
        let mut stats = TableStats::with_row_count(1000);
        stats.column_stats.insert("val".into(), col);

        let sel = stats.estimate_range_selectivity("val", "0", "50");
        assert!(sel > 0.0 && sel <= 1.0);
        // Matches buckets where upper_bound >= 0 and bucket_lower <= 50.
        // Bucket 0: [0,25] matches, Bucket 1: [25,50] matches.
        // Bucket 2: [50,75] also matches since bucket_lower (50) <= high (50).
        assert!(sel > 0.4 && sel <= 0.75);
    }

    #[test]
    fn test_histogram_build_from_sorted() {
        let values: Vec<String> = (0..100).map(|i| i.to_string()).collect();
        let hist = Histogram::build_from_sorted(&values, 4).unwrap();
        assert_eq!(hist.buckets.len(), 4);
        assert_eq!(hist.min_value, "0");
        assert_eq!(hist.max_value, "99");
        assert_eq!(hist.total_rows, 100);
    }

    #[test]
    fn test_histogram_empty_values() {
        let values: Vec<String> = vec![];
        let hist = Histogram::build_from_sorted(&values, 4);
        assert!(hist.is_none());
    }

    #[test]
    fn test_stats_serialization() {
        let mut stats = TableStats::with_row_count(500);
        stats.column_stats.insert(
            "id".into(),
            ColumnStats {
                ndv: 500,
                null_count: 0,
                min_value: Some("1".into()),
                max_value: Some("500".into()),
                avg_width: 8,
                histogram: None,
            },
        );
        let json = serde_json::to_string(&stats).unwrap();
        let decoded: TableStats = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.row_count, 500);
        assert_eq!(decoded.column_stats.get("id").unwrap().ndv, 500);
    }

    #[test]
    fn test_in_memory_stats_provider() {
        let mut provider = InMemoryStatsProvider::new();
        let stats = TableStats::with_row_count(100);
        provider.add_table_stats("mydb", "mytable", stats);
        let result = provider.get_table_stats("mydb", "mytable");
        assert!(result.is_some());
        assert_eq!(result.unwrap().row_count, 100);
        assert!(provider.get_table_stats("mydb", "other").is_none());
    }
}
