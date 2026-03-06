use workdesk_runner::{ExecutionLanguage, ToolchainBinary, ToolchainManager};

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
