mod workbench;

use crate::config::AppConfig;
use crate::errors::{ApiHttpError, CoreError};
use crate::repository::SqliteCoreRepository;
use crate::service::CoreService;
use crate::types::{
    ApiEnvelope, ApprovalInput, AuthLoginInput, AuthLogoutInput, AuthSwitchInput, CancelRunInput,
    CreateProposalInput, CreateWorkflowInput, FsDiffInput, FsDiffLine, FsDiffResponse, FsMoveInput,
    FsQuery, FsReadResponse, FsSearchMatch, FsSearchQuery, FsTreeEntry, FsWriteInput,
    OfficeOpenInput, OfficeSaveInput, OfficeVersionResponse, OnlyOfficeCallbackInput,
    PatchWorkflowInput, PdfAnnotateInput, PdfOperationResponse, PdfPreviewInput,
    PdfReplaceTextInput, PdfSaveVersionInput, RetryRunInput, RunEventsQuery, RunListQuery,
    RunWorkflowInput, TerminalSessionResponse, TerminalStartInput, UpdateWorkflowStatusInput,
    UpsertMemoryInput, UpsertSkillInput,
};
use anyhow::{Context, Result};
use axum::extract::{Path, Query, State};
use axum::routing::{delete, get, patch, post};
use axum::{Json, Router};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use chrono::Utc;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Component, PathBuf};
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::RwLock;
use uuid::Uuid;
use workbench::{
    answer_choice_prompt, create_agent_session, create_choice_prompt, list_agent_capabilities,
    list_agent_sessions, list_choice_prompts, list_session_messages, post_session_message,
    update_agent_session_config,
};

#[derive(Clone)]
struct ApiState {
    service: CoreService,
    terminal_sessions: Arc<RwLock<HashMap<String, TerminalSessionResponse>>>,
}

pub fn build_router(service: CoreService) -> Router {
    Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/auth/login", post(auth_login))
        .route("/api/v1/auth/logout", post(auth_logout))
        .route("/api/v1/auth/switch", post(auth_switch))
        .route(
            "/api/v1/workflows",
            get(list_workflows).post(create_workflow),
        )
        .route(
            "/api/v1/workflows/{id}",
            get(get_workflow).patch(patch_workflow),
        )
        .route(
            "/api/v1/workflows/{id}/status",
            patch(update_workflow_status),
        )
        .route("/api/v1/workflows/{id}/run", post(run_workflow))
        .route("/api/v1/workflows/{id}/proposals", post(create_proposal))
        .route(
            "/api/v1/workflows/{id}/proposals/{proposal_id}/approve",
            post(approve_proposal),
        )
        .route("/api/v1/skills", get(list_skills).post(upsert_skill))
        .route("/api/v1/skills/export", get(export_skills))
        .route("/api/v1/skills/import", post(import_skills))
        .route("/api/v1/memory", get(list_memory).post(upsert_memory))
        .route("/api/v1/memory/export", get(export_memory))
        .route("/api/v1/memory/import", post(import_memory))
        .route("/api/v1/agent/capabilities", get(list_agent_capabilities))
        .route(
            "/api/v1/agent/sessions",
            get(list_agent_sessions).post(create_agent_session),
        )
        .route(
            "/api/v1/agent/sessions/{session_id}/config",
            patch(update_agent_session_config),
        )
        .route(
            "/api/v1/agent/sessions/{session_id}/messages",
            get(list_session_messages).post(post_session_message),
        )
        .route(
            "/api/v1/agent/sessions/{session_id}/choice-prompts",
            get(list_choice_prompts).post(create_choice_prompt),
        )
        .route(
            "/api/v1/agent/sessions/{session_id}/choice-prompts/{prompt_id}/answer",
            post(answer_choice_prompt),
        )
        .route("/api/v1/runs", get(list_runs))
        .route("/api/v1/runs/{run_id}", get(get_run))
        .route("/api/v1/runs/{run_id}/events", get(list_run_events))
        .route("/api/v1/runs/{run_id}/nodes", get(list_run_nodes))
        .route("/api/v1/runs/{run_id}/skills", get(list_run_skills))
        .route("/api/v1/runs/{run_id}/cancel", post(cancel_run))
        .route("/api/v1/runs/{run_id}/retry", post(retry_run))
        .route("/api/v1/fs/tree", get(fs_tree))
        .route("/api/v1/fs/search", get(fs_search))
        .route("/api/v1/fs/file", get(fs_read).put(fs_write))
        .route("/api/v1/fs/move", post(fs_move))
        .route("/api/v1/fs/diff", post(fs_diff))
        .route("/api/v1/fs/terminal/start", post(fs_terminal_start))
        .route(
            "/api/v1/fs/terminal/session/{session_id}",
            get(fs_terminal_session),
        )
        .route("/api/v1/fs/path", delete(fs_delete))
        .route("/api/v1/office/open", post(office_open))
        .route("/api/v1/office/save", post(office_save))
        .route("/api/v1/office/version", get(office_versions))
        .route(
            "/api/v1/office/onlyoffice/callback",
            post(onlyoffice_callback),
        )
        .route("/api/v1/office/pdf/preview", post(pdf_preview))
        .route("/api/v1/office/pdf/annotate", post(pdf_annotate))
        .route("/api/v1/office/pdf/replace", post(pdf_replace_text))
        .route("/api/v1/office/pdf/save-version", post(pdf_save_version))
        .with_state(ApiState {
            service,
            terminal_sessions: Arc::new(RwLock::new(HashMap::new())),
        })
}

