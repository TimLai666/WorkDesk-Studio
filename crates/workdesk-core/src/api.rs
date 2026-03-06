use crate::config::AppConfig;
use crate::errors::{ApiHttpError, CoreError};
use crate::repository::SqliteCoreRepository;
use crate::service::CoreService;
use crate::types::{
    ApiEnvelope, ApprovalInput, AuthLoginInput, AuthLogoutInput, AuthSwitchInput, CancelRunInput,
    CreateProposalInput, CreateWorkflowInput, FsMoveInput, FsQuery, FsReadResponse, FsTreeEntry,
    FsWriteInput, OfficeOpenInput, OfficeSaveInput, OfficeVersionResponse, RetryRunInput,
    RunEventsQuery, RunListQuery, RunWorkflowInput, UpdateWorkflowStatusInput, UpsertMemoryInput,
    UpsertSkillInput,
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
use std::path::{Component, PathBuf};
use std::sync::Arc;

#[derive(Clone)]
struct ApiState {
    service: CoreService,
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
        .route("/api/v1/workflows/{id}", get(get_workflow))
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
        .route("/api/v1/runs", get(list_runs))
        .route("/api/v1/runs/{run_id}", get(get_run))
        .route("/api/v1/runs/{run_id}/events", get(list_run_events))
        .route("/api/v1/runs/{run_id}/skills", get(list_run_skills))
        .route("/api/v1/runs/{run_id}/cancel", post(cancel_run))
        .route("/api/v1/runs/{run_id}/retry", post(retry_run))
        .route("/api/v1/fs/tree", get(fs_tree))
        .route("/api/v1/fs/file", get(fs_read).put(fs_write))
        .route("/api/v1/fs/move", post(fs_move))
        .route("/api/v1/fs/path", delete(fs_delete))
        .route("/api/v1/office/open", post(office_open))
        .route("/api/v1/office/save", post(office_save))
        .route("/api/v1/office/version", get(office_versions))
        .with_state(ApiState { service })
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
    if tokio::fs::try_exists(&target).await.unwrap_or(false) {
        let previous = tokio::fs::read(&target)
            .await
            .map_err(|e| ApiHttpError::from(CoreError::Internal(e.to_string())))?;
        let version_name = format!("{}_{}", input.path, Utc::now().timestamp());
        state
            .service
            .repo()
            .insert_office_version(&input.path, &version_name, &previous)
            .await
            .map_err(|e| ApiHttpError::from(CoreError::Internal(e.to_string())))?;
    }
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
