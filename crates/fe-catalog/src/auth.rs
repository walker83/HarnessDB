use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use common::DrorisError;
use mysql_protocol::auth::{AuthPluginType, AuthUser};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAuth {
    pub username: String,
    pub hostname: Option<String>,
    pub auth_plugin: AuthPluginType,
    pub password_hash: Option<String>,
    pub roles: Vec<String>,
    pub max_connections: u32,
    pub timeout_secs: u64,
}

impl UserAuth {
    pub fn new_native_password(username: String, password_hash: String) -> Self {
        Self {
            username,
            hostname: None,
            auth_plugin: AuthPluginType::NativePassword,
            password_hash: Some(password_hash),
            roles: vec!["public".to_string()],
            max_connections: 100,
            timeout_secs: 3600,
        }
    }

    pub fn new_token_auth(username: String) -> Self {
        Self {
            username,
            hostname: None,
            auth_plugin: AuthPluginType::Token,
            password_hash: None,
            roles: vec!["public".to_string()],
            max_connections: 100,
            timeout_secs: 3600,
        }
    }
}

pub struct AuthManager {
    users: DashMap<String, UserAuth>,
    auth_cache: DashMap<String, AuthUser>,
}

impl AuthManager {
    pub fn new() -> Self {
        Self {
            users: DashMap::new(),
            auth_cache: DashMap::new(),
        }
    }

    pub fn create_user(&self, user: UserAuth) -> Result<(), DrorisError> {
        let key = user_key(&user.username, user.hostname.as_deref());
        if self.users.contains_key(&key) {
            return Err(DrorisError::Internal(format!(
                "User '{}' already exists",
                user.username
            )));
        }
        self.users.insert(key, user);
        Ok(())
    }

    pub fn drop_user(&self, username: &str, hostname: Option<&str>) -> Result<(), DrorisError> {
        let key = user_key(username, hostname);
        self.users
            .remove(&key)
            .ok_or_else(|| DrorisError::Internal(format!("User '{}' not found", username)))?;
        self.auth_cache.remove(&key);
        Ok(())
    }

    pub fn get_user(&self, username: &str, hostname: Option<&str>) -> Option<UserAuth> {
        let key = user_key(username, hostname);
        self.users.get(&key).map(|r| r.value().clone())
    }

    pub fn get_user_auth(&self, username: &str, hostname: Option<&str>) -> Option<AuthUser> {
        let key = user_key(username, hostname);
        if let Some(user) = self.users.get(&key) {
            Some(AuthUser {
                username: user.username.clone(),
                hostname: user.hostname.clone(),
                roles: user.roles.clone(),
                auth_plugin: user.auth_plugin,
            })
        } else {
            None
        }
    }

    pub fn verify_password(&self, username: &str, password_hash: &str) -> bool {
        if let Some(user) = self.users.get(&user_key(username, None)) {
            if let Some(stored_hash) = &user.password_hash {
                return stored_hash == password_hash;
            }
        }
        false
    }

    pub fn grant_role(&self, username: &str, role: &str) -> Result<(), DrorisError> {
        let key = user_key(username, None);
        if let Some(mut user) = self.users.get_mut(&key) {
            if !user.roles.contains(&role.to_string()) {
                user.roles.push(role.to_string());
            }
            self.auth_cache.remove(&key);
            Ok(())
        } else {
            Err(DrorisError::Internal(format!("User '{}' not found", username)))
        }
    }

    pub fn revoke_role(&self, username: &str, role: &str) -> Result<(), DrorisError> {
        let key = user_key(username, None);
        if let Some(mut user) = self.users.get_mut(&key) {
            user.roles.retain(|r| r != role);
            self.auth_cache.remove(&key);
            Ok(())
        } else {
            Err(DrorisError::Internal(format!("User '{}' not found", username)))
        }
    }

    pub fn list_users(&self) -> Vec<UserAuth> {
        self.users.iter().map(|r| r.value().clone()).collect()
    }

    pub fn save(&self, path: &str) -> Result<(), DrorisError> {
        let runtime = tokio::runtime::Runtime::new()?;
        runtime.block_on(async {
            let users: Vec<UserAuth> = self.list_users();
            let json = serde_json::to_string(&users)
                .map_err(|e| DrorisError::Internal(e.to_string()))?;
            tokio::fs::create_dir_all(path).await.map_err(|e| DrorisError::Internal(e.to_string()))?;
            let file_path = format!("{}/users.json", path);
            tokio::fs::write(&file_path, json).await.map_err(|e| DrorisError::Internal(e.to_string()))?;
            Ok(())
        })
    }

    pub fn load(&mut self, path: &str) -> Result<(), DrorisError> {
        let runtime = tokio::runtime::Runtime::new()?;
        runtime.block_on(async {
            let file_path = format!("{}/users.json", path);
            if !tokio::fs::try_exists(&file_path).await.unwrap_or(false) {
                return Ok(());
            }
            let contents = tokio::fs::read_to_string(&file_path)
                .await
                .map_err(|e| DrorisError::Internal(e.to_string()))?;
            let users: Vec<UserAuth> = serde_json::from_str(&contents)
                .map_err(|e| DrorisError::Internal(e.to_string()))?;
            for user in users {
                let key = user_key(&user.username, user.hostname.as_deref());
                self.users.insert(key, user);
            }
            Ok(())
        })
    }
}

impl Default for AuthManager {
    fn default() -> Self {
        Self::new()
    }
}

fn user_key(username: &str, hostname: Option<&str>) -> String {
    match hostname {
        Some(h) => format!("{}@{}", username, h),
        None => format!("{}@%", username),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_user() {
        let manager = AuthManager::new();
        let user = UserAuth::new_native_password("testuser".to_string(), "hash123".to_string());
        assert!(manager.create_user(user).is_ok());
    }

    #[test]
    fn test_create_duplicate_user() {
        let manager = AuthManager::new();
        let user1 = UserAuth::new_native_password("testuser".to_string(), "hash1".to_string());
        let user2 = UserAuth::new_native_password("testuser".to_string(), "hash2".to_string());
        manager.create_user(user1).unwrap();
        assert!(manager.create_user(user2).is_err());
    }

    #[test]
    fn test_grant_role() {
        let manager = AuthManager::new();
        let user = UserAuth::new_native_password("testuser".to_string(), "hash".to_string());
        manager.create_user(user).unwrap();
        manager.grant_role("testuser", "admin").unwrap();
        let user = manager.get_user("testuser", None).unwrap();
        assert!(user.roles.contains(&"admin".to_string()));
    }
}