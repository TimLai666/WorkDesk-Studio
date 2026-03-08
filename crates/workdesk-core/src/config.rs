use anyhow::{Context, Result};
use std::env;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

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
    pub sidecar_script_path: PathBuf,
    pub toolchain_manifest_path: PathBuf,
    pub app_update_channel: String,
    pub toolchain_update_channel: String,
    pub install_root: PathBuf,
    pub bundled_sidecar_dir: PathBuf,
    pub bundled_onlyoffice_dir: PathBuf,
    pub app_update_feed_url: Option<String>,
    pub app_update_public_key_path: PathBuf,
}

impl AppConfig {
    pub fn from_env() -> Result<Self> {
        let install_root = env::var("WORKDESK_INSTALL_ROOT")
            .map(PathBuf::from)
            .unwrap_or(Self::default_install_root()?);
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
        let sidecar_script_path = env::var("WORKDESK_SIDECAR_SCRIPT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| Self::default_sidecar_script_path(&sidecar_path));
        let toolchain_manifest_path = env::var("WORKDESK_TOOLCHAIN_MANIFEST")
            .map(PathBuf::from)
            .unwrap_or(Self::default_toolchain_manifest_path()?);
        let app_update_channel =
            env::var("WORKDESK_APP_UPDATE_CHANNEL").unwrap_or_else(|_| "stable".to_string());
        let toolchain_update_channel =
            env::var("WORKDESK_TOOLCHAIN_UPDATE_CHANNEL").unwrap_or_else(|_| "stable".to_string());
        let bundled_sidecar_dir = env::var("WORKDESK_BUNDLED_SIDECAR_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| install_root.join("resources").join("sidecar"));
        let bundled_onlyoffice_dir = env::var("WORKDESK_BUNDLED_ONLYOFFICE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| install_root.join("resources").join("onlyoffice"));
        let app_update_feed_url = env::var("WORKDESK_APP_UPDATE_FEED").ok().or_else(|| {
            let candidate = install_root
                .join("resources")
                .join("updates")
                .join("app-update-feed.json");
            candidate
                .exists()
                .then(|| format!("file://{}", candidate.display()))
        });
        let app_update_public_key_path = env::var("WORKDESK_APP_UPDATE_PUBLIC_KEY")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                install_root
                    .join("resources")
                    .join("updates")
                    .join("app-update-public-key.txt")
            });

        Ok(Self {
            db_path,
            workspace_root,
            core_bind,
            locale,
            onlyoffice_host,
            onlyoffice_port,
            onlyoffice_binary_path,
            sidecar_path,
            sidecar_script_path,
            toolchain_manifest_path,
            app_update_channel,
            toolchain_update_channel,
            install_root,
            bundled_sidecar_dir,
            bundled_onlyoffice_dir,
            app_update_feed_url,
            app_update_public_key_path,
        })
    }

    pub fn sidecar_runtime_root(&self) -> PathBuf {
        Self::sidecar_runtime_root_from_path(&self.sidecar_path)
    }

    pub fn onlyoffice_runtime_root(&self) -> PathBuf {
        self.onlyoffice_binary_path
            .parent()
            .map(PathBuf::from)
            .unwrap_or_else(|| self.install_root.join("runtime").join("onlyoffice"))
    }

    fn default_install_root() -> Result<PathBuf> {
        let current_exe = env::current_exe().context("resolve current executable path")?;
        Ok(current_exe
            .parent()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(".")))
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

    fn default_sidecar_script_path(sidecar_path: &Path) -> PathBuf {
        Self::sidecar_runtime_root_from_path(sidecar_path).join("sidecar.js")
    }

    fn sidecar_runtime_root_from_path(sidecar_path: &Path) -> PathBuf {
        let parent = sidecar_path
            .parent()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        let is_node_dir = parent
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.eq_ignore_ascii_case("node"))
            .unwrap_or(false);
        if is_node_dir {
            parent.parent().map(PathBuf::from).unwrap_or(parent)
        } else {
            parent
        }
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
