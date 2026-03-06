use std::sync::Arc;

use serde_json::json;
use tempfile::TempDir;
use workdesk_core::{
    AgentWorkspaceMessageRole, ChoicePromptAnswerInput, ChoicePromptOptionInput, CoreRepository,
    CoreService, CreateAgentWorkspaceSessionInput, CreateChoicePromptInput, CreateWorkflowInput,
    SqliteCoreRepository, UpdateAgentWorkspaceSessionConfigInput, WorkflowAgentDefaults,
    WorkflowNodeInput, WorkflowNodeKind,
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
async fn workbench_session_persists_native_config_messages_and_choice_prompt() {
    let tmp = TempDir::new().expect("tempdir");
    let (_repo, service, db_path, workspace_root) = setup_service(&tmp).await;

    let session = service
        .create_agent_workspace_session(CreateAgentWorkspaceSessionInput {
            title: "Release shell".into(),
            config: Some(workdesk_core::CodexNativeSessionConfig {
                model: Some("gpt-5.4".into()),
                model_reasoning_effort: Some("high".into()),
                speed: Some(true),
                plan_mode: true,
            }),
            last_active_panel: Some("runs".into()),
        })
        .await
        .expect("create session");

    service
        .append_agent_workspace_message(
            &session.session_id,
            AgentWorkspaceMessageRole::User,
            "Draft the rollout plan".into(),
        )
        .await
        .expect("append message");

    let prompt = service
        .create_choice_prompt(
            &session.session_id,
            CreateChoicePromptInput {
                question: "Which rollout path should we use?".into(),
                options: vec![
                    ChoicePromptOptionInput {
                        option_id: "safe".into(),
                        label: "Safe rollout".into(),
                        description: "Lower change risk".into(),
                    },
                    ChoicePromptOptionInput {
                        option_id: "fast".into(),
                        label: "Fast rollout".into(),
                        description: "Shorter delivery time".into(),
                    },
                ],
                recommended_option_id: Some("safe".into()),
                allow_freeform: true,
            },
        )
        .await
        .expect("create choice prompt");

    service
        .answer_choice_prompt(
            &session.session_id,
            &prompt.prompt_id,
            ChoicePromptAnswerInput {
                selected_option_id: Some("safe".into()),
                freeform_answer: None,
            },
        )
        .await
        .expect("answer choice prompt");

    drop(service);

    let repo = SqliteCoreRepository::connect(&db_path)
        .await
        .expect("reconnect sqlite");
    repo.migrate().await.expect("rerun migrations");
    let service = CoreService::new(Arc::new(repo), workspace_root);

    let sessions = service
        .list_agent_workspace_sessions()
        .await
        .expect("list sessions");
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].title, "Release shell");
    assert_eq!(sessions[0].config.model.as_deref(), Some("gpt-5.4"));
    assert_eq!(
        sessions[0].config.model_reasoning_effort.as_deref(),
        Some("high")
    );
    assert_eq!(sessions[0].config.speed, Some(true));
    assert!(sessions[0].config.plan_mode);
    assert_eq!(sessions[0].last_active_panel.as_deref(), Some("runs"));

    let messages = service
        .list_agent_workspace_messages(&sessions[0].session_id)
        .await
        .expect("list messages");
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].content, "Draft the rollout plan");

    let prompts = service
        .list_choice_prompts(&sessions[0].session_id)
        .await
        .expect("list prompts");
    assert_eq!(prompts.len(), 1);
    assert_eq!(prompts[0].recommended_option_id.as_deref(), Some("safe"));
    assert_eq!(prompts[0].selected_option_id.as_deref(), Some("safe"));
    assert!(prompts[0].answered_at.is_some());
}

#[tokio::test]
async fn workflow_persists_canvas_coordinates_and_agent_defaults_without_speed() {
    let tmp = TempDir::new().expect("tempdir");
    let (_repo, service, _db_path, _workspace_root) = setup_service(&tmp).await;

    let workflow = service
        .create_workflow(CreateWorkflowInput {
            name: "agent defaults".into(),
            timezone: "Asia/Taipei".into(),
            agent_defaults: Some(WorkflowAgentDefaults {
                model: Some("gpt-5.4".into()),
                model_reasoning_effort: Some("medium".into()),
            }),
            nodes: vec![WorkflowNodeInput {
                id: "agent-1".into(),
                kind: WorkflowNodeKind::AgentPrompt,
                x: Some(320.0),
                y: Some(180.0),
                config: Some(json!({
                    "instructions": "Summarize the run",
                    "language": "en"
                })),
            }],
            edges: vec![],
        })
        .await
        .expect("create workflow");

    let loaded = service
        .get_workflow(&workflow.id)
        .await
        .expect("get workflow");
    assert_eq!(loaded.agent_defaults, workflow.agent_defaults);
    assert_eq!(loaded.nodes.len(), 1);
    assert_eq!(loaded.nodes[0].x, Some(320.0));
    assert_eq!(loaded.nodes[0].y, Some(180.0));
    assert_eq!(
        loaded.nodes[0]
            .config
            .as_ref()
            .and_then(|cfg| cfg.get("language")),
        Some(&json!("en"))
    );
}

#[tokio::test]
async fn updating_session_config_uses_native_codex_fields() {
    let tmp = TempDir::new().expect("tempdir");
    let (_repo, service, _db_path, _workspace_root) = setup_service(&tmp).await;

    let session = service
        .create_agent_workspace_session(CreateAgentWorkspaceSessionInput {
            title: "Session".into(),
            config: None,
            last_active_panel: None,
        })
        .await
        .expect("create session");

    let updated = service
        .update_agent_workspace_session_config(
            &session.session_id,
            UpdateAgentWorkspaceSessionConfigInput {
                model: Some("gpt-5.4".into()),
                model_reasoning_effort: Some("xhigh".into()),
                speed: Some(false),
                plan_mode: Some(true),
                last_active_panel: Some("files".into()),
            },
        )
        .await
        .expect("update session config");

    assert_eq!(updated.config.model.as_deref(), Some("gpt-5.4"));
    assert_eq!(
        updated.config.model_reasoning_effort.as_deref(),
        Some("xhigh")
    );
    assert_eq!(updated.config.speed, Some(false));
    assert!(updated.config.plan_mode);
    assert_eq!(updated.last_active_panel.as_deref(), Some("files"));
}
