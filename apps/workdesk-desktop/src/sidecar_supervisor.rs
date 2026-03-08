use crate::controller::{DesktopAppController, UiDiagnostic};
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tracing::{debug, warn};
use workdesk_core::AppConfig;
use workdesk_runner::sidecar::default_sidecar_endpoint;

#[derive(Debug, Clone)]
pub struct SidecarSupervisorConfig {
    pub command_path: PathBuf,
    pub script_path: PathBuf,
    pub endpoint: String,
    pub probe_interval: Duration,
}

impl SidecarSupervisorConfig {
    pub fn from_app_config(config: &AppConfig) -> Self {
        Self {
            command_path: config.sidecar_path.clone(),
            script_path: config.sidecar_script_path.clone(),
            endpoint: std::env::var("WORKDESK_SIDECAR_ENDPOINT")
                .unwrap_or_else(|_| default_sidecar_endpoint()),
            probe_interval: Duration::from_secs(3),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SidecarProbe {
    pub healthy: bool,
    pub diagnostic: Option<UiDiagnostic>,
}

#[derive(Debug, Clone)]
pub struct SidecarSupervisor {
    config: SidecarSupervisorConfig,
    child: Arc<Mutex<Option<Child>>>,
}

impl SidecarSupervisor {
    pub fn new(config: SidecarSupervisorConfig) -> Self {
        Self {
            config,
            child: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn probe(&self) -> SidecarProbe {
        if self.endpoint_healthy().await {
            return SidecarProbe {
                healthy: true,
                diagnostic: None,
            };
        }

        let diagnostic = if !self.config.command_path.exists() {
            UiDiagnostic {
                code: "SIDECAR_UNAVAILABLE".into(),
                message: format!(
                    "Sidecar runtime missing: {}",
                    self.config.command_path.display()
                ),
                run_id: None,
            }
        } else if !self.config.script_path.exists() {
            UiDiagnostic {
                code: "SIDECAR_UNAVAILABLE".into(),
                message: format!(
                    "Sidecar script missing: {}",
                    self.config.script_path.display()
                ),
                run_id: None,
            }
        } else {
            UiDiagnostic {
                code: "SIDECAR_UNAVAILABLE".into(),
                message: format!("Sidecar endpoint is not healthy: {}", self.config.endpoint),
                run_id: None,
            }
        };

        SidecarProbe {
            healthy: false,
            diagnostic: Some(diagnostic),
        }
    }

    pub async fn run(self, controller: DesktopAppController) -> Result<()> {
        loop {
            let probe = self.probe().await;
            if probe.healthy {
                controller.set_runtime_diagnostic("sidecar", None);
            } else {
                controller.set_runtime_diagnostic("sidecar", probe.diagnostic);
                if let Err(error) = self.ensure_process().await {
                    warn!("sidecar supervisor failed to ensure process: {error:#}");
                }
            }
            tokio::time::sleep(self.config.probe_interval).await;
        }
    }

    async fn ensure_process(&self) -> Result<()> {
        if !self.config.command_path.exists() || !self.config.script_path.exists() {
            return Ok(());
        }

        let mut child_guard = self.child.lock().await;
        if let Some(child) = child_guard.as_mut() {
            if child
                .try_wait()
                .context("query sidecar process state")?
                .is_none()
            {
                return Ok(());
            }
        }

        let mut command = Command::new(&self.config.command_path);
        command.arg(&self.config.script_path);
        command.env("WORKDESK_SIDECAR_ENDPOINT", &self.config.endpoint);
        if let Some(parent) = self.config.script_path.parent() {
            command.current_dir(parent);
        }
        command.kill_on_drop(true);
        let child = command.spawn().with_context(|| {
            format!(
                "spawn sidecar process {}",
                self.config.command_path.display()
            )
        })?;
        debug!("spawned sidecar supervisor child");
        *child_guard = Some(child);
        Ok(())
    }

    async fn endpoint_healthy(&self) -> bool {
        if self.config.endpoint.starts_with("http://")
            || self.config.endpoint.starts_with("https://")
        {
            return reqwest::get(&self.config.endpoint)
                .await
                .map(|response| response.status().is_success())
                .unwrap_or(false);
        }

        #[cfg(windows)]
        if self.config.endpoint.starts_with(r"\\.\pipe\") {
            return tokio::net::windows::named_pipe::ClientOptions::new()
                .open(&self.config.endpoint)
                .is_ok();
        }

        tokio::net::TcpStream::connect(&self.config.endpoint)
            .await
            .map(|_| true)
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::{SidecarSupervisor, SidecarSupervisorConfig};
    use std::path::PathBuf;
    use std::time::Duration;

    #[tokio::test]
    async fn probe_reports_missing_runtime_files() {
        let supervisor = SidecarSupervisor::new(SidecarSupervisorConfig {
            command_path: PathBuf::from("missing-node.exe"),
            script_path: PathBuf::from("missing-sidecar.js"),
            endpoint: "127.0.0.1:49991".into(),
            probe_interval: Duration::from_millis(25),
        });

        let probe = supervisor.probe().await;
        assert!(!probe.healthy);
        assert_eq!(
            probe.diagnostic.expect("diagnostic").code,
            "SIDECAR_UNAVAILABLE"
        );
    }

    #[tokio::test]
    async fn probe_accepts_healthy_tcp_endpoint() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind tcp listener");
        let endpoint = listener.local_addr().expect("local addr").to_string();
        let accept_task = tokio::spawn(async move {
            let _ = listener.accept().await;
        });

        let supervisor = SidecarSupervisor::new(SidecarSupervisorConfig {
            command_path: PathBuf::from("missing-node.exe"),
            script_path: PathBuf::from("missing-sidecar.js"),
            endpoint,
            probe_interval: Duration::from_millis(25),
        });

        let probe = supervisor.probe().await;
        assert!(probe.healthy);
        assert!(probe.diagnostic.is_none());
        accept_task.abort();
    }
}
