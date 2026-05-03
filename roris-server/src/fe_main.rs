use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(name = "roris-fe", about = "Roris Frontend Server")]
struct Args {
    #[arg(long, default_value = "conf/fe.conf")]
    config: String,

    #[arg(long, default_value = "8030")]
    http_port: u16,

    #[arg(long, default_value = "9020")]
    rpc_port: u16,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    tracing::info!("Roris FE starting...");
    tracing::info!("Config file: {}", args.config);
    tracing::info!("HTTP port: {}, RPC port: {}", args.http_port, args.rpc_port);

    // TODO: Initialize catalog manager
    // TODO: Start SQL service
    // TODO: Start RPC service for BE communication

    tracing::info!("Roris FE started successfully");
    tokio::signal::ctrl_c().await?;
    tracing::info!("Roris FE shutting down...");
    Ok(())
}
