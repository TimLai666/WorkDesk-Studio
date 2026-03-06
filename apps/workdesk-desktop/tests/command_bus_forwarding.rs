use anyhow::Result;
use async_trait::async_trait;
use base64::Engine;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use workdesk_core::{
    AgentWorkspaceMessage, AgentWorkspaceSession, ChoicePrompt, CodexModelCapability,
    CodexNativeSessionConfig, FsDiffResponse, FsReadResponse, FsSearchMatch, FsTreeEntry,
    OfficeVersionResponse, PdfOperationResponse, RunNodeStatus, RunSkillSnapshot, RunStatus,
    Scope, TerminalSessionResponse, TerminalStartInput, WorkflowDefinition, WorkflowRun,
    WorkflowRunEvent, WorkflowRunNodeState, WorkflowStatus,
};
use workdesk_desktop::command::DesktopCommand;
use workdesk_desktop::command_bus::{CommandBusClient, CommandBusServer};
use workdesk_desktop::controller::{DesktopApi, DesktopAppController, UiRoute};

#[derive(Default)]
struct FakeDesktopApi {
    runs: Mutex<Vec<WorkflowRun>>,
    events: Mutex<HashMap<String, Vec<WorkflowRunEvent>>>,
    skills: Mutex<HashMap<String, Vec<RunSkillSnapshot>>>,
    nodes: Mutex<HashMap<String, Vec<WorkflowRunNodeState>>>,
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
        Self {
            runs: Mutex::new(vec![run]),
            events: Mutex::new(events),
            skills: Mutex::new(skills),
            nodes: Mutex::new(nodes),
        }
    }
}

#[async_trait]
impl DesktopApi for FakeDesktopApi {
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
            run_id: "run-new".into(),
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

    async fn office_save(
        &self,
        _path: &str,
        _content_base64: String,
    ) -> Result<serde_json::Value> {
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

    async fn list_agent_capabilities(&self) -> Result<Vec<CodexModelCapability>> {
        Ok(Vec::new())
    }

    async fn list_agent_workspace_sessions(&self) -> Result<Vec<AgentWorkspaceSession>> {
        Ok(Vec::new())
    }

    async fn update_agent_workspace_session_config(
        &self,
        _session_id: &str,
        config: CodexNativeSessionConfig,
        last_active_panel: Option<&str>,
    ) -> Result<AgentWorkspaceSession> {
        Ok(AgentWorkspaceSession {
            session_id: "session-1".into(),
            title: "Workbench".into(),
            config,
            last_active_panel: last_active_panel.map(ToString::to_string),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    async fn list_agent_workspace_messages(
        &self,
        _session_id: &str,
    ) -> Result<Vec<AgentWorkspaceMessage>> {
        Ok(Vec::new())
    }

    async fn list_choice_prompts(&self, _session_id: &str) -> Result<Vec<ChoicePrompt>> {
        Ok(Vec::new())
    }

    async fn answer_choice_prompt(
        &self,
        _session_id: &str,
        _prompt_id: &str,
        _selected_option_id: Option<&str>,
        _freeform_answer: Option<&str>,
    ) -> Result<ChoicePrompt> {
        anyhow::bail!("no prompt available")
    }
}

#[tokio::test]
async fn secondary_command_is_forwarded_to_primary_controller() {
    let endpoint = unique_endpoint("CommandBus");
    let api = Arc::new(FakeDesktopApi::seeded());
    let controller = DesktopAppController::new(api);
    let server_controller = controller.clone();
    let endpoint_for_server = endpoint.clone();
    let server_task = tokio::spawn(async move {
        CommandBusServer::new(endpoint_for_server)
            .run(Arc::new(server_controller))
            .await
    });

    tokio::time::sleep(Duration::from_millis(120)).await;

    let client = CommandBusClient::new(endpoint);
    let response = client
        .send(&DesktopCommand::OpenRun {
            run_id: "run-1".into(),
        })
        .await
        .expect("send command");
    assert!(response.ok);

    tokio::time::sleep(Duration::from_millis(120)).await;
    let snapshot = controller.snapshot();
    assert_eq!(snapshot.route, UiRoute::RunDetail);
    assert_eq!(snapshot.selected_run_id.as_deref(), Some("run-1"));
    assert_eq!(snapshot.run_skills.len(), 1);

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
            .expect("bind ephemeral endpoint for command bus test");
        let endpoint = listener.local_addr().expect("local addr").to_string();
        drop(listener);
        endpoint
    }
}
