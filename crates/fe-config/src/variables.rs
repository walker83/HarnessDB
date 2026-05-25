//! System variable management with global and session scope

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// Variable scope
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarScope {
    /// Only global scope (affects all sessions)
    Global,
    /// Only session scope (per-connection)
    Session,
    /// Can be set at both global and session level
    Both,
}

/// Variable type for validation
#[derive(Debug, Clone)]
pub enum VarKind {
    Bool,
    Int,
    Float,
    String,
    Enum(&'static [&'static str]),
}

/// System variable definition
#[derive(Debug, Clone)]
pub struct VarDef {
    pub name: &'static str,
    pub default_value: &'static str,
    pub scope: VarScope,
    pub kind: VarKind,
    pub description: &'static str,
}

/// All defined system variables
pub static SYSTEM_VARIABLE_DEFS: &[VarDef] = &[
    VarDef { name: "version", default_value: "5.7.42", scope: VarScope::Global, kind: VarKind::String, description: "Server version string" },
    VarDef { name: "version_comment", default_value: "RorisDB", scope: VarScope::Global, kind: VarKind::String, description: "Server version comment" },
    VarDef { name: "version_compile_os", default_value: "Linux", scope: VarScope::Global, kind: VarKind::String, description: "Compile OS" },
    VarDef { name: "version_compile_machine", default_value: "x86_64", scope: VarScope::Global, kind: VarKind::String, description: "Compile machine type" },
    VarDef { name: "max_connections", default_value: "100", scope: VarScope::Global, kind: VarKind::Int, description: "Maximum number of simultaneous connections" },
    VarDef { name: "query_timeout", default_value: "300", scope: VarScope::Both, kind: VarKind::Int, description: "Query timeout in seconds" },
    VarDef { name: "max_allowed_packet", default_value: "4194304", scope: VarScope::Both, kind: VarKind::Int, description: "Maximum packet size in bytes" },
    VarDef { name: "storage_compression", default_value: "zstd", scope: VarScope::Global, kind: VarKind::Enum(&["zstd", "snappy", "uncompressed"]), description: "Default storage compression algorithm" },
    VarDef { name: "enable_audit_log", default_value: "true", scope: VarScope::Global, kind: VarKind::Bool, description: "Enable audit logging" },
    VarDef { name: "slow_query_threshold", default_value: "1000", scope: VarScope::Both, kind: VarKind::Int, description: "Slow query threshold in milliseconds" },
    VarDef { name: "default_storage_backend", default_value: "parquet", scope: VarScope::Global, kind: VarKind::String, description: "Default storage engine" },
    VarDef { name: "wait_timeout", default_value: "28800", scope: VarScope::Both, kind: VarKind::Int, description: "Connection idle timeout in seconds" },
    VarDef { name: "interactive_timeout", default_value: "28800", scope: VarScope::Both, kind: VarKind::Int, description: "Interactive connection timeout in seconds" },
    VarDef { name: "autocommit", default_value: "1", scope: VarScope::Both, kind: VarKind::Bool, description: "Auto-commit mode (1=on, 0=off)" },
    VarDef { name: "character_set_client", default_value: "utf8mb4", scope: VarScope::Session, kind: VarKind::String, description: "Client character set" },
    VarDef { name: "character_set_connection", default_value: "utf8mb4", scope: VarScope::Session, kind: VarKind::String, description: "Connection character set" },
    VarDef { name: "character_set_results", default_value: "utf8mb4", scope: VarScope::Session, kind: VarKind::String, description: "Results character set" },
    VarDef { name: "character_set_server", default_value: "utf8mb4", scope: VarScope::Global, kind: VarKind::String, description: "Server character set" },
    VarDef { name: "collation_connection", default_value: "utf8mb4_general_ci", scope: VarScope::Session, kind: VarKind::String, description: "Connection collation" },
    VarDef { name: "collation_server", default_value: "utf8mb4_general_ci", scope: VarScope::Global, kind: VarKind::String, description: "Server collation" },
    VarDef { name: "sql_mode", default_value: "", scope: VarScope::Both, kind: VarKind::String, description: "SQL mode" },
    VarDef { name: "time_zone", default_value: "SYSTEM", scope: VarScope::Both, kind: VarKind::String, description: "Server time zone" },
    VarDef { name: "net_buffer_length", default_value: "16384", scope: VarScope::Both, kind: VarKind::Int, description: "Network buffer length" },
    VarDef { name: "audit_log_slow_only", default_value: "false", scope: VarScope::Global, kind: VarKind::Bool, description: "Log only slow queries to audit log" },
    VarDef { name: "http_port", default_value: "8080", scope: VarScope::Global, kind: VarKind::Int, description: "HTTP port for SQL editor web UI" },
    VarDef { name: "tx_isolation", default_value: "REPEATABLE-READ", scope: VarScope::Both, kind: VarKind::Enum(&["READ-UNCOMMITTED", "READ-COMMITTED", "REPEATABLE-READ", "SERIALIZABLE"]), description: "Transaction isolation level" },
    VarDef { name: "tx_read_only", default_value: "0", scope: VarScope::Both, kind: VarKind::Bool, description: "Transaction read-only mode" },
    VarDef { name: "profiling", default_value: "0", scope: VarScope::Both, kind: VarKind::Bool, description: "Query profiling" },
    VarDef { name: "lower_case_table_names", default_value: "0", scope: VarScope::Global, kind: VarKind::Int, description: "Lowercase table names (0=case sensitive, 1=lowercase)" },
    VarDef { name: "init_connect", default_value: "", scope: VarScope::Global, kind: VarKind::String, description: "SQL executed on each client connect" },
];

