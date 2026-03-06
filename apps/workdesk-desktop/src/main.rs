mod api_client;

use anyhow::Result;
use serde::Deserialize;
use std::net::SocketAddr;
use std::path::Path;
use std::path::PathBuf;
use tracing::info;
use tracing::warn;
use workdesk_core::{run_server, AppConfig, AuthLoginInput};
use workdesk_runner::{RunnerConfig, WorkflowRunnerDaemon};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DesktopMode {
    Local,
    Remote,
}

#[derive(Debug, Deserialize)]
struct LocaleBundle {
    app_name: String,
    mode_local: String,
    mode_remote: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info,workdesk_desktop=debug")
        .compact()
        .init();

    let mode = parse_mode(std::env::args().collect());
    let locale = std::env::var("WORKDESK_LOCALE").unwrap_or_else(|_| "zh-TW".into());
    let bundle = load_locale_bundle(&locale)?;
    info!("starting {}", bundle.app_name);

    match mode {
        DesktopMode::Local => {
            let bind = std::env::var("WORKDESK_CORE_BIND")
                .unwrap_or_else(|_| "127.0.0.1:4000".into())
                .parse::<SocketAddr>()?;
            let workspace_root =
                std::env::var("WORKDESK_WORKSPACE_ROOT").unwrap_or_else(|_| ".".into());
            let app_config = AppConfig::from_env()?;
            let tools_root = std::env::var("WORKDESK_TOOLS_ROOT")
                .map(PathBuf::from)
                .unwrap_or_else(|_| default_tools_root());
            if !Path::new(&workspace_root).exists() {
                tokio::fs::create_dir_all(&workspace_root).await?;
            }
            let runner = WorkflowRunnerDaemon::new(RunnerConfig {
                db_path: app_config.db_path.clone(),
                tools_root,
                runner_id: std::env::var("WORKDESK_RUNNER_ID")
                    .unwrap_or_else(|_| "desktop-runner".into()),
                poll_interval_ms: std::env::var("WORKDESK_RUNNER_POLL_MS")
                    .ok()
                    .and_then(|value| value.parse::<u64>().ok())
                    .unwrap_or(1500),
                lease_seconds: std::env::var("WORKDESK_RUNNER_LEASE_SEC")
                    .ok()
                    .and_then(|value| value.parse::<i64>().ok())
                    .unwrap_or(30),
            })
            .await?;
            info!("{}", bundle.mode_local);
            tokio::try_join!(
                run_server(bind, PathBuf::from(workspace_root)),
                runner.run_forever()
            )?;
        }
        DesktopMode::Remote => {
            let remote = std::env::var("WORKDESK_REMOTE_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:4000".into());
            info!("{}: {}", bundle.mode_remote, remote);
            let client = api_client::ApiClient::new(&remote)?;
            match client.health().await {
                Ok(health) => info!("remote health: {}", health),
                Err(error) => warn!("remote health check failed: {}", error),
            }
            match client.list_workflows().await {
                Ok(workflows) => info!("loaded workflows: {}", workflows.len()),
                Err(error) => warn!("list workflows failed: {}", error),
            }
            if let (Ok(account_id), Ok(password)) = (
                std::env::var("WORKDESK_LOGIN_ACCOUNT"),
                std::env::var("WORKDESK_LOGIN_PASSWORD"),
            ) {
                match client
                    .login(&AuthLoginInput {
                        account_id,
                        password,
                    })
                    .await
                {
                    Ok(session) => info!("login ok for {}", session.account_id),
                    Err(error) => warn!("login failed: {}", error),
                }
            }
        }
    }

    Ok(())
}

fn parse_mode(args: Vec<String>) -> DesktopMode {
    if args.iter().any(|arg| arg == "--remote") {
        return DesktopMode::Remote;
    }
    DesktopMode::Local
}

fn load_locale_bundle(locale: &str) -> Result<LocaleBundle> {
    let file = match locale {
        "en" | "en-US" => include_str!("../resources/i18n/en.json"),
        _ => include_str!("../resources/i18n/zh-TW.json"),
    };
    Ok(serde_json::from_str(file)?)
}

fn default_tools_root() -> PathBuf {
    if cfg!(windows) {
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            return PathBuf::from(local).join("WorkDeskStudio").join("tools");
        }
    }
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".local")
        .join("share")
        .join("WorkDeskStudio")
        .join("tools")
}
