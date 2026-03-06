use sha2::{Digest, Sha256};
use tempfile::TempDir;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use workdesk_runner::{
    ExecutionLanguage, ManagedToolchainRecord, ToolchainBinary, ToolchainManager, ToolchainManifest,
    ToolchainReleaseChannel, ToolchainReleaseFeed,
};

#[test]
fn parses_semver_from_tool_output() {
    let manager = ToolchainManager::new("C:/workdesk/tools".into());
    let parsed = manager
        .parse_version_output("uv 0.6.8", ToolchainBinary::Uv)
        .expect("version parsing should succeed");
    assert_eq!(parsed.major, 0);
    assert_eq!(parsed.minor, 6);
    assert_eq!(parsed.patch, 8);
}

#[test]
fn workflow_runtime_paths_are_isolated_by_workflow_id() {
    let manager = ToolchainManager::new("C:/workdesk/tools".into());
    let wf_a = manager.workflow_runtime_root("wf-a", ExecutionLanguage::Python);
    let wf_b = manager.workflow_runtime_root("wf-b", ExecutionLanguage::Python);

    assert_ne!(wf_a, wf_b);
    assert!(wf_a.to_string_lossy().contains("wf-a"));
    assert!(wf_b.to_string_lossy().contains("wf-b"));
}

#[test]
fn binary_path_is_scoped_under_app_tools_dir() {
    let manager = ToolchainManager::new("C:/workdesk/tools".into());
    let codex = manager.binary_path(ToolchainBinary::Codex);
    assert!(codex.to_string_lossy().contains("C:/workdesk/tools"));
    assert!(codex.to_string_lossy().contains("codex"));
}

#[tokio::test]
async fn manifest_roundtrip_persists_records() {
    let tmp = TempDir::new().expect("tempdir");
    let manager = ToolchainManager::new(tmp.path().join("tools"));
    let manifest_path = tmp.path().join("toolchains.json");
    let manifest = ToolchainManifest {
        records: vec![ManagedToolchainRecord {
            binary: "uv".into(),
            version: "0.6.8".into(),
            source: "https://example.com/uv.zip".into(),
            checksum_sha256: "abc123".into(),
        }],
    };

    manager
        .save_manifest(&manifest_path, &manifest)
        .await
        .expect("save manifest");
    let loaded = manager
        .load_manifest(&manifest_path)
        .await
        .expect("load manifest");
    assert_eq!(loaded, manifest);
}

#[tokio::test]
async fn rollback_restores_previous_binary() {
    let tmp = TempDir::new().expect("tempdir");
    let manager = ToolchainManager::new(tmp.path().join("tools"));
    let binary = manager.binary_path(ToolchainBinary::Codex);
    if let Some(parent) = binary.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .expect("create binary dir");
    }
    tokio::fs::write(&binary, "new-version")
        .await
        .expect("write current binary");

    manager
        .stage_for_update(ToolchainBinary::Codex)
        .await
        .expect("stage for update");
    tokio::fs::write(&binary, "broken-version")
        .await
        .expect("write broken current");

    let rolled_back = manager
        .rollback_binary(ToolchainBinary::Codex)
        .await
        .expect("rollback");
    assert!(rolled_back);

    let content = tokio::fs::read_to_string(&binary)
        .await
        .expect("read restored binary");
    assert_eq!(content, "new-version");
}

