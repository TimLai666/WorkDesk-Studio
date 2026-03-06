use anyhow::{Context, Result};
use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub db_path: PathBuf,
    pub workspace_root: PathBuf,
    pub core_bind: SocketAddr,
    pub locale: String,
    pub onlyoffice_host: String,
    pub onlyoffice_port: u16,
    pub onlyoffice_binary_path: PathBuf,
    pub sidecar_path: PathBuf,
    pub toolchain_manifest_path: PathBuf,
    pub app_update_channel: String,
    pub toolchain_update_channel: String,
}

impl AppConfig {
    pub fn from_env() -> Result<Self> {
        let core_bind = env::var("WORKDESK_CORE_BIND")
            .unwrap_or_else(|_| "127.0.0.1:4000".to_string())
            .parse::<SocketAddr>()
            .context("parse WORKDESK_CORE_BIND")?;
        let workspace_root =
            PathBuf::from(env::var("WORKDESK_WORKSPACE_ROOT").unwrap_or_else(|_| ".".to_string()));
        let locale = env::var("WORKDESK_LOCALE").unwrap_or_else(|_| "zh-TW".to_string());
        let db_path = env::var("WORKDESK_DB_PATH")
            .map(PathBuf::from)
            .unwrap_or(Self::default_db_path()?);
        let onlyoffice_host =
            env::var("WORKDESK_ONLYOFFICE_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let onlyoffice_port = env::var("WORKDESK_ONLYOFFICE_PORT")
            .ok()
            .and_then(|value| value.parse::<u16>().ok())
            .unwrap_or(8044);
        let onlyoffice_binary_path = env::var("WORKDESK_ONLYOFFICE_BIN")
            .map(PathBuf::from)
            .unwrap_or(Self::default_onlyoffice_bin()?);
        let sidecar_path = env::var("WORKDESK_SIDECAR_PATH")
            .map(PathBuf::from)
            .unwrap_or(Self::default_sidecar_path()?);
        let toolchain_manifest_path = env::var("WORKDESK_TOOLCHAIN_MANIFEST")
            .map(PathBuf::from)
            .unwrap_or(Self::default_toolchain_manifest_path()?);
        let app_update_channel =
            env::var("WORKDESK_APP_UPDATE_CHANNEL").unwrap_or_else(|_| "stable".to_string());
        let toolchain_update_channel =
            env::var("WORKDESK_TOOLCHAIN_UPDATE_CHANNEL").unwrap_or_else(|_| "stable".to_string());

        Ok(Self {
            db_path,
            workspace_root,
            core_bind,
            locale,
            onlyoffice_host,
            onlyoffice_port,
            onlyoffice_binary_path,
            sidecar_path,
            toolchain_manifest_path,
            app_update_channel,
            toolchain_update_channel,
        })
    }

    fn default_db_path() -> Result<PathBuf> {
        if cfg!(windows) {
            let local = env::var("LOCALAPPDATA")
                .context("LOCALAPPDATA is required for default SQLite path")?;
            return Ok(PathBuf::from(local)
                .join("WorkDeskStudio")
                .join("data")
                .join("workdesk.db"));
        }

        let home = env::var("HOME").context("HOME is required for default SQLite path")?;
        Ok(PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("WorkDeskStudio")
            .join("data")
            .join("workdesk.db"))
    }

    fn default_onlyoffice_bin() -> Result<PathBuf> {
        if cfg!(windows) {
            let local = env::var("LOCALAPPDATA")
                .context("LOCALAPPDATA is required for default OnlyOffice path")?;
            return Ok(PathBuf::from(local)
                .join("WorkDeskStudio")
                .join("onlyoffice")
                .join("documentserver")
                .join("documentserver.exe"));
        }

        let home = env::var("HOME").context("HOME is required for default OnlyOffice path")?;
        Ok(PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("WorkDeskStudio")
            .join("onlyoffice")
            .join("documentserver"))
    }

    fn default_sidecar_path() -> Result<PathBuf> {
        if cfg!(windows) {
            let local =
                env::var("LOCALAPPDATA").context("LOCALAPPDATA is required for sidecar path")?;
            return Ok(PathBuf::from(local)
                .join("WorkDeskStudio")
                .join("sidecar")
                .join("node")
                .join("node.exe"));
        }

        let home = env::var("HOME").context("HOME is required for sidecar path")?;
        Ok(PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("WorkDeskStudio")
            .join("sidecar")
            .join("node"))
    }

    fn default_toolchain_manifest_path() -> Result<PathBuf> {
        if cfg!(windows) {
            let local = env::var("LOCALAPPDATA")
                .context("LOCALAPPDATA is required for toolchain manifest path")?;
            return Ok(PathBuf::from(local)
                .join("WorkDeskStudio")
                .join("config")
                .join("toolchains.json"));
        }

        let home = env::var("HOME").context("HOME is required for toolchain manifest path")?;
        Ok(PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("WorkDeskStudio")
            .join("config")
            .join("toolchains.json"))
    }
}
