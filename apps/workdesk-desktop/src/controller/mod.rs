mod canvas;
mod diagnostics;
mod files;
mod office;
mod runs;
mod state;
mod workbench;

pub use state::{
    reduce_ui_state, CanvasNodeState, ControllerAction, UiDiagnostic, UiRoute, UiStateSnapshot,
};

use crate::api_client::ApiClient;
use crate::command::DesktopCommand;
use crate::command_bus::CommandDispatcher;
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use workdesk_core::{
    AgentWorkspaceMessage, AgentWorkspaceSession, AppendAgentWorkspaceMessageInput, AuthLoginInput,
    AuthLogoutInput, AuthSessionResponse, AuthSwitchInput, ChoicePrompt, CodexModelCapability,
    CodexNativeSessionConfig, CreateAgentWorkspaceSessionInput, FsDiffResponse, FsSearchMatch,
    FsTreeEntry, PatchWorkflowInput, RunSkillSnapshot, TerminalSessionResponse, TerminalStartInput,
    WorkflowDefinition, WorkflowEdge, WorkflowRun, WorkflowRunEvent, WorkflowRunNodeState,
    WorkflowStatus,
};

#[derive(Debug, Clone)]
pub(super) struct CanvasSnapshot {
    pub(super) nodes: Vec<CanvasNodeState>,
    pub(super) edges: Vec<WorkflowEdge>,
    pub(super) selected: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub(super) struct CanvasHistory {
    pub(super) past: Vec<CanvasSnapshot>,
    pub(super) future: Vec<CanvasSnapshot>,
}

#[async_trait]
pub trait DesktopApi: Send + Sync {
    async fn login(&self, input: &AuthLoginInput) -> Result<AuthSessionResponse>;
    async fn logout(&self, input: &AuthLogoutInput) -> Result<serde_json::Value>;
    async fn switch_account(&self, input: &AuthSwitchInput) -> Result<AuthSessionResponse>;

    async fn list_workflows(&self) -> Result<Vec<WorkflowDefinition>>;
    async fn update_workflow_status(
        &self,
        workflow_id: &str,
        status: WorkflowStatus,
    ) -> Result<WorkflowDefinition>;
    async fn patch_workflow(
        &self,
        workflow_id: &str,
        patch: &PatchWorkflowInput,
    ) -> Result<WorkflowDefinition>;

    async fn list_runs(&self, limit: usize) -> Result<Vec<WorkflowRun>>;
    async fn list_run_events(
        &self,
        run_id: &str,
        after_seq: i64,
        limit: usize,
    ) -> Result<Vec<WorkflowRunEvent>>;
    async fn list_run_skills(&self, run_id: &str) -> Result<Vec<RunSkillSnapshot>>;
    async fn list_run_nodes(&self, run_id: &str) -> Result<Vec<WorkflowRunNodeState>>;
    async fn run_workflow(
        &self,
        workflow_id: &str,
        requested_by: Option<&str>,
    ) -> Result<WorkflowRun>;
    async fn cancel_run(&self, run_id: &str, requested_by: Option<&str>) -> Result<WorkflowRun>;
    async fn retry_run(&self, run_id: &str, requested_by: Option<&str>) -> Result<WorkflowRun>;

    async fn fs_tree(&self, path: &str) -> Result<Vec<FsTreeEntry>>;
    async fn fs_read(&self, path: &str) -> Result<workdesk_core::FsReadResponse>;
    async fn fs_write(&self, path: &str, content_base64: String) -> Result<serde_json::Value>;
    async fn fs_move(&self, from: &str, to: &str) -> Result<serde_json::Value>;
    async fn fs_delete(&self, path: &str) -> Result<serde_json::Value>;
    async fn fs_search(&self, path: &str, query: &str, limit: usize) -> Result<Vec<FsSearchMatch>>;
    async fn fs_diff(&self, left_path: &str, right_path: &str) -> Result<FsDiffResponse>;
    async fn terminal_start(&self, input: &TerminalStartInput) -> Result<TerminalSessionResponse>;
    async fn terminal_session(&self, session_id: &str) -> Result<TerminalSessionResponse>;

