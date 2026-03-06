use anyhow::Result;
use std::net::SocketAddr;
use std::path::PathBuf;
use workdesk_core::run_server;

#[tokio::main]
async fn main() -> Result<()> {
    let bind = std::env::var("WORKDESK_CORE_BIND").unwrap_or_else(|_| "127.0.0.1:4000".into());
    let workspace_root = std::env::var("WORKDESK_WORKSPACE_ROOT").unwrap_or_else(|_| ".".into());
    let socket: SocketAddr = bind.parse()?;
    run_server(socket, PathBuf::from(workspace_root)).await
}
