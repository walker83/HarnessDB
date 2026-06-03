//! TOML configuration file loading and structure

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Top-level HarnessDB configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessConfig {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub storage: StorageConfig,
    #[serde(default)]
    pub query: QueryConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
    #[serde(default)]
    pub security: SecurityConfig,
}

/// Server configuration section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_mysql_port")]
    pub mysql_port: u16,
    #[serde(default = "default_bind_addr")]
    pub bind_addr: String,
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
    #[serde(default = "default_wait_timeout")]
    pub wait_timeout: u32,
    #[serde(default = "default_http_port")]
    pub http_port: u16,
    #[serde(default = "default_meta_dir")]
    pub meta_dir: String,
}

/// Storage configuration section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    #[serde(default = "default_data_dir")]
    pub data_dir: String,
    #[serde(default = "default_compression")]
    pub compression: String,
    #[serde(default = "default_page_size")]
    pub page_size: u32,
}

/// Query engine configuration section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryConfig {
    #[serde(default = "default_query_timeout")]
    pub query_timeout: u32,
    #[serde(default = "default_max_allowed_packet")]
    pub max_allowed_packet: u64,
    #[serde(default)]
    pub sql_mode: String,
    #[serde(default = "default_time_zone")]
    pub time_zone: String,
    #[serde(default = "default_max_concurrent_queries")]
    pub max_concurrent_queries: u32,
    #[serde(default = "default_max_dml_rows")]
    pub max_dml_rows: u64,
}

/// Logging and audit configuration section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_true")]
    pub enable_audit_log: bool,
    #[serde(default = "default_slow_query_threshold")]
    pub slow_query_threshold_ms: u64,
    #[serde(default = "default_audit_log_dir")]
    pub audit_log_dir: String,
    #[serde(default = "default_audit_log_max_size_mb")]
    pub audit_log_max_size_mb: u64,
    #[serde(default = "default_audit_log_max_files")]
    pub audit_log_max_files: u32,
    #[serde(default)]
    pub audit_log_slow_only: bool,
}

/// Security configuration section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    #[serde(default)]
    pub auth_enabled: bool,
    /// JWT secret key for token-based authentication.
    /// If not set, reads from RORIS_JWT_SECRET environment variable.
    /// If neither is set, a random key is generated at startup.
    #[serde(default = "default_jwt_secret")]
    pub jwt_secret: String,
}

fn default_jwt_secret() -> String {
    std::env::var("RORIS_JWT_SECRET").unwrap_or_else(|_| {
        // Generate a pseudo-random key from process start time + PID.
        // This is not cryptographically secure but avoids a fixed default.
        // For production, set RORIS_JWT_SECRET environment variable.
        use std::sync::OnceLock;
        static KEY: OnceLock<String> = OnceLock::new();
        KEY.get_or_init(|| {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let pid = std::process::id() as u128;
            let seed = now ^ (pid << 64);
            let key = format!("{:032x}{:032x}", seed, seed.wrapping_mul(6364136223846793005));
            tracing::warn!("RORIS_JWT_SECRET not set — using generated JWT key. Set RORIS_JWT_SECRET for production use.");
            key
        }).clone()
    })
}

// Default value functions
fn default_mysql_port() -> u16 {
    9030
}
fn default_bind_addr() -> String {
    "127.0.0.1".to_string()
}
fn default_max_connections() -> u32 {
    100
}
fn default_wait_timeout() -> u32 {
    28800
}
fn default_http_port() -> u16 {
    8080
}
fn default_meta_dir() -> String {
    "data/fe/doris-meta".to_string()
}
fn default_data_dir() -> String {
    "data/fe/storage".to_string()
}
fn default_compression() -> String {
    "zstd".to_string()
}
fn default_page_size() -> u32 {
    4096
}
fn default_query_timeout() -> u32 {
    300
}
fn default_max_allowed_packet() -> u64 {
    4194304
}
fn default_time_zone() -> String {
    "SYSTEM".to_string()
}
fn default_max_concurrent_queries() -> u32 {
    50
}
fn default_max_dml_rows() -> u64 {
    10_000_000
}
fn default_true() -> bool {
    true
}
fn default_slow_query_threshold() -> u64 {
    1000
}
fn default_audit_log_dir() -> String {
    "data/fe/audit".to_string()
}
fn default_audit_log_max_size_mb() -> u64 {
    100
}
fn default_audit_log_max_files() -> u32 {
    10
}