/// Global system variables storage
pub struct GlobalVariables {
    values: RwLock<HashMap<String, String>>,
}

impl GlobalVariables {
    pub fn new() -> Self {
        let mut values = HashMap::new();
        for def in SYSTEM_VARIABLE_DEFS {
            values.insert(def.name.to_lowercase(), def.default_value.to_string());
        }
        Self {
            values: RwLock::new(values),
        }
    }

    pub fn get(&self, name: &str) -> Option<String> {
        let values = self.values.read();
        values.get(&name.to_lowercase()).cloned()
    }

    pub fn set(&self, name: &str, value: &str) -> Result<(), String> {
        let name_lower = name.to_lowercase();
        // Find variable definition for validation
        let def = SYSTEM_VARIABLE_DEFS.iter().find(|d| d.name.to_lowercase() == name_lower);
        if let Some(def) = def {
            // Validate value based on kind
            self.validate_value(def, value)?;
            // Check scope - global vars can always be set
            // Both and Global are fine
        }
        let mut values = self.values.write();
        values.insert(name_lower, value.to_string());
        Ok(())
    }

    fn validate_value(&self, def: &VarDef, value: &str) -> Result<(), String> {
        match &def.kind {
            VarKind::Bool => {
                let v = value.to_lowercase();
                if !["0", "1", "true", "false", "on", "off"].contains(&v.as_str()) {
                    return Err(format!("Variable '{}' requires a boolean value (0/1, true/false, on/off)", def.name));
                }
            }
            VarKind::Int => {
                if value.parse::<i64>().is_err() {
                    return Err(format!("Variable '{}' requires an integer value", def.name));
                }
            }
            VarKind::Float => {
                if value.parse::<f64>().is_err() {
                    return Err(format!("Variable '{}' requires a numeric value", def.name));
                }
            }
            VarKind::Enum(valid_values) => {
                let v = value.to_uppercase();
                if !valid_values.iter().any(|vv| vv.to_uppercase() == v) {
                    let vals: Vec<&str> = valid_values.to_vec();
                    return Err(format!(
                        "Variable '{}' must be one of: {}",
                        def.name,
                        vals.join(", ")
                    ));
                }
            }
            VarKind::String => {
                // Any string is valid
            }
        }
        Ok(())
    }

    pub fn all_vars(&self) -> Vec<(String, String)> {
        let values = self.values.read();
        let mut result: Vec<_> = values.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        result.sort_by(|a, b| a.0.cmp(&b.0));
        result
    }
}

impl Default for GlobalVariables {
    fn default() -> Self {
        Self::new()
    }
}

/// Session-level system variables (per-connection overrides)
pub struct SessionVariables {
    values: RwLock<HashMap<String, String>>,
    global: Arc<GlobalVariables>,
}