#[tokio::test]
async fn installs_release_from_file_feed_and_updates_manifest() {
    let tmp = TempDir::new().expect("tempdir");
    let manager = ToolchainManager::new(tmp.path().join("tools"));
    let manifest_path = tmp.path().join("toolchains.json");
    let asset_path = tmp.path().join("uv.exe");
    let asset_bytes = b"uv-managed-binary";
    tokio::fs::write(&asset_path, asset_bytes)
        .await
        .expect("write asset");

    let checksum = format!("{:x}", Sha256::digest(asset_bytes));
    let feed = ToolchainReleaseFeed {
        channels: vec![ToolchainReleaseChannel {
            name: "stable".into(),
            records: vec![ManagedToolchainRecord {
                binary: "uv".into(),
                version: "0.6.9".into(),
                source: asset_path.to_string_lossy().to_string(),
                checksum_sha256: checksum,
            }],
        }],
    };
    let feed_path = tmp.path().join("feed.json");
    tokio::fs::write(&feed_path, serde_json::to_vec_pretty(&feed).expect("feed json"))
        .await
        .expect("write feed");

    let record = manager
        .install_from_release_feed(
            ToolchainBinary::Uv,
            "stable",
            &feed_path.to_string_lossy(),
            &manifest_path,
        )
        .await
        .expect("install from release feed");

    assert_eq!(record.version, "0.6.9");

    let binary_path = manager.binary_path(ToolchainBinary::Uv);
    let installed = tokio::fs::read(&binary_path).await.expect("installed binary");
    assert_eq!(installed, asset_bytes);

    let manifest = manager
        .load_manifest(&manifest_path)
        .await
        .expect("load manifest");
    assert_eq!(manifest.records, vec![record]);
}

#[tokio::test]
async fn checksum_mismatch_rolls_back_staged_binary() {
    let tmp = TempDir::new().expect("tempdir");
    let manager = ToolchainManager::new(tmp.path().join("tools"));
    let manifest_path = tmp.path().join("toolchains.json");
    let current_binary = manager.binary_path(ToolchainBinary::Codex);
    if let Some(parent) = current_binary.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .expect("create current dir");
    }
    tokio::fs::write(&current_binary, "trusted-binary")
        .await
        .expect("write current");

    let asset_path = tmp.path().join("codex.exe");
    tokio::fs::write(&asset_path, "tampered-binary")
        .await
        .expect("write tampered asset");

    let feed = ToolchainReleaseFeed {
        channels: vec![ToolchainReleaseChannel {
            name: "stable".into(),
            records: vec![ManagedToolchainRecord {
                binary: "codex".into(),
                version: "1.0.1".into(),
                source: asset_path.to_string_lossy().to_string(),
                checksum_sha256: "deadbeef".into(),
            }],
        }],
    };
    let feed_path = tmp.path().join("feed.json");
    tokio::fs::write(&feed_path, serde_json::to_vec_pretty(&feed).expect("feed json"))
        .await
        .expect("write feed");

    let error = manager
        .install_from_release_feed(
            ToolchainBinary::Codex,
            "stable",
            &feed_path.to_string_lossy(),
            &manifest_path,
        )
        .await
        .expect_err("checksum mismatch should fail");
    assert!(error.to_string().contains("checksum"));

    let restored = tokio::fs::read_to_string(&current_binary)
        .await
        .expect("restored binary");
    assert_eq!(restored, "trusted-binary");
}

#[tokio::test]
async fn loads_release_feed_over_http() {
    let manager = ToolchainManager::new("C:/workdesk/tools".into());
    let feed = ToolchainReleaseFeed {
        channels: vec![ToolchainReleaseChannel {
            name: "stable".into(),
            records: vec![ManagedToolchainRecord {
                binary: "bun".into(),
                version: "1.1.0".into(),
                source: "https://example.invalid/bun.exe".into(),
                checksum_sha256: "abc123".into(),
            }],
        }],
    };
    let body = serde_json::to_string(&feed).expect("serialize feed");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let addr = listener.local_addr().expect("local addr");

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept");
        let mut request = [0u8; 1024];
        let _ = socket.read(&mut request).await.expect("read request");
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        socket
            .write_all(response.as_bytes())
            .await
            .expect("write response");
    });

    let loaded = manager
        .load_release_feed(&format!("http://{addr}/feed.json"))
        .await
        .expect("load release feed");
    server.await.expect("server task");

    assert_eq!(loaded, feed);
}
