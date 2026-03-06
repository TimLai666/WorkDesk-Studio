use super::lib::{
    ManagedToolchainRecord, Semver, ToolchainBinary, ToolchainManifest, ToolchainReleaseFeed,
};
use anyhow::{anyhow, Context, Result};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use tokio::process::Command;
use workdesk_domain::ExecutionLanguage;

#[derive(Debug, Clone)]
pub struct ToolchainManager {
    tools_root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolchainStatus {
    pub binary: ToolchainBinary,
    pub installed: bool,
    pub version: Option<Semver>,
}

impl ToolchainManager {
    pub fn new(tools_root: PathBuf) -> Self {
        Self { tools_root }
    }

    pub fn tools_root(&self) -> &PathBuf {
        &self.tools_root
    }

    pub fn binary_dir(&self, binary: ToolchainBinary) -> PathBuf {
        self.tools_root.join(Self::binary_slug(binary))
    }

    pub fn binary_path(&self, binary: ToolchainBinary) -> PathBuf {
        let executable = format!("{}.exe", Self::binary_slug(binary));
        self.binary_dir(binary).join(executable)
    }

    pub fn backup_binary_path(&self, binary: ToolchainBinary) -> PathBuf {
        let current = self.binary_path(binary);
        let mut backup_name = current
            .file_name()
            .map(|name| name.to_os_string())
            .unwrap_or_else(|| std::ffi::OsString::from("tool.exe"));
        backup_name.push(".previous");
        current.with_file_name(backup_name)
    }

    pub fn workflow_runtime_root(&self, workflow_id: &str, language: ExecutionLanguage) -> PathBuf {
        let lang_dir = match language {
            ExecutionLanguage::Python => "python",
            ExecutionLanguage::Javascript => "javascript",
            ExecutionLanguage::Go => "go",
        };
        self.tools_root
            .join("workflows")
            .join(workflow_id)
            .join(lang_dir)
    }

    pub fn parse_version_output(&self, output: &str, _binary: ToolchainBinary) -> Result<Semver> {
        let token = output
            .split_whitespace()
            .find(|item| item.chars().any(|ch| ch.is_ascii_digit()))
            .ok_or_else(|| anyhow!("no semver token found in output"))?;

        let version = token.trim_start_matches('v');
        let mut parts = version.split('.');
        let major = parts
            .next()
            .ok_or_else(|| anyhow!("missing major version"))?
            .parse::<u64>()
            .context("invalid major version")?;
        let minor = parts
            .next()
            .ok_or_else(|| anyhow!("missing minor version"))?
            .parse::<u64>()
            .context("invalid minor version")?;
        let patch = parts
            .next()
            .ok_or_else(|| anyhow!("missing patch version"))?
            .chars()
            .take_while(|ch| ch.is_ascii_digit())
            .collect::<String>()
            .parse::<u64>()
            .context("invalid patch version")?;

        Ok(Semver {
            major,
            minor,
            patch,
        })
    }

    pub async fn detect_installed_version(
        &self,
        binary: ToolchainBinary,
    ) -> Result<Option<Semver>> {
        let binary_path = self.binary_path(binary);
        if !binary_path.exists() {
            return Ok(None);
        }

        let output = Command::new(binary_path).arg("--version").output().await?;
        if !output.status.success() {
            return Ok(None);
        }
        let parsed = self.parse_version_output(&String::from_utf8_lossy(&output.stdout), binary)?;
        Ok(Some(parsed))
    }

    pub async fn toolchain_status(&self, binary: ToolchainBinary) -> Result<ToolchainStatus> {
        let version = self.detect_installed_version(binary).await?;
        Ok(ToolchainStatus {
            binary,
            installed: version.is_some(),
            version,
        })
    }

    pub async fn load_manifest(&self, manifest_path: &PathBuf) -> Result<ToolchainManifest> {
        if !manifest_path.exists() {
            return Ok(ToolchainManifest::default());
        }
        let raw = tokio::fs::read_to_string(manifest_path)
            .await
            .with_context(|| format!("read toolchain manifest {}", manifest_path.display()))?;
        Ok(serde_json::from_str(&raw)
            .with_context(|| format!("parse toolchain manifest {}", manifest_path.display()))?)
    }

    pub async fn save_manifest(
        &self,
        manifest_path: &PathBuf,
        manifest: &ToolchainManifest,
    ) -> Result<()> {
        if let Some(parent) = manifest_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let raw = serde_json::to_string_pretty(manifest)?;
        tokio::fs::write(manifest_path, raw)
            .await
            .with_context(|| format!("write toolchain manifest {}", manifest_path.display()))?;
        Ok(())
    }

