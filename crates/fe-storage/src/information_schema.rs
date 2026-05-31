use std::any::Any;
use std::sync::Arc;

use arrow_array::{ArrayRef, RecordBatch, StringArray, UInt64Array};
use arrow_schema::{DataType as ArrowDataType, Schema as ArrowSchema, SchemaRef};
use async_trait::async_trait;
use dashmap::DashMap;
use datafusion::catalog::{SchemaProvider, TableProvider};
use datafusion::error::Result as DFResult;
use datafusion::logical_expr::TableType;
use datafusion_datasource::memory::MemorySourceConfig;

#[allow(unused_imports)]
use crate::ParquetStorage;
#[allow(unused_imports)]
use fe_catalog::CatalogManager;
use fe_datafusion::types::to_arrow_data_type;

/// Custom information_schema provider that returns MySQL-compatible metadata
pub struct InformationSchemaProvider {
    #[allow(dead_code)]
    catalog: Arc<CatalogManager>,
    #[allow(dead_code)]
    storage: Arc<ParquetStorage>,
    tables: DashMap<String, Arc<dyn TableProvider>>,
}

impl InformationSchemaProvider {
    pub fn new(catalog: Arc<CatalogManager>, storage: Arc<ParquetStorage>) -> Self {
        let tables = DashMap::new();

        // Register information_schema tables
        tables.insert(
            "tables".to_string(),
            Arc::new(InformationSchemaTables::new(
                catalog.clone(),
                storage.clone(),
            )) as Arc<dyn TableProvider>,
        );
        tables.insert(
            "columns".to_string(),
            Arc::new(InformationSchemaColumns::new(catalog.clone())) as Arc<dyn TableProvider>,
        );
        tables.insert(
            "schemata".to_string(),
            Arc::new(InformationSchemaSchemata::new(catalog.clone())) as Arc<dyn TableProvider>,
        );
        tables.insert(
            "table_constraints".to_string(),
            Arc::new(InformationSchemaTableConstraints::new(catalog.clone()))
                as Arc<dyn TableProvider>,
        );
        tables.insert(
            "key_column_usage".to_string(),
            Arc::new(InformationSchemaKeyColumnUsage::new(catalog.clone()))
                as Arc<dyn TableProvider>,
        );

        Self {
            catalog,
            storage,
            tables,
        }
    }
}

impl std::fmt::Debug for InformationSchemaProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InformationSchemaProvider").finish()
    }
}

#[async_trait]
impl SchemaProvider for InformationSchemaProvider {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn table_names(&self) -> Vec<String> {
        self.tables.iter().map(|r| r.key().clone()).collect()
    }

    async fn table(&self, name: &str) -> DFResult<Option<Arc<dyn TableProvider>>> {
        Ok(self.tables.get(name).map(|r| r.value().clone()))
    }

    fn table_exist(&self, name: &str) -> bool {
        self.tables.contains_key(name)
    }
}

/// information_schema.TABLES table
struct InformationSchemaTables {
    schema: SchemaRef,
    catalog: Arc<CatalogManager>,
    storage: Arc<ParquetStorage>,
}

impl InformationSchemaTables {
    fn new(catalog: Arc<CatalogManager>, storage: Arc<ParquetStorage>) -> Self {
        let schema = Arc::new(ArrowSchema::new(vec![
            arrow_schema::Field::new("table_catalog", ArrowDataType::Utf8, false),
            arrow_schema::Field::new("table_schema", ArrowDataType::Utf8, false),
            arrow_schema::Field::new("table_name", ArrowDataType::Utf8, false),
            arrow_schema::Field::new("table_type", ArrowDataType::Utf8, false),
            arrow_schema::Field::new("engine", ArrowDataType::Utf8, true),
            arrow_schema::Field::new("table_rows", ArrowDataType::UInt64, true),
            arrow_schema::Field::new("data_length", ArrowDataType::UInt64, true),
            arrow_schema::Field::new("table_collation", ArrowDataType::Utf8, true),
            arrow_schema::Field::new("table_comment", ArrowDataType::Utf8, true),
        ]));
        Self {
            schema,
            catalog,
            storage,
        }
    }
}