pub async fn run_server_with_config(config: AppConfig) -> Result<()> {
    let repo = SqliteCoreRepository::connect(&config.db_path).await?;
    crate::repository::CoreRepository::migrate(&repo).await?;
    let service = CoreService::new(Arc::new(repo), config.workspace_root.clone());
    let router = build_router(service);
    let listener = tokio::net::TcpListener::bind(config.core_bind)
        .await
        .context("bind core server listener")?;
    axum::serve(listener, router).await.context("serve axum")?;
    Ok(())
}

pub async fn run_server(bind: std::net::SocketAddr, workspace_root: PathBuf) -> Result<()> {
    let mut config = AppConfig::from_env()?;
    config.core_bind = bind;
    config.workspace_root = workspace_root;
    run_server_with_config(config).await
}

fn ok<T: Serialize>(data: T) -> Json<ApiEnvelope<T>> {
    Json(ApiEnvelope::success(data))
}

async fn health() -> Json<ApiEnvelope<Value>> {
    ok(json!({"status":"ok"}))
}

async fn auth_login(
    State(state): State<ApiState>,
    Json(input): Json<AuthLoginInput>,
) -> Result<Json<ApiEnvelope<crate::types::AuthSessionResponse>>, ApiHttpError> {
    let session = state
        .service
        .login(input)
        .await
        .map_err(ApiHttpError::from)?;
    Ok(ok(session))
}

async fn auth_logout(
    State(state): State<ApiState>,
    Json(input): Json<AuthLogoutInput>,
) -> Result<Json<ApiEnvelope<Value>>, ApiHttpError> {
    state
        .service
        .logout(&input.account_id)
        .await
        .map_err(ApiHttpError::from)?;
    Ok(ok(json!({"ok": true})))
}

async fn auth_switch(
    State(state): State<ApiState>,
    Json(input): Json<AuthSwitchInput>,
) -> Result<Json<ApiEnvelope<crate::types::AuthSessionResponse>>, ApiHttpError> {
    let session = state
        .service
        .switch_account(input)
        .await
        .map_err(ApiHttpError::from)?;
    Ok(ok(session))
}

async fn create_workflow(
    State(state): State<ApiState>,
    Json(input): Json<CreateWorkflowInput>,
) -> Result<Json<ApiEnvelope<crate::types::WorkflowDefinition>>, ApiHttpError> {
    let workflow = state
        .service
        .create_workflow(input)
        .await
        .map_err(ApiHttpError::from)?;
    Ok(ok(workflow))
}

async fn list_workflows(
    State(state): State<ApiState>,
) -> Result<Json<ApiEnvelope<Vec<crate::types::WorkflowDefinition>>>, ApiHttpError> {
    let workflows = state
        .service
        .list_workflows()
        .await
        .map_err(ApiHttpError::from)?;
    Ok(ok(workflows))
}

