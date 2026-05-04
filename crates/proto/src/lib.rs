// Include generated proto code directly
tonic::include_proto!("proto");

// Re-export gRPC service types
pub use backend_service_server::BackendService;
pub use backend_service_client::BackendServiceClient;