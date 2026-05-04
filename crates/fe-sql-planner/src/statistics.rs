// Statistics types live in fe-catalog for persistence access.
// Re-export here for backward compatibility with existing planner code.
pub use fe_catalog::stats::{
    ColumnStats, Histogram, HistogramBucket, InMemoryStatsProvider, StatisticsProvider, TableStats,
};
