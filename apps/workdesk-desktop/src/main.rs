mod api_client;

use anyhow::Result;
use serde::Deserialize;
use std::net::SocketAddr;
use std::path::PathBuf;
use tracing::info;
use tracing::warn;
use workdesk_core::run_server;
use workdesk_core::AuthLoginInput;

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
            info!("{}", bundle.mode_local);
            run_server(bind, PathBuf::from(workspace_root)).await?;
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
