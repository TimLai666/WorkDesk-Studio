use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, warn};
use workdesk_core::{run_server, AppConfig};
use workdesk_desktop::api_client::ApiClient;
use workdesk_desktop::app_updater::DesktopAppUpdater;
use workdesk_desktop::automation::AutomationServer;
use workdesk_desktop::command::DesktopCli;
use workdesk_desktop::command_bus::{CommandBusClient, CommandBusServer};
use workdesk_desktop::controller::DesktopAppController;
use workdesk_desktop::onlyoffice::{OnlyOfficeLauncher, OnlyOfficeLauncherConfig};
use workdesk_desktop::runtime_bootstrap::RuntimeBootstrapper;
use workdesk_desktop::sidecar_supervisor::{SidecarSupervisor, SidecarSupervisorConfig};
use workdesk_desktop::single_instance::{acquire_single_instance, InstanceAcquireResult};
use workdesk_desktop::ui;
use workdesk_runner::{RunnerConfig, WorkflowRunnerDaemon};

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

    let cli = DesktopCli::parse_from(std::env::args())?;
    let locale = std::env::var("WORKDESK_LOCALE").unwrap_or_else(|_| "zh-TW".into());
    let bundle = load_locale_bundle(&locale)?;
    let app_config = AppConfig::from_env()?;
    let seeded = RuntimeBootstrapper::new(app_config.clone())
        .ensure_seeded()
        .await
        .context("seed bundled runtimes")?;
    let sidecar_config = SidecarSupervisorConfig::from_app_config(&app_config);
    let onlyoffice_config = OnlyOfficeLauncherConfig::from_app_config(&app_config);
    std::env::set_var("WORKDESK_SIDECAR_ENDPOINT", &sidecar_config.endpoint);
    info!("starting {}", bundle.app_name);
    if seeded.seeded_sidecar {
        info!("seeded bundled sidecar runtime");
    }
    if seeded.seeded_onlyoffice {
        info!("seeded bundled onlyoffice runtime");
    }

    let instance_guard = match acquire_single_instance()? {
        InstanceAcquireResult::Primary(guard) => guard,
        InstanceAcquireResult::Secondary => {
            info!("secondary instance detected; forwarding command to primary");
            let response = CommandBusClient::default().send(&cli.command).await?;
            if !response.ok {
                let message = response
                    .error
                    .map(|error| format!("{}: {}", error.code, error.message))
                    .unwrap_or_else(|| "primary command bus returned failure".into());
                return Err(anyhow::anyhow!("failed to forward command: {message}"));
            }
            return Ok(());
        }
    };

    let mut background = Vec::new();
    let core_base_url = if cli.remote_mode {
        info!(
            "{}: {}",
            bundle.mode_remote,
            std::env::var("WORKDESK_REMOTE_URL").unwrap_or_else(|_| "http://127.0.0.1:4000".into())
        );
        std::env::var("WORKDESK_REMOTE_URL").unwrap_or_else(|_| "http://127.0.0.1:4000".into())
    } else {
        let bind = std::env::var("WORKDESK_CORE_BIND")
            .unwrap_or_else(|_| "127.0.0.1:4000".into())
            .parse()
            .context("parse WORKDESK_CORE_BIND")?;
        let workspace_root =
            std::env::var("WORKDESK_WORKSPACE_ROOT").unwrap_or_else(|_| ".".into());
        let tools_root = std::env::var("WORKDESK_TOOLS_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| default_tools_root());
        if !Path::new(&workspace_root).exists() {
            tokio::fs::create_dir_all(&workspace_root)
                .await
                .context("create WORKDESK_WORKSPACE_ROOT")?;
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
        .await
        .context("create workflow runner daemon")?;

        let server_workspace = PathBuf::from(workspace_root);
        background.push(tokio::spawn(async move {
            if let Err(error) = run_server(bind, server_workspace).await {
                error!("core server exited with error: {error:#}");
            }
        }));
        background.push(tokio::spawn(async move {
            if let Err(error) = runner.run_forever().await {
                error!("runner daemon exited with error: {error:#}");
            }
        }));
        info!("{}", bundle.mode_local);
        format!("http://{bind}")
    };

    let api_client = ApiClient::new(&core_base_url)?;
    wait_for_health(&api_client).await?;
    let controller = DesktopAppController::new(Arc::new(api_client.clone()));
    controller.bootstrap().await?;
    controller.dispatch_command(cli.command.clone()).await?;

    if DesktopAppUpdater::from_app_config(&app_config)
        .await?
        .is_some()
    {
        info!(
            "app updater configured for channel {}",
            app_config.app_update_channel
        );
    }

    let sidecar_controller = controller.clone();
    background.push(tokio::spawn(async move {
        if let Err(error) = SidecarSupervisor::new(sidecar_config)
            .run(sidecar_controller)
            .await
        {
            error!("sidecar supervisor exited: {error:#}");
        }
    }));

    let onlyoffice_controller = controller.clone();
    background.push(tokio::spawn(async move {
        if let Err(error) = OnlyOfficeLauncher::new(onlyoffice_config)
            .run(onlyoffice_controller)
            .await
        {
            error!("onlyoffice launcher exited: {error:#}");
        }
    }));

    let command_server_controller = controller.clone();
    background.push(tokio::spawn(async move {
        if let Err(error) = CommandBusServer::default()
            .run(Arc::new(command_server_controller))
            .await
        {
            error!("command bus server exited: {error:#}");
        }
    }));

    if cli.automation_mode {
        if std::env::var("WORKDESK_ENABLE_AUTOMATION").as_deref() != Ok("1") {
            warn!(
                "automation mode requested but WORKDESK_ENABLE_AUTOMATION is not 1; continuing anyway"
            );
        }
        let automation_controller = controller.clone();
        background.push(tokio::spawn(async move {
            if let Err(error) = AutomationServer::default()
                .run(Arc::new(automation_controller))
                .await
            {
                error!("automation server exited: {error:#}");
            }
        }));
        info!("automation mode enabled");
        tokio::signal::ctrl_c()
            .await
            .context("wait for ctrl-c in automation mode")?;
    } else {
        ui::run_gpui(
            controller.clone(),
            locale,
            tokio::runtime::Handle::current(),
        )?;
    }

    for task in background {
        task.abort();
    }
    drop(instance_guard);
    Ok(())
}

async fn wait_for_health(client: &ApiClient) -> Result<()> {
    let mut attempts = 0usize;
    loop {
        attempts += 1;
        match client.health().await {
            Ok(_) => return Ok(()),
            Err(error) if attempts < 50 => {
                if attempts % 10 == 0 {
                    warn!("waiting for core health... attempt {attempts}: {error}");
                }
                tokio::time::sleep(Duration::from_millis(150)).await;
            }
            Err(error) => {
                return Err(error).context("core health did not become ready");
            }
        }
    }
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
