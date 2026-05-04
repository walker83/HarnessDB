use std::sync::Arc;
use tokio::sync::RwLock;
use fe_catalog::{CatalogManager, Table, table::TableColumn};
use types::{DataType, ScalarValue, Schema, Field, Vector, Block};

/// Information Schema provider for system metadata views
pub struct InformationSchema {
    catalog: Arc<RwLock<CatalogManager>>,
}

impl InformationSchema {
    pub fn new(catalog: Arc<RwLock<CatalogManager>>) -> Self {
        Self { catalog }
    }

    /// Query a specific information schema table
    pub async fn query_table(&self, table_name: &str) -> Result<Block, String> {
        match table_name {
            "tables" => self.query_tables().await,
            "columns" => self.query_columns().await,
            "databases" => self.query_databases().await,
            "processlist" => self.query_processlist().await,
            _ => Err(format!("Unknown information_schema table: {}", table_name)),
        }
    }

    /// Query all tables metadata
    async fn query_tables(&self) -> Result<Block, String> {
        let catalog = self.catalog.read().await;
        let mut rows = Vec::new();

        for db_entry in catalog.list_databases() {
            if let Some(tables) = catalog.list_tables(&db_entry) {
                for table_name in tables {
                    if let Some(table) = catalog.get_table(&db_entry, &table_name) {
                        rows.push(self.build_table_row(&db_entry, &table));
                    }
                }
            }
        }

        self.build_tables_block(rows)
    }

    /// Query all columns metadata
    async fn query_columns(&self) -> Result<Block, String> {
        let catalog = self.catalog.read().await;
        let mut rows = Vec::new();

        for db_entry in catalog.list_databases() {
            if let Some(tables) = catalog.list_tables(&db_entry) {
                for table_name in tables {
                    if let Some(table) = catalog.get_table(&db_entry, &table_name) {
                        for (pos, col) in table.columns.iter().enumerate() {
                            rows.push(self.build_column_row(&db_entry, &table_name, pos, col));
                        }
                    }
                }
            }
        }

        self.build_columns_block(rows)
    }

    /// Query all databases
    async fn query_databases(&self) -> Result<Block, String> {
        let catalog = self.catalog.read().await;
        let mut rows = Vec::new();

        for db_name in catalog.list_databases() {
            rows.push(self.build_database_row(&db_name));
        }

        self.build_databases_block(rows)
    }

    /// Query running queries (processlist)
    async fn query_processlist(&self) -> Result<Block, String> {
        let rows = Vec::new();
        self.build_processlist_block(rows)
    }

    fn build_table_row(&self, db_name: &str, table: &Table) -> Vec<ScalarValue> {
        vec![
            ScalarValue::String("def".to_string()),
            ScalarValue::String(db_name.to_string()),
            ScalarValue::String(table.name.clone()),
            ScalarValue::String("BASE TABLE".to_string()),
            ScalarValue::Null,
            ScalarValue::Null,
            ScalarValue::Null,
            ScalarValue::Null,
            ScalarValue::String(format!("{}", table.id)),
            ScalarValue::Null,
            ScalarValue::Null,
            ScalarValue::Null,
            ScalarValue::Null,
            ScalarValue::Null,
            ScalarValue::Null,
            ScalarValue::String("utf8mb4_general_ci".to_string()),
            ScalarValue::String("utf8mb4_general_ci".to_string()),
            ScalarValue::String("utf8mb4".to_string()),
            ScalarValue::Null,
            ScalarValue::Null,
            ScalarValue::Null,
        ]
    }

