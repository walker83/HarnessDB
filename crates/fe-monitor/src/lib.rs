pub mod information_schema;
pub mod metrics;
pub mod audit_log;
pub mod query_profile;
pub mod http_server;

pub use information_schema::InformationSchema;
pub use metrics::{MetricsCollector, FeMetrics, BeMetrics};
pub use audit_log::AuditLogger;
pub use query_profile::QueryProfiler;

use std::sync::{Arc, RwLock};
use fe_catalog::CatalogManager;

/// Monitoring manager that integrates all monitoring components
pub struct MonitoringManager {
    pub information_schema: Arc<RwLock<InformationSchema>>,
    pub metrics: Arc<MetricsCollector>,
    pub audit_log: Arc<AuditLogger>,
    pub query_profiler: Arc<QueryProfiler>,
}

impl MonitoringManager {
    pub fn new(catalog: Arc<RwLock<CatalogManager>>) -> Self {
        Self {
            information_schema: Arc::new(RwLock::new(InformationSchema::new(catalog))),
            metrics: Arc::new(MetricsCollector::new()),
            audit_log: Arc::new(AuditLogger::new()),
            query_profiler: Arc::new(QueryProfiler::new()),
        }
    }
}

impl Default for MonitoringManager {
    fn default() -> Self {
        Self::new(Arc::new(RwLock::new(CatalogManager::new())))
    }
}
