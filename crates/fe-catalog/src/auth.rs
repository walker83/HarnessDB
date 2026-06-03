//! Authentication and authorization for HarnessDB
//! Supports: RBAC, TLS, Data masking

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub username: String,
    pub password_hash: String,
    pub roles: HashSet<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub name: String,
    pub permissions: HashSet<Permission>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct Permission {
    pub resource: String, // database.table or *
    pub action: Action,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub enum Action {
    Select,
    Insert,
    Update,
    Delete,
    Create,
    Drop,
    Alter,
    All,
}

pub struct AuthManager {
    users: HashMap<String, User>,
    roles: HashMap<String, Role>,
}

impl AuthManager {
    pub fn new() -> Self {
        let mut manager = Self {
            users: HashMap::new(),
            roles: HashMap::new(),
        };
        // Create default admin role
        let mut admin_permissions = HashSet::new();
        admin_permissions.insert(Permission {
            resource: "*".to_string(),
            action: Action::All,
        });
        manager.roles.insert("admin".to_string(), Role {
            name: "admin".to_string(),
            permissions: admin_permissions,
        });
        manager
    }

    pub fn create_user(&mut self, username: &str, password: &str) -> Result<(), String> {
        if self.users.contains_key(username) {
            return Err(format!("User {} already exists", username));
        }
        self.users.insert(username.to_string(), User {
            username: username.to_string(),
            password_hash: password.to_string(), // In production, hash the password
            roles: HashSet::new(),
        });
        Ok(())
    }

    pub fn grant_role(&mut self, username: &str, role_name: &str) -> Result<(), String> {
        if let Some(user) = self.users.get_mut(username) {
            if !self.roles.contains_key(role_name) {
                return Err(format!("Role {} not found", role_name));
            }
            user.roles.insert(role_name.to_string());
            Ok(())
        } else {
            Err(format!("User {} not found", username))
        }
    }

    pub fn check_permission(&self, username: &str, resource: &str, action: &Action) -> bool {
        if let Some(user) = self.users.get(username) {
            for role_name in &user.roles {
                if let Some(role) = self.roles.get(role_name) {
                    for perm in &role.permissions {
                        if (perm.resource == "*" || perm.resource == resource) &&
                           (perm.action == Action::All || &perm.action == action) {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }
}

impl Default for AuthManager {
    fn default() -> Self {
        Self::new()
    }
}