async fn get_workflow(
    Path(workflow_id): Path<String>,
    State(state): State<ApiState>,
) -> Result<Json<ApiEnvelope<crate::types::WorkflowDefinition>>, ApiHttpError> {
    let workflow = state
        .service
        .get_workflow(&workflow_id)
        .await
        .map_err(ApiHttpError::from)?;
    Ok(ok(workflow))
}

async fn patch_workflow(
    Path(workflow_id): Path<String>,
    State(state): State<ApiState>,
    Json(input): Json<PatchWorkflowInput>,
) -> Result<Json<ApiEnvelope<crate::types::WorkflowDefinition>>, ApiHttpError> {
    let workflow = state
        .service
        .update_workflow_definition(&workflow_id, input)
        .await
        .map_err(ApiHttpError::from)?;
    Ok(ok(workflow))
}

async fn update_workflow_status(
    Path(workflow_id): Path<String>,
    State(state): State<ApiState>,
    Json(input): Json<UpdateWorkflowStatusInput>,
) -> Result<Json<ApiEnvelope<crate::types::WorkflowDefinition>>, ApiHttpError> {
    let workflow = state
        .service
        .update_workflow_status(&workflow_id, input.status)
        .await
        .map_err(ApiHttpError::from)?;
    Ok(ok(workflow))
}

async fn run_workflow(
    Path(workflow_id): Path<String>,
    State(state): State<ApiState>,
    Json(input): Json<RunWorkflowInput>,
) -> Result<Json<ApiEnvelope<crate::types::WorkflowRun>>, ApiHttpError> {
    let run = state
        .service
        .queue_workflow_run(&workflow_id, input.requested_by.as_deref())
        .await
        .map_err(ApiHttpError::from)?;
    Ok(ok(run))
}

async fn create_proposal(
    Path(workflow_id): Path<String>,
    State(state): State<ApiState>,
    Json(input): Json<CreateProposalInput>,
) -> Result<Json<ApiEnvelope<crate::types::WorkflowChangeProposal>>, ApiHttpError> {
    let proposal = state
        .service
        .propose_workflow_change(workflow_id, input)
        .await
        .map_err(ApiHttpError::from)?;
    Ok(ok(proposal))
}

async fn approve_proposal(
    Path((_workflow_id, proposal_id)): Path<(String, String)>,
    State(state): State<ApiState>,
    Json(input): Json<ApprovalInput>,
) -> Result<Json<ApiEnvelope<crate::types::WorkflowChangeProposal>>, ApiHttpError> {
    let proposal = state
        .service
        .approve_proposal(&proposal_id, input.approved_by)
        .await
        .map_err(ApiHttpError::from)?;
    Ok(ok(proposal))
}

async fn upsert_skill(
    State(state): State<ApiState>,
    Json(input): Json<UpsertSkillInput>,
) -> Result<Json<ApiEnvelope<crate::types::SkillRecord>>, ApiHttpError> {
    let skill = state
        .service
        .upsert_skill(input)
        .await
        .map_err(ApiHttpError::from)?;
    Ok(ok(skill))
}

async fn list_skills(
    State(state): State<ApiState>,
) -> Result<Json<ApiEnvelope<Vec<crate::types::SkillRecord>>>, ApiHttpError> {
    let skills = state
        .service
        .list_skills()
        .await
        .map_err(ApiHttpError::from)?;
    Ok(ok(skills))
}

async fn export_skills(
    State(state): State<ApiState>,
) -> Result<Json<ApiEnvelope<Vec<crate::types::SkillRecord>>>, ApiHttpError> {
    list_skills(State(state)).await
}

async fn import_skills(
    State(state): State<ApiState>,
    Json(inputs): Json<Vec<UpsertSkillInput>>,
) -> Result<Json<ApiEnvelope<Vec<crate::types::SkillRecord>>>, ApiHttpError> {
    let mut out = Vec::with_capacity(inputs.len());
    for input in inputs {
        out.push(
            state
                .service
                .upsert_skill(input)
                .await
                .map_err(ApiHttpError::from)?,
        );
    }
    Ok(ok(out))
}

async fn upsert_memory(
    State(state): State<ApiState>,
    Json(input): Json<UpsertMemoryInput>,
) -> Result<Json<ApiEnvelope<crate::types::MemoryRecord>>, ApiHttpError> {
    let memory = state
        .service
        .upsert_memory(input)
        .await
        .map_err(ApiHttpError::from)?;
    Ok(ok(memory))
}

