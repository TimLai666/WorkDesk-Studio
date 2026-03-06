use anyhow::Result;
use async_trait::async_trait;
use base64::Engine;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use workdesk_core::{
    AgentWorkspaceMessage, AgentWorkspaceSession, AppendAgentWorkspaceMessageInput, AuthLoginInput,
    AuthLogoutInput, AuthSessionResponse, AuthSwitchInput, ChoicePrompt, ChoicePromptOption,
    ChoicePromptStatus, CodexNativeSessionConfig, CreateAgentWorkspaceSessionInput, FsDiffResponse,
    FsReadResponse, FsSearchMatch, FsTreeEntry, OfficeVersionResponse, PatchWorkflowInput,
    PdfOperationResponse, RunNodeStatus, RunSkillSnapshot, RunStatus, Scope,
    TerminalSessionResponse, TerminalStartInput, WorkflowDefinition, WorkflowRun, WorkflowRunEvent,
    WorkflowRunNodeState, WorkflowStatus,
};
use workdesk_desktop::automation::{AutomationClient, AutomationServer};
use workdesk_desktop::command::DesktopCommand;
use workdesk_desktop::controller::{DesktopApi, DesktopAppController, UiRoute};

#[derive(Default)]
struct FakeDesktopApi {
    runs: Mutex<Vec<WorkflowRun>>,
    events: Mutex<HashMap<String, Vec<WorkflowRunEvent>>>,
    skills: Mutex<HashMap<String, Vec<RunSkillSnapshot>>>,
    nodes: Mutex<HashMap<String, Vec<WorkflowRunNodeState>>>,
    sessions: Mutex<Vec<AgentWorkspaceSession>>,
    messages: Mutex<HashMap<String, Vec<AgentWorkspaceMessage>>>,
    prompts: Mutex<HashMap<String, Vec<ChoicePrompt>>>,
}

