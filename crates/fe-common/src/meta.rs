use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaService;

impl MetaService {
    pub fn new() -> Self {
        Self
    }

    pub fn load_meta(&self) -> Result<(), String> {
        Ok(())
    }

    pub fn save_meta(&self) -> Result<(), String> {
        Ok(())
    }
}

impl Default for MetaService {
    fn default() -> Self {
        Self::new()
    }
}