    async fn office_open(&self, path: &str) -> Result<workdesk_core::FsReadResponse>;
    async fn office_save(&self, path: &str, content_base64: String) -> Result<serde_json::Value>;
    async fn office_versions(&self, path: &str) -> Result<workdesk_core::OfficeVersionResponse>;

    async fn pdf_preview(&self, path: &str) -> Result<workdesk_core::FsReadResponse>;
    async fn pdf_annotate(
        &self,
        path: &str,
        annotation: &str,
    ) -> Result<workdesk_core::PdfOperationResponse>;
    async fn pdf_replace_text(
        &self,
        path: &str,
        search: &str,
        replace: &str,
    ) -> Result<workdesk_core::PdfOperationResponse>;
    async fn pdf_save_version(&self, path: &str) -> Result<workdesk_core::PdfOperationResponse>;

    async fn list_agent_capabilities(&self) -> Result<Vec<CodexModelCapability>>;
    async fn list_agent_workspace_sessions(&self) -> Result<Vec<AgentWorkspaceSession>>;
    async fn create_agent_workspace_session(
        &self,
        input: &CreateAgentWorkspaceSessionInput,
    ) -> Result<AgentWorkspaceSession>;
    async fn update_agent_workspace_session_config(
        &self,
        session_id: &str,
        config: CodexNativeSessionConfig,
        last_active_panel: Option<&str>,
    ) -> Result<AgentWorkspaceSession>;
    async fn list_agent_workspace_messages(
        &self,
        session_id: &str,
    ) -> Result<Vec<AgentWorkspaceMessage>>;
    async fn list_choice_prompts(&self, session_id: &str) -> Result<Vec<ChoicePrompt>>;
    async fn append_agent_workspace_message(
        &self,
        session_id: &str,
        input: &AppendAgentWorkspaceMessageInput,
    ) -> Result<AgentWorkspaceMessage>;
    async fn answer_choice_prompt(
        &self,
        session_id: &str,
        prompt_id: &str,
        selected_option_id: Option<&str>,
        freeform_answer: Option<&str>,
    ) -> Result<ChoicePrompt>;
}

#[async_trait]
impl DesktopApi for ApiClient {
    async fn login(&self, input: &AuthLoginInput) -> Result<AuthSessionResponse> {
        self.login(input).await
    }

    async fn logout(&self, input: &AuthLogoutInput) -> Result<serde_json::Value> {
        self.logout(input).await
    }

    async fn switch_account(&self, input: &AuthSwitchInput) -> Result<AuthSessionResponse> {
        self.switch_account(input).await
    }

    async fn list_workflows(&self) -> Result<Vec<WorkflowDefinition>> {
        self.list_workflows().await
    }

    async fn update_workflow_status(
        &self,
        workflow_id: &str,
        status: WorkflowStatus,
    ) -> Result<WorkflowDefinition> {
        self.update_workflow_status(workflow_id, status).await
    }

    async fn patch_workflow(
        &self,
        workflow_id: &str,
        patch: &PatchWorkflowInput,
    ) -> Result<WorkflowDefinition> {
        self.patch_workflow(workflow_id, patch).await
    }

    async fn list_runs(&self, limit: usize) -> Result<Vec<WorkflowRun>> {
        self.list_runs(limit).await
    }

    async fn list_run_events(
        &self,
        run_id: &str,
        after_seq: i64,
        limit: usize,
    ) -> Result<Vec<WorkflowRunEvent>> {
        self.list_run_events(run_id, after_seq, limit).await
    }

    async fn list_run_skills(&self, run_id: &str) -> Result<Vec<RunSkillSnapshot>> {
        self.list_run_skills(run_id).await
    }

    async fn list_run_nodes(&self, run_id: &str) -> Result<Vec<WorkflowRunNodeState>> {
        self.list_run_nodes(run_id).await
    }

    async fn run_workflow(
        &self,
        workflow_id: &str,
        requested_by: Option<&str>,
    ) -> Result<WorkflowRun> {
        self.run_workflow(workflow_id, requested_by).await
    }

