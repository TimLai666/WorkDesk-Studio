use crate::errors::CoreError;
use crate::repository::CoreRepository;
use crate::types::{
    AgentWorkspaceMessage, AgentWorkspaceMessageRole, AgentWorkspaceSession, ApprovalState,
    AppendAgentWorkspaceMessageInput, AuthLoginInput, AuthSessionResponse, AuthSwitchInput,
    ChoicePrompt, ChoicePromptAnswerInput, ChoicePromptOption, CreateAgentWorkspaceSessionInput,
    CreateChoicePromptInput, CreateProposalInput, CreateWorkflowInput, MemoryRecord,
    RunSkillSnapshot, SkillRecord, UpdateAgentWorkspaceSessionConfigInput, UpsertMemoryInput,
    UpsertSkillInput, WorkflowChangeProposal, WorkflowDefinition, WorkflowNode, WorkflowRun,
    WorkflowRunEvent, WorkflowRunNodeState, WorkflowStatus,
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
            agent_defaults: input.agent_defaults,
            nodes: input
                .nodes
                .into_iter()
                .map(|node| WorkflowNode {
                    id: node.id,
                    kind: node.kind,
                    x: node.x,
                    y: node.y,
                    config: node.config,
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

    pub async fn create_agent_workspace_session(
        &self,
        input: CreateAgentWorkspaceSessionInput,
    ) -> std::result::Result<AgentWorkspaceSession, CoreError> {
        let session = AgentWorkspaceSession {
            session_id: Uuid::new_v4().to_string(),
            title: input.title,
            config: input.config.unwrap_or_default(),
            last_active_panel: input.last_active_panel,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        self.repo
            .create_agent_workspace_session(&session)
            .await
            .map_err(|e| CoreError::Internal(e.to_string()))?;
        Ok(session)
    }

    pub async fn list_agent_workspace_sessions(
        &self,
    ) -> std::result::Result<Vec<AgentWorkspaceSession>, CoreError> {
        self.repo
            .list_agent_workspace_sessions()
            .await
            .map_err(|e| CoreError::Internal(e.to_string()))
    }

    pub async fn update_agent_workspace_session_config(
        &self,
        session_id: &str,
        input: UpdateAgentWorkspaceSessionConfigInput,
    ) -> std::result::Result<AgentWorkspaceSession, CoreError> {
        let mut session = self
            .repo
            .get_agent_workspace_session(session_id)
            .await
            .map_err(|e| CoreError::Internal(e.to_string()))?
            .ok_or_else(|| CoreError::BadRequest("agent session not found".into()))?;
        if input.model.is_some() {
            session.config.model = input.model;
        }
        if input.model_reasoning_effort.is_some() {
            session.config.model_reasoning_effort = input.model_reasoning_effort;
        }
        if input.speed.is_some() {
            session.config.speed = input.speed;
        }
        if let Some(plan_mode) = input.plan_mode {
            session.config.plan_mode = plan_mode;
        }
        if input.last_active_panel.is_some() {
            session.last_active_panel = input.last_active_panel;
        }
        session.updated_at = chrono::Utc::now();
        self.repo
            .update_agent_workspace_session(&session)
            .await
            .map_err(|e| CoreError::Internal(e.to_string()))?;
        Ok(session)
    }

    pub async fn append_agent_workspace_message(
        &self,
        session_id: &str,
        role: AgentWorkspaceMessageRole,
        content: String,
    ) -> std::result::Result<AgentWorkspaceMessage, CoreError> {
        let _ = self
            .repo
            .get_agent_workspace_session(session_id)
            .await
            .map_err(|e| CoreError::Internal(e.to_string()))?
            .ok_or_else(|| CoreError::BadRequest("agent session not found".into()))?;
        let message = AgentWorkspaceMessage {
            message_id: Uuid::new_v4().to_string(),
            session_id: session_id.to_string(),
            role,
            content,
            created_at: chrono::Utc::now(),
        };
        self.repo
            .append_agent_workspace_message(&message)
            .await
            .map_err(|e| CoreError::Internal(e.to_string()))?;
        Ok(message)
    }

    pub async fn append_agent_workspace_message_input(
        &self,
        session_id: &str,
        input: AppendAgentWorkspaceMessageInput,
    ) -> std::result::Result<AgentWorkspaceMessage, CoreError> {
        self.append_agent_workspace_message(session_id, input.role, input.content)
            .await
    }

    pub async fn list_agent_workspace_messages(
        &self,
        session_id: &str,
    ) -> std::result::Result<Vec<AgentWorkspaceMessage>, CoreError> {
        self.repo
            .list_agent_workspace_messages(session_id)
            .await
            .map_err(|e| CoreError::Internal(e.to_string()))
    }

    pub async fn create_choice_prompt(
        &self,
        session_id: &str,
        input: CreateChoicePromptInput,
    ) -> std::result::Result<ChoicePrompt, CoreError> {
        let _ = self
            .repo
            .get_agent_workspace_session(session_id)
            .await
            .map_err(|e| CoreError::Internal(e.to_string()))?
            .ok_or_else(|| CoreError::BadRequest("agent session not found".into()))?;
        let prompt = ChoicePrompt {
            prompt_id: Uuid::new_v4().to_string(),
            session_id: session_id.to_string(),
            question: input.question,
            options: input
                .options
                .into_iter()
                .map(|option| ChoicePromptOption {
                    option_id: option.option_id,
                    label: option.label,
                    description: option.description,
                })
                .collect(),
            recommended_option_id: input.recommended_option_id,
            allow_freeform: input.allow_freeform,
            status: crate::types::ChoicePromptStatus::Pending,
            selected_option_id: None,
            freeform_answer: None,
            created_at: chrono::Utc::now(),
            answered_at: None,
        };
        self.repo
            .create_choice_prompt(&prompt)
            .await
            .map_err(|e| CoreError::Internal(e.to_string()))?;
        Ok(prompt)
    }

    pub async fn list_choice_prompts(
        &self,
        session_id: &str,
    ) -> std::result::Result<Vec<ChoicePrompt>, CoreError> {
        self.repo
            .list_choice_prompts(session_id)
            .await
            .map_err(|e| CoreError::Internal(e.to_string()))
    }

    pub async fn answer_choice_prompt(
        &self,
        session_id: &str,
        prompt_id: &str,
        input: ChoicePromptAnswerInput,
    ) -> std::result::Result<ChoicePrompt, CoreError> {
        let mut prompt = self
            .repo
            .get_choice_prompt(session_id, prompt_id)
            .await
            .map_err(|e| CoreError::Internal(e.to_string()))?
            .ok_or_else(|| CoreError::BadRequest("choice prompt not found".into()))?;
        prompt.status = crate::types::ChoicePromptStatus::Answered;
        prompt.selected_option_id = input.selected_option_id;
        prompt.freeform_answer = input.freeform_answer;
        prompt.answered_at = Some(chrono::Utc::now());
        self.repo
            .update_choice_prompt(&prompt)
            .await
            .map_err(|e| CoreError::Internal(e.to_string()))?;
        Ok(prompt)
    }

    pub async fn queue_workflow_run(
        &self,
        workflow_id: &str,
        requested_by: Option<&str>,
    ) -> std::result::Result<WorkflowRun, CoreError> {
        let workflow = self.get_workflow(workflow_id).await?;
        let run = self
            .repo
            .create_run(workflow_id, requested_by)
            .await
            .map_err(|e| CoreError::Internal(e.to_string()))?;
        self.repo
            .create_run_skill_snapshots(&run.run_id)
            .await
            .map_err(|e| CoreError::Internal(e.to_string()))?;
        self.repo
            .create_run_node_states(&run.run_id, &workflow.nodes)
            .await
            .map_err(|e| CoreError::Internal(e.to_string()))?;
        self.repo
            .append_run_event(&run.run_id, "run_queued", "workflow run queued")
            .await
            .map_err(|e| CoreError::Internal(e.to_string()))?;
        self.repo
            .get_run(&run.run_id)
            .await
            .map_err(|e| CoreError::Internal(e.to_string()))?
            .ok_or(CoreError::RunNotFound)
    }

    pub async fn list_runs(
        &self,
        limit: usize,
    ) -> std::result::Result<Vec<WorkflowRun>, CoreError> {
        self.repo
            .list_runs(limit)
            .await
            .map_err(|e| CoreError::Internal(e.to_string()))
    }

    pub async fn get_run(&self, run_id: &str) -> std::result::Result<WorkflowRun, CoreError> {
        self.repo
            .get_run(run_id)
            .await
            .map_err(|e| CoreError::Internal(e.to_string()))?
            .ok_or(CoreError::RunNotFound)
    }

    pub async fn list_run_events(
        &self,
        run_id: &str,
        after_seq: i64,
        limit: usize,
    ) -> std::result::Result<Vec<WorkflowRunEvent>, CoreError> {
        let _ = self.get_run(run_id).await?;
        self.repo
            .list_run_events(run_id, after_seq, limit)
            .await
            .map_err(|e| CoreError::Internal(e.to_string()))
    }

    pub async fn list_run_nodes(
        &self,
        run_id: &str,
    ) -> std::result::Result<Vec<WorkflowRunNodeState>, CoreError> {
        let _ = self.get_run(run_id).await?;
        self.repo
            .list_run_node_states(run_id)
            .await
            .map_err(|e| CoreError::Internal(e.to_string()))
    }

    pub async fn list_run_skills(
        &self,
        run_id: &str,
    ) -> std::result::Result<Vec<RunSkillSnapshot>, CoreError> {
        let _ = self.get_run(run_id).await?;
        self.repo
            .list_run_skill_snapshots(run_id)
            .await
            .map_err(|e| CoreError::Internal(e.to_string()))
    }

    pub async fn cancel_run(&self, run_id: &str) -> std::result::Result<WorkflowRun, CoreError> {
        let run = self
            .repo
            .request_cancel_run(run_id)
            .await
            .map_err(|e| CoreError::Internal(e.to_string()))?
            .ok_or(CoreError::RunNotFound)?;
        if !run.cancel_requested {
            return Err(CoreError::RunNotCancelable);
        }
        self.repo
            .append_run_event(run_id, "cancel_requested", "cancel requested by operator")
            .await
            .map_err(|e| CoreError::Internal(e.to_string()))?;
        self.get_run(run_id).await
    }

    pub async fn retry_run(
        &self,
        run_id: &str,
        requested_by: Option<&str>,
    ) -> std::result::Result<WorkflowRun, CoreError> {
        let run = self
            .repo
            .retry_run(run_id, requested_by)
            .await
            .map_err(|e| CoreError::Internal(e.to_string()))?
            .ok_or(CoreError::RunNotFound)?;
        self.repo
            .create_run_skill_snapshots(&run.run_id)
            .await
            .map_err(|e| CoreError::Internal(e.to_string()))?;
        self.repo
            .append_run_event(&run.run_id, "run_queued", "workflow run retried")
            .await
            .map_err(|e| CoreError::Internal(e.to_string()))?;
        self.get_run(&run.run_id).await
    }
}
