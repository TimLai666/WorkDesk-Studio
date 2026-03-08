use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use workdesk_core::AppConfig;

#[derive(Debug, Clone)]
pub struct RuntimeBootstrapper {
    config: AppConfig,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuntimeSeedReport {
    pub seeded_sidecar: bool,
    pub seeded_onlyoffice: bool,
}

impl RuntimeBootstrapper {
    pub fn new(config: AppConfig) -> Self {
        Self { config }
    }

    pub async fn ensure_seeded(&self) -> Result<RuntimeSeedReport> {
        let mut report = RuntimeSeedReport::default();

        if self.sidecar_missing() && self.config.bundled_sidecar_dir.exists() {
            copy_path_recursive(
                self.config.bundled_sidecar_dir.clone(),
                self.config.sidecar_runtime_root(),
            )
            .await
            .context("seed bundled sidecar runtime")?;
            report.seeded_sidecar = true;
        }

        if self.onlyoffice_missing() && self.config.bundled_onlyoffice_dir.exists() {
            copy_path_recursive(
                self.config.bundled_onlyoffice_dir.clone(),
                self.config.onlyoffice_runtime_root(),
            )
            .await
            .context("seed bundled onlyoffice runtime")?;
            report.seeded_onlyoffice = true;
        }

        Ok(report)
    }

    fn sidecar_missing(&self) -> bool {
        !self.config.sidecar_path.exists() || !self.config.sidecar_script_path.exists()
    }

    fn onlyoffice_missing(&self) -> bool {
        !self.config.onlyoffice_binary_path.exists()
    }
}

pub async fn copy_path_recursive(source: PathBuf, target: PathBuf) -> Result<()> {
    tokio::task::spawn_blocking(move || copy_path_recursive_sync(&source, &target))
        .await
        .context("join runtime bootstrap copy task")??;
    Ok(())
}

fn copy_path_recursive_sync(source: &Path, target: &Path) -> Result<()> {
    let metadata = std::fs::metadata(source)
        .with_context(|| format!("runtime bundle path not found: {}", source.display()))?;
    if metadata.is_file() {
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(source, target)?;
        return Ok(());
    }

    std::fs::create_dir_all(target)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let path = entry.path();
        let nested_target = target.join(entry.file_name());
        copy_path_recursive_sync(&path, &nested_target)?;
    }
    Ok(())
}
