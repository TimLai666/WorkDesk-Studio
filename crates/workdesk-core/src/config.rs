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

        Ok(Self {
            db_path,
            workspace_root,
            core_bind,
            locale,
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
}
