//! AnalyticDB MySQL protocol implementation
//! Provides MPP (Massively Parallel Processing) analytical query capabilities
//! using the standard MySQL wire protocol for compatibility with mysql CLI clients.

pub mod server;
pub mod handler;
pub mod storage;

pub use server::AdbMysqlServer;
pub use handler::AdbMysqlHandler;
pub use storage::AdbMysqlStorage;
