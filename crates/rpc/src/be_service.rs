use proto::data::QueryResult;
use proto::internal::PExecPlanFragmentRequest;
use proto::{Status, RowBatch, Column, DataType};
use proto::{backend_service_server::BackendService, *};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;
use tonic::{Request, Response, Status as TonicStatus, Code};

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

// Fragment execution state
#[derive(Debug, Clone)]
struct FragmentState {
    query_id: String,
    status: ExecutionStatus,
    result: Option<QueryResult>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ExecutionStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
}

// Actual gRPC server implementation
pub struct BeGrpcServer {
    fragments: Arc<RwLock<HashMap<String, FragmentState>>>,
}

impl BeGrpcServer {
    pub fn new() -> Self {
        Self {
            fragments: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn exec_plan_fragment_internal(&self, req: ExecPlanFragmentRequest) -> Result<ExecPlanFragmentResponse, TonicStatus> {
        let fragment_id = req.fragment_instance_id.clone();

        // Create fragment state
        let state = FragmentState {
            query_id: req.query_id.clone(),
            status: ExecutionStatus::Running,
            result: None,
        };

        self.fragments.write().await.insert(fragment_id.clone(), state);

        // TODO: Actually execute the plan fragment
        // For now, just mark as completed
        {
            let mut fragments = self.fragments.write().await;
            if let Some(fragment) = fragments.get_mut(&fragment_id) {
                fragment.status = ExecutionStatus::Completed;
            }
        }

        Ok(ExecPlanFragmentResponse {
            status: Some(Status {
                code: 0,
                message: "Fragment executed successfully".to_string(),
                details: vec![],
            }),
            query_id: req.query_id,
        })
    }

    async fn cancel_plan_fragment_internal(&self, fragment_id: String) -> Result<Status, TonicStatus> {
        let mut fragments = self.fragments.write().await;
        if let Some(fragment) = fragments.get_mut(&fragment_id) {
            fragment.status = ExecutionStatus::Cancelled;
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
        let fragments = self.fragments.read().await;
        if let Some(fragment) = fragments.get(&req.fragment_instance_id) {
            Ok(FetchDataResponse {
                status: Some(Status {
                    code: 0,
                    message: "Data fetched".to_string(),
                    details: vec![],
                }),
                row_batch: None, // TODO: Return actual data
                query_id: fragment.query_id.clone(),
            })
        } else {
            Err(TonicStatus::new(Code::NotFound, "Fragment not found"))
        }
    }

    async fn heartbeat_internal(&self, req: HeartbeatRequest) -> Result<HeartbeatResponse, TonicStatus> {
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
    ) -> Result<Response<proto::be_proto::be::Status>, TonicStatus> {
        let req = request.into_inner();
        let response = self.cancel_plan_fragment_internal(req.fragment_instance_id).await?;
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

// Legacy service implementation for backward compatibility
pub struct BeServiceImpl {
    grpc_server: Arc<BeGrpcServer>,
}

impl BeServiceImpl {
    pub fn new() -> Self {
        Self {
            grpc_server: Arc::new(BeGrpcServer::new()),
        }
    }

    pub fn grpc_server(&self) -> Arc<BeGrpcServer> {
        self.grpc_server.clone()
    }
}

impl Default for BeServiceImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl BeService for BeServiceImpl {
    async fn exec_plan_fragment(&self, req: PExecPlanFragmentRequest) -> Status {
        // Convert legacy request to new request format
        let new_req = ExecPlanFragmentRequest {
            fragment_instance_id: req.fragment_instance_id,
            plan: req.plan,
            desc_tbl: req.desc_tbl,
            params: req.params.map(|p| QueryOptions {
                query_timeout: p.query_timeout,
                mem_limit: p.mem_limit,
                query_type: p.query_type,
            }),
            query_id: String::new(), // Will be generated
        };

        match self.grpc_server.exec_plan_fragment_internal(new_req).await {
            Ok(_) => Status::ok(),
            Err(e) => Status::error(e.code() as i32, e.message().to_string()),
        }
    }

    async fn cancel_plan_fragment(&self, fragment_instance_id: &str) -> Status {
        match self.grpc_server.cancel_plan_fragment_internal(fragment_instance_id.to_string()).await {
            Ok(_) => Status::ok(),
            Err(e) => Status::error(e.code() as i32, e.message().to_string()),
        }
    }

    async fn fetch_data(&self, fragment_instance_id: &str) -> QueryResult {
        let req = FetchDataRequest {
            fragment_instance_id: fragment_instance_id.to_string(),
            max_rows: 1000,
        };

        match self.grpc_server.fetch_data_internal(req).await {
            Ok(resp) => QueryResult {
                status: Status::ok(),
                row_batch: None,
                query_id: resp.query_id,
            },
            Err(e) => QueryResult {
                status: Status::error(e.code() as i32, e.message().to_string()),
                row_batch: None,
                query_id: String::new(),
            },
        }
    }
}