    fn build_column_row(&self, db_name: &str, table_name: &str, pos: usize, col: &TableColumn) -> Vec<ScalarValue> {
        vec![
            ScalarValue::String("def".to_string()),
            ScalarValue::String(db_name.to_string()),
            ScalarValue::String(table_name.to_string()),
            ScalarValue::String(col.name.clone()),
            ScalarValue::Int32(pos as i32),
            ScalarValue::Null,
            ScalarValue::String(format!("{}", col.data_type).to_uppercase()),
            ScalarValue::String(if col.nullable { "YES" } else { "NO" }.to_string()),
            ScalarValue::Null,
            ScalarValue::Null,
            ScalarValue::Null,
            ScalarValue::Null,
            ScalarValue::Null,
            ScalarValue::String("utf8mb4_general_ci".to_string()),
            ScalarValue::Null,
            ScalarValue::String("select".to_string()),
            ScalarValue::Null,
            ScalarValue::String("".to_string()),
            ScalarValue::String("select,insert,update,references".to_string()),
            ScalarValue::String(col.comment.clone()),
            ScalarValue::Null,
        ]
    }

    fn build_database_row(&self, db_name: &str) -> Vec<ScalarValue> {
        vec![
            ScalarValue::String(db_name.to_string()),
        ]
    }

    fn build_tables_block(&self, rows: Vec<Vec<ScalarValue>>) -> Result<Block, String> {
        let schema = vec![
            Field::new("TABLE_CATALOG", DataType::String, true),
            Field::new("TABLE_SCHEMA", DataType::String, true),
            Field::new("TABLE_NAME", DataType::String, true),
            Field::new("TABLE_TYPE", DataType::String, true),
            Field::new("ENGINE", DataType::String, true),
            Field::new("VERSION", DataType::String, true),
            Field::new("ROW_FORMAT", DataType::String, true),
            Field::new("TABLE_ROWS", DataType::String, true),
            Field::new("AVG_ROW_LENGTH", DataType::String, true),
            Field::new("DATA_LENGTH", DataType::String, true),
            Field::new("MAX_DATA_LENGTH", DataType::String, true),
            Field::new("INDEX_LENGTH", DataType::String, true),
            Field::new("DATA_FREE", DataType::String, true),
            Field::new("AUTO_INCREMENT", DataType::String, true),
            Field::new("CREATE_TIME", DataType::String, true),
            Field::new("UPDATE_TIME", DataType::String, true),
            Field::new("CHECK_TIME", DataType::String, true),
            Field::new("TABLE_COLLATION", DataType::String, true),
            Field::new("CHECKSUM", DataType::String, true),
            Field::new("CREATE_OPTIONS", DataType::String, true),
            Field::new("TABLE_COMMENT", DataType::String, true),
        ];

        let schema = Schema::new(schema);

        if rows.is_empty() {
            return Ok(Block::empty(schema));
        }

        let num_cols = rows[0].len();
        let mut column_data: Vec<Vec<ScalarValue>> = (0..num_cols).map(|_| Vec::new()).collect();

        for row in &rows {
            for (col_idx, value) in row.iter().enumerate() {
                if col_idx < column_data.len() {
                    column_data[col_idx].push(value.clone());
                }
            }
        }

        let columns: Result<Vec<Vector>, String> = schema.fields().iter()
            .enumerate()
            .map(|(idx, field)| self.scalar_values_to_vector(&column_data[idx], &field.data_type))
            .collect();

        let columns = columns?;
        Ok(Block::new(schema, columns))
    }

    fn scalar_values_to_vector(&self, values: &[ScalarValue], dtype: &DataType) -> Result<Vector, String> {
        match dtype {
            DataType::String => {
                let mut vec = types::vector::StringVector::new();
                for val in values {
                    match val {
                        ScalarValue::String(s) => vec.push(Some(s)),
                        ScalarValue::Null => vec.push(None),
                        _ => vec.push(Some(&format!("{:?}", val))),
                    }
                }
                Ok(Vector::String(vec))
            }
            DataType::Int32 => {
                let mut vec = types::vector::Int32Vector::new();
                for val in values {
                    match val {
                        ScalarValue::Int32(i) => vec.push(Some(*i)),
                        ScalarValue::Null => vec.push(None),
                        _ => return Err(format!("Cannot convert {:?} to Int32", val)),
                    }
                }
                Ok(Vector::Int32(vec))
            }
            _ => {
                let vec = types::vector::NullVector::new(values.len());
                Ok(Vector::Null(vec))
            }
        }
    }