    async fn cancel_run(&self, run_id: &str, requested_by: Option<&str>) -> Result<WorkflowRun> {
        self.cancel_run(run_id, requested_by).await
    }

    async fn retry_run(&self, run_id: &str, requested_by: Option<&str>) -> Result<WorkflowRun> {
        self.retry_run(run_id, requested_by).await
    }

    async fn fs_tree(&self, path: &str) -> Result<Vec<FsTreeEntry>> {
        self.fs_tree(path).await
    }

    async fn fs_read(&self, path: &str) -> Result<workdesk_core::FsReadResponse> {
        self.fs_read(path).await
    }

    async fn fs_write(&self, path: &str, content_base64: String) -> Result<serde_json::Value> {
        self.fs_write(path, content_base64).await
    }

    async fn fs_move(&self, from: &str, to: &str) -> Result<serde_json::Value> {
        self.fs_move(from, to).await
    }

    async fn fs_delete(&self, path: &str) -> Result<serde_json::Value> {
        self.fs_delete(path).await
    }

    async fn fs_search(&self, path: &str, query: &str, limit: usize) -> Result<Vec<FsSearchMatch>> {
        self.fs_search(path, query, limit).await
    }

    async fn fs_diff(&self, left_path: &str, right_path: &str) -> Result<FsDiffResponse> {
        self.fs_diff(left_path, right_path).await
    }

    async fn terminal_start(&self, input: &TerminalStartInput) -> Result<TerminalSessionResponse> {
        self.terminal_start(input).await
    }

    async fn terminal_session(&self, session_id: &str) -> Result<TerminalSessionResponse> {
        self.terminal_session(session_id).await
    }

    async fn office_open(&self, path: &str) -> Result<workdesk_core::FsReadResponse> {
        self.office_open(path).await
    }

    async fn office_save(&self, path: &str, content_base64: String) -> Result<serde_json::Value> {
        self.office_save(path, content_base64).await
    }

    async fn office_versions(&self, path: &str) -> Result<workdesk_core::OfficeVersionResponse> {
        self.office_versions(path).await
    }

    async fn pdf_preview(&self, path: &str) -> Result<workdesk_core::FsReadResponse> {
        self.pdf_preview(path).await
    }

    async fn pdf_annotate(
        &self,
        path: &str,
        annotation: &str,
    ) -> Result<workdesk_core::PdfOperationResponse> {
        self.pdf_annotate(path, annotation).await
    }

    async fn pdf_replace_text(
        &self,
        path: &str,
        search: &str,
        replace: &str,
    ) -> Result<workdesk_core::PdfOperationResponse> {
        self.pdf_replace_text(path, search, replace).await
    }

    async fn pdf_save_version(&self, path: &str) -> Result<workdesk_core::PdfOperationResponse> {
        self.pdf_save_version(path).await
    }

    async fn list_agent_capabilities(&self) -> Result<Vec<CodexModelCapability>> {
        self.list_agent_capabilities().await
    }

    async fn list_agent_workspace_sessions(&self) -> Result<Vec<AgentWorkspaceSession>> {
        self.list_agent_workspace_sessions().await
    }

    async fn create_agent_workspace_session(
        &self,
        input: &CreateAgentWorkspaceSessionInput,
    ) -> Result<AgentWorkspaceSession> {
        self.create_agent_workspace_session(input).await
    }

    async fn update_agent_workspace_session_config(
        &self,
        session_id: &str,
        config: CodexNativeSessionConfig,
        last_active_panel: Option<&str>,
    ) -> Result<AgentWorkspaceSession> {
        self.update_agent_workspace_session_config(session_id, config, last_active_panel)
            .await
    }

    async fn list_agent_workspace_messages(
        &self,
        session_id: &str,
    ) -> Result<Vec<AgentWorkspaceMessage>> {
        self.list_agent_workspace_messages(session_id).await
    }

    async fn list_choice_prompts(&self, session_id: &str) -> Result<Vec<ChoicePrompt>> {
        self.list_choice_prompts(session_id).await
    }

