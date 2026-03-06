use std::sync::Arc;
use tempfile::TempDir;
use workdesk_core::{
    CoreRepository, CoreService, CreateWorkflowInput, RunNodeStatus, Scope, SqliteCoreRepository,
    UpsertSkillInput, WorkflowNodeInput, WorkflowNodeKind,
};

async fn setup() -> (TempDir, CoreService) {
    let tmp = TempDir::new().expect("tempdir");
    let db_path = tmp.path().join("workdesk.db");
    let workspace_root = tmp.path().join("workspace");
    tokio::fs::create_dir_all(&workspace_root)
        .await
        .expect("workspace root");
    let repo = SqliteCoreRepository::connect(&db_path)
        .await
        .expect("sqlite connect");
    repo.migrate().await.expect("migrate");
    let service = CoreService::new(Arc::new(repo), workspace_root);
    (tmp, service)
}

#[tokio::test]
async fn run_skill_snapshot_prefers_user_scope_over_shared() {
    let (_tmp, service) = setup().await;
    service
        .upsert_skill(UpsertSkillInput {
            scope: Scope::Shared,
            name: "deploy".into(),
            manifest: "shared".into(),
            content_path: ".".into(),
            version: "1.0.0".into(),
        })
        .await
        .expect("insert shared skill");
    service
        .upsert_skill(UpsertSkillInput {
            scope: Scope::User,
            name: "deploy".into(),
            manifest: "user".into(),
            content_path: ".".into(),
            version: "2.0.0".into(),
        })
        .await
        .expect("insert user skill");

    let workflow = service
        .create_workflow(CreateWorkflowInput {
            name: "run".into(),
            timezone: "Asia/Taipei".into(),
            nodes: vec![WorkflowNodeInput {
                id: "n1".into(),
                kind: WorkflowNodeKind::ScheduleTrigger,
            }],
            edges: vec![],
        })
        .await
        .expect("create workflow");

    let run = service
        .queue_workflow_run(&workflow.id, Some("tester"))
        .await
        .expect("queue run");
    let snapshots = service
        .list_run_skills(&run.run_id)
        .await
        .expect("list run skills");
    let node_states = service
        .list_run_nodes(&run.run_id)
        .await
        .expect("list run nodes");

    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].name, "deploy");
    assert_eq!(snapshots[0].scope, Scope::User);
    assert_eq!(snapshots[0].version, "2.0.0");
    assert_eq!(node_states.len(), 1);
    assert_eq!(node_states[0].node_id, "n1");
    assert_eq!(node_states[0].status, RunNodeStatus::Pending);
}