    fn build_columns_block(&self, rows: Vec<Vec<ScalarValue>>) -> Result<Block, String> {
        let schema = vec![
            Field::new("TABLE_CATALOG", DataType::String, true),
            Field::new("TABLE_SCHEMA", DataType::String, true),
            Field::new("TABLE_NAME", DataType::String, true),
            Field::new("COLUMN_NAME", DataType::String, true),
            Field::new("ORDINAL_POSITION", DataType::Int32, true),
            Field::new("COLUMN_DEFAULT", DataType::String, true),
            Field::new("IS_NULLABLE", DataType::String, true),
            Field::new("DATA_TYPE", DataType::String, true),
            Field::new("CHARACTER_MAXIMUM_LENGTH", DataType::Int32, true),
            Field::new("CHARACTER_OCTET_LENGTH", DataType::Int32, true),
            Field::new("NUMERIC_PRECISION", DataType::Int32, true),
            Field::new("NUMERIC_SCALE", DataType::Int32, true),
            Field::new("DATETIME_PRECISION", DataType::Int32, true),
            Field::new("CHARACTER_SET_NAME", DataType::String, true),
            Field::new("COLLATION_NAME", DataType::String, true),
            Field::new("COLUMN_TYPE", DataType::String, true),
            Field::new("COLUMN_KEY", DataType::String, true),
            Field::new("EXTRA", DataType::String, true),
            Field::new("PRIVILEGES", DataType::String, true),
            Field::new("COLUMN_COMMENT", DataType::String, true),
            Field::new("GENERATION_EXPRESSION", DataType::String, true),
        ];

        let schema = Schema::new(schema);

        if rows.is_empty() {
            return Ok(Block::empty(schema));
        }

        let num_cols = rows[0].len();
        let mut column_data: Vec<Vec<ScalarValue>> = (0..num_cols).map(|_| Vec::new()).collect();

        for row in &rows {
            for (col_idx, value) in row.iter().enumerate() {
                if col_idx < column_data.len() {
                    column_data[col_idx].push(value.clone());
                }
            }
        }

        let columns: Result<Vec<Vector>, String> = schema.fields().iter()
            .enumerate()
            .map(|(idx, field)| self.scalar_values_to_vector(&column_data[idx], &field.data_type))
            .collect();

        let columns = columns?;
        Ok(Block::new(schema, columns))
    }

    fn build_databases_block(&self, rows: Vec<Vec<ScalarValue>>) -> Result<Block, String> {
        let schema = vec![
            Field::new("DATABASE_NAME", DataType::String, true),
        ];

        let schema = Schema::new(schema);

        if rows.is_empty() {
            return Ok(Block::empty(schema));
        }

        let num_cols = rows[0].len();
        let mut column_data: Vec<Vec<ScalarValue>> = (0..num_cols).map(|_| Vec::new()).collect();

        for row in &rows {
            for (col_idx, value) in row.iter().enumerate() {
                if col_idx < column_data.len() {
                    column_data[col_idx].push(value.clone());
                }
            }
        }

        let columns: Result<Vec<Vector>, String> = schema.fields().iter()
            .enumerate()
            .map(|(idx, field)| self.scalar_values_to_vector(&column_data[idx], &field.data_type))
            .collect();

        let columns = columns?;
        Ok(Block::new(schema, columns))
    }