    async fn append_agent_workspace_message(
        &self,
        session_id: &str,
        input: &AppendAgentWorkspaceMessageInput,
    ) -> Result<AgentWorkspaceMessage> {
        self.append_agent_workspace_message(session_id, input).await
    }

    async fn answer_choice_prompt(
        &self,
        session_id: &str,
        prompt_id: &str,
        selected_option_id: Option<&str>,
        freeform_answer: Option<&str>,
    ) -> Result<ChoicePrompt> {
        self.answer_choice_prompt(session_id, prompt_id, selected_option_id, freeform_answer)
            .await
    }
}

#[derive(Clone)]
pub struct DesktopAppController {
    api: Arc<dyn DesktopApi>,
    state: Arc<RwLock<UiStateSnapshot>>,
    runtime_diagnostics: Arc<RwLock<HashMap<String, UiDiagnostic>>>,
    canvas_history: Arc<RwLock<CanvasHistory>>,
    canvas_dragging: Arc<RwLock<Option<String>>>,
}

impl DesktopAppController {
    pub fn new(api: Arc<dyn DesktopApi>) -> Self {
        Self {
            api,
            state: Arc::new(RwLock::new(UiStateSnapshot::default())),
            runtime_diagnostics: Arc::new(RwLock::new(HashMap::new())),
            canvas_history: Arc::new(RwLock::new(CanvasHistory::default())),
            canvas_dragging: Arc::new(RwLock::new(None)),
        }
    }

    pub fn snapshot(&self) -> UiStateSnapshot {
        self.state.read().expect("ui state read lock").clone()
    }

    pub fn shared_state(&self) -> Arc<RwLock<UiStateSnapshot>> {
        self.state.clone()
    }

    pub async fn bootstrap(&self) -> Result<()> {
        self.refresh_agent_capabilities().await?;
        self.refresh_agent_sessions().await?;
        self.refresh_workflows().await?;
        self.refresh_runs().await?;
        if self.snapshot().active_agent_session_id.is_some() {
            self.refresh_active_agent_workspace().await?;
        }
        Ok(())
    }

    pub async fn dispatch_command(&self, command: DesktopCommand) -> Result<()> {
        self.apply(ControllerAction::FocusWindow);
        self.apply(ControllerAction::SetError(None));

        let result = match command {
            DesktopCommand::Open => {
                self.apply(ControllerAction::SetRoute(UiRoute::Workbench));
                self.refresh_agent_sessions().await?;
                self.refresh_runs().await
            }
            DesktopCommand::OpenRun { run_id } => {
                self.apply(ControllerAction::SetRoute(UiRoute::RunDetail));
                self.apply(ControllerAction::SelectRun(Some(run_id.clone())));
                self.refresh_runs().await?;
                self.refresh_run_detail(&run_id).await
            }
            DesktopCommand::OpenWorkflow { workflow_id } => {
                self.apply(ControllerAction::SetRoute(UiRoute::WorkflowDetail));
                self.apply(ControllerAction::SelectWorkflow(Some(workflow_id.clone())));
                self.refresh_workflows().await?;
                self.load_canvas_for_selected_workflow()
            }
            DesktopCommand::RunWorkflow { workflow_id } => {
                let run = self
                    .api
                    .run_workflow(&workflow_id, Some("desktop-cli"))
                    .await?;
                self.apply(ControllerAction::SetRoute(UiRoute::RunDetail));
                self.apply(ControllerAction::SelectWorkflow(Some(workflow_id)));
                self.apply(ControllerAction::SelectRun(Some(run.run_id.clone())));
                self.refresh_runs().await?;
                self.refresh_run_detail(&run.run_id).await
            }
        };

        if let Err(error) = &result {
            self.apply(ControllerAction::SetError(Some(error.to_string())));
        }
        result
    }

    pub(crate) fn apply(&self, action: ControllerAction) {
        let mut state = self.state.write().expect("ui state write lock");
        reduce_ui_state(&mut state, action);
    }
}

#[async_trait]
impl CommandDispatcher for DesktopAppController {
    async fn dispatch(&self, command: DesktopCommand) -> Result<()> {
        self.dispatch_command(command).await
    }
}
