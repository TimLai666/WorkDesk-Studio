use std::sync::Arc;
use tempfile::TempDir;
use workdesk_core::{
    AppConfig, AuthLoginInput, AuthSwitchInput, CoreError, CoreRepository, CoreService,
    CreateProposalInput, CreateWorkflowInput, SqliteCoreRepository, WorkflowNodeInput,
    WorkflowNodeKind,
};

async fn setup_service(
    tmp: &TempDir,
) -> (
    SqliteCoreRepository,
    CoreService,
    std::path::PathBuf,
    std::path::PathBuf,
) {
    let db_path = tmp.path().join("workdesk.db");
    let workspace_root = tmp.path().join("workspace");
    tokio::fs::create_dir_all(&workspace_root)
        .await
        .expect("create workspace dir");

    let repo = SqliteCoreRepository::connect(&db_path)
        .await
        .expect("connect sqlite");
    repo.migrate().await.expect("run migrations");
    let service = CoreService::new(Arc::new(repo.clone()), workspace_root.clone());
    (repo, service, db_path, workspace_root)
}

#[tokio::test]
async fn migration_is_idempotent() {
    let tmp = TempDir::new().expect("tempdir");
    let db_path = tmp.path().join("workdesk.db");
    let repo = SqliteCoreRepository::connect(&db_path)
        .await
        .expect("connect sqlite");
    repo.migrate().await.expect("first migration pass");
    repo.migrate().await.expect("second migration pass");
}

#[tokio::test]
async fn workflow_persists_across_restart() {
    let tmp = TempDir::new().expect("tempdir");
    let (repo, service, db_path, workspace_root) = setup_service(&tmp).await;
    let workflow = service
        .create_workflow(CreateWorkflowInput {
            name: "ops".into(),
            timezone: "Asia/Taipei".into(),
            agent_defaults: None,
            nodes: vec![WorkflowNodeInput {
                id: "n1".into(),
                kind: WorkflowNodeKind::ScheduleTrigger,
                x: None,
                y: None,
                config: None,
            }],
            edges: vec![],
        })
        .await
        .expect("create workflow");
    drop(service);
    drop(repo);

    let repo2 = SqliteCoreRepository::connect(&db_path)
        .await
        .expect("reconnect sqlite");
    repo2.migrate().await.expect("rerun migrations");
    let service2 = CoreService::new(Arc::new(repo2), workspace_root);
    let loaded = service2
        .get_workflow(&workflow.id)
        .await
        .expect("load workflow after restart");
    assert_eq!(loaded.name, "ops");
}

#[tokio::test]
async fn switch_account_revokes_old_session_and_returns_new_token() {
    let tmp = TempDir::new().expect("tempdir");
    let (repo, service, _db_path, _workspace_root) = setup_service(&tmp).await;

    let first = service
        .login(AuthLoginInput {
            account_id: "alice".into(),
            password: "password1".into(),
        })
        .await
        .expect("alice login");
    let _ = service
        .login(AuthLoginInput {
            account_id: "bob".into(),
            password: "password2".into(),
        })
        .await
        .expect("bob login");

    let switched = service
        .switch_account(AuthSwitchInput {
            from_account: "alice".into(),
            to_account: "bob".into(),
        })
        .await
        .expect("switch account");
    assert_eq!(switched.account_id, "bob");
    assert_ne!(switched.session_token, first.session_token);

    let revoked_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sessions WHERE account_id = ? AND revoked_at IS NOT NULL",
    )
    .bind("alice")
    .fetch_one(repo.pool())
    .await
    .expect("count revoked sessions");
    assert!(revoked_count >= 1);
}