impl std::fmt::Debug for InformationSchemaTables {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InformationSchemaTables").finish()
    }
}

#[async_trait]
impl TableProvider for InformationSchemaTables {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn schema(&self) -> SchemaRef {
        self.schema.clone()
    }

    fn table_type(&self) -> TableType {
        TableType::View
    }

    async fn scan(
        &self,
        _state: &dyn datafusion::catalog::Session,
        _projection: Option<&Vec<usize>>,
        _filters: &[datafusion::prelude::Expr],
        _limit: Option<usize>,
    ) -> DFResult<Arc<dyn datafusion::physical_plan::ExecutionPlan>> {
        let mut table_catalog = Vec::new();
        let mut table_schema = Vec::new();
        let mut table_name = Vec::new();
        let mut table_type = Vec::new();
        let mut engine = Vec::new();
        let mut table_rows = Vec::new();
        let mut data_length = Vec::new();
        let mut table_collation = Vec::new();
        let mut table_comment = Vec::new();

        for db_name in self.catalog.list_databases() {
            if let Some(db) = self.catalog.get_database(&db_name) {
                let table_names: Vec<String> = db
                    .table_names()
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect();
                for tbl_name in table_names {
                    // Read row count from Parquet footer metadata (no data scan)
                    let parquet_path = self
                        .storage
                        .table_dir(&db_name, &tbl_name)
                        .join("data.parquet");
                    let row_count = if parquet_path.exists() {
                        std::fs::File::open(&parquet_path).ok()
                            .and_then(|f| {
                                parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder::try_new(f).ok()
                            })
                            .map(|b| b.metadata().file_metadata().num_rows() as u64)
                            .unwrap_or(0)
                    } else {
                        0u64
                    };

                    // Get actual file size
                    let file_size = std::fs::metadata(&parquet_path)
                        .map(|m| m.len())
                        .unwrap_or(0u64);

                    table_catalog.push(Some("roris".to_string()));
                    table_schema.push(Some(db_name.clone()));
                    table_name.push(Some(tbl_name.clone()));
                    table_type.push(Some("BASE TABLE".to_string()));
                    engine.push(Some("InnoDB".to_string()));
                    table_rows.push(Some(row_count));
                    data_length.push(Some(file_size));
                    table_collation.push(Some("utf8mb4_general_ci".to_string()));
                    table_comment.push(Some("".to_string()));
                }
            }
        }

        let batch = RecordBatch::try_new(
            self.schema.clone(),
            vec![
                Arc::new(StringArray::from(table_catalog)) as ArrayRef,
                Arc::new(StringArray::from(table_schema)) as ArrayRef,
                Arc::new(StringArray::from(table_name)) as ArrayRef,
                Arc::new(StringArray::from(table_type)) as ArrayRef,
                Arc::new(StringArray::from(engine)) as ArrayRef,
                Arc::new(UInt64Array::from(table_rows)) as ArrayRef,
                Arc::new(UInt64Array::from(data_length)) as ArrayRef,
                Arc::new(StringArray::from(table_collation)) as ArrayRef,
                Arc::new(StringArray::from(table_comment)) as ArrayRef,
            ],
        )?;

        Ok(MemorySourceConfig::try_new_exec(
            &[vec![batch]],
            self.schema.clone(),
            _projection.cloned(),
        )?)
    }
}

/// information_schema.COLUMNS table with MySQL-compatible type names
struct InformationSchemaColumns {
    schema: SchemaRef,
    catalog: Arc<CatalogManager>,
}