impl Default for HarnessConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            storage: StorageConfig::default(),
            query: QueryConfig::default(),
            logging: LoggingConfig::default(),
            security: SecurityConfig::default(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            mysql_port: default_mysql_port(),
            bind_addr: default_bind_addr(),
            max_connections: default_max_connections(),
            wait_timeout: default_wait_timeout(),
            http_port: default_http_port(),
            meta_dir: default_meta_dir(),
        }
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            data_dir: default_data_dir(),
            compression: default_compression(),
            page_size: default_page_size(),
        }
    }
}

impl Default for QueryConfig {
    fn default() -> Self {
        Self {
            query_timeout: default_query_timeout(),
            max_allowed_packet: default_max_allowed_packet(),
            sql_mode: String::new(),
            time_zone: default_time_zone(),
            max_concurrent_queries: default_max_concurrent_queries(),
            max_dml_rows: default_max_dml_rows(),
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            enable_audit_log: default_true(),
            slow_query_threshold_ms: default_slow_query_threshold(),
            audit_log_dir: default_audit_log_dir(),
            audit_log_max_size_mb: default_audit_log_max_size_mb(),
            audit_log_max_files: default_audit_log_max_files(),
            audit_log_slow_only: false,
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            auth_enabled: false,
            jwt_secret: default_jwt_secret(),
        }
    }
}

impl HarnessConfig {
    /// Load configuration from a TOML file
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        let content = std::fs::read_to_string(path.as_ref()).map_err(|e| {
            format!(
                "Failed to read config file '{}': {}",
                path.as_ref().display(),
                e
            )
        })?;
        toml::from_str(&content).map_err(|e| {
            format!(
                "Failed to parse config file '{}': {}",
                path.as_ref().display(),
                e
            )
        })
    }

    /// Load configuration from a file, or return default if file doesn't exist
    pub fn load_or_default(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref();
        if !path.exists() {
            // Config file doesn't exist — use defaults (normal for first-time setup)
            return Self::default();
        }
        match Self::load(path) {
            Ok(config) => config,
            Err(e) => {
                // Config file exists but has errors — this is dangerous, log prominently
                tracing::error!(
                    "CRITICAL: Failed to parse config file {}: {}. \
                     Check syntax and try again. Using defaults (this may not be what you want).",
                    path.display(),
                    e
                );
                Self::default()
            }
        }
    }

    /// Default config file path
    pub fn default_path() -> &'static str {
        "harness.toml"
    }

    /// Apply CLI argument overrides (CLI takes precedence over config file)
    pub fn apply_cli_overrides(
        &mut self,
        mysql_port: Option<u16>,
        data_dir: Option<String>,
        meta_dir: Option<String>,
    ) {
        if let Some(port) = mysql_port {
            self.server.mysql_port = port;
        }
        if let Some(dir) = data_dir {
            self.storage.data_dir = dir;
        }
        if let Some(dir) = meta_dir {
            self.server.meta_dir = dir;
        }
    }

    /// Generate a default TOML configuration string
    pub fn generate_default_toml() -> String {
        r#"# HarnessDB Configuration File

[server]
mysql_port = 9030
bind_addr = "127.0.0.1"
max_connections = 100
wait_timeout = 28800
http_port = 8080
meta_dir = "data/fe/doris-meta"

[storage]
data_dir = "data/fe/storage"
compression = "zstd"       # zstd | snappy | uncompressed
page_size = 4096

[query]
query_timeout = 300
max_allowed_packet = 4194304
max_concurrent_queries = 50
sql_mode = ""
time_zone = "SYSTEM"

[logging]
enable_audit_log = true
slow_query_threshold_ms = 1000
audit_log_dir = "data/fe/audit"
audit_log_max_size_mb = 100
audit_log_max_files = 10
audit_log_slow_only = false

[security]
auth_enabled = false
"#
        .to_string()
    }
}