async fn list_memory(
    State(state): State<ApiState>,
) -> Result<Json<ApiEnvelope<Vec<crate::types::MemoryRecord>>>, ApiHttpError> {
    let memory = state
        .service
        .list_memory()
        .await
        .map_err(ApiHttpError::from)?;
    Ok(ok(memory))
}

async fn export_memory(
    State(state): State<ApiState>,
) -> Result<Json<ApiEnvelope<Vec<crate::types::MemoryRecord>>>, ApiHttpError> {
    list_memory(State(state)).await
}

async fn import_memory(
    State(state): State<ApiState>,
    Json(inputs): Json<Vec<UpsertMemoryInput>>,
) -> Result<Json<ApiEnvelope<Vec<crate::types::MemoryRecord>>>, ApiHttpError> {
    let mut out = Vec::with_capacity(inputs.len());
    for input in inputs {
        out.push(
            state
                .service
                .upsert_memory(input)
                .await
                .map_err(ApiHttpError::from)?,
        );
    }
    Ok(ok(out))
}

async fn fs_tree(
    State(state): State<ApiState>,
    Query(query): Query<FsQuery>,
) -> Result<Json<ApiEnvelope<Vec<FsTreeEntry>>>, ApiHttpError> {
    let root = resolve_workspace_path(state.service.workspace_root(), &query.path)?;
    let entries = list_directory_tree(state.service.workspace_root(), &root)
        .await
        .map_err(|e| ApiHttpError::from(CoreError::Internal(e.to_string())))?;
    Ok(ok(entries))
}

async fn fs_search(
    State(state): State<ApiState>,
    Query(query): Query<FsSearchQuery>,
) -> Result<Json<ApiEnvelope<Vec<FsSearchMatch>>>, ApiHttpError> {
    if query.query.trim().is_empty() {
        return Err(ApiHttpError::from(CoreError::BadRequest(
            "query must not be empty".into(),
        )));
    }
    let root = resolve_workspace_path(state.service.workspace_root(), &query.path)?;
    let limit = query.limit.unwrap_or(200).clamp(1, 2000);
    let matches = search_workspace_text(
        state.service.workspace_root().clone(),
        root,
        query.query,
        limit,
    )
    .await
    .map_err(|e| ApiHttpError::from(CoreError::Internal(e.to_string())))?;
    Ok(ok(matches))
}

async fn fs_read(
    State(state): State<ApiState>,
    Query(query): Query<FsQuery>,
) -> Result<Json<ApiEnvelope<FsReadResponse>>, ApiHttpError> {
    let target = resolve_workspace_path(state.service.workspace_root(), &query.path)?;
    let bytes = tokio::fs::read(&target)
        .await
        .map_err(|e| ApiHttpError::from(CoreError::Internal(e.to_string())))?;
    Ok(ok(FsReadResponse {
        path: query.path,
        content_base64: STANDARD.encode(bytes),
    }))
}

async fn fs_write(
    State(state): State<ApiState>,
    Json(input): Json<FsWriteInput>,
) -> Result<Json<ApiEnvelope<Value>>, ApiHttpError> {
    let target = resolve_workspace_path(state.service.workspace_root(), &input.path)?;
    if let Some(parent) = target.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| ApiHttpError::from(CoreError::Internal(e.to_string())))?;
    }
    let bytes = STANDARD
        .decode(input.content_base64.as_bytes())
        .map_err(|err| {
            ApiHttpError::from(CoreError::BadRequest(format!(
                "invalid base64 payload: {err}"
            )))
        })?;
    tokio::fs::write(target, bytes)
        .await
        .map_err(|e| ApiHttpError::from(CoreError::Internal(e.to_string())))?;
    Ok(ok(json!({"ok": true})))
}

async fn fs_move(
    State(state): State<ApiState>,
    Json(input): Json<FsMoveInput>,
) -> Result<Json<ApiEnvelope<Value>>, ApiHttpError> {
    let from = resolve_workspace_path(state.service.workspace_root(), &input.from)?;
    let to = resolve_workspace_path(state.service.workspace_root(), &input.to)?;
    if let Some(parent) = to.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| ApiHttpError::from(CoreError::Internal(e.to_string())))?;
    }
    tokio::fs::rename(from, to)
        .await
        .map_err(|e| ApiHttpError::from(CoreError::Internal(e.to_string())))?;
    Ok(ok(json!({"ok": true})))
}

