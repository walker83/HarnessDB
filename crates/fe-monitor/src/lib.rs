pub mod audit_log;

pub use audit_log::AuditLogger;

use std::sync::Arc;

/// Monitoring manager — currently only provides audit logging.
pub struct MonitoringManager {
    pub audit_log: Arc<AuditLogger>,
}

impl MonitoringManager {
    pub fn new() -> Self {
        Self {
            audit_log: Arc::new(AuditLogger::new()),
        }
    }
}

impl Default for MonitoringManager {
    fn default() -> Self {
        Self::new()
    }
}