    pub async fn stage_for_update(&self, binary: ToolchainBinary) -> Result<()> {
        let current = self.binary_path(binary);
        if !current.exists() {
            return Ok(());
        }
        let backup = self.backup_binary_path(binary);
        if let Some(parent) = backup.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        if backup.exists() {
            tokio::fs::remove_file(&backup).await?;
        }
        tokio::fs::rename(&current, &backup).await?;
        Ok(())
    }

    pub async fn rollback_binary(&self, binary: ToolchainBinary) -> Result<bool> {
        let current = self.binary_path(binary);
        let backup = self.backup_binary_path(binary);
        if !backup.exists() {
            return Ok(false);
        }
        if current.exists() {
            tokio::fs::remove_file(&current).await?;
        }
        tokio::fs::rename(&backup, &current).await?;
        Ok(true)
    }

    pub async fn load_release_feed(&self, source: &str) -> Result<ToolchainReleaseFeed> {
        let raw = self.read_source_bytes(source).await?;
        serde_json::from_slice(&raw).with_context(|| format!("parse release feed from {source}"))
    }

    pub async fn install_from_release_feed(
        &self,
        binary: ToolchainBinary,
        channel: &str,
        feed_source: &str,
        manifest_path: &PathBuf,
    ) -> Result<ManagedToolchainRecord> {
        let feed = self.load_release_feed(feed_source).await?;
        let record = self.select_release_record(&feed, binary, channel)?;
        let payload = self.read_source_bytes(&record.source).await?;
        let had_existing_binary = self.binary_path(binary).exists();

        self.stage_for_update(binary).await?;

        let install_result = async {
            self.verify_checksum(&payload, &record.checksum_sha256)?;
            let binary_path = self.binary_path(binary);
            if let Some(parent) = binary_path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            tokio::fs::write(&binary_path, &payload)
                .await
                .with_context(|| format!("write managed binary {}", binary_path.display()))?;

            let mut manifest = self.load_manifest(manifest_path).await?;
            upsert_manifest_record(&mut manifest.records, record.clone());
            self.save_manifest(manifest_path, &manifest).await?;
            Ok::<ManagedToolchainRecord, anyhow::Error>(record.clone())
        }
        .await;

        match install_result {
            Ok(record) => Ok(record),
            Err(error) => {
                if had_existing_binary {
                    let _ = self.rollback_binary(binary).await;
                } else {
                    let current = self.binary_path(binary);
                    if current.exists() {
                        let _ = tokio::fs::remove_file(&current).await;
                    }
                }
                Err(error)
            }
        }
    }

    fn select_release_record(
        &self,
        feed: &ToolchainReleaseFeed,
        binary: ToolchainBinary,
        channel: &str,
    ) -> Result<ManagedToolchainRecord> {
        let channel_record = feed
            .channels
            .iter()
            .find(|item| item.name == channel)
            .ok_or_else(|| anyhow!("release channel not found: {channel}"))?;
        channel_record
            .records
            .iter()
            .find(|item| item.binary == Self::binary_slug(binary))
            .cloned()
            .ok_or_else(|| {
                anyhow!(
                    "release record not found for {} in channel {channel}",
                    Self::binary_slug(binary)
                )
            })
    }

    async fn read_source_bytes(&self, source: &str) -> Result<Vec<u8>> {
        if source.starts_with("http://") || source.starts_with("https://") {
            let response = reqwest::get(source)
                .await
                .with_context(|| format!("download release source {source}"))?
                .error_for_status()
                .with_context(|| format!("release source returned error status: {source}"))?;
            let body = response
                .bytes()
                .await
                .with_context(|| format!("read release source body {source}"))?;
            return Ok(body.to_vec());
        }

        let path = source
            .strip_prefix("file://")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(source));
        tokio::fs::read(&path)
            .await
            .with_context(|| format!("read release source {}", path.display()))
    }

    fn verify_checksum(&self, payload: &[u8], expected_sha256: &str) -> Result<()> {
        let expected = expected_sha256.trim();
        if expected.is_empty() {
            return Err(anyhow!(
                "checksum is required for managed toolchain install"
            ));
        }
        let actual = format!("{:x}", Sha256::digest(payload));
        if actual.eq_ignore_ascii_case(expected) {
            return Ok(());
        }
        Err(anyhow!(
            "checksum mismatch: expected {expected}, got {actual}"
        ))
    }

    fn binary_slug(binary: ToolchainBinary) -> &'static str {
        match binary {
            ToolchainBinary::Codex => "codex",
            ToolchainBinary::Uv => "uv",
            ToolchainBinary::Bun => "bun",
            ToolchainBinary::Go => "go",
        }
    }
}

fn upsert_manifest_record(
    records: &mut Vec<ManagedToolchainRecord>,
    record: ManagedToolchainRecord,
) {
    if let Some(existing) = records.iter_mut().find(|item| item.binary == record.binary) {
        *existing = record;
    } else {
        records.push(record);
    }
}