async fn fs_diff(
    State(state): State<ApiState>,
    Json(input): Json<FsDiffInput>,
) -> Result<Json<ApiEnvelope<FsDiffResponse>>, ApiHttpError> {
    let left = resolve_workspace_path(state.service.workspace_root(), &input.left_path)?;
    let right = resolve_workspace_path(state.service.workspace_root(), &input.right_path)?;
    let left_text = tokio::fs::read_to_string(&left)
        .await
        .map_err(|e| ApiHttpError::from(CoreError::Internal(e.to_string())))?;
    let right_text = tokio::fs::read_to_string(&right)
        .await
        .map_err(|e| ApiHttpError::from(CoreError::Internal(e.to_string())))?;

    Ok(ok(FsDiffResponse {
        left_path: input.left_path,
        right_path: input.right_path,
        hunks: compute_line_diff(&left_text, &right_text),
    }))
}

async fn fs_terminal_start(
    State(state): State<ApiState>,
    Json(input): Json<TerminalStartInput>,
) -> Result<Json<ApiEnvelope<TerminalSessionResponse>>, ApiHttpError> {
    let target = resolve_workspace_path(state.service.workspace_root(), &input.path)?;
    let metadata = tokio::fs::metadata(&target)
        .await
        .map_err(|e| ApiHttpError::from(CoreError::Internal(e.to_string())))?;
    if !metadata.is_dir() {
        return Err(ApiHttpError::from(CoreError::BadRequest(format!(
            "terminal path must be directory: {}",
            input.path
        ))));
    }

    let output = execute_terminal_command(&target, &input.command)
        .await
        .map_err(|e| ApiHttpError::from(CoreError::Internal(e.to_string())))?;

    let session = TerminalSessionResponse {
        session_id: Uuid::new_v4().to_string(),
        status: "exited".to_string(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_code: output.status.code(),
    };
    state
        .terminal_sessions
        .write()
        .await
        .insert(session.session_id.clone(), session.clone());
    Ok(ok(session))
}

async fn fs_terminal_session(
    Path(session_id): Path<String>,
    State(state): State<ApiState>,
) -> Result<Json<ApiEnvelope<TerminalSessionResponse>>, ApiHttpError> {
    let sessions = state.terminal_sessions.read().await;
    let session = sessions.get(&session_id).cloned().ok_or_else(|| {
        ApiHttpError::from(CoreError::BadRequest("terminal session not found".into()))
    })?;
    Ok(ok(session))
}

async fn fs_delete(
    State(state): State<ApiState>,
    Query(query): Query<FsQuery>,
) -> Result<Json<ApiEnvelope<Value>>, ApiHttpError> {
    let target = resolve_workspace_path(state.service.workspace_root(), &query.path)?;
    let metadata = tokio::fs::metadata(&target)
        .await
        .map_err(|e| ApiHttpError::from(CoreError::Internal(e.to_string())))?;
    if metadata.is_dir() {
        tokio::fs::remove_dir_all(target)
            .await
            .map_err(|e| ApiHttpError::from(CoreError::Internal(e.to_string())))?;
    } else {
        tokio::fs::remove_file(target)
            .await
            .map_err(|e| ApiHttpError::from(CoreError::Internal(e.to_string())))?;
    }
    Ok(ok(json!({"ok": true})))
}

async fn office_open(
    State(state): State<ApiState>,
    Json(input): Json<OfficeOpenInput>,
) -> Result<Json<ApiEnvelope<FsReadResponse>>, ApiHttpError> {
    fs_read(State(state), Query(FsQuery { path: input.path })).await
}

async fn office_save(
    State(state): State<ApiState>,
    Json(input): Json<OfficeSaveInput>,
) -> Result<Json<ApiEnvelope<Value>>, ApiHttpError> {
    let target = resolve_workspace_path(state.service.workspace_root(), &input.path)?;
    save_previous_version_if_exists(&state, &input.path, &target).await?;
    fs_write(
        State(state),
        Json(FsWriteInput {
            path: input.path,
            content_base64: input.content_base64,
        }),
    )
    .await
}

async fn office_versions(
    State(state): State<ApiState>,
    Query(query): Query<FsQuery>,
) -> Result<Json<ApiEnvelope<OfficeVersionResponse>>, ApiHttpError> {
    let versions = state
        .service
        .repo()
        .list_office_versions(&query.path)
        .await
        .map_err(|e| ApiHttpError::from(CoreError::Internal(e.to_string())))?;
    Ok(ok(OfficeVersionResponse {
        path: query.path,
        versions,
    }))
}

async fn onlyoffice_callback(
    State(state): State<ApiState>,
    Json(input): Json<OnlyOfficeCallbackInput>,
) -> Result<Json<ApiEnvelope<Value>>, ApiHttpError> {
    let path = input
        .payload
        .get("path")
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let content_base64 = input
        .payload
        .get("content_base64")
        .and_then(Value::as_str)
        .map(ToString::to_string);

    if let (Some(path), Some(content_base64)) = (path, content_base64) {
        let target = resolve_workspace_path(state.service.workspace_root(), &path)?;
        save_previous_version_if_exists(&state, &path, &target).await?;
        let _ = fs_write(
            State(state),
            Json(FsWriteInput {
                path: path.clone(),
                content_base64,
            }),
        )
        .await?;
        return Ok(ok(json!({
            "accepted": true,
            "saved": true,
            "path": path
        })));
    }

    Ok(ok(json!({
        "accepted": true,
        "echo": input.payload
    })))
}

async fn pdf_preview(
    State(state): State<ApiState>,
    Json(input): Json<PdfPreviewInput>,
) -> Result<Json<ApiEnvelope<FsReadResponse>>, ApiHttpError> {
    fs_read(State(state), Query(FsQuery { path: input.path })).await
}

async fn pdf_annotate(
    State(state): State<ApiState>,
    Json(input): Json<PdfAnnotateInput>,
) -> Result<Json<ApiEnvelope<PdfOperationResponse>>, ApiHttpError> {
    let target = resolve_workspace_path(state.service.workspace_root(), &input.path)?;
    save_previous_version_if_exists(&state, &input.path, &target).await?;

    let annotation_target = target.with_extension("pdf.annotations.txt");
    let line = format!(
        "{} | {}\n",
        Utc::now().to_rfc3339(),
        input.annotation.replace('\n', " ")
    );
    if let Some(parent) = annotation_target.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| ApiHttpError::from(CoreError::Internal(e.to_string())))?;
    }
    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&annotation_target)
        .await
        .map_err(|e| ApiHttpError::from(CoreError::Internal(e.to_string())))?;
    use tokio::io::AsyncWriteExt;
    file.write_all(line.as_bytes())
        .await
        .map_err(|e| ApiHttpError::from(CoreError::Internal(e.to_string())))?;
    file.flush()
        .await
        .map_err(|e| ApiHttpError::from(CoreError::Internal(e.to_string())))?;

    let version_name = format!("{}_annotate_{}", input.path, Utc::now().timestamp());
    Ok(ok(PdfOperationResponse {
        path: input.path,
        replaced_count: 0,
        version_name,
    }))
}