impl InformationSchemaColumns {
    fn new(catalog: Arc<CatalogManager>) -> Self {
        let schema = Arc::new(ArrowSchema::new(vec![
            arrow_schema::Field::new("table_catalog", ArrowDataType::Utf8, false),
            arrow_schema::Field::new("table_schema", ArrowDataType::Utf8, false),
            arrow_schema::Field::new("table_name", ArrowDataType::Utf8, false),
            arrow_schema::Field::new("column_name", ArrowDataType::Utf8, false),
            arrow_schema::Field::new("ordinal_position", ArrowDataType::UInt64, false),
            arrow_schema::Field::new("column_default", ArrowDataType::Utf8, true),
            arrow_schema::Field::new("is_nullable", ArrowDataType::Utf8, false),
            arrow_schema::Field::new("data_type", ArrowDataType::Utf8, false),
            arrow_schema::Field::new("character_maximum_length", ArrowDataType::UInt64, true),
            arrow_schema::Field::new("numeric_precision", ArrowDataType::UInt64, true),
            arrow_schema::Field::new("numeric_scale", ArrowDataType::UInt64, true),
            arrow_schema::Field::new("column_type", ArrowDataType::Utf8, false),
            arrow_schema::Field::new("column_key", ArrowDataType::Utf8, true),
            arrow_schema::Field::new("extra", ArrowDataType::Utf8, true),
            arrow_schema::Field::new("column_comment", ArrowDataType::Utf8, true),
        ]));
        Self { schema, catalog }
    }

    /// Convert Arrow DataType to MySQL type name
    fn arrow_to_mysql_type(arrow_type: &ArrowDataType) -> String {
        match arrow_type {
            ArrowDataType::Boolean => "TINYINT(1)".to_string(),
            ArrowDataType::Int8 => "TINYINT".to_string(),
            ArrowDataType::Int16 => "SMALLINT".to_string(),
            ArrowDataType::Int32 => "INT".to_string(),
            ArrowDataType::Int64 => "BIGINT".to_string(),
            ArrowDataType::UInt8 => "TINYINT UNSIGNED".to_string(),
            ArrowDataType::UInt16 => "SMALLINT UNSIGNED".to_string(),
            ArrowDataType::UInt32 => "INT UNSIGNED".to_string(),
            ArrowDataType::UInt64 => "BIGINT UNSIGNED".to_string(),
            ArrowDataType::Float16 => "FLOAT".to_string(),
            ArrowDataType::Float32 => "FLOAT".to_string(),
            ArrowDataType::Float64 => "DOUBLE".to_string(),
            ArrowDataType::Utf8 | ArrowDataType::LargeUtf8 => "TEXT".to_string(),
            ArrowDataType::Date32 | ArrowDataType::Date64 => "DATE".to_string(),
            ArrowDataType::Timestamp(_, _) => "DATETIME".to_string(),
            ArrowDataType::Decimal128(precision, scale) => {
                format!("DECIMAL({},{})", precision, scale)
            }
            ArrowDataType::Binary | ArrowDataType::LargeBinary => "BLOB".to_string(),
            _ => "TEXT".to_string(),
        }
    }
}

impl std::fmt::Debug for InformationSchemaColumns {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InformationSchemaColumns").finish()
    }
}

#[async_trait]
impl TableProvider for InformationSchemaColumns {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn schema(&self) -> SchemaRef {
        self.schema.clone()
    }

    fn table_type(&self) -> TableType {
        TableType::View
    }

