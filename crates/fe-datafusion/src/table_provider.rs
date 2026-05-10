use std::any::Any;
use std::fmt;
use std::sync::Arc;

use datafusion::arrow::datatypes::Schema as ArrowSchema;
use datafusion::catalog::TableProvider;
use datafusion::datasource::MemTable;
use datafusion::error::{DataFusionError, Result as DFResult};
use datafusion::logical_expr::TableType;
use datafusion::physical_plan::ExecutionPlan;

use be_storage::StorageEngine;

/// A DataFusion `TableProvider` backed by RorisDB's storage engine.
///
/// On each `scan()`, reads all data from the storage tablet, converts to Arrow
/// `RecordBatch`, and delegates execution to `MemTable`.
pub struct RorisTableProvider {
    schema: Arc<ArrowSchema>,
    storage: Arc<StorageEngine>,
    tablet_id: u64,
}

impl fmt::Debug for RorisTableProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RorisTableProvider")
            .field("tablet_id", &self.tablet_id)
            .field("schema", &self.schema)
            .finish()
    }
}

impl RorisTableProvider {
    pub fn new(
        storage: Arc<StorageEngine>,
        tablet_id: u64,
        schema: Arc<ArrowSchema>,
    ) -> Self {
        Self { schema, storage, tablet_id }
    }

    /// Create from a RorisDB `TabletSchema`.
    #[allow(dead_code)]
    pub fn from_tablet_schema(
        storage: Arc<StorageEngine>,
        tablet_id: u64,
        tablet_schema: &be_storage::tablet::TabletSchema,
    ) -> Self {
        let fields: Vec<arrow_schema::Field> = tablet_schema
            .columns
            .iter()
            .map(|c| {
                arrow_schema::Field::new(
                    &c.name,
                    crate::types::to_arrow_data_type(&c.data_type),
                    c.nullable,
                )
            })
            .collect();
        let schema = Arc::new(ArrowSchema::new(fields));
        Self { schema, storage, tablet_id }
    }
}

#[async_trait::async_trait]
impl TableProvider for RorisTableProvider {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn schema(&self) -> Arc<ArrowSchema> {
        self.schema.clone()
    }

    fn table_type(&self) -> TableType {
        TableType::Base
    }

    async fn scan(
        &self,
        state: &dyn datafusion::catalog::Session,
        projection: Option<&Vec<usize>>,
        filters: &[datafusion::prelude::Expr],
        limit: Option<usize>,
    ) -> DFResult<Arc<dyn ExecutionPlan>> {
        // Read all data from storage
        let block = self.storage
            .read_tablet(self.tablet_id, None, &[])
            .map_err(|e| DataFusionError::Execution(format!("Failed to read tablet {}: {}", self.tablet_id, e)))?;

        let rb = crate::block_convert::block_to_record_batch(&block)
            .map_err(DataFusionError::Execution)?;

        // Wrap in MemTable and delegate scan to it — this handles projection,
        // filter pushdown, and creates the proper MemoryExec internally.
        let mem = MemTable::try_new(self.schema.clone(), vec![vec![rb]])?;
        mem.scan(state, projection, filters, limit).await
    }
}
