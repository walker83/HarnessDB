use proto::{Status, ExecPlanFragmentRequest, ExecPlanFragmentResponse, CancelPlanFragmentRequest, FetchDataRequest, FetchDataResponse, HeartbeatRequest, HeartbeatResponse, BackendService};
use dashmap::DashMap;
use tonic::{Request, Response, Status as TonicStatus, Code};

// Fragment execution state
#[derive(Debug, Clone)]
struct FragmentState {
    query_id: String,
    status: ExecutionStatus,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ExecutionStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
}

// gRPC server implementation
pub struct BeGrpcServer {
    fragments: DashMap<String, FragmentState>,
}

impl Default for BeGrpcServer {
    fn default() -> Self {
        Self::new()
    }
}

impl BeGrpcServer {
    pub fn new() -> Self {
        Self {
            fragments: DashMap::new(),
        }
    }

    async fn exec_plan_fragment_internal(&self, req: ExecPlanFragmentRequest) -> Result<ExecPlanFragmentResponse, TonicStatus> {
        let fragment_id = req.fragment_instance_id.clone();

        let state = FragmentState {
            query_id: req.fragment_instance_id.clone(),
            status: ExecutionStatus::Running,
        };

        self.fragments.insert(fragment_id.clone(), state);

        // Mark as completed
        if let Some(mut fragment_ref) = self.fragments.get_mut(&fragment_id) {
            fragment_ref.status = ExecutionStatus::Completed;
        }

        Ok(ExecPlanFragmentResponse {
            status: Some(Status {
                code: 0,
                message: "Fragment executed successfully".to_string(),
                details: vec![],
            }),
            query_id: req.fragment_instance_id,
        })
    }

    async fn cancel_plan_fragment_internal(&self, fragment_id: String) -> Result<Status, TonicStatus> {
        if let Some(mut fragment_ref) = self.fragments.get_mut(&fragment_id) {
            fragment_ref.status = ExecutionStatus::Cancelled;
            Ok(Status {
                code: 0,
                message: "Fragment cancelled".to_string(),
                details: vec![],
            })
        } else {
            Err(TonicStatus::new(Code::NotFound, "Fragment not found"))
        }
    }

    async fn fetch_data_internal(&self, req: FetchDataRequest) -> Result<FetchDataResponse, TonicStatus> {
        if let Some(fragment_ref) = self.fragments.get(&req.fragment_id) {
            Ok(FetchDataResponse {
                status: Some(Status {
                    code: 0,
                    message: "Data fetched".to_string(),
                    details: vec![],
                }),
                row_batch: None,
                query_id: fragment_ref.query_id.clone(),
            })
        } else {
            Err(TonicStatus::new(Code::NotFound, "Fragment not found"))
        }
    }

    async fn heartbeat_internal(&self, _req: HeartbeatRequest) -> Result<HeartbeatResponse, TonicStatus> {
        Ok(HeartbeatResponse {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
        })
    }
}

#[tonic::async_trait]
impl BackendService for BeGrpcServer {
    async fn exec_plan_fragment(
        &self,
        request: Request<ExecPlanFragmentRequest>,
    ) -> Result<Response<ExecPlanFragmentResponse>, TonicStatus> {
        let req = request.into_inner();
        let response = self.exec_plan_fragment_internal(req).await?;
        Ok(Response::new(response))
    }

    async fn cancel_plan_fragment(
        &self,
        request: Request<CancelPlanFragmentRequest>,
    ) -> Result<Response<Status>, TonicStatus> {
        let req = request.into_inner();
        let response = self.cancel_plan_fragment_internal(req.fragment_id).await?;
        Ok(Response::new(response))
    }

    async fn fetch_data(
        &self,
        request: Request<FetchDataRequest>,
    ) -> Result<Response<FetchDataResponse>, TonicStatus> {
        let req = request.into_inner();
        let response = self.fetch_data_internal(req).await?;
        Ok(Response::new(response))
    }

    async fn heartbeat(
        &self,
        request: Request<HeartbeatRequest>,
    ) -> Result<Response<HeartbeatResponse>, TonicStatus> {
        let req = request.into_inner();
        let response = self.heartbeat_internal(req).await?;
        Ok(Response::new(response))
    }
}