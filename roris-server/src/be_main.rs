use anyhow::Result;
use clap::Parser;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Parser)]
#[command(name = "roris-be", about = "Roris Backend Server")]
struct Args {
    #[arg(long, default_value = "conf/be.conf")]
    config: String,

    #[arg(long, default_value = "8060")]
    http_port: u16,

    #[arg(long, default_value = "9060")]
    rpc_port: u16,

    #[arg(long, default_value = "9050")]
    heartbeat_port: u16,

    #[arg(long, default_value = "data/be/storage")]
    storage_dir: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    tracing::info!("Roris BE starting...");
    tracing::info!("Config file: {}", args.config);
    tracing::info!("HTTP port: {}, RPC port: {}", args.http_port, args.rpc_port);
    tracing::info!("Heartbeat port: {}", args.heartbeat_port);
    tracing::info!("Storage directory: {}", args.storage_dir);

    // Initialize storage engine
    let storage = Arc::new(RwLock::new(be_storage::StorageEngine::open(&args.storage_dir)?));
    tracing::info!("Storage engine initialized");

    // Start HTTP server for health checks and metrics
    let http_port = args.http_port;
    tokio::spawn(async move {
        use axum::{routing::get, Router};
        let app = Router::new()
            .route("/health", get(|| async { "OK" }))
            .route("/api/v1/metrics", get(|| async { "metrics" }));

        let addr = format!("0.0.0.0:{}", http_port);
        tracing::info!("HTTP server listening on {}", addr);
        let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
        axum::serve(listener, app).await.unwrap();
    });
    tracing::info!("HTTP server started on port {}", args.http_port);

    // Start RPC service for query execution
    let rpc_port = args.rpc_port;
    let storage_clone = storage.clone();
    tokio::spawn(async move {
        use proto::backend_service_server::{BackendServiceServer, BackendService};
        use proto::{ExecPlanFragmentRequest, ExecPlanFragmentResponse, CancelPlanFragmentRequest,
                    FetchDataRequest, FetchDataResponse, HeartbeatRequest, HeartbeatResponse, Status};
        use tonic::transport::Server;
        use tonic::{Request, Response, Status as TonicStatus};

        #[derive(Clone)]
        struct BackendServiceImpl {
            #[allow(dead_code)]
            storage: Arc<RwLock<be_storage::StorageEngine>>,
        }

        #[tonic::async_trait]
        impl BackendService for BackendServiceImpl {
            async fn exec_plan_fragment(
                &self,
                request: Request<ExecPlanFragmentRequest>,
            ) -> Result<Response<ExecPlanFragmentResponse>, TonicStatus> {
                let req = request.into_inner();
                tracing::info!("Received ExecPlanFragment: fragment_id={}", req.fragment_instance_id);

                let response = ExecPlanFragmentResponse {
                    status: Some(Status {
                        code: 0,
                        message: "OK".to_string(),
                        details: vec![],
                    }),
                    query_id: req.fragment_instance_id.clone(),
                };
                Ok(Response::new(response))
            }

            async fn cancel_plan_fragment(
                &self,
                request: Request<CancelPlanFragmentRequest>,
            ) -> Result<Response<Status>, TonicStatus> {
                let req = request.into_inner();
                tracing::info!("Received CancelPlanFragment: fragment_id={}", req.fragment_id);

                Ok(Response::new(Status {
                    code: 0,
                    message: "OK".to_string(),
                    details: vec![],
                }))
            }

            async fn fetch_data(
                &self,
                request: Request<FetchDataRequest>,
            ) -> Result<Response<FetchDataResponse>, TonicStatus> {
                let req = request.into_inner();
                tracing::info!("Received FetchData: fragment_id={}", req.fragment_id);

                Ok(Response::new(FetchDataResponse {
                    status: Some(Status {
                        code: 0,
                        message: "OK".to_string(),
                        details: vec![],
                    }),
                    row_batch: None,
                    query_id: req.fragment_id.clone(),
                }))
            }

            async fn heartbeat(
                &self,
                request: Request<HeartbeatRequest>,
            ) -> Result<Response<HeartbeatResponse>, TonicStatus> {
                let req = request.into_inner();
                tracing::debug!("Received heartbeat: timestamp={}", req.timestamp);

                Ok(Response::new(HeartbeatResponse {
                    timestamp: req.timestamp,
                }))
            }
        }

        let service = BackendServiceImpl {
            storage: storage_clone,
        };

        let addr = format!("0.0.0.0:{}", rpc_port);
        tracing::info!("RPC server listening on {}", addr);
        Server::builder()
            .add_service(BackendServiceServer::new(service))
            .serve(addr.parse().unwrap())
            .await
            .unwrap();
    });
    tracing::info!("RPC server started on port {}", args.rpc_port);

    // Start heartbeat service to FE
    let heartbeat_port = args.heartbeat_port;
    tokio::spawn(async move {
        use tokio::time::{interval, Duration};
        let mut ticker = interval(Duration::from_secs(5));
        loop {
            ticker.tick().await;
            tracing::debug!("Heartbeat tick on port {}", heartbeat_port);
        }
    });
    tracing::info!("Heartbeat service started on port {}", args.heartbeat_port);

    tracing::info!("Roris BE started successfully");

    tokio::signal::ctrl_c().await?;
    tracing::info!("Roris BE shutting down...");

    Ok(())
}
