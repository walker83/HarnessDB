//! Elasticsearch REST API implementation for HarnessDB
//! Compatible with OpenSearch and all Elasticsearch clients

pub mod handler;
pub mod storage;
pub mod server;

pub use server::{ElasticsearchServer, ElasticsearchServerConfig};
pub use handler::ElasticsearchCommandHandler;
pub use storage::ElasticsearchStorage;