    fn build_processlist_block(&self, rows: Vec<Vec<ScalarValue>>) -> Result<Block, String> {
        let schema = vec![
            Field::new("ID", DataType::Int64, true),
            Field::new("USER", DataType::String, true),
            Field::new("HOST", DataType::String, true),
            Field::new("DB", DataType::String, true),
            Field::new("COMMAND", DataType::String, true),
            Field::new("TIME", DataType::Int64, true),
            Field::new("STATE", DataType::String, true),
            Field::new("INFO", DataType::String, true),
        ];

        let schema = Schema::new(schema);

        if rows.is_empty() {
            return Ok(Block::empty(schema));
        }

        let num_cols = rows[0].len();
        let mut column_data: Vec<Vec<ScalarValue>> = (0..num_cols).map(|_| Vec::new()).collect();

        for row in &rows {
            for (col_idx, value) in row.iter().enumerate() {
                if col_idx < column_data.len() {
                    column_data[col_idx].push(value.clone());
                }
            }
        }

        let columns: Result<Vec<Vector>, String> = schema.fields().iter()
            .enumerate()
            .map(|(idx, field)| self.scalar_values_to_vector(&column_data[idx], &field.data_type))
            .collect();

        let columns = columns?;
        Ok(Block::new(schema, columns))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fe_catalog::{CatalogManager, Table};
    use fe_catalog::table::{TableColumn, KeysType};
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use types::DataType;

    #[tokio::test]
    async fn test_information_schema_query_tables() {
        let catalog = Arc::new(RwLock::new(CatalogManager::new()));
        let info_schema = InformationSchema::new(catalog);

        let result = info_schema.query_table("tables").await;
        assert!(result.is_ok());

        let block = result.unwrap();
        assert!(!block.columns().is_empty());
    }

    #[tokio::test]
    async fn test_information_schema_query_databases() {
        let catalog = Arc::new(RwLock::new(CatalogManager::new()));
        let info_schema = InformationSchema::new(catalog);

        let result = info_schema.query_table("databases").await;
        assert!(result.is_ok());

        let block = result.unwrap();
        assert!(!block.columns().is_empty());
        assert!(block.columns()[0].len() > 0);
    }

    #[tokio::test]
    async fn test_information_schema_query_columns() {
        let catalog = Arc::new(RwLock::new(CatalogManager::new()));
        let info_schema = InformationSchema::new(catalog);

        let result = info_schema.query_table("columns").await;
        assert!(result.is_ok());

        let block = result.unwrap();
        assert!(!block.columns().is_empty());
    }

    #[tokio::test]
    async fn test_information_schema_query_unknown_table() {
        let catalog = Arc::new(RwLock::new(CatalogManager::new()));
        let info_schema = InformationSchema::new(catalog);

        let result = info_schema.query_table("unknown_table").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_information_schema_with_actual_data() {
        let catalog = Arc::new(RwLock::new(CatalogManager::new()));
        {
            let catalog_guard = catalog.write().await;
            catalog_guard.create_database("test_db").unwrap();

            let table = Table {
                id: 1,
                name: "test_table".to_string(),
                database: "test_db".to_string(),
                columns: vec![
                    TableColumn {
                        name: "id".to_string(),
                        data_type: DataType::Int64,
                        nullable: false,
                        default_value: None,
                        agg_type: None,
                        comment: "Primary key".to_string(),
                    },
                    TableColumn {
                        name: "name".to_string(),
                        data_type: DataType::String,
                        nullable: true,
                        default_value: None,
                        agg_type: None,
                        comment: "User name".to_string(),
                    },
                ],
                keys_type: KeysType::Duplicate,
                partition_info: None,
                distribution_info: None,
                replication_num: 1,
                properties: HashMap::new(),
                row_count: 0,
                data_size: 0,
                stats: None,
            };

            catalog_guard.create_table("test_db", table).unwrap();
        }

        let info_schema = InformationSchema::new(catalog);

        let tables_result = info_schema.query_table("tables").await;
        assert!(tables_result.is_ok());
        let tables_block = tables_result.unwrap();
        assert!(tables_block.columns()[2].len() > 0);

        let columns_result = info_schema.query_table("columns").await;
        assert!(columns_result.is_ok());
        let columns_block = columns_result.unwrap();
        assert!(columns_block.columns()[3].len() >= 2);
    }
}
