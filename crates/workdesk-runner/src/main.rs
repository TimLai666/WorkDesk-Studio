use anyhow::{Context, Result};
use std::env;
use std::path::PathBuf;
use uuid::Uuid;
use workdesk_core::AppConfig;
use workdesk_runner::{RunnerConfig, WorkflowRunnerDaemon};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info,workdesk_runner=debug")
        .compact()
        .init();

    let app_config = AppConfig::from_env()?;
    let tools_root = env::var("WORKDESK_TOOLS_ROOT")
        .map(PathBuf::from)
        .unwrap_or(default_tools_root()?);
    let runner_id = env::var("WORKDESK_RUNNER_ID").unwrap_or_else(|_| Uuid::new_v4().to_string());
    let poll_interval_ms = env::var("WORKDESK_RUNNER_POLL_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(1500);
    let lease_seconds = env::var("WORKDESK_RUNNER_LEASE_SEC")
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(30);

    let daemon = WorkflowRunnerDaemon::new(RunnerConfig {
        db_path: app_config.db_path,
        tools_root,
        runner_id,
        poll_interval_ms,
        lease_seconds,
    })
    .await
    .context("start workflow runner daemon")?;

    daemon.run_forever().await
}

fn default_tools_root() -> Result<PathBuf> {
    if cfg!(windows) {
        let local = env::var("LOCALAPPDATA").context("LOCALAPPDATA is required")?;
        return Ok(PathBuf::from(local).join("WorkDeskStudio").join("tools"));
    }
    let home = env::var("HOME").context("HOME is required")?;
    Ok(PathBuf::from(home)
        .join(".local")
        .join("share")
        .join("WorkDeskStudio")
        .join("tools"))
}
