pub mod fe_service;
pub mod be_service;
pub mod be_client;
pub mod heartbeat;

// Re-export common types
pub use be_service::{BeService, BeServiceImpl, BeGrpcServer};
pub use be_client::BeGrpcClient;