async fn pdf_replace_text(
    State(state): State<ApiState>,
    Json(input): Json<PdfReplaceTextInput>,
) -> Result<Json<ApiEnvelope<PdfOperationResponse>>, ApiHttpError> {
    if input.search.is_empty() {
        return Err(ApiHttpError::from(CoreError::BadRequest(
            "search must not be empty".into(),
        )));
    }

    let target = resolve_workspace_path(state.service.workspace_root(), &input.path)?;
    save_previous_version_if_exists(&state, &input.path, &target).await?;

    let bytes = tokio::fs::read(&target)
        .await
        .map_err(|e| ApiHttpError::from(CoreError::Internal(e.to_string())))?;
    let content = String::from_utf8_lossy(&bytes);
    let replaced_count = content.matches(&input.search).count();
    let replaced = content.replace(&input.search, &input.replace);
    tokio::fs::write(&target, replaced.as_bytes())
        .await
        .map_err(|e| ApiHttpError::from(CoreError::Internal(e.to_string())))?;

    let version_name = format!("{}_replace_{}", input.path, Utc::now().timestamp());
    Ok(ok(PdfOperationResponse {
        path: input.path,
        replaced_count,
        version_name,
    }))
}

async fn pdf_save_version(
    State(state): State<ApiState>,
    Json(input): Json<PdfSaveVersionInput>,
) -> Result<Json<ApiEnvelope<PdfOperationResponse>>, ApiHttpError> {
    let target = resolve_workspace_path(state.service.workspace_root(), &input.path)?;
    let version_name = save_previous_version_if_exists(&state, &input.path, &target).await?;
    Ok(ok(PdfOperationResponse {
        path: input.path,
        replaced_count: 0,
        version_name,
    }))
}

