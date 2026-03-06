use anyhow::{anyhow, Context, Result};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct AppUpdateFeed {
    pub manifests: Vec<AppUpdateManifest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppUpdateManifest {
    pub channel: String,
    pub version: String,
    pub package_url: String,
    pub package_sha256: String,
    pub signature: String,
}

impl AppUpdateFeed {
    pub async fn load(source: &str) -> Result<Self> {
        let raw = read_source_bytes(source).await?;
        serde_json::from_slice(&raw).with_context(|| format!("parse app update feed from {source}"))
    }

    pub fn select_channel(&self, channel: &str) -> Result<&AppUpdateManifest> {
        self.manifests
            .iter()
            .find(|manifest| manifest.channel == channel)
            .ok_or_else(|| anyhow!("update channel not found: {channel}"))
    }
}

async fn read_source_bytes(source: &str) -> Result<Vec<u8>> {
    if source.starts_with("http://") || source.starts_with("https://") {
        let response = reqwest::get(source)
            .await
            .with_context(|| format!("download app update feed {source}"))?
            .error_for_status()
            .with_context(|| format!("app update feed returned error status: {source}"))?;
        let bytes = response
            .bytes()
            .await
            .with_context(|| format!("read app update feed body {source}"))?;
        return Ok(bytes.to_vec());
    }

    let path = source
        .strip_prefix("file://")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from(source));
    tokio::fs::read(&path)
        .await
        .with_context(|| format!("read app update feed {}", path.display()))
}

impl AppUpdateManifest {
    pub fn signing_payload(&self) -> String {
        format!(
            "{}\n{}\n{}\n{}",
            self.channel, self.version, self.package_url, self.package_sha256
        )
    }

    pub fn verify_signature(&self, public_key_base64: &str) -> Result<()> {
        let key_bytes = STANDARD
            .decode(public_key_base64)
            .context("decode update public key")?;
        let key_bytes: [u8; 32] = key_bytes
            .try_into()
            .map_err(|_| anyhow!("update public key must be 32 bytes"))?;
        let verifying_key =
            VerifyingKey::from_bytes(&key_bytes).context("parse update public key")?;

        let signature_bytes = STANDARD
            .decode(&self.signature)
            .context("decode update signature")?;
        let signature_bytes: [u8; 64] = signature_bytes
            .try_into()
            .map_err(|_| anyhow!("update signature must be 64 bytes"))?;
        let signature = Signature::from_bytes(&signature_bytes);

        verifying_key
            .verify(self.signing_payload().as_bytes(), &signature)
            .map_err(|_| anyhow!("manifest signature verification failed"))
    }

    pub fn verify_package(&self, package_bytes: &[u8], public_key_base64: &str) -> Result<()> {
        self.verify_signature(public_key_base64)?;
        let actual = format!("{:x}", Sha256::digest(package_bytes));
        if actual.eq_ignore_ascii_case(self.package_sha256.trim()) {
            return Ok(());
        }
        Err(anyhow!(
            "package sha256 mismatch: expected {}, got {}",
            self.package_sha256,
            actual
        ))
    }
}