#[tokio::test]
async fn proposal_must_be_pending_to_approve() {
    let tmp = TempDir::new().expect("tempdir");
    let (_repo, service, _db_path, _workspace_root) = setup_service(&tmp).await;
    let workflow = service
        .create_workflow(CreateWorkflowInput {
            name: "ops".into(),
            timezone: "Asia/Taipei".into(),
            agent_defaults: None,
            nodes: vec![WorkflowNodeInput {
                id: "n1".into(),
                kind: WorkflowNodeKind::ScheduleTrigger,
                x: None,
                y: None,
                config: None,
            }],
            edges: vec![],
        })
        .await
        .expect("create workflow");

    let proposal = service
        .propose_workflow_change(
            workflow.id.clone(),
            CreateProposalInput {
                diff: "diff".into(),
                created_by_agent: "agent".into(),
            },
        )
        .await
        .expect("create proposal");
    let _approved = service
        .approve_proposal(&proposal.proposal_id, "human".into())
        .await
        .expect("first approval");

    let err = service
        .approve_proposal(&proposal.proposal_id, "human".into())
        .await
        .expect_err("second approval should fail");
    assert!(matches!(err, CoreError::ProposalNotPending));
}

#[test]
fn app_config_allows_db_override() {
    let old_db = std::env::var("WORKDESK_DB_PATH").ok();
    let old_bind = std::env::var("WORKDESK_CORE_BIND").ok();
    let old_workspace = std::env::var("WORKDESK_WORKSPACE_ROOT").ok();
    let old_locale = std::env::var("WORKDESK_LOCALE").ok();
    let old_onlyoffice_host = std::env::var("WORKDESK_ONLYOFFICE_HOST").ok();
    let old_onlyoffice_port = std::env::var("WORKDESK_ONLYOFFICE_PORT").ok();
    let old_sidecar = std::env::var("WORKDESK_SIDECAR_PATH").ok();
    let old_manifest = std::env::var("WORKDESK_TOOLCHAIN_MANIFEST").ok();
    let old_app_channel = std::env::var("WORKDESK_APP_UPDATE_CHANNEL").ok();
    let old_toolchain_channel = std::env::var("WORKDESK_TOOLCHAIN_UPDATE_CHANNEL").ok();
    let db_override = std::env::temp_dir().join("workdesk-config-test.db");
    std::env::set_var("WORKDESK_DB_PATH", &db_override);
    std::env::set_var("WORKDESK_CORE_BIND", "127.0.0.1:4100");
    std::env::set_var("WORKDESK_WORKSPACE_ROOT", ".");
    std::env::set_var("WORKDESK_LOCALE", "en");
    std::env::set_var("WORKDESK_ONLYOFFICE_HOST", "127.0.0.1");
    std::env::set_var("WORKDESK_ONLYOFFICE_PORT", "9001");
    std::env::set_var("WORKDESK_SIDECAR_PATH", "C:/tmp/sidecar/node.exe");
    std::env::set_var("WORKDESK_TOOLCHAIN_MANIFEST", "C:/tmp/toolchains.json");
    std::env::set_var("WORKDESK_APP_UPDATE_CHANNEL", "beta");
    std::env::set_var("WORKDESK_TOOLCHAIN_UPDATE_CHANNEL", "canary");
    let cfg = AppConfig::from_env().expect("load app config");
    assert_eq!(cfg.db_path, db_override);
    assert_eq!(cfg.onlyoffice_port, 9001);
    assert_eq!(cfg.app_update_channel, "beta");
    assert_eq!(cfg.toolchain_update_channel, "canary");

    restore_var("WORKDESK_DB_PATH", old_db);
    restore_var("WORKDESK_CORE_BIND", old_bind);
    restore_var("WORKDESK_WORKSPACE_ROOT", old_workspace);
    restore_var("WORKDESK_LOCALE", old_locale);
    restore_var("WORKDESK_ONLYOFFICE_HOST", old_onlyoffice_host);
    restore_var("WORKDESK_ONLYOFFICE_PORT", old_onlyoffice_port);
    restore_var("WORKDESK_SIDECAR_PATH", old_sidecar);
    restore_var("WORKDESK_TOOLCHAIN_MANIFEST", old_manifest);
    restore_var("WORKDESK_APP_UPDATE_CHANNEL", old_app_channel);
    restore_var("WORKDESK_TOOLCHAIN_UPDATE_CHANNEL", old_toolchain_channel);
}

fn restore_var(key: &str, value: Option<String>) {
    if let Some(v) = value {
        std::env::set_var(key, v);
    } else {
        std::env::remove_var(key);
    }
}