async fn list_runs(
    State(state): State<ApiState>,
    Query(query): Query<RunListQuery>,
) -> Result<Json<ApiEnvelope<Vec<crate::types::WorkflowRun>>>, ApiHttpError> {
    let runs = state
        .service
        .list_runs(query.limit.unwrap_or(50).clamp(1, 500))
        .await
        .map_err(ApiHttpError::from)?;
    Ok(ok(runs))
}

async fn get_run(
    Path(run_id): Path<String>,
    State(state): State<ApiState>,
) -> Result<Json<ApiEnvelope<crate::types::WorkflowRun>>, ApiHttpError> {
    let run = state
        .service
        .get_run(&run_id)
        .await
        .map_err(ApiHttpError::from)?;
    Ok(ok(run))
}

async fn list_run_events(
    Path(run_id): Path<String>,
    State(state): State<ApiState>,
    Query(query): Query<RunEventsQuery>,
) -> Result<Json<ApiEnvelope<Vec<crate::types::WorkflowRunEvent>>>, ApiHttpError> {
    let events = state
        .service
        .list_run_events(
            &run_id,
            query.after_seq.unwrap_or(0),
            query.limit.unwrap_or(200).clamp(1, 2000),
        )
        .await
        .map_err(ApiHttpError::from)?;
    Ok(ok(events))
}

async fn list_run_nodes(
    Path(run_id): Path<String>,
    State(state): State<ApiState>,
) -> Result<Json<ApiEnvelope<Vec<crate::types::WorkflowRunNodeState>>>, ApiHttpError> {
    let nodes = state
        .service
        .list_run_nodes(&run_id)
        .await
        .map_err(ApiHttpError::from)?;
    Ok(ok(nodes))
}

async fn list_run_skills(
    Path(run_id): Path<String>,
    State(state): State<ApiState>,
) -> Result<Json<ApiEnvelope<Vec<crate::types::RunSkillSnapshot>>>, ApiHttpError> {
    let skills = state
        .service
        .list_run_skills(&run_id)
        .await
        .map_err(ApiHttpError::from)?;
    Ok(ok(skills))
}

async fn cancel_run(
    Path(run_id): Path<String>,
    State(state): State<ApiState>,
    Json(_input): Json<CancelRunInput>,
) -> Result<Json<ApiEnvelope<crate::types::WorkflowRun>>, ApiHttpError> {
    let run = state
        .service
        .cancel_run(&run_id)
        .await
        .map_err(ApiHttpError::from)?;
    Ok(ok(run))
}

async fn retry_run(
    Path(run_id): Path<String>,
    State(state): State<ApiState>,
    Json(input): Json<RetryRunInput>,
) -> Result<Json<ApiEnvelope<crate::types::WorkflowRun>>, ApiHttpError> {
    let run = state
        .service
        .retry_run(&run_id, input.requested_by.as_deref())
        .await
        .map_err(ApiHttpError::from)?;
    Ok(ok(run))
}

async fn save_previous_version_if_exists(
    state: &ApiState,
    logical_path: &str,
    target_path: &PathBuf,
) -> Result<String, ApiHttpError> {
    let version_name = format!("{}_{}", logical_path, Utc::now().timestamp());
    if tokio::fs::try_exists(target_path)
        .await
        .map_err(|e| ApiHttpError::from(CoreError::Internal(e.to_string())))?
    {
        let previous = tokio::fs::read(target_path)
            .await
            .map_err(|e| ApiHttpError::from(CoreError::Internal(e.to_string())))?;
        state
            .service
            .repo()
            .insert_office_version(logical_path, &version_name, &previous)
            .await
            .map_err(|e| ApiHttpError::from(CoreError::Internal(e.to_string())))?;
    }
    Ok(version_name)
}

