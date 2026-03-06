use anyhow::{anyhow, Context, Result};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, patch, post};
use axum::{Json, Router};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::{Component, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
pub use workdesk_domain::{
    ApprovalState, DomainError, MemoryRecord, Scope, SkillRecord, WorkflowChangeProposal,
    WorkflowDefinition, WorkflowEdge, WorkflowNode, WorkflowNodeKind, WorkflowStatus,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowNodeInput {
    pub id: String,
    pub kind: WorkflowNodeKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkflowInput {
    pub name: String,
    pub timezone: String,
    pub nodes: Vec<WorkflowNodeInput>,
    pub edges: Vec<WorkflowEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateWorkflowStatusInput {
    pub status: WorkflowStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProposalInput {
    pub diff: String,
    pub created_by_agent: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalInput {
    pub approved_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthLoginInput {
    pub account_id: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthLogoutInput {
    pub account_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSwitchInput {
    pub from_account: String,
    pub to_account: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSessionResponse {
    pub session_id: String,
    pub account_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertSkillInput {
    pub scope: Scope,
    pub name: String,
    pub manifest: String,
    pub content_path: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertMemoryInput {
    pub scope: Scope,
    pub namespace: String,
    pub key: String,
    pub value: String,
    pub embedding_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsWriteInput {
    pub path: String,
    pub content_base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsMoveInput {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsQuery {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsTreeEntry {
    pub path: String,
    pub is_dir: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsReadResponse {
    pub path: String,
    pub content_base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfficeOpenInput {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfficeSaveInput {
    pub path: String,
    pub content_base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfficeVersionResponse {
    pub path: String,
    pub versions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunWorkflowResponse {
    pub run_id: String,
    pub workflow_id: String,
    pub status: String,
}

#[derive(Default)]
struct InMemoryState {
    workflows: HashMap<String, WorkflowDefinition>,
    proposals: HashMap<String, WorkflowChangeProposal>,
    sessions: HashMap<String, String>,
    skills: HashMap<String, SkillRecord>,
    memories: HashMap<String, MemoryRecord>,
}

#[derive(Clone)]
pub struct InMemoryCoreService {
    state: Arc<RwLock<InMemoryState>>,
}

impl Default for InMemoryCoreService {
    fn default() -> Self {
        Self {
            state: Arc::new(RwLock::new(InMemoryState::default())),
        }
    }
}

impl InMemoryCoreService {
    pub async fn create_workflow(&self, input: CreateWorkflowInput) -> Result<WorkflowDefinition> {
        let workflow = WorkflowDefinition {
            id: Uuid::new_v4().to_string(),
            name: input.name,
            timezone: input.timezone,
            nodes: input
                .nodes
                .into_iter()
                .map(|node| WorkflowNode {
                    id: node.id,
                    kind: node.kind,
                })
                .collect(),
            edges: input.edges,
            version: 1,
            status: WorkflowStatus::Draft,
        };
        workflow.validate()?;

        let mut state = self.state.write().await;
        state
            .workflows
            .insert(workflow.id.clone(), workflow.clone());
        Ok(workflow)
    }

    pub async fn list_workflows(&self) -> Vec<WorkflowDefinition> {
        let state = self.state.read().await;
        state.workflows.values().cloned().collect()
    }

    pub async fn get_workflow(&self, workflow_id: &str) -> Option<WorkflowDefinition> {
        let state = self.state.read().await;
        state.workflows.get(workflow_id).cloned()
    }

    pub async fn update_workflow_status(
        &self,
        workflow_id: &str,
        status: WorkflowStatus,
    ) -> Result<WorkflowDefinition> {
        let mut state = self.state.write().await;
        let workflow = state
            .workflows
            .get_mut(workflow_id)
            .ok_or_else(|| anyhow!("workflow not found"))?;
        workflow.status = status;
        workflow.version += 1;
        Ok(workflow.clone())
    }

    pub async fn propose_workflow_change(
        &self,
        workflow_id: String,
        diff: String,
        created_by_agent: String,
    ) -> Result<WorkflowChangeProposal> {
        let mut state = self.state.write().await;
        if !state.workflows.contains_key(&workflow_id) {
            return Err(anyhow!("workflow not found"));
        }
        let proposal = WorkflowChangeProposal::new(workflow_id, diff, created_by_agent);
        state
            .proposals
            .insert(proposal.proposal_id.clone(), proposal.clone());
        Ok(proposal)
    }

    pub async fn approve_workflow_change_with_state(
        &self,
        mut proposal: WorkflowChangeProposal,
    ) -> Result<WorkflowChangeProposal> {
        if proposal.approval_state != ApprovalState::Pending {
            return Err(anyhow!("proposal must be pending"));
        }
        proposal.approve("system".into())?;
        proposal.approval_state = ApprovalState::Applied;
        let mut state = self.state.write().await;
        state
            .proposals
            .insert(proposal.proposal_id.clone(), proposal.clone());
        Ok(proposal)
    }

    pub async fn approve_workflow_change(
        &self,
        proposal_id: &str,
        approved_by: String,
    ) -> Result<WorkflowChangeProposal> {
        let mut state = self.state.write().await;
        let proposal = state
            .proposals
            .get_mut(proposal_id)
            .ok_or_else(|| anyhow!("proposal not found"))?;
        proposal.approve(approved_by)?;
        proposal.approval_state = ApprovalState::Applied;
        Ok(proposal.clone())
    }

    pub async fn login(&self, account_id: String) -> AuthSessionResponse {
        let session_id = Uuid::new_v4().to_string();
        let mut state = self.state.write().await;
        state
            .sessions
            .insert(account_id.clone(), session_id.clone());
        AuthSessionResponse {
            session_id,
            account_id,
        }
    }

    pub async fn logout(&self, account_id: &str) {
        let mut state = self.state.write().await;
        state.sessions.remove(account_id);
    }

    pub async fn switch_account(
        &self,
        from_account: &str,
        to_account: &str,
    ) -> AuthSessionResponse {
        self.logout(from_account).await;
        self.login(to_account.to_string()).await
    }

    pub async fn upsert_skill(&self, input: UpsertSkillInput) -> SkillRecord {
        let key = format!("{:?}:{}", input.scope, input.name);
        let record = SkillRecord {
            scope: input.scope,
            name: input.name,
            manifest: input.manifest,
            content_path: input.content_path,
            version: input.version,
        };
        let mut state = self.state.write().await;
        state.skills.insert(key, record.clone());
        record
    }

    pub async fn list_skills(&self) -> Vec<SkillRecord> {
        let state = self.state.read().await;
        state.skills.values().cloned().collect()
    }

    pub async fn upsert_memory(&self, input: UpsertMemoryInput) -> MemoryRecord {
        let key = format!("{:?}:{}:{}", input.scope, input.namespace, input.key);
        let record = MemoryRecord {
            scope: input.scope,
            namespace: input.namespace,
            key: input.key,
            value: input.value,
            embedding_ref: input.embedding_ref,
            updated_at: Utc::now(),
        };
        let mut state = self.state.write().await;
        state.memories.insert(key, record.clone());
        record
    }

    pub async fn list_memory(&self) -> Vec<MemoryRecord> {
        let state = self.state.read().await;
        state.memories.values().cloned().collect()
    }
}

#[derive(Clone)]
struct ApiState {
    service: InMemoryCoreService,
    workspace_root: PathBuf,
}

pub fn build_router(service: InMemoryCoreService, workspace_root: PathBuf) -> Router {
    let state = ApiState {
        service,
        workspace_root,
    };
    Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/auth/login", post(auth_login))
        .route("/api/v1/auth/logout", post(auth_logout))
        .route("/api/v1/auth/switch", post(auth_switch))
        .route(
            "/api/v1/workflows",
            get(list_workflows).post(create_workflow),
        )
        .route("/api/v1/workflows/:id", get(get_workflow))
        .route(
            "/api/v1/workflows/:id/status",
            patch(update_workflow_status),
        )
        .route("/api/v1/workflows/:id/run", post(run_workflow))
        .route("/api/v1/workflows/:id/proposals", post(create_proposal))
        .route(
            "/api/v1/workflows/:id/proposals/:proposal_id/approve",
            post(approve_proposal),
        )
        .route("/api/v1/skills", get(list_skills).post(upsert_skill))
        .route("/api/v1/skills/export", get(export_skills))
        .route("/api/v1/skills/import", post(import_skills))
        .route("/api/v1/memory", get(list_memory).post(upsert_memory))
        .route("/api/v1/memory/export", get(export_memory))
        .route("/api/v1/memory/import", post(import_memory))
        .route("/api/v1/fs/tree", get(fs_tree))
        .route("/api/v1/fs/file", get(fs_read).put(fs_write))
        .route("/api/v1/fs/move", post(fs_move))
        .route("/api/v1/fs/path", delete(fs_delete))
        .route("/api/v1/office/open", post(office_open))
        .route("/api/v1/office/save", post(office_save))
        .route("/api/v1/office/version", get(office_versions))
        .with_state(state)
}

pub async fn run_server(bind: SocketAddr, workspace_root: PathBuf) -> Result<()> {
    let service = InMemoryCoreService::default();
    let router = build_router(service, workspace_root);
    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .context("bind core server listener")?;
    axum::serve(listener, router).await.context("serve axum")?;
    Ok(())
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({"status":"ok"}))
}

async fn auth_login(
    State(state): State<ApiState>,
    Json(input): Json<AuthLoginInput>,
) -> Json<AuthSessionResponse> {
    let session = state.service.login(input.account_id).await;
    Json(session)
}

async fn auth_logout(
    State(state): State<ApiState>,
    Json(input): Json<AuthLogoutInput>,
) -> StatusCode {
    state.service.logout(&input.account_id).await;
    StatusCode::NO_CONTENT
}

async fn auth_switch(
    State(state): State<ApiState>,
    Json(input): Json<AuthSwitchInput>,
) -> Json<AuthSessionResponse> {
    let session = state
        .service
        .switch_account(&input.from_account, &input.to_account)
        .await;
    Json(session)
}

async fn create_workflow(
    State(state): State<ApiState>,
    Json(input): Json<CreateWorkflowInput>,
) -> Result<Json<WorkflowDefinition>, (StatusCode, String)> {
    state
        .service
        .create_workflow(input)
        .await
        .map(Json)
        .map_err(internal_err)
}

async fn list_workflows(State(state): State<ApiState>) -> Json<Vec<WorkflowDefinition>> {
    Json(state.service.list_workflows().await)
}

async fn get_workflow(
    Path(workflow_id): Path<String>,
    State(state): State<ApiState>,
) -> Result<Json<WorkflowDefinition>, (StatusCode, String)> {
    state
        .service
        .get_workflow(&workflow_id)
        .await
        .ok_or_else(|| (StatusCode::NOT_FOUND, "workflow not found".into()))
        .map(Json)
}

async fn update_workflow_status(
    Path(workflow_id): Path<String>,
    State(state): State<ApiState>,
    Json(input): Json<UpdateWorkflowStatusInput>,
) -> Result<Json<WorkflowDefinition>, (StatusCode, String)> {
    state
        .service
        .update_workflow_status(&workflow_id, input.status)
        .await
        .map(Json)
        .map_err(internal_err)
}

async fn run_workflow(
    Path(workflow_id): Path<String>,
    State(state): State<ApiState>,
) -> Result<Json<RunWorkflowResponse>, (StatusCode, String)> {
    if state.service.get_workflow(&workflow_id).await.is_none() {
        return Err((StatusCode::NOT_FOUND, "workflow not found".into()));
    }
    Ok(Json(RunWorkflowResponse {
        run_id: Uuid::new_v4().to_string(),
        workflow_id,
        status: "queued".into(),
    }))
}

async fn create_proposal(
    Path(workflow_id): Path<String>,
    State(state): State<ApiState>,
    Json(input): Json<CreateProposalInput>,
) -> Result<Json<WorkflowChangeProposal>, (StatusCode, String)> {
    state
        .service
        .propose_workflow_change(workflow_id, input.diff, input.created_by_agent)
        .await
        .map(Json)
        .map_err(internal_err)
}

async fn approve_proposal(
    Path((_workflow_id, proposal_id)): Path<(String, String)>,
    State(state): State<ApiState>,
    Json(input): Json<ApprovalInput>,
) -> Result<Json<WorkflowChangeProposal>, (StatusCode, String)> {
    state
        .service
        .approve_workflow_change(&proposal_id, input.approved_by)
        .await
        .map(Json)
        .map_err(internal_err)
}

async fn upsert_skill(
    State(state): State<ApiState>,
    Json(input): Json<UpsertSkillInput>,
) -> Json<SkillRecord> {
    Json(state.service.upsert_skill(input).await)
}

async fn list_skills(State(state): State<ApiState>) -> Json<Vec<SkillRecord>> {
    Json(state.service.list_skills().await)
}

async fn export_skills(State(state): State<ApiState>) -> Json<Vec<SkillRecord>> {
    Json(state.service.list_skills().await)
}

async fn import_skills(
    State(state): State<ApiState>,
    Json(inputs): Json<Vec<UpsertSkillInput>>,
) -> Json<Vec<SkillRecord>> {
    let mut out = Vec::with_capacity(inputs.len());
    for input in inputs {
        out.push(state.service.upsert_skill(input).await);
    }
    Json(out)
}

async fn upsert_memory(
    State(state): State<ApiState>,
    Json(input): Json<UpsertMemoryInput>,
) -> Json<MemoryRecord> {
    Json(state.service.upsert_memory(input).await)
}

async fn list_memory(State(state): State<ApiState>) -> Json<Vec<MemoryRecord>> {
    Json(state.service.list_memory().await)
}

async fn export_memory(State(state): State<ApiState>) -> Json<Vec<MemoryRecord>> {
    Json(state.service.list_memory().await)
}

async fn import_memory(
    State(state): State<ApiState>,
    Json(inputs): Json<Vec<UpsertMemoryInput>>,
) -> Json<Vec<MemoryRecord>> {
    let mut out = Vec::with_capacity(inputs.len());
    for input in inputs {
        out.push(state.service.upsert_memory(input).await);
    }
    Json(out)
}

async fn fs_tree(
    State(state): State<ApiState>,
    Query(query): Query<FsQuery>,
) -> Result<Json<Vec<FsTreeEntry>>, (StatusCode, String)> {
    let root = resolve_workspace_path(&state.workspace_root, &query.path).map_err(internal_err)?;
    let entries = list_directory_tree(&state.workspace_root, &root)
        .await
        .map_err(internal_err)?;
    Ok(Json(entries))
}

async fn fs_read(
    State(state): State<ApiState>,
    Query(query): Query<FsQuery>,
) -> Result<Json<FsReadResponse>, (StatusCode, String)> {
    let target =
        resolve_workspace_path(&state.workspace_root, &query.path).map_err(internal_err)?;
    let bytes = tokio::fs::read(&target).await.map_err(internal_err)?;
    Ok(Json(FsReadResponse {
        path: query.path,
        content_base64: STANDARD.encode(bytes),
    }))
}

async fn fs_write(
    State(state): State<ApiState>,
    Json(input): Json<FsWriteInput>,
) -> Result<StatusCode, (StatusCode, String)> {
    let target =
        resolve_workspace_path(&state.workspace_root, &input.path).map_err(internal_err)?;
    if let Some(parent) = target.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(internal_err)?;
    }
    let bytes = STANDARD
        .decode(input.content_base64.as_bytes())
        .map_err(|err| {
            (
                StatusCode::BAD_REQUEST,
                format!("invalid base64 payload: {err}"),
            )
        })?;
    tokio::fs::write(target, bytes)
        .await
        .map_err(internal_err)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn fs_move(
    State(state): State<ApiState>,
    Json(input): Json<FsMoveInput>,
) -> Result<StatusCode, (StatusCode, String)> {
    let from = resolve_workspace_path(&state.workspace_root, &input.from).map_err(internal_err)?;
    let to = resolve_workspace_path(&state.workspace_root, &input.to).map_err(internal_err)?;
    if let Some(parent) = to.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(internal_err)?;
    }
    tokio::fs::rename(from, to).await.map_err(internal_err)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn fs_delete(
    State(state): State<ApiState>,
    Query(query): Query<FsQuery>,
) -> Result<StatusCode, (StatusCode, String)> {
    let target =
        resolve_workspace_path(&state.workspace_root, &query.path).map_err(internal_err)?;
    let metadata = tokio::fs::metadata(&target).await.map_err(internal_err)?;
    if metadata.is_dir() {
        tokio::fs::remove_dir_all(target)
            .await
            .map_err(internal_err)?;
    } else {
        tokio::fs::remove_file(target).await.map_err(internal_err)?;
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn office_open(
    State(state): State<ApiState>,
    Json(input): Json<OfficeOpenInput>,
) -> Result<Json<FsReadResponse>, (StatusCode, String)> {
    fs_read(State(state), Query(FsQuery { path: input.path })).await
}

async fn office_save(
    State(state): State<ApiState>,
    Json(input): Json<OfficeSaveInput>,
) -> Result<StatusCode, (StatusCode, String)> {
    let target =
        resolve_workspace_path(&state.workspace_root, &input.path).map_err(internal_err)?;
    if tokio::fs::try_exists(&target).await.unwrap_or(false) {
        let version_root = state.workspace_root.join(".workdesk_versions");
        let version_file = version_root.join(format!(
            "{}_{}.bak",
            input.path.replace(['/', '\\', ':'], "_"),
            Utc::now().timestamp()
        ));
        if let Some(parent) = version_file.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(internal_err)?;
        }
        let previous = tokio::fs::read(&target).await.map_err(internal_err)?;
        tokio::fs::write(version_file, previous)
            .await
            .map_err(internal_err)?;
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
) -> Result<Json<OfficeVersionResponse>, (StatusCode, String)> {
    let version_root = state.workspace_root.join(".workdesk_versions");
    if !tokio::fs::try_exists(&version_root).await.unwrap_or(false) {
        return Ok(Json(OfficeVersionResponse {
            path: query.path,
            versions: vec![],
        }));
    }

    let key = query.path.replace(['/', '\\', ':'], "_");
    let mut versions = vec![];
    let mut dir = tokio::fs::read_dir(version_root)
        .await
        .map_err(internal_err)?;
    while let Some(entry) = dir.next_entry().await.map_err(internal_err)? {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with(&key) {
            versions.push(name);
        }
    }
    versions.sort();
    Ok(Json(OfficeVersionResponse {
        path: query.path,
        versions,
    }))
}

fn resolve_workspace_path(workspace_root: &PathBuf, relative: &str) -> Result<PathBuf> {
    let relative_path = PathBuf::from(relative);
    if relative_path.is_absolute() {
        return Err(anyhow!("absolute paths are not allowed"));
    }
    for component in relative_path.components() {
        if matches!(component, Component::ParentDir) {
            return Err(anyhow!("path traversal is not allowed"));
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

fn internal_err(err: impl std::fmt::Display) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}
