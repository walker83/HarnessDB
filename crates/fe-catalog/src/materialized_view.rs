use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterializedView {
    pub id: u64,
    pub name: String,
    pub database: String,
    pub definition: String,
    pub base_tables: Vec<(String, String)>,
    pub refresh: RefreshStrategy,
    pub schema: Vec<MaterializedViewColumn>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterializedViewColumn {
    pub name: String,
    pub data_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RefreshStrategy {
    Manual,
    Immediate,
    Scheduled(String),
}

impl MaterializedView {
    pub fn new(id: u64, name: String, database: String, definition: String) -> Self {
        Self {
            id,
            name,
            database,
            definition,
            base_tables: Vec::new(),
            refresh: RefreshStrategy::Manual,
            schema: Vec::new(),
        }
    }

    pub fn with_base_tables(mut self, base_tables: Vec<(String, String)>) -> Self {
        self.base_tables = base_tables;
        self
    }

    pub fn with_refresh(mut self, refresh: RefreshStrategy) -> Self {
        self.refresh = refresh;
        self
    }

    pub fn with_schema(mut self, schema: Vec<MaterializedViewColumn>) -> Self {
        self.schema = schema;
        self
    }
}
