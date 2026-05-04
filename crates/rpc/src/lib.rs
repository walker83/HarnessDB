pub mod fe_service;
pub mod be_service;
pub mod be_client;
pub mod heartbeat;

// Re-export types
pub use be_service::BeGrpcServer;
pub use be_client::BeGrpcClient;
pub use fe_service::{FeService, FeServiceImpl, FeQueryResult};