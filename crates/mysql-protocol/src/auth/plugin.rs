use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthPluginType {
    NativePassword,
    Ldap,
    Token,
    Kerberos,
}

impl AuthPluginType {
    pub fn name(&self) -> &'static [u8] {
        match self {
            AuthPluginType::NativePassword => b"mysql_native_password",
            AuthPluginType::Ldap => b"ldap_password",
            AuthPluginType::Token => b"auth_token",
            AuthPluginType::Kerberos => b"kerberos",
        }
    }

    pub fn from_name(name: &[u8]) -> Option<Self> {
        match name {
            b"mysql_native_password" => Some(AuthPluginType::NativePassword),
            b"ldap_password" => Some(AuthPluginType::Ldap),
            b"auth_token" => Some(AuthPluginType::Token),
            b"kerberos" => Some(AuthPluginType::Kerberos),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub username: String,
    pub hostname: Option<String>,
    pub roles: Vec<String>,
    pub auth_plugin: AuthPluginType,
}

#[async_trait]
pub trait AuthPlugin: Send + Sync {
    fn plugin_type(&self) -> AuthPluginType;

    async fn authenticate(
        &self,
        username: &str,
        auth_response: &[u8],
        auth_plugin_data: &[u8],
    ) -> Result<AuthUser, AuthError>;
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Authentication failed: {0}")]
    Failed(String),

    #[error("User not found: {0}")]
    UserNotFound(String),

    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Plugin not supported: {0}")]
    PluginNotSupported(String),
}