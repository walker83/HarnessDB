use proto::data::QueryResult;
use proto::internal::PExecPlanFragmentRequest;
use proto::status::Status;

pub trait BeService: Send + Sync + 'static {
    fn exec_plan_fragment(
        &self,
        req: PExecPlanFragmentRequest,
    ) -> impl std::future::Future<Output = Status> + Send;

    fn cancel_plan_fragment(
        &self,
        fragment_instance_id: &str,
    ) -> impl std::future::Future<Output = Status> + Send;

    fn fetch_data(
        &self,
        fragment_instance_id: &str,
    ) -> impl std::future::Future<Output = QueryResult> + Send;
}

pub struct BeServiceImpl {
    // TODO: exec env, storage engine
}

impl BeServiceImpl {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for BeServiceImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl BeService for BeServiceImpl {
    async fn exec_plan_fragment(&self, _req: PExecPlanFragmentRequest) -> Status {
        Status::ok()
    }

    async fn cancel_plan_fragment(&self, _fragment_instance_id: &str) -> Status {
        Status::ok()
    }

    async fn fetch_data(&self, _fragment_instance_id: &str) -> QueryResult {
        QueryResult {
            status: Status::ok(),
            row_batch: None,
            query_id: String::new(),
        }
    }
}