impl FakeDesktopApi {
    fn seeded() -> Self {
        let run = WorkflowRun {
            run_id: "run-1".into(),
            workflow_id: "wf-1".into(),
            requested_by: Some("tester".into()),
            status: RunStatus::Running,
            started_at: Some(Utc::now()),
            finished_at: None,
            cancel_requested: false,
            error_message: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let event = WorkflowRunEvent {
            run_id: "run-1".into(),
            seq: 1,
            event_type: "started".into(),
            payload: "{\"ok\":true}".into(),
            created_at: Utc::now(),
        };
        let skill = RunSkillSnapshot {
            run_id: "run-1".into(),
            scope: Scope::User,
            name: "skill-A".into(),
            manifest: "{}".into(),
            content_path: "skills/skill-A".into(),
            version: "1.0.0".into(),
            materialized_path: Some("runtime/skill-A".into()),
        };
        let mut events = HashMap::new();
        events.insert("run-1".into(), vec![event]);
        let mut skills = HashMap::new();
        skills.insert("run-1".into(), vec![skill]);
        let mut nodes = HashMap::new();
        nodes.insert(
            "run-1".into(),
            vec![WorkflowRunNodeState {
                run_id: "run-1".into(),
                node_id: "n1".into(),
                kind: workdesk_core::WorkflowNodeKind::ScheduleTrigger,
                status: RunNodeStatus::Succeeded,
                attempt: 1,
                error_message: None,
                started_at: Some(Utc::now()),
                finished_at: Some(Utc::now()),
                updated_at: Utc::now(),
            }],
        );
        let session = AgentWorkspaceSession {
            session_id: "session-1".into(),
            title: "Workbench".into(),
            config: CodexNativeSessionConfig {
                model: Some("gpt-5.4".into()),
                model_reasoning_effort: Some("high".into()),
                speed: Some(true),
                plan_mode: true,
            },
            last_active_panel: Some("runs".into()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let mut messages = HashMap::new();
        messages.insert("session-1".into(), Vec::new());
        let mut prompts = HashMap::new();
        prompts.insert(
            "session-1".into(),
            vec![ChoicePrompt {
                prompt_id: "prompt-1".into(),
                session_id: "session-1".into(),
                question: "Choose rollout path".into(),
                options: vec![
                    ChoicePromptOption {
                        option_id: "safe".into(),
                        label: "Safe rollout".into(),
                        description: "Lower change risk".into(),
                    },
                    ChoicePromptOption {
                        option_id: "fast".into(),
                        label: "Fast rollout".into(),
                        description: "Shorter delivery time".into(),
                    },
                ],
                recommended_option_id: Some("safe".into()),
                allow_freeform: true,
                status: ChoicePromptStatus::Pending,
                selected_option_id: None,
                freeform_answer: None,
                created_at: Utc::now(),
                answered_at: None,
            }],
        );
        Self {
            runs: Mutex::new(vec![run]),
            events: Mutex::new(events),
            skills: Mutex::new(skills),
            nodes: Mutex::new(nodes),
            sessions: Mutex::new(vec![session]),
            messages: Mutex::new(messages),
            prompts: Mutex::new(prompts),
        }
    }
}

#[async_trait]
impl DesktopApi for FakeDesktopApi {
    async fn login(&self, input: &AuthLoginInput) -> Result<AuthSessionResponse> {
        Ok(AuthSessionResponse {
            session_token: "token-1".into(),
            account_id: input.account_id.clone(),
        })
    }

    async fn logout(&self, _input: &AuthLogoutInput) -> Result<serde_json::Value> {
        Ok(serde_json::json!({"ok": true}))
    }

    async fn switch_account(&self, input: &AuthSwitchInput) -> Result<AuthSessionResponse> {
        Ok(AuthSessionResponse {
            session_token: "token-2".into(),
            account_id: input.to_account.clone(),
        })
    }

    async fn list_workflows(&self) -> Result<Vec<WorkflowDefinition>> {
        Ok(Vec::new())
    }

    async fn update_workflow_status(
        &self,
        workflow_id: &str,
        status: WorkflowStatus,
    ) -> Result<WorkflowDefinition> {
        Ok(WorkflowDefinition {
            id: workflow_id.to_string(),
            name: workflow_id.to_string(),
            timezone: "Asia/Taipei".into(),
            nodes: Vec::new(),
            edges: Vec::new(),
            version: 1,
            status,
            agent_defaults: None,
        })
    }

    async fn patch_workflow(
        &self,
        workflow_id: &str,
        patch: &PatchWorkflowInput,
    ) -> Result<WorkflowDefinition> {
        Ok(WorkflowDefinition {
            id: workflow_id.to_string(),
            name: patch
                .name
                .clone()
                .unwrap_or_else(|| workflow_id.to_string()),
            timezone: patch
                .timezone
                .clone()
                .unwrap_or_else(|| "Asia/Taipei".into()),
            nodes: Vec::new(),
            edges: Vec::new(),
            version: 2,
            status: WorkflowStatus::Draft,
            agent_defaults: patch.agent_defaults.clone(),
        })
    }

    async fn list_runs(&self, _limit: usize) -> Result<Vec<WorkflowRun>> {
        Ok(self.runs.lock().expect("runs lock").clone())
    }

    async fn list_run_events(
        &self,
        run_id: &str,
        _after_seq: i64,
        _limit: usize,
    ) -> Result<Vec<WorkflowRunEvent>> {
        Ok(self
            .events
            .lock()
            .expect("events lock")
            .get(run_id)
            .cloned()
            .unwrap_or_default())
    }

    async fn list_run_skills(&self, run_id: &str) -> Result<Vec<RunSkillSnapshot>> {
        Ok(self
            .skills
            .lock()
            .expect("skills lock")
            .get(run_id)
            .cloned()
            .unwrap_or_default())
    }

    async fn list_run_nodes(&self, run_id: &str) -> Result<Vec<WorkflowRunNodeState>> {
        Ok(self
            .nodes
            .lock()
            .expect("nodes lock")
            .get(run_id)
            .cloned()
            .unwrap_or_default())
    }

    async fn run_workflow(
        &self,
        workflow_id: &str,
        requested_by: Option<&str>,
    ) -> Result<WorkflowRun> {
        let run = WorkflowRun {
            run_id: format!("run-{}", self.runs.lock().expect("runs lock").len() + 1),
            workflow_id: workflow_id.to_string(),
            requested_by: requested_by.map(ToString::to_string),
            status: RunStatus::Queued,
            started_at: None,
            finished_at: None,
            cancel_requested: false,
            error_message: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        self.runs.lock().expect("runs lock").insert(0, run.clone());
        Ok(run)
    }

    async fn cancel_run(&self, run_id: &str, _requested_by: Option<&str>) -> Result<WorkflowRun> {
        let mut runs = self.runs.lock().expect("runs lock");
        let run = runs
            .iter_mut()
            .find(|item| item.run_id == run_id)
            .expect("run exists");
        run.status = RunStatus::Canceled;
        Ok(run.clone())
    }

    async fn retry_run(&self, run_id: &str, requested_by: Option<&str>) -> Result<WorkflowRun> {
        let workflow_id = self
            .runs
            .lock()
            .expect("runs lock")
            .iter()
            .find(|item| item.run_id == run_id)
            .map(|item| item.workflow_id.clone())
            .unwrap_or_else(|| "wf-1".into());
        self.run_workflow(&workflow_id, requested_by).await
    }

    async fn fs_tree(&self, _path: &str) -> Result<Vec<FsTreeEntry>> {
        Ok(vec![FsTreeEntry {
            path: ".".into(),
            is_dir: true,
        }])
    }

    async fn fs_read(&self, path: &str) -> Result<FsReadResponse> {
        Ok(FsReadResponse {
            path: path.to_string(),
            content_base64: base64::engine::general_purpose::STANDARD.encode("hello"),
        })
    }

    async fn fs_write(&self, _path: &str, _content_base64: String) -> Result<serde_json::Value> {
        Ok(serde_json::json!({"ok": true}))
    }

    async fn fs_move(&self, _from: &str, _to: &str) -> Result<serde_json::Value> {
        Ok(serde_json::json!({"ok": true}))
    }

    async fn fs_delete(&self, _path: &str) -> Result<serde_json::Value> {
        Ok(serde_json::json!({"ok": true}))
    }

    async fn fs_search(
        &self,
        _path: &str,
        _query: &str,
        _limit: usize,
    ) -> Result<Vec<FsSearchMatch>> {
        Ok(Vec::new())
    }

    async fn fs_diff(&self, _left_path: &str, _right_path: &str) -> Result<FsDiffResponse> {
        Ok(FsDiffResponse {
            left_path: "left".into(),
            right_path: "right".into(),
            hunks: Vec::new(),
        })
    }

    async fn terminal_start(&self, _input: &TerminalStartInput) -> Result<TerminalSessionResponse> {
        Ok(TerminalSessionResponse {
            session_id: "terminal-1".into(),
            status: "exited".into(),
            stdout: String::new(),
            stderr: String::new(),
            exit_code: Some(0),
        })
    }

    async fn terminal_session(&self, _session_id: &str) -> Result<TerminalSessionResponse> {
        Ok(TerminalSessionResponse {
            session_id: "terminal-1".into(),
            status: "exited".into(),
            stdout: String::new(),
            stderr: String::new(),
            exit_code: Some(0),
        })
    }

    async fn office_open(&self, path: &str) -> Result<FsReadResponse> {
        self.fs_read(path).await
    }

    async fn office_save(&self, _path: &str, _content_base64: String) -> Result<serde_json::Value> {
        Ok(serde_json::json!({"ok": true}))
    }

    async fn office_versions(&self, path: &str) -> Result<OfficeVersionResponse> {
        Ok(OfficeVersionResponse {
            path: path.to_string(),
            versions: vec!["v1".into()],
        })
    }

    async fn pdf_preview(&self, path: &str) -> Result<FsReadResponse> {
        self.fs_read(path).await
    }

    async fn pdf_annotate(&self, path: &str, _annotation: &str) -> Result<PdfOperationResponse> {
        Ok(PdfOperationResponse {
            path: path.to_string(),
            replaced_count: 0,
            version_name: "v2".into(),
        })
    }

    async fn pdf_replace_text(
        &self,
        path: &str,
        _search: &str,
        _replace: &str,
    ) -> Result<PdfOperationResponse> {
        Ok(PdfOperationResponse {
            path: path.to_string(),
            replaced_count: 1,
            version_name: "v3".into(),
        })
    }

    async fn pdf_save_version(&self, path: &str) -> Result<PdfOperationResponse> {
        Ok(PdfOperationResponse {
            path: path.to_string(),
            replaced_count: 0,
            version_name: "v4".into(),
        })
    }

    async fn list_agent_workspace_sessions(&self) -> Result<Vec<AgentWorkspaceSession>> {
        Ok(self.sessions.lock().expect("sessions lock").clone())
    }

    async fn create_agent_workspace_session(
        &self,
        input: &CreateAgentWorkspaceSessionInput,
    ) -> Result<AgentWorkspaceSession> {
        let mut sessions = self.sessions.lock().expect("sessions lock");
        let created = AgentWorkspaceSession {
            session_id: format!("session-{}", sessions.len() + 1),
            title: input.title.clone(),
            config: input.config.clone().unwrap_or_default(),
            last_active_panel: input.last_active_panel.clone(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        sessions.push(created.clone());
        self.messages
            .lock()
            .expect("messages lock")
            .entry(created.session_id.clone())
            .or_default();
        self.prompts
            .lock()
            .expect("prompts lock")
            .entry(created.session_id.clone())
            .or_default();
        Ok(created)
    }

    async fn list_agent_capabilities(&self) -> Result<Vec<workdesk_core::CodexModelCapability>> {
        Ok(vec![workdesk_core::CodexModelCapability {
            model: "gpt-5.4".into(),
            display_name: "gpt-5.4".into(),
            reasoning_values: vec![workdesk_core::CodexReasoningEffortOption {
                reasoning_effort: "high".into(),
                description: "High reasoning".into(),
            }],
            default_reasoning_effort: Some("high".into()),
            supports_speed: false,
            supports_plan_mode: true,
        }])
    }

    async fn update_agent_workspace_session_config(
        &self,
        session_id: &str,
        config: CodexNativeSessionConfig,
        last_active_panel: Option<&str>,
    ) -> Result<AgentWorkspaceSession> {
        let mut sessions = self.sessions.lock().expect("sessions lock");
        let session = sessions
            .iter_mut()
            .find(|item| item.session_id == session_id)
            .expect("session exists");
        session.config = config;
        session.last_active_panel = last_active_panel.map(ToString::to_string);
        Ok(session.clone())
    }

    async fn list_agent_workspace_messages(
        &self,
        session_id: &str,
    ) -> Result<Vec<AgentWorkspaceMessage>> {
        Ok(self
            .messages
            .lock()
            .expect("messages lock")
            .get(session_id)
            .cloned()
            .unwrap_or_default())
    }

    async fn list_choice_prompts(&self, session_id: &str) -> Result<Vec<ChoicePrompt>> {
        Ok(self
            .prompts
            .lock()
            .expect("prompts lock")
            .get(session_id)
            .cloned()
            .unwrap_or_default())
    }

    async fn append_agent_workspace_message(
        &self,
        session_id: &str,
        input: &AppendAgentWorkspaceMessageInput,
    ) -> Result<AgentWorkspaceMessage> {
        let message = AgentWorkspaceMessage {
            message_id: format!(
                "msg-{}",
                self.messages
                    .lock()
                    .expect("messages lock")
                    .get(session_id)
                    .map(|items| items.len() + 1)
                    .unwrap_or(1)
            ),
            session_id: session_id.to_string(),
            role: input.role.clone(),
            content: input.content.clone(),
            created_at: Utc::now(),
        };
        self.messages
            .lock()
            .expect("messages lock")
            .entry(session_id.to_string())
            .or_default()
            .push(message.clone());
        Ok(message)
    }

    async fn answer_choice_prompt(
        &self,
        session_id: &str,
        prompt_id: &str,
        selected_option_id: Option<&str>,
        freeform_answer: Option<&str>,
    ) -> Result<ChoicePrompt> {
        let mut prompts = self.prompts.lock().expect("prompts lock");
        let prompt = prompts
            .get_mut(session_id)
            .and_then(|items| items.iter_mut().find(|item| item.prompt_id == prompt_id))
            .expect("prompt exists");
        prompt.status = ChoicePromptStatus::Answered;
        prompt.selected_option_id = selected_option_id.map(ToString::to_string);
        prompt.freeform_answer = freeform_answer.map(ToString::to_string);
        prompt.answered_at = Some(Utc::now());
        Ok(prompt.clone())
    }
}

#[tokio::test]
async fn automation_channel_supports_state_and_actions() {
    let endpoint = unique_endpoint("Automation");
    let controller = DesktopAppController::new(Arc::new(FakeDesktopApi::seeded()));
    controller.bootstrap().await.expect("bootstrap");

    let endpoint_for_server = endpoint.clone();
    let server_controller = controller.clone();
    let server_task = tokio::spawn(async move {
        AutomationServer::new(endpoint_for_server)
            .run(Arc::new(server_controller))
            .await
    });
    tokio::time::sleep(Duration::from_millis(120)).await;

    let client = AutomationClient::new(endpoint);
    let state = client.get_state().await.expect("get state");
    assert_eq!(state.runs.len(), 1);

    let state = client
        .dispatch_command(DesktopCommand::OpenRun {
            run_id: "run-1".into(),
        })
        .await
        .expect("dispatch command");
    assert_eq!(state.route, UiRoute::RunDetail);
    assert_eq!(state.selected_run_id.as_deref(), Some("run-1"));
    assert_eq!(state.run_skills.len(), 1);

    let state = client
        .cancel_selected_run()
        .await
        .expect("cancel selected run");
    let current = state
        .runs
        .iter()
        .find(|run| run.run_id == "run-1")
        .expect("run-1 should exist");
    assert!(matches!(current.status, RunStatus::Canceled));

    let state = client
        .retry_selected_run()
        .await
        .expect("retry selected run");
    assert_eq!(state.route, UiRoute::RunDetail);
    assert!(state
        .selected_run_id
        .as_ref()
        .map(|run_id| run_id != "run-1")
        .unwrap_or(false));

    server_task.abort();
}

#[tokio::test]
async fn automation_channel_exposes_pending_choice_prompt_and_answers_it() {
    let endpoint = unique_endpoint("Automation");
    let controller = DesktopAppController::new(Arc::new(FakeDesktopApi::seeded()));
    controller.bootstrap().await.expect("bootstrap");

    let endpoint_for_server = endpoint.clone();
    let server_controller = controller.clone();
    let server_task = tokio::spawn(async move {
        AutomationServer::new(endpoint_for_server)
            .run(Arc::new(server_controller))
            .await
    });
    tokio::time::sleep(Duration::from_millis(120)).await;

    let client = AutomationClient::new(endpoint);
    let prompt = client
        .get_pending_choice_prompt()
        .await
        .expect("get pending choice prompt")
        .expect("pending choice prompt");
    assert_eq!(prompt.prompt_id, "prompt-1");
    assert_eq!(prompt.recommended_option_id.as_deref(), Some("safe"));

    let state = client
        .submit_choice_prompt_option("session-1", "prompt-1", "safe")
        .await
        .expect("submit choice prompt option");
    assert!(state.pending_choice_prompt.is_none());
    assert_eq!(state.active_agent_session_id.as_deref(), Some("session-1"));
    assert_eq!(state.agent_sessions.len(), 1);

    server_task.abort();
}

fn unique_endpoint(namespace: &str) -> String {
    #[cfg(windows)]
    {
        format!(
            r"\\.\pipe\WorkDeskStudio.{}.{}",
            namespace,
            uuid::Uuid::new_v4()
        )
    }
    #[cfg(not(windows))]
    {
        let listener = std::net::TcpListener::bind("127.0.0.1:0")
            .expect("bind ephemeral endpoint for automation test");
        let endpoint = listener.local_addr().expect("local addr").to_string();
        drop(listener);
        endpoint
    }
}
