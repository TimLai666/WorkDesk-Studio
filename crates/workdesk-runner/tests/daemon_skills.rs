use tempfile::TempDir;
use workdesk_core::{
    CoreRepository, RunNodeStatus, RunStatus, Scope, SkillRecord, SqliteCoreRepository,
    WorkflowDefinition, WorkflowNode, WorkflowNodeKind, WorkflowStatus,
};
use workdesk_runner::{RunnerConfig, WorkflowRunnerDaemon};

#[tokio::test]
async fn daemon_materializes_skill_snapshots_before_finishing_run() {
    let tmp = TempDir::new().expect("tempdir");
    let db_path = tmp.path().join("workdesk.db");
    let tools_root = tmp.path().join("tools");
    tokio::fs::create_dir_all(&tools_root)
        .await
        .expect("create tools root");

    let skill_source = tmp.path().join("skill-source");
    tokio::fs::create_dir_all(&skill_source)
        .await
        .expect("skill source dir");
    tokio::fs::write(skill_source.join("README.md"), "deploy-skill")
        .await
        .expect("write skill file");

    let repo = SqliteCoreRepository::connect(&db_path)
        .await
        .expect("connect sqlite");
    repo.migrate().await.expect("migrate");

    let workflow = WorkflowDefinition {
        id: "wf-runner".into(),
        name: "runner".into(),
        timezone: "Asia/Taipei".into(),
        nodes: vec![WorkflowNode {
            id: "n1".into(),
            kind: WorkflowNodeKind::ScheduleTrigger,
        }],
        edges: vec![],
        version: 1,
        status: WorkflowStatus::Draft,
    };
    repo.create_workflow(&workflow).await.expect("create workflow");
    repo.upsert_skill(&SkillRecord {
        scope: Scope::User,
        name: "deploy".into(),
        manifest: "demo".into(),
        content_path: skill_source.to_string_lossy().to_string(),
        version: "1.0.0".into(),
    })
    .await
    .expect("insert skill");

    let run = repo
        .create_run("wf-runner", Some("tester"))
        .await
        .expect("create run");
    repo.create_run_skill_snapshots(&run.run_id)
        .await
        .expect("create snapshots");
    repo.create_run_node_states(&run.run_id, &workflow.nodes)
        .await
        .expect("create run node states");

    let daemon = WorkflowRunnerDaemon::new(RunnerConfig {
        db_path: db_path.clone(),
        tools_root,
        runner_id: "runner-test".into(),
        poll_interval_ms: 50,
        lease_seconds: 30,
    })
    .await
    .expect("create daemon");
    assert!(daemon.run_once().await.expect("run once should execute"));

    let finished = repo
        .get_run(&run.run_id)
        .await
        .expect("get run")
        .expect("run exists");
    assert_eq!(finished.status, RunStatus::Succeeded);

    let snapshots = repo
        .list_run_skill_snapshots(&run.run_id)
        .await
        .expect("list snapshots");
    assert_eq!(snapshots.len(), 1);
    let materialized = snapshots[0]
        .materialized_path
        .clone()
        .expect("materialized path");
    let copied = std::path::PathBuf::from(materialized).join("README.md");
    assert!(
        copied.exists(),
        "expected copied skill file at {}",
        copied.display()
    );

    let node_states = repo
        .list_run_node_states(&run.run_id)
        .await
        .expect("list node states");
    assert_eq!(node_states.len(), 1);
    assert_eq!(node_states[0].node_id, "n1");
    assert_eq!(node_states[0].status, RunNodeStatus::Succeeded);
}