    async fn scan(
        &self,
        _state: &dyn datafusion::catalog::Session,
        _projection: Option<&Vec<usize>>,
        _filters: &[datafusion::prelude::Expr],
        _limit: Option<usize>,
    ) -> DFResult<Arc<dyn datafusion::physical_plan::ExecutionPlan>> {
        let mut table_catalog = Vec::new();
        let mut table_schema = Vec::new();
        let mut table_name = Vec::new();
        let mut column_name = Vec::new();
        let mut ordinal_position = Vec::new();
        let mut column_default: Vec<Option<String>> = Vec::new();
        let mut is_nullable = Vec::new();
        let mut data_type = Vec::new();
        let mut character_maximum_length = Vec::new();
        let mut numeric_precision = Vec::new();
        let mut numeric_scale = Vec::new();
        let mut column_type = Vec::new();
        let mut column_key = Vec::new();
        let mut extra = Vec::new();
        let mut column_comment = Vec::new();

        for db_name in self.catalog.list_databases() {
            if let Some(db) = self.catalog.get_database(&db_name) {
                let table_names: Vec<String> = db
                    .table_names()
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect();
                for tbl_name in table_names {
                    if let Some(table) = self.catalog.get_table(&db_name, &tbl_name) {
                        for (idx, col) in table.columns.iter().enumerate() {
                            let arrow_type = to_arrow_data_type(&col.data_type);
                            let mysql_type = Self::arrow_to_mysql_type(&arrow_type);

                            table_catalog.push(Some("roris".to_string()));
                            table_schema.push(Some(db_name.clone()));
                            table_name.push(Some(tbl_name.clone()));
                            column_name.push(Some(col.name.clone()));
                            ordinal_position.push(Some((idx + 1) as u64)); // MySQL starts at 1
                            column_default.push(None);
                            is_nullable
                                .push(Some(if col.nullable { "YES" } else { "NO" }.to_string()));
                            data_type.push(Some(mysql_type.clone()));

                            // Character max length for string types
                            if matches!(arrow_type, ArrowDataType::Utf8 | ArrowDataType::LargeUtf8)
                            {
                                character_maximum_length.push(Some(65535u64));
                            } else {
                                character_maximum_length.push(None);
                            }

                            // Numeric precision and scale
                            if matches!(arrow_type, ArrowDataType::Decimal128(_, _)) {
                                if let ArrowDataType::Decimal128(p, s) = arrow_type {
                                    numeric_precision.push(Some(p as u64));
                                    numeric_scale.push(Some(s as u64));
                                } else {
                                    numeric_precision.push(None);
                                    numeric_scale.push(None);
                                }
                            } else {
                                numeric_precision.push(None);
                                numeric_scale.push(None);
                            }

                            column_type.push(Some(mysql_type));
                            column_key.push(Some("".to_string())); // TODO: support keys
                            extra.push(Some("".to_string())); // TODO: support auto_increment
                            column_comment.push(Some("".to_string()));
                        }
                    }
                }
            }
        }

        let batch = RecordBatch::try_new(
            self.schema.clone(),
            vec![
                Arc::new(StringArray::from(table_catalog)) as ArrayRef,
                Arc::new(StringArray::from(table_schema)) as ArrayRef,
                Arc::new(StringArray::from(table_name)) as ArrayRef,
                Arc::new(StringArray::from(column_name)) as ArrayRef,
                Arc::new(UInt64Array::from(ordinal_position)) as ArrayRef,
                Arc::new(StringArray::from(column_default)) as ArrayRef,
                Arc::new(StringArray::from(is_nullable)) as ArrayRef,
                Arc::new(StringArray::from(data_type)) as ArrayRef,
                Arc::new(UInt64Array::from(character_maximum_length)) as ArrayRef,
                Arc::new(UInt64Array::from(numeric_precision)) as ArrayRef,
                Arc::new(UInt64Array::from(numeric_scale)) as ArrayRef,
                Arc::new(StringArray::from(column_type)) as ArrayRef,
                Arc::new(StringArray::from(column_key)) as ArrayRef,
                Arc::new(StringArray::from(extra)) as ArrayRef,
                Arc::new(StringArray::from(column_comment)) as ArrayRef,
            ],
        )?;

        Ok(MemorySourceConfig::try_new_exec(
            &[vec![batch]],
            self.schema.clone(),
            _projection.cloned(),
        )?)
    }
}

/// information_schema.SCHEMATA table
struct InformationSchemaSchemata {
    schema: SchemaRef,
    catalog: Arc<CatalogManager>,
}

