//! Repository management for backup storage

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Repository information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryInfo {
    pub name: String,
    pub path: String,
    pub created_at: String,
}

/// Repository persistence structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct RepositoryStore {
    repositories: HashMap<String, RepositoryInfo>,
}

/// Manages backup repositories
pub struct RepositoryManager {
    meta_dir: PathBuf,
    store: RepositoryStore,
}

impl RepositoryManager {
    pub fn new(meta_dir: &str) -> Self {
        let meta_dir = PathBuf::from(meta_dir);
        let store = Self::load_store(&meta_dir);
        Self { meta_dir, store }
    }

    fn store_path(meta_dir: &Path) -> PathBuf {
        meta_dir.join("repositories.json")
    }

    fn load_store(meta_dir: &Path) -> RepositoryStore {
        let path = Self::store_path(meta_dir);
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(content) => match serde_json::from_str(&content) {
                    Ok(store) => store,
                    Err(e) => {
                        tracing::warn!(
                            "Corrupted repositories.json: {}. Starting with empty repository list.",
                            e
                        );
                        RepositoryStore::default()
                    }
                },
                Err(_) => RepositoryStore::default(),
            }
        } else {
            RepositoryStore::default()
        }
    }

    fn save_store(&self) -> Result<(), String> {
        let path = Self::store_path(&self.meta_dir);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create meta directory: {}", e))?;
        }
        let json = serde_json::to_string_pretty(&self.store)
            .map_err(|e| format!("Failed to serialize repositories: {}", e))?;
        std::fs::write(&path, json).map_err(|e| format!("Failed to write repositories: {}", e))?;
        Ok(())
    }

    /// Create a new repository
    pub fn create_repository(&mut self, name: &str, path: &str) -> Result<(), String> {
        if self.store.repositories.contains_key(name) {
            return Err(format!("Repository '{}' already exists", name));
        }

        // Create the directory
        std::fs::create_dir_all(path)
            .map_err(|e| format!("Failed to create repository directory '{}': {}", path, e))?;

        let info = RepositoryInfo {
            name: name.to_string(),
            path: path.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        self.store.repositories.insert(name.to_string(), info);
        self.save_store()?;

        Ok(())
    }

    /// Drop a repository
    pub fn drop_repository(&mut self, name: &str) -> Result<(), String> {
        if self.store.repositories.remove(name).is_none() {
            return Err(format!("Repository '{}' not found", name));
        }
        self.save_store()?;
        Ok(())
    }

    /// List all repositories
    pub fn list_repositories(&self) -> Vec<String> {
        self.store.repositories.keys().cloned().collect()
    }

    /// Get repository info
    pub fn get_repository(&self, name: &str) -> Option<&RepositoryInfo> {
        self.store.repositories.get(name)
    }

    /// Get repository path
    pub fn get_repo_path(&self, name: &str) -> Result<PathBuf, String> {
        self.store
            .repositories
            .get(name)
            .map(|r| PathBuf::from(&r.path))
            .ok_or_else(|| format!("Repository '{}' not found", name))
    }

    /// Get all repository details for SHOW
    pub fn list_repository_details(&self) -> Vec<(&str, &str, &str)> {
        self.store
            .repositories
            .values()
            .map(|r| (r.name.as_str(), r.path.as_str(), r.created_at.as_str()))
            .collect()
    }
}
