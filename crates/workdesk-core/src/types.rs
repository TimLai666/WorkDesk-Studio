use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;
pub use workdesk_domain::{
    ApprovalState, MemoryRecord, Scope, SkillRecord, WorkflowChangeProposal, WorkflowDefinition,
    WorkflowEdge, WorkflowNode, WorkflowNodeKind, WorkflowStatus,
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
    pub session_token: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunWorkflowInput {
    pub requested_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelRunInput {
    pub requested_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    Canceled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRun {
    pub run_id: String,
    pub workflow_id: String,
    pub requested_by: Option<String>,
    pub status: RunStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub cancel_requested: bool,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRunEvent {
    pub run_id: String,
    pub seq: i64,
    pub event_type: String,
    pub payload: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunSkillSnapshot {
    pub run_id: String,
    pub scope: Scope,
    pub name: String,
    pub manifest: String,
    pub content_path: String,
    pub version: String,
    pub materialized_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunListQuery {
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunEventsQuery {
    pub after_seq: Option<i64>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryRunInput {
    pub requested_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiEnvelope<T> {
    pub data: Option<T>,
    pub error: Option<ApiErrorPayload>,
    pub meta: ApiMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiErrorPayload {
    pub code: String,
    pub message: String,
    pub details: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiMeta {
    pub request_id: String,
    pub timestamp: String,
}

impl ApiMeta {
    pub fn new() -> Self {
        Self {
            request_id: Uuid::new_v4().to_string(),
            timestamp: Utc::now().to_rfc3339(),
        }
    }
}

impl<T> ApiEnvelope<T> {
    pub fn success(data: T) -> Self {
        Self {
            data: Some(data),
            error: None,
            meta: ApiMeta::new(),
        }
    }
}

impl ApiEnvelope<Value> {
    pub fn failure(code: &str, message: String, details: Option<Value>) -> Self {
        Self {
            data: None,
            error: Some(ApiErrorPayload {
                code: code.to_string(),
                message,
                details,
            }),
            meta: ApiMeta::new(),
        }
    }
}

pub fn workflow_status_to_db(status: &WorkflowStatus) -> &'static str {
    match status {
        WorkflowStatus::Draft => "draft",
        WorkflowStatus::Active => "active",
        WorkflowStatus::Disabled => "disabled",
    }
}

pub fn workflow_status_from_db(value: &str) -> anyhow::Result<WorkflowStatus> {
    match value {
        "draft" => Ok(WorkflowStatus::Draft),
        "active" => Ok(WorkflowStatus::Active),
        "disabled" => Ok(WorkflowStatus::Disabled),
        other => Err(anyhow::anyhow!("unknown workflow status: {other}")),
    }
}

pub fn workflow_kind_to_db(kind: &WorkflowNodeKind) -> &'static str {
    match kind {
        WorkflowNodeKind::ScheduleTrigger => "schedule_trigger",
        WorkflowNodeKind::AgentPrompt => "agent_prompt",
        WorkflowNodeKind::CodeExec => "code_exec",
        WorkflowNodeKind::FileOps => "file_ops",
        WorkflowNodeKind::ApprovalGate => "approval_gate",
    }
}

pub fn workflow_kind_from_db(value: &str) -> anyhow::Result<WorkflowNodeKind> {
    match value {
        "schedule_trigger" => Ok(WorkflowNodeKind::ScheduleTrigger),
        "agent_prompt" => Ok(WorkflowNodeKind::AgentPrompt),
        "code_exec" => Ok(WorkflowNodeKind::CodeExec),
        "file_ops" => Ok(WorkflowNodeKind::FileOps),
        "approval_gate" => Ok(WorkflowNodeKind::ApprovalGate),
        other => Err(anyhow::anyhow!("unknown workflow kind: {other}")),
    }
}

pub fn approval_state_to_db(state: &ApprovalState) -> &'static str {
    match state {
        ApprovalState::Pending => "pending",
        ApprovalState::Approved => "approved",
        ApprovalState::Rejected => "rejected",
        ApprovalState::Applied => "applied",
    }
}

pub fn approval_state_from_db(value: &str) -> anyhow::Result<ApprovalState> {
    match value {
        "pending" => Ok(ApprovalState::Pending),
        "approved" => Ok(ApprovalState::Approved),
        "rejected" => Ok(ApprovalState::Rejected),
        "applied" => Ok(ApprovalState::Applied),
        other => Err(anyhow::anyhow!("unknown approval state: {other}")),
    }
}

pub fn scope_to_db(scope: &Scope) -> &'static str {
    match scope {
        Scope::User => "user",
        Scope::Shared => "shared",
    }
}

pub fn scope_from_db(value: &str) -> anyhow::Result<Scope> {
    match value {
        "user" => Ok(Scope::User),
        "shared" => Ok(Scope::Shared),
        other => Err(anyhow::anyhow!("unknown scope: {other}")),
    }
}

pub fn parse_rfc3339_utc(value: &str) -> anyhow::Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(value)?.with_timezone(&Utc))
}

pub fn run_status_to_db(status: &RunStatus) -> &'static str {
    match status {
        RunStatus::Queued => "queued",
        RunStatus::Running => "running",
        RunStatus::Succeeded => "succeeded",
        RunStatus::Failed => "failed",
        RunStatus::Canceled => "canceled",
    }
}

pub fn run_status_from_db(value: &str) -> anyhow::Result<RunStatus> {
    match value {
        "queued" => Ok(RunStatus::Queued),
        "running" => Ok(RunStatus::Running),
        "succeeded" => Ok(RunStatus::Succeeded),
        "failed" => Ok(RunStatus::Failed),
        "canceled" => Ok(RunStatus::Canceled),
        other => Err(anyhow::anyhow!("unknown run status: {other}")),
    }
}