impl InformationSchemaSchemata {
    fn new(catalog: Arc<CatalogManager>) -> Self {
        let schema = Arc::new(ArrowSchema::new(vec![
            arrow_schema::Field::new("catalog_name", ArrowDataType::Utf8, false),
            arrow_schema::Field::new("schema_name", ArrowDataType::Utf8, false),
            arrow_schema::Field::new("schema_owner", ArrowDataType::Utf8, true),
            arrow_schema::Field::new("default_character_set_catalog", ArrowDataType::Utf8, true),
            arrow_schema::Field::new("default_character_set_schema", ArrowDataType::Utf8, true),
            arrow_schema::Field::new("default_character_set_name", ArrowDataType::Utf8, true),
            arrow_schema::Field::new("sql_path", ArrowDataType::Utf8, true),
        ]));
        Self { schema, catalog }
    }
}

impl std::fmt::Debug for InformationSchemaSchemata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InformationSchemaSchemata").finish()
    }
}

#[async_trait]
impl TableProvider for InformationSchemaSchemata {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn schema(&self) -> SchemaRef {
        self.schema.clone()
    }

    fn table_type(&self) -> TableType {
        TableType::View
    }

    async fn scan(
        &self,
        _state: &dyn datafusion::catalog::Session,
        _projection: Option<&Vec<usize>>,
        _filters: &[datafusion::prelude::Expr],
        _limit: Option<usize>,
    ) -> DFResult<Arc<dyn datafusion::physical_plan::ExecutionPlan>> {
        let mut catalog_name = Vec::new();
        let mut schema_name = Vec::new();
        let mut schema_owner = Vec::new();
        let mut default_character_set_catalog: Vec<Option<String>> = Vec::new();
        let mut default_character_set_schema: Vec<Option<String>> = Vec::new();
        let mut default_character_set_name = Vec::new();
        let mut sql_path: Vec<Option<String>> = Vec::new();

        for db_name in self.catalog.list_databases() {
            catalog_name.push(Some("roris".to_string()));
            schema_name.push(Some(db_name));
            schema_owner.push(Some("root".to_string()));
            default_character_set_catalog.push(None);
            default_character_set_schema.push(None);
            default_character_set_name.push(Some("utf8mb4".to_string()));
            sql_path.push(None);
        }

        let batch = RecordBatch::try_new(
            self.schema.clone(),
            vec![
                Arc::new(StringArray::from(catalog_name)) as ArrayRef,
                Arc::new(StringArray::from(schema_name)) as ArrayRef,
                Arc::new(StringArray::from(schema_owner)) as ArrayRef,
                Arc::new(StringArray::from(default_character_set_catalog)) as ArrayRef,
                Arc::new(StringArray::from(default_character_set_schema)) as ArrayRef,
                Arc::new(StringArray::from(default_character_set_name)) as ArrayRef,
                Arc::new(StringArray::from(sql_path)) as ArrayRef,
            ],
        )?;

        Ok(MemorySourceConfig::try_new_exec(
            &[vec![batch]],
            self.schema.clone(),
            _projection.cloned(),
        )?)
    }
}

/// information_schema.TABLE_CONSTRAINTS table (stub - no constraints in RorisDB yet)
struct InformationSchemaTableConstraints {
    schema: SchemaRef,
    _catalog: Arc<CatalogManager>,
}

impl InformationSchemaTableConstraints {
    fn new(catalog: Arc<CatalogManager>) -> Self {
        let schema = Arc::new(ArrowSchema::new(vec![
            arrow_schema::Field::new("constraint_catalog", ArrowDataType::Utf8, false),
            arrow_schema::Field::new("constraint_schema", ArrowDataType::Utf8, false),
            arrow_schema::Field::new("constraint_name", ArrowDataType::Utf8, false),
            arrow_schema::Field::new("table_catalog", ArrowDataType::Utf8, false),
            arrow_schema::Field::new("table_schema", ArrowDataType::Utf8, false),
            arrow_schema::Field::new("table_name", ArrowDataType::Utf8, false),
            arrow_schema::Field::new("constraint_type", ArrowDataType::Utf8, false),
        ]));
        Self {
            schema,
            _catalog: catalog,
        }
    }
}