impl SessionVariables {
    pub fn new(global: Arc<GlobalVariables>) -> Self {
        // Session starts with a copy of session-scoped global values
        let mut values = HashMap::new();
        for def in SYSTEM_VARIABLE_DEFS {
            if def.scope == VarScope::Session || def.scope == VarScope::Both {
                values.insert(def.name.to_lowercase(), def.default_value.to_string());
            }
        }
        Self {
            values: RwLock::new(values),
            global,
        }
    }

    pub fn get(&self, name: &str) -> Option<String> {
        let name_lower = name.to_lowercase();
        // Check session overrides first
        {
            let values = self.values.read();
            if let Some(v) = values.get(&name_lower) {
                return Some(v.clone());
            }
        }
        // Fall back to global
        self.global.get(name)
    }

    pub fn set(&self, name: &str, value: &str) -> Result<(), String> {
        let name_lower = name.to_lowercase();
        let def = SYSTEM_VARIABLE_DEFS.iter().find(|d| d.name.to_lowercase() == name_lower);
        if let Some(def) = def {
            // Check scope - session can only set Session or Both scope vars
            if def.scope == VarScope::Global {
                return Err(format!(
                    "Variable '{}' is GLOBAL only and cannot be set at session level. Use SET GLOBAL {} = '{}'",
                    def.name, def.name, value
                ));
            }
        }
        let mut values = self.values.write();
        values.insert(name_lower, value.to_string());
        Ok(())
    }

    pub fn all_vars(&self) -> Vec<(String, String)> {
        let session_values = self.values.read();
        let global_values = self.global.all_vars();
        let mut result: HashMap<String, String> = global_values.into_iter().collect();
        // Override with session values
        for (k, v) in session_values.iter() {
            result.insert(k.clone(), v.clone());
        }
        let mut sorted: Vec<_> = result.into_iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        sorted
    }
}

/// Combined system variable manager
pub struct SystemVariableManager {
    global: Arc<GlobalVariables>,
}

impl SystemVariableManager {
    pub fn new() -> Self {
        Self {
            global: Arc::new(GlobalVariables::new()),
        }
    }

    pub fn global(&self) -> &Arc<GlobalVariables> {
        &self.global
    }

    pub fn create_session(&self) -> SessionVariables {
        SessionVariables::new(self.global.clone())
    }

    /// Get a variable value (session overrides global)
    pub fn get(&self, name: &str, session: Option<&SessionVariables>) -> Option<String> {
        if let Some(sess) = session {
            sess.get(name)
        } else {
            self.global.get(name)
        }
    }

    /// Set a global variable
    pub fn set_global(&self, name: &str, value: &str) -> Result<(), String> {
        let name_lower = name.to_lowercase();
        let def = SYSTEM_VARIABLE_DEFS.iter().find(|d| d.name.to_lowercase() == name_lower);
        if let Some(def) = def {
            if def.scope == VarScope::Session {
                return Err(format!(
                    "Variable '{}' is SESSION only and cannot be set globally",
                    def.name
                ));
            }
        }
        self.global.set(name, value)
    }

    /// Set a session variable
    pub fn set_session(&self, name: &str, value: &str, session: &SessionVariables) -> Result<(), String> {
        session.set(name, value)
    }

    /// Get all variables matching a LIKE pattern
    pub fn match_like(
        &self,
        pattern: Option<&str>,
        session: Option<&SessionVariables>,
    ) -> Vec<(String, String)> {
        let all_vars = if let Some(sess) = session {
            sess.all_vars()
        } else {
            self.global.all_vars()
        };

        match pattern {
            None => all_vars,
            Some(pat) => {
                let pat_lower = pat.to_lowercase();
                all_vars
                    .into_iter()
                    .filter(|(name, _)| like_match(&pat_lower, &name.to_lowercase()))
                    .collect()
            }
        }
    }
}

impl Default for SystemVariableManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple LIKE pattern matching (% = any chars, _ = single char)
fn like_match(pattern: &str, text: &str) -> bool {
    let pattern = pattern.replace('%', ".*").replace('_', ".");
    let re_pattern = format!("^{}$", pattern);
    // Simple regex-free matching
    like_match_simple(&re_pattern, text)
}

