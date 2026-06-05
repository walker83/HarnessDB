use std::sync::Arc;
use mysql_protocol::server::QueryHandler;
use tds_protocol::{TdsServer, TdsServerConfig};

#[derive(Debug, Clone)]
pub struct SybaseServerConfig {
    pub bind_addr: String,
    pub port: u16,
    pub max_connections: u32,
}

impl Default for SybaseServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0".to_string(),
            port: 5000,
            max_connections: 100,
        }
    }
}

pub struct SybaseServer {
    tds_server: TdsServer,
}

impl SybaseServer {
    pub fn new(config: SybaseServerConfig, handler: Arc<dyn QueryHandler>) -> Self {
        let tds_config = TdsServerConfig {
            bind_addr: config.bind_addr,
            port: config.port,
            max_connections: config.max_connections,
        };
        Self {
            tds_server: TdsServer::new(tds_config, handler),
        }
    }

    pub async fn run(&self) -> std::io::Result<()> {
        self.tds_server.run().await
    }
}
