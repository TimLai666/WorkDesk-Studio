use crate::errors::CoreError;
use crate::repository::CoreRepository;
use crate::types::{
    ApprovalState, AuthLoginInput, AuthSessionResponse, AuthSwitchInput, CreateProposalInput,
    CreateWorkflowInput, MemoryRecord, SkillRecord, UpsertMemoryInput, UpsertSkillInput,
    WorkflowChangeProposal, WorkflowDefinition, WorkflowNode, WorkflowStatus,
};
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct CoreService {
    repo: Arc<dyn CoreRepository>,
    workspace_root: PathBuf,
}

impl CoreService {
    pub fn new(repo: Arc<dyn CoreRepository>, workspace_root: PathBuf) -> Self {
        Self {
            repo,
            workspace_root,
        }
    }

    pub fn repo(&self) -> Arc<dyn CoreRepository> {
        self.repo.clone()
    }

    pub fn workspace_root(&self) -> &PathBuf {
        &self.workspace_root
    }

    pub async fn login(
        &self,
        input: AuthLoginInput,
    ) -> std::result::Result<AuthSessionResponse, CoreError> {
        self.repo
            .verify_or_create_user(&input.account_id, &input.password)
            .await
            .map_err(|_| CoreError::InvalidCredentials)?;
        self.repo
            .revoke_sessions(&input.account_id)
            .await
            .map_err(|e| CoreError::Internal(e.to_string()))?;
        self.repo
            .create_session(&input.account_id)
            .await
            .map_err(|e| CoreError::Internal(e.to_string()))
    }

    pub async fn logout(&self, account_id: &str) -> std::result::Result<(), CoreError> {
        self.repo
            .revoke_sessions(account_id)
            .await
            .map_err(|e| CoreError::Internal(e.to_string()))
    }

    pub async fn switch_account(
        &self,
        input: AuthSwitchInput,
    ) -> std::result::Result<AuthSessionResponse, CoreError> {
        self.repo
            .revoke_sessions(&input.from_account)
            .await
            .map_err(|e| CoreError::Internal(e.to_string()))?;

        let exists = self
            .repo
            .account_exists(&input.to_account)
            .await
            .map_err(|e| CoreError::Internal(e.to_string()))?;
        if !exists {
            return Err(CoreError::AccountNotFound);
        }
        self.repo
            .create_session(&input.to_account)
            .await
            .map_err(|e| CoreError::Internal(e.to_string()))
    }

    pub async fn create_workflow(
        &self,
        input: CreateWorkflowInput,
    ) -> std::result::Result<WorkflowDefinition, CoreError> {
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
        workflow
            .validate()
            .map_err(|e| CoreError::Validation(e.to_string()))?;
        self.repo
            .create_workflow(&workflow)
            .await
            .map_err(|e| CoreError::Internal(e.to_string()))?;
        Ok(workflow)
    }

    pub async fn list_workflows(&self) -> std::result::Result<Vec<WorkflowDefinition>, CoreError> {
        self.repo
            .list_workflows()
            .await
            .map_err(|e| CoreError::Internal(e.to_string()))
    }

    pub async fn get_workflow(
        &self,
        workflow_id: &str,
    ) -> std::result::Result<WorkflowDefinition, CoreError> {
        self.repo
            .get_workflow(workflow_id)
            .await
            .map_err(|e| CoreError::Internal(e.to_string()))?
            .ok_or(CoreError::WorkflowNotFound)
    }

    pub async fn update_workflow_status(
        &self,
        workflow_id: &str,
        status: WorkflowStatus,
    ) -> std::result::Result<WorkflowDefinition, CoreError> {
        self.repo
            .update_workflow_status(workflow_id, status)
            .await
            .map_err(|e| CoreError::Internal(e.to_string()))?
            .ok_or(CoreError::WorkflowNotFound)
    }

    pub async fn propose_workflow_change(
        &self,
        workflow_id: String,
        input: CreateProposalInput,
    ) -> std::result::Result<WorkflowChangeProposal, CoreError> {
        let exists = self
            .repo
            .get_workflow(&workflow_id)
            .await
            .map_err(|e| CoreError::Internal(e.to_string()))?;
        if exists.is_none() {
            return Err(CoreError::WorkflowNotFound);
        }
        let proposal = WorkflowChangeProposal::new(workflow_id, input.diff, input.created_by_agent);
        self.repo
            .create_proposal(&proposal)
            .await
            .map_err(|e| CoreError::Internal(e.to_string()))?;
        Ok(proposal)
    }

    pub async fn approve_proposal(
        &self,
        proposal_id: &str,
        approved_by: String,
    ) -> std::result::Result<WorkflowChangeProposal, CoreError> {
        let mut proposal = self
            .repo
            .get_proposal(proposal_id)
            .await
            .map_err(|e| CoreError::Internal(e.to_string()))?
            .ok_or(CoreError::ProposalNotFound)?;
        if proposal.approval_state != ApprovalState::Pending {
            return Err(CoreError::ProposalNotPending);
        }
        proposal
            .approve(approved_by)
            .map_err(|e| CoreError::Validation(e.to_string()))?;
        proposal.approval_state = ApprovalState::Applied;
        self.repo
            .update_proposal(&proposal)
            .await
            .map_err(|e| CoreError::Internal(e.to_string()))?;
        Ok(proposal)
    }

    pub async fn upsert_skill(
        &self,
        input: UpsertSkillInput,
    ) -> std::result::Result<SkillRecord, CoreError> {
        let record = SkillRecord {
            scope: input.scope,
            name: input.name,
            manifest: input.manifest,
            content_path: input.content_path,
            version: input.version,
        };
        self.repo
            .upsert_skill(&record)
            .await
            .map_err(|e| CoreError::Internal(e.to_string()))?;
        Ok(record)
    }

    pub async fn list_skills(&self) -> std::result::Result<Vec<SkillRecord>, CoreError> {
        self.repo
            .list_skills()
            .await
            .map_err(|e| CoreError::Internal(e.to_string()))
    }

    pub async fn upsert_memory(
        &self,
        input: UpsertMemoryInput,
    ) -> std::result::Result<MemoryRecord, CoreError> {
        let record = MemoryRecord {
            scope: input.scope,
            namespace: input.namespace,
            key: input.key,
            value: input.value,
            embedding_ref: input.embedding_ref,
            updated_at: chrono::Utc::now(),
        };
        self.repo
            .upsert_memory(&record)
            .await
            .map_err(|e| CoreError::Internal(e.to_string()))?;
        Ok(record)
    }

    pub async fn list_memory(&self) -> std::result::Result<Vec<MemoryRecord>, CoreError> {
        self.repo
            .list_memory()
            .await
            .map_err(|e| CoreError::Internal(e.to_string()))
    }
}
