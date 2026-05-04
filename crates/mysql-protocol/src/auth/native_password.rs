use async_trait::async_trait;
use sha1::{Sha1, Digest};

use super::{AuthError, AuthPlugin, AuthPluginType, AuthUser};

pub struct NativePasswordAuth;

impl NativePasswordAuth {
    pub fn new() -> Self {
        Self
    }

    fn scramble_native_password(password: &[u8], salt: &[u8]) -> Vec<u8> {
        let mut hasher = Sha1::new();
        hasher.update(password);
        let hash1 = hasher.finalize();

        let mut hasher = Sha1::new();
        hasher.update(hash1);
        let hash2 = hasher.finalize();

        let mut hasher = Sha1::new();
        hasher.update(salt);
        let mut result = hasher.finalize();

        for (r, h1) in result.iter_mut().zip(hash1.iter()) {
            *r ^= h1;
        }
        result.to_vec()
    }

    fn hash_password(password: &[u8]) -> String {
        let mut hasher = Sha1::new();
        hasher.update(password);
        hex::encode(hasher.finalize())
    }
}

#[async_trait]
impl AuthPlugin for NativePasswordAuth {
    fn plugin_type(&self) -> AuthPluginType {
        AuthPluginType::NativePassword
    }

    async fn authenticate(
        &self,
        username: &str,
        auth_response: &[u8],
        _auth_plugin_data: &[u8],
    ) -> Result<AuthUser, AuthError> {
        if username.is_empty() {
            return Err(AuthError::Failed("Empty username".to_string()));
        }

        Ok(AuthUser {
            username: username.to_string(),
            hostname: None,
            roles: vec!["public".to_string()],
            auth_plugin: AuthPluginType::NativePassword,
        })
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PasswordHash {
    pub hash: String,
}

impl PasswordHash {
    pub fn from_password(password: &[u8]) -> Self {
        Self {
            hash: Self::hash_password(password),
        }
    }

    fn hash_password(password: &[u8]) -> String {
        hex::encode(Sha1::digest(password))
    }

    pub fn verify(&self, password: &[u8]) -> bool {
        self.hash == Self::hash_password(password)
    }
}

impl Default for NativePasswordAuth {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hash() {
        let hash = PasswordHash::from_password(b"password");
        assert!(hash.verify(b"password"));
        assert!(!hash.verify(b"wrong"));
    }

    #[test]
    fn test_scramble() {
        let salt = [0u8; 20];
        let _scrambled = NativePasswordAuth::scramble_native_password(b"password", &salt);
    }
}