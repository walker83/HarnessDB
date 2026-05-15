use std::any::Any;
use std::fmt;
use std::sync::Arc;

use async_trait::async_trait;
use arrow_schema::Schema as ArrowSchema;
use datafusion::catalog::TableProvider;
use datafusion::datasource::MemTable;
use datafusion::error::{DataFusionError, Result as DFResult};
use datafusion::logical_expr::TableType;
use datafusion::physical_plan::ExecutionPlan;

use crate::ParquetStorage;

/// DataFusion TableProvider backed by a Parquet file.
///
/// On each `scan()`, reads the Parquet file and delegates to `MemTable`
/// for projection/filter/limit execution.
pub struct ParquetTableProvider {
    schema: Arc<ArrowSchema>,
    storage: Arc<ParquetStorage>,
    db_name: String,
    table_name: String,
}

impl fmt::Debug for ParquetTableProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ParquetTableProvider")
            .field("db", &self.db_name)
            .field("table", &self.table_name)
            .finish()
    }
}

impl ParquetTableProvider {
    pub fn new(
        storage: Arc<ParquetStorage>,
        db_name: String,
        table_name: String,
        schema: Arc<ArrowSchema>,
    ) -> Self {
        Self {
            schema,
            storage,
            db_name,
            table_name,
        }
    }
}

#[async_trait]
impl TableProvider for ParquetTableProvider {
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
        _filters: &[datafusion::prelude::Expr],
        limit: Option<usize>,
    ) -> DFResult<Arc<dyn ExecutionPlan>> {
        let rb = self
            .storage
            .read(&self.db_name, &self.table_name)
            .map_err(|e| {
                DataFusionError::Execution(format!(
                    "Failed to read {}.{}: {}",
                    self.db_name, self.table_name, e
                ))
            })?;

        let mem = MemTable::try_new(self.schema.clone(), vec![vec![rb]])?;
        mem.scan(state, projection, _filters, limit).await
    }
}
