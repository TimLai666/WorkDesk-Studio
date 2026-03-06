use tempfile::TempDir;
use workdesk_runner::{
    ExecutionLanguage, ManagedToolchainRecord, ToolchainBinary, ToolchainManager, ToolchainManifest,
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
