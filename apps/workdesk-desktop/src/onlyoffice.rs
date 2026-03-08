use crate::controller::{DesktopAppController, UiDiagnostic};
use crate::runtime_bootstrap::copy_path_recursive;
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tracing::{debug, warn};
use workdesk_core::AppConfig;

#[derive(Debug, Clone)]
pub struct OnlyOfficeLauncherConfig {
    pub binary_path: PathBuf,
    pub bundle_dir: Option<PathBuf>,
    pub host: String,
    pub port: u16,
    pub health_path: String,
    pub probe_interval: Duration,
}

impl OnlyOfficeLauncherConfig {
    pub fn from_app_config(config: &AppConfig) -> Self {
        Self {
            binary_path: config.onlyoffice_binary_path.clone(),
            bundle_dir: Some(config.bundled_onlyoffice_dir.clone()),
            host: config.onlyoffice_host.clone(),
            port: config.onlyoffice_port,
            health_path: std::env::var("WORKDESK_ONLYOFFICE_HEALTH_PATH")
                .unwrap_or_else(|_| "/health".into()),
            probe_interval: Duration::from_secs(5),
        }
    }

    pub fn health_url(&self) -> String {
        format!("http://{}:{}{}", self.host, self.port, self.health_path)
    }
}

#[derive(Debug, Clone)]
pub struct OnlyOfficeProbe {
    pub healthy: bool,
    pub diagnostic: Option<UiDiagnostic>,
}

#[derive(Debug, Clone)]
pub struct OnlyOfficeLauncher {
    config: OnlyOfficeLauncherConfig,
    child: Arc<Mutex<Option<Child>>>,
}

impl OnlyOfficeLauncher {
    pub fn new(config: OnlyOfficeLauncherConfig) -> Self {
        Self {
            config,
            child: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn probe(&self) -> OnlyOfficeProbe {
        if self.health_check().await {
            return OnlyOfficeProbe {
                healthy: true,
                diagnostic: None,
            };
        }

        let diagnostic = if !self.config.binary_path.exists() {
            UiDiagnostic {
                code: "DOCSERVER_UNAVAILABLE".into(),
                message: format!(
                    "OnlyOffice Document Server runtime missing: {}",
                    self.config.binary_path.display()
                ),
                run_id: None,
            }
        } else {
            UiDiagnostic {
                code: "DOCSERVER_UNAVAILABLE".into(),
                message: format!(
                    "OnlyOffice health endpoint is not healthy: {}",
                    self.config.health_url()
                ),
                run_id: None,
            }
        };

        OnlyOfficeProbe {
            healthy: false,
            diagnostic: Some(diagnostic),
        }
    }

    pub async fn run(self, controller: DesktopAppController) -> Result<()> {
        loop {
            let probe = self.probe().await;
            if probe.healthy {
                controller.set_runtime_diagnostic("onlyoffice", None);
            } else {
                controller.set_runtime_diagnostic("onlyoffice", probe.diagnostic);
                if let Err(error) = self.ensure_process().await {
                    warn!("onlyoffice launcher failed to ensure process: {error:#}");
                }
            }
            tokio::time::sleep(self.config.probe_interval).await;
        }
    }

    async fn ensure_process(&self) -> Result<()> {
        self.ensure_runtime_from_bundle().await?;
        if !self.config.binary_path.exists() {
            return Ok(());
        }

        let mut child_guard = self.child.lock().await;
        if let Some(child) = child_guard.as_mut() {
            if child
                .try_wait()
                .context("query onlyoffice process state")?
                .is_none()
            {
                return Ok(());
            }
        }

        if let Some(parent) = self.config.binary_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .with_context(|| format!("create onlyoffice runtime dir {}", parent.display()))?;
        }

        let mut command = Command::new(&self.config.binary_path);
        command.env("PORT", self.config.port.to_string());
        if let Some(parent) = self.config.binary_path.parent() {
            command.current_dir(parent);
        }
        command.kill_on_drop(true);
        let child = command.spawn().with_context(|| {
            format!(
                "spawn onlyoffice document server {}",
                self.config.binary_path.display()
            )
        })?;
        debug!("spawned onlyoffice document server child");
        *child_guard = Some(child);
        Ok(())
    }

    async fn ensure_runtime_from_bundle(&self) -> Result<()> {
        if self.config.binary_path.exists() {
            return Ok(());
        }
        let Some(bundle_dir) = self.config.bundle_dir.as_ref() else {
            return Ok(());
        };
        if !bundle_dir.exists() {
            return Ok(());
        }
        let runtime_dir = self
            .config
            .binary_path
            .parent()
            .map(PathBuf::from)
            .ok_or_else(|| anyhow::anyhow!("onlyoffice binary path has no parent directory"))?;
        copy_path_recursive(bundle_dir.clone(), runtime_dir).await
    }

    async fn health_check(&self) -> bool {
        reqwest::get(self.config.health_url())
            .await
            .map(|response| response.status().is_success())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::{OnlyOfficeLauncher, OnlyOfficeLauncherConfig};
    use std::path::PathBuf;
    use std::time::Duration;
    use tempfile::TempDir;
    use tokio::io::AsyncReadExt;
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn probe_reports_missing_runtime() {
        let launcher = OnlyOfficeLauncher::new(OnlyOfficeLauncherConfig {
            binary_path: PathBuf::from("missing-docserver.exe"),
            bundle_dir: None,
            host: "127.0.0.1".into(),
            port: 49981,
            health_path: "/health".into(),
            probe_interval: Duration::from_millis(25),
        });

        let probe = launcher.probe().await;
        assert!(!probe.healthy);
        assert_eq!(
            probe.diagnostic.expect("diagnostic").code,
            "DOCSERVER_UNAVAILABLE"
        );
    }

    #[tokio::test]
    async fn probe_accepts_healthy_http_endpoint() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind http listener");
        let port = listener.local_addr().expect("addr").port();
        let server = tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let mut buffer = [0u8; 1024];
                let _ = socket.read(&mut buffer).await;
                let _ = socket
                    .write_all(
                        b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok",
                    )
                    .await;
            }
        });

        let launcher = OnlyOfficeLauncher::new(OnlyOfficeLauncherConfig {
            binary_path: PathBuf::from("missing-docserver.exe"),
            bundle_dir: None,
            host: "127.0.0.1".into(),
            port,
            health_path: "/health".into(),
            probe_interval: Duration::from_millis(25),
        });

        let probe = launcher.probe().await;
        assert!(probe.healthy);
        assert!(probe.diagnostic.is_none());
        server.abort();
    }

    #[tokio::test]
    async fn runtime_bundle_is_copied_when_binary_missing() {
        let tmp = TempDir::new().expect("tempdir");
        let bundle_dir = tmp.path().join("bundle");
        let runtime_dir = tmp.path().join("runtime");
        tokio::fs::create_dir_all(&bundle_dir)
            .await
            .expect("create bundle dir");
        tokio::fs::write(bundle_dir.join("documentserver.exe"), "stub")
            .await
            .expect("write bundle binary");

        let launcher = OnlyOfficeLauncher::new(OnlyOfficeLauncherConfig {
            binary_path: runtime_dir.join("documentserver.exe"),
            bundle_dir: Some(bundle_dir),
            host: "127.0.0.1".into(),
            port: 49001,
            health_path: "/health".into(),
            probe_interval: Duration::from_millis(25),
        });

        launcher
            .ensure_runtime_from_bundle()
            .await
            .expect("copy runtime from bundle");
        assert!(runtime_dir.join("documentserver.exe").exists());
    }
}
