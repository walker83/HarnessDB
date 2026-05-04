use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LdapConfig {
    pub url: String,
    pub base_dn: String,
    pub bind_dn: Option<String>,
    pub bind_password: Option<String>,
    pub user_filter: String,
    pub group_filter: String,
    pub use_ssl: bool,
    pub pool_size: usize,
}

impl LdapConfig {
    pub fn new(url: String, base_dn: String) -> Self {
        Self {
            url,
            base_dn,
            bind_dn: None,
            bind_password: None,
            user_filter: "(uid={username})".to_string(),
            group_filter: "(member={user_dn})".to_string(),
            use_ssl: false,
            pool_size: 10,
        }
    }

    pub fn with_bind_dn(mut self, bind_dn: String, bind_password: String) -> Self {
        self.bind_dn = Some(bind_dn);
        self.bind_password = Some(bind_password);
        self
    }
}

pub struct LdapAuthenticator {
    config: LdapConfig,
}

impl LdapAuthenticator {
    pub fn new(config: LdapConfig) -> Self {
        Self { config }
    }

    pub fn authenticate(&self, _username: &str, _password: &str) -> Result<LdapUserInfo, LdapError> {
        Err(LdapError::NotImplemented(
            "LDAP authentication not yet implemented".to_string(),
        ))
    }

    pub fn get_user_groups(&self, _username: &str) -> Result<Vec<String>, LdapError> {
        Err(LdapError::NotImplemented(
            "LDAP group lookup not yet implemented".to_string(),
        ))
    }
}

#[derive(Debug, Clone)]
pub struct LdapUserInfo {
    pub username: String,
    pub dn: String,
    pub groups: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum LdapError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    #[error("User not found: {0}")]
    UserNotFound(String),

    #[error("LDAP not implemented: {0}")]
    NotImplemented(String),
}