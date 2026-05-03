use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(name = "roris-be", about = "Roris Backend Server")]
struct Args {
    #[arg(long, default_value = "conf/be.conf")]
    config: String,

    #[arg(long, default_value = "8060")]
    http_port: u16,

    #[arg(long, default_value = "9060")]
    rpc_port: u16,

    #[arg(long, default_value = "9050")]
    heartbeart_port: u16,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    tracing::info!("Roris BE starting...");
    tracing::info!("Config file: {}", args.config);
    tracing::info!("HTTP port: {}, RPC port: {}", args.http_port, args.rpc_port);

    // TODO: Initialize storage engine
    // TODO: Initialize execution engine
    // TODO: Start heartbeat service to FE
    // TODO: Start RPC service for query execution

    tracing::info!("Roris BE started successfully");
    tokio::signal::ctrl_c().await?;
    tracing::info!("Roris BE shutting down...");
    Ok(())
}
