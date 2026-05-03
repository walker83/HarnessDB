use proto::data::QueryResult;
use proto::internal::PExecPlanFragmentRequest;

pub trait FeService: Send + Sync + 'static {
    fn execute_query(&self, query: &str) -> impl std::future::Future<Output = QueryResult> + Send;
    fn get_query_status(&self, query_id: &str) -> impl std::future::Future<Output = QueryResult> + Send;
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
    async fn execute_query(&self, _query: &str) -> QueryResult {
        // TODO: parse SQL -> plan -> schedule -> execute
        QueryResult {
            status: proto::status::Status::ok(),
            row_batch: None,
            query_id: uuid::Uuid::new_v4().to_string(),
        }
    }

    async fn get_query_status(&self, query_id: &str) -> QueryResult {
        QueryResult {
            status: proto::status::Status::ok(),
            row_batch: None,
            query_id: query_id.to_string(),
        }
    }
}
