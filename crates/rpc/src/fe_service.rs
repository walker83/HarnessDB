use proto::{Status, RowBatch};

pub trait FeService: Send + Sync + 'static {
    fn execute_query(&self, query: &str) -> impl std::future::Future<Output = FeQueryResult> + Send;
    fn get_query_status(&self, query_id: &str) -> impl std::future::Future<Output = FeQueryResult> + Send;
}

#[derive(Debug, Clone)]
pub struct FeQueryResult {
    pub status: Status,
    pub row_batch: Option<RowBatch>,
    pub query_id: String,
}

pub struct FeServiceImpl {
    // TODO: catalog manager, planner, scheduler
}

impl FeServiceImpl {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for FeServiceImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl FeService for FeServiceImpl {
    async fn execute_query(&self, _query: &str) -> FeQueryResult {
        // TODO: parse SQL -> plan -> schedule -> execute
        FeQueryResult {
            status: Status { code: 0, message: "OK".to_string(), details: vec![] },
            row_batch: None,
            query_id: uuid::Uuid::new_v4().to_string(),
        }
    }

    async fn get_query_status(&self, query_id: &str) -> FeQueryResult {
        FeQueryResult {
            status: Status { code: 0, message: "OK".to_string(), details: vec![] },
            row_batch: None,
            query_id: query_id.to_string(),
        }
    }
}