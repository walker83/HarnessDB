//! AnalyticDB MySQL TCP server — uses mysql-protocol for proper MySQL wire protocol
//! handshake, authentication, and packet framing.

use crate::handler::AdbMysqlHandler;
use crate::storage::AdbMysqlStorage;
use mysql_protocol::server::{MysqlServer, ServerConfig};
use mysql_protocol::auth::default_credentials;
use std::sync::Arc;
use tracing::info;

pub struct AdbMysqlServer {
    port: u16,
    storage: Arc<AdbMysqlStorage>,
}

impl AdbMysqlServer {
    pub fn new(port: u16) -> Self {
        Self {
            port,
            storage: Arc::new(AdbMysqlStorage::new()),
        }
    }

    pub async fn start(&self) -> anyhow::Result<()> {
        // Ensure "default" database exists
        self.storage.create_database("default");

        let handler = Arc::new(AdbMysqlHandler::new(self.storage.clone()));

        let config = ServerConfig {
            bind_addr: "0.0.0.0".to_string(),
            port: self.port,
            default_auth_plugin: mysql_protocol::AuthPluginType::NativePassword,
            auth_timeout_secs: 30,
            max_connections: 100,
            credentials: default_credentials(),
        };

        info!("AnalyticDB MySQL server (wire protocol) listening on 0.0.0.0:{}", self.port);

        let server = MysqlServer::new(config, handler);
        server.run().await?;
        Ok(())
    }
}
