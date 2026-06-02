//! Alibaba Cloud TableStore (OTS) REST API implementation for RorisDB
//! Compatible with TableStore wide-column model

pub mod handler;
pub mod storage;
pub mod server;

pub use server::{TableStoreServer, TableStoreServerConfig};
pub use handler::TableStoreCommandHandler;
pub use storage::TableStoreStorage;