/// Simple pattern matching without regex crate
fn like_match_simple(pattern: &str, text: &str) -> bool {
    let pat_bytes = pattern.as_bytes();
    let txt_bytes = text.as_bytes();
    like_match_recursive(pat_bytes, 0, txt_bytes, 0)
}

fn like_match_recursive(pat: &[u8], mut pi: usize, txt: &[u8], mut ti: usize) -> bool {
    while pi < pat.len() && ti < txt.len() {
        if pat[pi] == b'*' {
            // Handle .* (from %)
            if pi + 1 < pat.len() && pat[pi + 1] == b'.' {
                pi += 2;
                // Try matching rest of pattern at each position
                for i in ti..=txt.len() {
                    if like_match_recursive(pat, pi, txt, i) {
                        return true;
                    }
                }
                return false;
            }
            pi += 1;
            continue;
        } else if pat[pi] == b'.' && pi + 1 > 0 && pi > 0 && pat[pi - 1] != b'.' {
            // Handle . (from _) - matches single char
            pi += 1;
            ti += 1;
        } else if pat[pi] == txt[ti] || pat[pi] == b'\\' && pi + 1 < pat.len() && pat[pi + 1] == txt[ti] {
            if pat[pi] == b'\\' {
                pi += 1;
            }
            pi += 1;
            ti += 1;
        } else {
            return false;
        }
    }

    // Skip trailing wildcards
    while pi < pat.len() && (pat[pi] == b'*' || pat[pi] == b'.') {
        if pat[pi] == b'*' && pi + 1 < pat.len() && pat[pi + 1] == b'.' {
            pi += 2;
        } else {
            pi += 1;
        }
    }

    pi == pat.len() && ti == txt.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_variables() {
        let globals = GlobalVariables::new();
        assert_eq!(globals.get("version"), Some("5.7.42".to_string()));
        assert_eq!(globals.get("max_connections"), Some("100".to_string()));

        globals.set("max_connections", "200").unwrap();
        assert_eq!(globals.get("max_connections"), Some("200".to_string()));
    }

    #[test]
    fn test_session_variables() {
        let globals = Arc::new(GlobalVariables::new());
        let session = SessionVariables::new(globals.clone());

        // Session should see global values
        assert_eq!(session.get("version"), Some("5.7.42".to_string()));

        // Session can override session-scoped vars
        session.set("sql_mode", "STRICT_TRANS_TABLES").unwrap();
        assert_eq!(session.get("sql_mode"), Some("STRICT_TRANS_TABLES".to_string()));

        // Global unchanged
        assert_eq!(globals.get("sql_mode"), Some("".to_string()));
    }

    #[test]
    fn test_variable_validation() {
        let globals = GlobalVariables::new();

        // Valid int
        assert!(globals.set("max_connections", "500").is_ok());

        // Invalid int
        assert!(globals.set("max_connections", "not_a_number").is_err());

        // Valid bool
        assert!(globals.set("autocommit", "0").is_ok());
        assert!(globals.set("autocommit", "true").is_ok());

        // Invalid bool
        assert!(globals.set("autocommit", "maybe").is_err());
    }

    #[test]
    fn test_like_match() {
        assert!(like_match("%version%", "version"));
        assert!(like_match("%version%", "my_version_string"));
        assert!(like_match("version", "version"));
        assert!(!like_match("version", "versions"));
        assert!(like_match("ver%on", "version"));
    }

    #[test]
    fn test_system_variable_manager() {
        let mgr = SystemVariableManager::new();
        let session = mgr.create_session();

        // Get from global
        assert_eq!(mgr.get("version", Some(&session)), Some("5.7.42".to_string()));
        assert_eq!(mgr.get("version", None), Some("5.7.42".to_string()));

        // Set global
        mgr.set_global("max_connections", "500").unwrap();
        assert_eq!(mgr.get("max_connections", Some(&session)), Some("500".to_string()));

        // Set session
        mgr.set_session("sql_mode", "STRICT", &session).unwrap();
        assert_eq!(session.get("sql_mode"), Some("STRICT".to_string()));

        // Match like
        let vars = mgr.match_like(Some("%version%"), Some(&session));
        assert!(vars.iter().any(|(name, _)| name == "version"));
    }
}
