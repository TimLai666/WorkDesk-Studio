use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("workflow graph has at least one cycle")]
    GraphContainsCycle,
    #[error("workflow references unknown node: {0}")]
    UnknownNode(String),
    #[error("workflow contains duplicated node id: {0}")]
    DuplicateNode(String),
    #[error("proposal state must be pending, current state: {0:?}")]
    InvalidApprovalState(ApprovalState),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStatus {
    Draft,
    Active,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowNodeKind {
    ScheduleTrigger,
    AgentPrompt,
    CodeExec,
    FileOps,
    ApprovalGate,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkflowNode {
    pub id: String,
    pub kind: WorkflowNodeKind,
}

impl WorkflowNode {
    pub fn new(id: impl Into<String>, kind: WorkflowNodeKind) -> Self {
        Self {
            id: id.into(),
            kind,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkflowEdge {
    pub from: String,
    pub to: String,
}

impl WorkflowEdge {
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkflowDefinition {
    pub id: String,
    pub name: String,
    pub timezone: String,
    pub nodes: Vec<WorkflowNode>,
    pub edges: Vec<WorkflowEdge>,
    pub version: u64,
    pub status: WorkflowStatus,
}

impl WorkflowDefinition {
    pub fn validate(&self) -> Result<(), DomainError> {
        let mut node_ids: HashMap<&str, usize> = HashMap::with_capacity(self.nodes.len());
        for node in &self.nodes {
            if node_ids.insert(node.id.as_str(), 0).is_some() {
                return Err(DomainError::DuplicateNode(node.id.clone()));
            }
        }

        let mut adjacency: HashMap<&str, Vec<&str>> = HashMap::new();
        for edge in &self.edges {
            if !node_ids.contains_key(edge.from.as_str()) {
                return Err(DomainError::UnknownNode(edge.from.clone()));
            }
            if !node_ids.contains_key(edge.to.as_str()) {
                return Err(DomainError::UnknownNode(edge.to.clone()));
            }
            adjacency
                .entry(edge.from.as_str())
                .or_default()
                .push(edge.to.as_str());
            *node_ids.get_mut(edge.to.as_str()).expect("checked above") += 1;
        }

        let mut queue: VecDeque<&str> = node_ids
            .iter()
            .filter_map(|(id, indegree)| (*indegree == 0).then_some(*id))
            .collect();
        let mut visited = 0usize;

        while let Some(node_id) = queue.pop_front() {
            visited += 1;
            if let Some(children) = adjacency.get(node_id) {
                for child in children {
                    let indegree = node_ids.get_mut(child).expect("child must exist");
                    *indegree -= 1;
                    if *indegree == 0 {
                        queue.push_back(child);
                    }
                }
            }
        }

        if visited != self.nodes.len() {
            return Err(DomainError::GraphContainsCycle);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalState {
    Pending,
    Approved,
    Rejected,
    Applied,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkflowChangeProposal {
    pub proposal_id: String,
    pub workflow_id: String,
    pub diff: String,
    pub created_by_agent: String,
    pub approval_state: ApprovalState,
    pub approved_by: Option<String>,
    pub approved_at: Option<DateTime<Utc>>,
}

impl WorkflowChangeProposal {
    pub fn new(workflow_id: String, diff: String, created_by_agent: String) -> Self {
        Self {
            proposal_id: Uuid::new_v4().to_string(),
            workflow_id,
            diff,
            created_by_agent,
            approval_state: ApprovalState::Pending,
            approved_by: None,
            approved_at: None,
        }
    }

    pub fn approve(&mut self, approved_by: String) -> Result<(), DomainError> {
        if self.approval_state != ApprovalState::Pending {
            return Err(DomainError::InvalidApprovalState(
                self.approval_state.clone(),
            ));
        }
        self.approval_state = ApprovalState::Approved;
        self.approved_by = Some(approved_by);
        self.approved_at = Some(Utc::now());
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    User,
    Shared,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillRecord {
    pub scope: Scope,
    pub name: String,
    pub manifest: String,
    pub content_path: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemoryRecord {
    pub scope: Scope,
    pub namespace: String,
    pub key: String,
    pub value: String,
    pub embedding_ref: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionLanguage {
    Python,
    Javascript,
    Go,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResourceLimits {
    pub timeout_sec: u64,
    pub max_memory_mb: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CodeNodeSpec {
    pub language: ExecutionLanguage,
    pub entry: String,
    pub deps: Vec<String>,
    pub timeout_sec: u64,
    pub resource_limits: ResourceLimits,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentSession {
    pub session_id: String,
    pub account_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentEvent {
    pub kind: String,
    pub payload: String,
}

#[async_trait]
pub trait AgentProvider: Send + Sync {
    async fn start_session(&self, account_id: &str) -> anyhow::Result<AgentSession>;
    async fn run_prompt(&self, session: &AgentSession, prompt: &str) -> anyhow::Result<String>;
    async fn stream_events(&self, session: &AgentSession) -> anyhow::Result<Vec<AgentEvent>>;
    async fn logout(&self, account_id: &str) -> anyhow::Result<()>;
    async fn switch_account(
        &self,
        from_account: &str,
        to_account: &str,
    ) -> anyhow::Result<AgentSession>;
}
