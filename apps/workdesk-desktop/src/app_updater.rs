use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};
use workdesk_core::{AppConfig, AppUpdateFeed, AppUpdateManifest};

#[derive(Debug, Clone)]
pub struct DesktopAppUpdater {
    feed_source: String,
    public_key: String,
    download_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct PreparedAppUpdate {
    pub manifest: AppUpdateManifest,
    pub version: String,
    pub installer_path: PathBuf,
}

impl DesktopAppUpdater {
    pub fn new(feed_source: String, public_key: String, download_dir: PathBuf) -> Self {
        Self {
            feed_source,
            public_key,
            download_dir,
        }
    }

    pub async fn from_app_config(config: &AppConfig) -> Result<Option<Self>> {
        let Some(feed_source) = config.app_update_feed_url.clone() else {
            return Ok(None);
        };
        if !config.app_update_public_key_path.exists() {
            return Ok(None);
        }
        let public_key = tokio::fs::read_to_string(&config.app_update_public_key_path)
            .await
            .with_context(|| {
                format!(
                    "read app update public key {}",
                    config.app_update_public_key_path.display()
                )
            })?;
        let download_dir = config.install_root.join("downloads").join("app-updates");
        Ok(Some(Self::new(
            feed_source,
            public_key.trim().to_string(),
            download_dir,
        )))
    }

    pub async fn prepare_update(&self, channel: &str) -> Result<PreparedAppUpdate> {
        let manifest = self.select_manifest(channel).await?;
        let package = read_source_bytes(&manifest.package_url).await?;
        manifest.verify_package(&package, &self.public_key)?;

        tokio::fs::create_dir_all(&self.download_dir)
            .await
            .with_context(|| {
                format!(
                    "create app update download dir {}",
                    self.download_dir.display()
                )
            })?;
        let installer_path = self.download_dir.join(installer_file_name(&manifest)?);
        tokio::fs::write(&installer_path, &package)
            .await
            .with_context(|| format!("write prepared installer {}", installer_path.display()))?;

        Ok(PreparedAppUpdate {
            version: manifest.version.clone(),
            manifest,
            installer_path,
        })
    }

    pub async fn select_manifest(&self, channel: &str) -> Result<AppUpdateManifest> {
        let feed = AppUpdateFeed::load(&self.feed_source).await?;
        Ok(feed.select_channel(channel)?.clone())
    }
}

fn installer_file_name(manifest: &AppUpdateManifest) -> Result<String> {
    let package_url = manifest.package_url.trim();
    if package_url.is_empty() {
        return Err(anyhow!("app update package url is empty"));
    }
    let name = Path::new(package_url)
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("WorkDeskStudio-{}.msi", manifest.version));
    Ok(name)
}

async fn read_source_bytes(source: &str) -> Result<Vec<u8>> {
    if source.starts_with("http://") || source.starts_with("https://") {
        let response = reqwest::get(source)
            .await
            .with_context(|| format!("download app update package {source}"))?
            .error_for_status()
            .with_context(|| format!("app update package returned error status: {source}"))?;
        let bytes = response
            .bytes()
            .await
            .with_context(|| format!("read app update package body {source}"))?;
        return Ok(bytes.to_vec());
    }

    let path = source
        .strip_prefix("file://")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(source));
    tokio::fs::read(&path)
        .await
        .with_context(|| format!("read app update package {}", path.display()))
}