impl std::fmt::Debug for InformationSchemaTableConstraints {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InformationSchemaTableConstraints").finish()
    }
}

#[async_trait]
impl TableProvider for InformationSchemaTableConstraints {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn schema(&self) -> SchemaRef {
        self.schema.clone()
    }

    fn table_type(&self) -> TableType {
        TableType::View
    }

    async fn scan(
        &self,
        _state: &dyn datafusion::catalog::Session,
        _projection: Option<&Vec<usize>>,
        _filters: &[datafusion::prelude::Expr],
        _limit: Option<usize>,
    ) -> DFResult<Arc<dyn datafusion::physical_plan::ExecutionPlan>> {
        // Return empty result set - RorisDB doesn't support constraints yet
        let batch = RecordBatch::new_empty(self.schema.clone());

        Ok(MemorySourceConfig::try_new_exec(
            &[vec![batch]],
            self.schema.clone(),
            _projection.cloned(),
        )?)
    }
}

/// information_schema.KEY_COLUMN_USAGE table (stub - no keys in RorisDB yet)
struct InformationSchemaKeyColumnUsage {
    schema: SchemaRef,
    _catalog: Arc<CatalogManager>,
}

impl InformationSchemaKeyColumnUsage {
    fn new(catalog: Arc<CatalogManager>) -> Self {
        let schema = Arc::new(ArrowSchema::new(vec![
            arrow_schema::Field::new("constraint_catalog", ArrowDataType::Utf8, false),
            arrow_schema::Field::new("constraint_schema", ArrowDataType::Utf8, false),
            arrow_schema::Field::new("constraint_name", ArrowDataType::Utf8, false),
            arrow_schema::Field::new("table_catalog", ArrowDataType::Utf8, false),
            arrow_schema::Field::new("table_schema", ArrowDataType::Utf8, false),
            arrow_schema::Field::new("table_name", ArrowDataType::Utf8, false),
            arrow_schema::Field::new("column_name", ArrowDataType::Utf8, false),
            arrow_schema::Field::new("ordinal_position", ArrowDataType::UInt64, false),
            arrow_schema::Field::new("position_in_unique_constraint", ArrowDataType::UInt64, true),
            arrow_schema::Field::new("referenced_table_schema", ArrowDataType::Utf8, true),
            arrow_schema::Field::new("referenced_table_name", ArrowDataType::Utf8, true),
            arrow_schema::Field::new("referenced_column_name", ArrowDataType::Utf8, true),
        ]));
        Self {
            schema,
            _catalog: catalog,
        }
    }
}

impl std::fmt::Debug for InformationSchemaKeyColumnUsage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InformationSchemaKeyColumnUsage").finish()
    }
}

#[async_trait]
impl TableProvider for InformationSchemaKeyColumnUsage {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn schema(&self) -> SchemaRef {
        self.schema.clone()
    }

    fn table_type(&self) -> TableType {
        TableType::View
    }

    async fn scan(
        &self,
        _state: &dyn datafusion::catalog::Session,
        _projection: Option<&Vec<usize>>,
        _filters: &[datafusion::prelude::Expr],
        _limit: Option<usize>,
    ) -> DFResult<Arc<dyn datafusion::physical_plan::ExecutionPlan>> {
        // Return empty result set - RorisDB doesn't support keys yet
        let batch = RecordBatch::new_empty(self.schema.clone());

        Ok(MemorySourceConfig::try_new_exec(
            &[vec![batch]],
            self.schema.clone(),
            _projection.cloned(),
        )?)
    }
}
