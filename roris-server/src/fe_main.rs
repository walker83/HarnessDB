use anyhow::Result;
use clap::Parser;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};

use fe_common::edit_log::EditLog;
use fe_catalog::CatalogManager;
use fe_scheduler::ClusterManager;
use fe_monitor::{MonitoringManager};
use fe_monitor::http_server::MonitoringHttpServer;

#[derive(Parser)]
#[command(name = "roris-fe", about = "Roris Frontend Server")]
struct Args {
    #[arg(long, default_value = "conf/fe.conf")]
    config: String,

    #[arg(long, default_value = "8030")]
    http_port: u16,

    #[arg(long, default_value = "9020")]
    rpc_port: u16,

    #[arg(long, default_value = "data/fe/doris-meta")]
    meta_dir: String,

    #[arg(long, default_value = "8040")]
    metrics_port: u16,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    tracing::info!("Roris FE starting...");
    tracing::info!("Config file: {}", args.config);
    tracing::info!("HTTP port: {}, RPC port: {}", args.http_port, args.rpc_port);
    tracing::info!("Meta directory: {}", args.meta_dir);
    tracing::info!("Metrics port: {}", args.metrics_port);

    // Initialize catalog manager with persistence path
    let catalog = Arc::new(RwLock::new(CatalogManager::with_path(&args.meta_dir)));

    // Load catalog from disk if it exists
    {
        let mut catalog_guard = catalog.write().await;
        catalog_guard.load()?;
        tracing::info!("Catalog loaded from disk");
    }

    // Initialize edit log
    let edit_log = Arc::new(RwLock::new(EditLog::new(&args.meta_dir)));

    // Replay edit log on startup
    {
        let mut log = edit_log.write().await;
        log.replay().await?;
        tracing::info!("Edit log replayed, last_applied_index: {}", log.last_applied_index());
    }

    // Apply edit log entries to catalog
    {
        let mut catalog_guard = catalog.write().await;
        let log = edit_log.read().await;
        catalog_guard.replay_edit_log(&log)?;
        tracing::info!("Edit log applied to catalog");
    }

    // Initialize cluster manager for BE health monitoring
    let _cluster = Arc::new(RwLock::new(ClusterManager::new(fe_scheduler::cluster::ClusterConfig::default())));

    // Initialize monitoring manager
    let monitoring = Arc::new(MonitoringManager::new(catalog.clone()));
    tracing::info!("Monitoring manager initialized");

    // Start monitoring HTTP server
    let http_server = MonitoringHttpServer::new(args.metrics_port, monitoring.clone());
    tokio::spawn(async move {
        if let Err(e) = http_server.start().await {
            tracing::error!("Monitoring HTTP server failed: {}", e);
        }
    });
    tracing::info!("Monitoring HTTP server started on port {}", args.metrics_port);

    // Background task to periodically flush the EditLog
    let edit_log_clone = edit_log.clone();
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(10));
        loop {
            ticker.tick().await;
            let mut log = edit_log_clone.write().await;
            if let Err(e) = log.flush().await {
                tracing::error!("EditLog flush failed: {}", e);
            } else {
                tracing::debug!("EditLog flushed");
            }
        }
    });

    tracing::info!("Roris FE started successfully");

    // Periodically save catalog
    let catalog_clone = catalog.clone();
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(30));
        loop {
            ticker.tick().await;
            if let Err(e) = catalog_clone.read().await.save() {
                tracing::error!("Catalog save failed: {}", e);
            }
        }
    });

    tokio::signal::ctrl_c().await?;
    tracing::info!("Roris FE shutting down...");

    // Flush audit logs before shutdown
    monitoring.audit_log.flush().await;
    tracing::info!("Audit logs flushed");

    // Final save on shutdown
    catalog.read().await.save()?;
    tracing::info!("Catalog saved on shutdown");

    Ok(())
}