fn resolve_workspace_path(
    workspace_root: &PathBuf,
    relative: &str,
) -> Result<PathBuf, ApiHttpError> {
    let relative_path = PathBuf::from(relative);
    if relative_path.is_absolute() {
        return Err(ApiHttpError::from(CoreError::PathTraversal));
    }
    for component in relative_path.components() {
        if matches!(component, Component::ParentDir) {
            return Err(ApiHttpError::from(CoreError::PathTraversal));
        }
    }
    Ok(workspace_root.join(relative_path))
}

async fn list_directory_tree(workspace_root: &PathBuf, root: &PathBuf) -> Result<Vec<FsTreeEntry>> {
    let mut entries = vec![];
    let mut stack = vec![root.clone()];
    while let Some(path) = stack.pop() {
        let metadata = tokio::fs::metadata(&path).await?;
        let relative = path
            .strip_prefix(workspace_root)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();
        entries.push(FsTreeEntry {
            path: relative,
            is_dir: metadata.is_dir(),
        });
        if metadata.is_dir() {
            let mut dir = tokio::fs::read_dir(path).await?;
            while let Some(entry) = dir.next_entry().await? {
                stack.push(entry.path());
            }
        }
    }
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(entries)
}

async fn search_workspace_text(
    workspace_root: PathBuf,
    root: PathBuf,
    query: String,
    limit: usize,
) -> Result<Vec<FsSearchMatch>> {
    tokio::task::spawn_blocking(move || {
        let mut matches = Vec::new();
        let mut stack = vec![root];
        while let Some(path) = stack.pop() {
            let metadata = std::fs::metadata(&path)?;
            if metadata.is_dir() {
                for entry in std::fs::read_dir(path)? {
                    stack.push(entry?.path());
                }
                continue;
            }

            let content = match std::fs::read_to_string(&path) {
                Ok(content) => content,
                Err(_) => continue,
            };
            for (index, line) in content.lines().enumerate() {
                if !line.contains(&query) {
                    continue;
                }
                let relative = path
                    .strip_prefix(&workspace_root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");
                matches.push(FsSearchMatch {
                    path: relative,
                    line: index + 1,
                    preview: line.to_string(),
                });
                if matches.len() >= limit {
                    return Ok(matches);
                }
            }
        }
        Ok(matches)
    })
    .await?
}

fn compute_line_diff(left: &str, right: &str) -> Vec<FsDiffLine> {
    let left_lines: Vec<&str> = left.lines().collect();
    let right_lines: Vec<&str> = right.lines().collect();
    let mut hunks = Vec::new();
    let mut i = 0usize;
    let mut j = 0usize;

    while i < left_lines.len() || j < right_lines.len() {
        match (left_lines.get(i), right_lines.get(j)) {
            (Some(&l), Some(&r)) if l == r => {
                i += 1;
                j += 1;
            }
            (Some(&l), Some(&r)) => {
                hunks.push(FsDiffLine {
                    kind: "delete".to_string(),
                    left_line: Some(i + 1),
                    right_line: None,
                    text: l.to_string(),
                });
                hunks.push(FsDiffLine {
                    kind: "insert".to_string(),
                    left_line: None,
                    right_line: Some(j + 1),
                    text: r.to_string(),
                });
                i += 1;
                j += 1;
            }
            (Some(&l), None) => {
                hunks.push(FsDiffLine {
                    kind: "delete".to_string(),
                    left_line: Some(i + 1),
                    right_line: None,
                    text: l.to_string(),
                });
                i += 1;
            }
            (None, Some(&r)) => {
                hunks.push(FsDiffLine {
                    kind: "insert".to_string(),
                    left_line: None,
                    right_line: Some(j + 1),
                    text: r.to_string(),
                });
                j += 1;
            }
            (None, None) => break,
        }
    }
    hunks
}

async fn execute_terminal_command(dir: &PathBuf, command: &str) -> Result<std::process::Output> {
    #[cfg(windows)]
    let mut process = {
        let mut cmd = Command::new("cmd");
        cmd.arg("/C").arg(command);
        cmd
    };

    #[cfg(not(windows))]
    let mut process = {
        let mut cmd = Command::new("sh");
        cmd.arg("-lc").arg(command);
        cmd
    };

    process.current_dir(dir);
    Ok(process.output().await?)
}
