pub mod catalog;
pub mod internal;
pub mod heartbeat;

// Include generated proto code
mod proto {
    tonic::include_proto!("proto");
}

// Re-export common types
pub use proto::{Status, RowBatch, Column, DataType};
pub use proto::backend_service_server::BackendService;
pub use proto::backend_service_client::BackendServiceClient as BeServiceClient;
