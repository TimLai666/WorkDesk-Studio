use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolchainBinary {
    Codex,
    Uv,
    Bun,
    Go,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Semver {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManagedToolchainRecord {
    pub binary: String,
    pub version: String,
    pub source: String,
    pub checksum_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ToolchainManifest {
    pub records: Vec<ManagedToolchainRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ToolchainReleaseFeed {
    pub channels: Vec<ToolchainReleaseChannel>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolchainReleaseChannel {
    pub name: String,
    pub records: Vec<ManagedToolchainRecord>,
}
