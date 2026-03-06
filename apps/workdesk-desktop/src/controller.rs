
use crate::api_client::ApiClient;
use crate::command::DesktopCommand;
use crate::command_bus::CommandDispatcher;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use workdesk_core::{
    FsDiffResponse, FsSearchMatch, FsTreeEntry, PdfOperationResponse, RunSkillSnapshot,
    TerminalSessionResponse, TerminalStartInput, WorkflowDefinition, WorkflowEdge, WorkflowNodeKind,
    WorkflowRun, WorkflowRunEvent, WorkflowRunNodeState, WorkflowStatus,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UiRoute {
    RunList,
    RunDetail,
    WorkflowDetail,
    FileManager,
    OfficeDesk,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UiDiagnostic {
    pub code: String,
    pub message: String,
    pub run_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CanvasNodeState {
    pub id: String,
    pub kind: WorkflowNodeKind,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiStateSnapshot {
    pub revision: u64,
    pub focus_seq: u64,
    pub route: UiRoute,
    pub selected_run_id: Option<String>,
    pub selected_workflow_id: Option<String>,
    pub workflows: Vec<WorkflowDefinition>,
    pub runs: Vec<WorkflowRun>,
    pub run_events: Vec<WorkflowRunEvent>,
    pub run_nodes: Vec<WorkflowRunNodeState>,
    pub run_skills: Vec<RunSkillSnapshot>,
    pub canvas_nodes: Vec<CanvasNodeState>,
    pub canvas_edges: Vec<WorkflowEdge>,
    pub selected_canvas_nodes: Vec<String>,
    pub canvas_undo_depth: usize,
    pub canvas_redo_depth: usize,
    pub workspace_entries: Vec<FsTreeEntry>,
    pub current_file_path: Option<String>,
    pub current_file_content: String,
    pub file_search_results: Vec<FsSearchMatch>,
    pub diff_result: Option<FsDiffResponse>,
    pub terminal_session: Option<TerminalSessionResponse>,
    pub office_path: Option<String>,
    pub office_content_base64: Option<String>,
    pub office_editor_text: String,
    pub office_versions: Vec<String>,
    pub pdf_last_operation: Option<PdfOperationResponse>,
    pub diagnostics: Vec<UiDiagnostic>,
    pub last_error: Option<String>,
}

impl Default for UiStateSnapshot {
    fn default() -> Self {
        Self {
            revision: 0,
            focus_seq: 0,
            route: UiRoute::RunList,
            selected_run_id: None,
            selected_workflow_id: None,
            workflows: Vec::new(),
            runs: Vec::new(),
            run_events: Vec::new(),
            run_nodes: Vec::new(),
            run_skills: Vec::new(),
            canvas_nodes: Vec::new(),
            canvas_edges: Vec::new(),
            selected_canvas_nodes: Vec::new(),
            canvas_undo_depth: 0,
            canvas_redo_depth: 0,
            workspace_entries: Vec::new(),
            current_file_path: None,
            current_file_content: String::new(),
            file_search_results: Vec::new(),
            diff_result: None,
            terminal_session: None,
            office_path: None,
            office_content_base64: None,
            office_editor_text: String::new(),
            office_versions: Vec::new(),
            pdf_last_operation: None,
            diagnostics: Vec::new(),
            last_error: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ControllerAction {
    FocusWindow,
    SetRoute(UiRoute),
    SelectRun(Option<String>),
    SelectWorkflow(Option<String>),
    SetRuns(Vec<WorkflowRun>),
    SetRunDetails {
        events: Vec<WorkflowRunEvent>,
        nodes: Vec<WorkflowRunNodeState>,
        skills: Vec<RunSkillSnapshot>,
    },
    SetWorkflows(Vec<WorkflowDefinition>),
    SetCanvas {
        nodes: Vec<CanvasNodeState>,
        edges: Vec<WorkflowEdge>,
        selected: Vec<String>,
    },
    SetCanvasHistoryDepth {
        undo_depth: usize,
        redo_depth: usize,
    },
    SetWorkspaceEntries(Vec<FsTreeEntry>),
    SetCurrentFile {
        path: Option<String>,
        content: String,
    },
    SetFileSearchResults(Vec<FsSearchMatch>),
    SetDiffResult(Option<FsDiffResponse>),
    SetTerminalSession(Option<TerminalSessionResponse>),
    SetOffice {
        path: Option<String>,
        content_base64: Option<String>,
        editor_text: String,
        versions: Vec<String>,
        pdf_last_operation: Option<PdfOperationResponse>,
    },
    SetDiagnostics(Vec<UiDiagnostic>),
    SetError(Option<String>),
}

pub fn reduce_ui_state(state: &mut UiStateSnapshot, action: ControllerAction) {
    match action {
        ControllerAction::FocusWindow => state.focus_seq += 1,
        ControllerAction::SetRoute(route) => state.route = route,
        ControllerAction::SelectRun(run_id) => state.selected_run_id = run_id,
        ControllerAction::SelectWorkflow(workflow_id) => state.selected_workflow_id = workflow_id,
        ControllerAction::SetRuns(runs) => state.runs = runs,
        ControllerAction::SetRunDetails {
            events,
            nodes,
            skills,
        } => {
            state.run_events = events;
            state.run_nodes = nodes;
            state.run_skills = skills;
        }
        ControllerAction::SetWorkflows(workflows) => state.workflows = workflows,
        ControllerAction::SetCanvas {
            nodes,
            edges,
            selected,
        } => {
            state.canvas_nodes = nodes;
            state.canvas_edges = edges;
            state.selected_canvas_nodes = selected;
        }
        ControllerAction::SetCanvasHistoryDepth {
            undo_depth,
            redo_depth,
        } => {
            state.canvas_undo_depth = undo_depth;
            state.canvas_redo_depth = redo_depth;
        }
        ControllerAction::SetWorkspaceEntries(entries) => state.workspace_entries = entries,
        ControllerAction::SetCurrentFile { path, content } => {
            state.current_file_path = path;
            state.current_file_content = content;
        }
        ControllerAction::SetFileSearchResults(results) => state.file_search_results = results,
        ControllerAction::SetDiffResult(diff) => state.diff_result = diff,
        ControllerAction::SetTerminalSession(session) => state.terminal_session = session,
        ControllerAction::SetOffice {
            path,
            content_base64,
            editor_text,
            versions,
            pdf_last_operation,
        } => {
            state.office_path = path;
            state.office_content_base64 = content_base64;
            state.office_editor_text = editor_text;
            state.office_versions = versions;
            state.pdf_last_operation = pdf_last_operation;
        }
        ControllerAction::SetDiagnostics(diagnostics) => state.diagnostics = diagnostics,
        ControllerAction::SetError(error) => state.last_error = error,
    }
    state.revision += 1;
}

#[derive(Debug, Clone)]
struct CanvasSnapshot {
    nodes: Vec<CanvasNodeState>,
    edges: Vec<WorkflowEdge>,
    selected: Vec<String>,
}

#[derive(Debug, Clone, Default)]
struct CanvasHistory {
    past: Vec<CanvasSnapshot>,
    future: Vec<CanvasSnapshot>,
}
#[async_trait]
pub trait DesktopApi: Send + Sync {
    async fn list_workflows(&self) -> Result<Vec<WorkflowDefinition>>;
    async fn update_workflow_status(
        &self,
        workflow_id: &str,
        status: WorkflowStatus,
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
}

#[async_trait]
impl DesktopApi for ApiClient {
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
}
#[derive(Clone)]
pub struct DesktopAppController {
    api: Arc<dyn DesktopApi>,
    state: Arc<RwLock<UiStateSnapshot>>,
    runtime_diagnostics: Arc<RwLock<HashMap<String, UiDiagnostic>>>,
    canvas_history: Arc<RwLock<CanvasHistory>>,
}

impl DesktopAppController {
    pub fn new(api: Arc<dyn DesktopApi>) -> Self {
        Self {
            api,
            state: Arc::new(RwLock::new(UiStateSnapshot::default())),
            runtime_diagnostics: Arc::new(RwLock::new(HashMap::new())),
            canvas_history: Arc::new(RwLock::new(CanvasHistory::default())),
        }
    }

    pub fn snapshot(&self) -> UiStateSnapshot {
        self.state.read().expect("ui state read lock").clone()
    }

    pub fn shared_state(&self) -> Arc<RwLock<UiStateSnapshot>> {
        self.state.clone()
    }

    pub fn set_runtime_diagnostic(&self, source: &str, diagnostic: Option<UiDiagnostic>) {
        {
            let mut runtime = self
                .runtime_diagnostics
                .write()
                .expect("runtime diagnostics write lock");
            if let Some(item) = diagnostic {
                runtime.insert(source.to_string(), item);
            } else {
                runtime.remove(source);
            }
        }
        self.sync_diagnostics();
    }

    pub async fn bootstrap(&self) -> Result<()> {
        self.refresh_workflows().await?;
        self.refresh_runs().await?;
        Ok(())
    }

    pub async fn dispatch_command(&self, command: DesktopCommand) -> Result<()> {
        self.apply(ControllerAction::FocusWindow);
        self.apply(ControllerAction::SetError(None));

        let result = match command {
            DesktopCommand::Open => {
                self.apply(ControllerAction::SetRoute(UiRoute::RunList));
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

    pub async fn refresh_runs(&self) -> Result<()> {
        let runs = self.api.list_runs(200).await?;
        self.apply(ControllerAction::SetRuns(runs));
        self.sync_diagnostics();
        Ok(())
    }

    pub async fn refresh_workflows(&self) -> Result<()> {
        let workflows = self.api.list_workflows().await?;
        self.apply(ControllerAction::SetWorkflows(workflows));
        Ok(())
    }

    pub fn navigate(&self, route: UiRoute) {
        self.apply(ControllerAction::SetRoute(route));
    }

    pub async fn refresh_selected_run_detail(&self) -> Result<()> {
        let run_id = self
            .snapshot()
            .selected_run_id
            .ok_or_else(|| anyhow!("no run selected"))?;
        self.refresh_run_detail(&run_id).await
    }

    pub async fn cancel_selected_run(&self) -> Result<()> {
        let run_id = self
            .snapshot()
            .selected_run_id
            .ok_or_else(|| anyhow!("no run selected"))?;
        self.api.cancel_run(&run_id, Some("desktop-ui")).await?;
        self.refresh_runs().await?;
        self.refresh_run_detail(&run_id).await?;
        Ok(())
    }

    pub async fn retry_selected_run(&self) -> Result<()> {
        let run_id = self
            .snapshot()
            .selected_run_id
            .ok_or_else(|| anyhow!("no run selected"))?;
        let retry = self.api.retry_run(&run_id, Some("desktop-ui")).await?;
        self.apply(ControllerAction::SelectRun(Some(retry.run_id.clone())));
        self.apply(ControllerAction::SetRoute(UiRoute::RunDetail));
        self.refresh_runs().await?;
        self.refresh_run_detail(&retry.run_id).await?;
        Ok(())
    }

    pub async fn open_file_manager(&self, root: &str) -> Result<()> {
        self.apply(ControllerAction::SetRoute(UiRoute::FileManager));
        let entries = self.api.fs_tree(root).await?;
        self.apply(ControllerAction::SetWorkspaceEntries(entries));
        Ok(())
    }

    pub async fn open_file(&self, path: &str) -> Result<()> {
        self.apply(ControllerAction::SetRoute(UiRoute::FileManager));
        let file = self.api.fs_read(path).await?;
        let raw = STANDARD.decode(file.content_base64.as_bytes())?;
        let text = String::from_utf8_lossy(&raw).to_string();
        self.apply(ControllerAction::SetCurrentFile {
            path: Some(path.to_string()),
            content: text,
        });
        Ok(())
    }

    pub async fn save_current_file(&self) -> Result<()> {
        let snapshot = self.snapshot();
        let path = snapshot
            .current_file_path
            .ok_or_else(|| anyhow!("no file selected"))?;
        let content_base64 = STANDARD.encode(snapshot.current_file_content.as_bytes());
        self.api.fs_write(&path, content_base64).await?;
        Ok(())
    }

    pub fn set_current_file_content(&self, content: String) {
        let path = self.snapshot().current_file_path;
        self.apply(ControllerAction::SetCurrentFile { path, content });
    }

    pub async fn create_file(&self, path: &str, content: &str) -> Result<()> {
        self.api
            .fs_write(path, STANDARD.encode(content.as_bytes()))
            .await?;
        let root = self.workspace_root_from_entries();
        let entries = self.api.fs_tree(&root).await?;
        self.apply(ControllerAction::SetWorkspaceEntries(entries));
        Ok(())
    }

    pub async fn move_path(&self, from: &str, to: &str) -> Result<()> {
        self.api.fs_move(from, to).await?;
        let root = self.workspace_root_from_entries();
        let entries = self.api.fs_tree(&root).await?;
        self.apply(ControllerAction::SetWorkspaceEntries(entries));
        Ok(())
    }

    pub async fn delete_path(&self, path: &str) -> Result<()> {
        self.api.fs_delete(path).await?;
        let root = self.workspace_root_from_entries();
        let entries = self.api.fs_tree(&root).await?;
        self.apply(ControllerAction::SetWorkspaceEntries(entries));
        Ok(())
    }

    pub async fn search_files(&self, root: &str, query: &str) -> Result<()> {
        let results = self.api.fs_search(root, query, 500).await?;
        self.apply(ControllerAction::SetFileSearchResults(results));
        Ok(())
    }

    pub async fn diff_files(&self, left_path: &str, right_path: &str) -> Result<()> {
        let diff = self.api.fs_diff(left_path, right_path).await?;
        self.apply(ControllerAction::SetDiffResult(Some(diff)));
        Ok(())
    }

    pub async fn run_terminal(&self, path: &str, command: &str) -> Result<()> {
        let session = self
            .api
            .terminal_start(&TerminalStartInput {
                path: path.to_string(),
                command: command.to_string(),
            })
            .await?;
        let session = self.api.terminal_session(&session.session_id).await?;
        self.apply(ControllerAction::SetTerminalSession(Some(session)));
        Ok(())
    }
    pub async fn open_office_document(&self, path: &str) -> Result<()> {
        self.apply(ControllerAction::SetRoute(UiRoute::OfficeDesk));
        let response = self.api.office_open(path).await?;
        let versions = self
            .api
            .office_versions(path)
            .await
            .map(|payload| payload.versions)
            .unwrap_or_default();
        let raw = STANDARD.decode(response.content_base64.as_bytes())?;
        let editor_text = String::from_utf8_lossy(&raw).to_string();
        self.apply(ControllerAction::SetOffice {
            path: Some(path.to_string()),
            content_base64: Some(response.content_base64),
            editor_text,
            versions,
            pdf_last_operation: None,
        });
        Ok(())
    }

    pub fn set_office_editor_text(&self, text: String) {
        let snapshot = self.snapshot();
        self.apply(ControllerAction::SetOffice {
            path: snapshot.office_path,
            content_base64: snapshot.office_content_base64,
            editor_text: text,
            versions: snapshot.office_versions,
            pdf_last_operation: snapshot.pdf_last_operation,
        });
    }

    pub async fn save_office_document(&self) -> Result<()> {
        let snapshot = self.snapshot();
        let path = snapshot
            .office_path
            .clone()
            .ok_or_else(|| anyhow!("no office document selected"))?;
        let base64_content = STANDARD.encode(snapshot.office_editor_text.as_bytes());
        self.api.office_save(&path, base64_content.clone()).await?;
        let versions = self
            .api
            .office_versions(&path)
            .await
            .map(|payload| payload.versions)
            .unwrap_or_default();
        self.apply(ControllerAction::SetOffice {
            path: Some(path),
            content_base64: Some(base64_content),
            editor_text: snapshot.office_editor_text,
            versions,
            pdf_last_operation: snapshot.pdf_last_operation,
        });
        Ok(())
    }

    pub async fn preview_pdf(&self, path: &str) -> Result<()> {
        self.apply(ControllerAction::SetRoute(UiRoute::OfficeDesk));
        let preview = self.api.pdf_preview(path).await?;
        let versions = self
            .api
            .office_versions(path)
            .await
            .map(|payload| payload.versions)
            .unwrap_or_default();
        let raw = STANDARD.decode(preview.content_base64.as_bytes())?;
        self.apply(ControllerAction::SetOffice {
            path: Some(path.to_string()),
            content_base64: Some(preview.content_base64),
            editor_text: String::from_utf8_lossy(&raw).to_string(),
            versions,
            pdf_last_operation: None,
        });
        Ok(())
    }

    pub async fn annotate_pdf(&self, annotation: &str) -> Result<()> {
        let path = self
            .snapshot()
            .office_path
            .ok_or_else(|| anyhow!("no PDF selected"))?;
        let operation = self.api.pdf_annotate(&path, annotation).await?;
        let versions = self
            .api
            .office_versions(&path)
            .await
            .map(|payload| payload.versions)
            .unwrap_or_default();
        let snapshot = self.snapshot();
        self.apply(ControllerAction::SetOffice {
            path: Some(path),
            content_base64: snapshot.office_content_base64,
            editor_text: snapshot.office_editor_text,
            versions,
            pdf_last_operation: Some(operation),
        });
        Ok(())
    }

    pub async fn replace_pdf_text(&self, search: &str, replace: &str) -> Result<()> {
        let path = self
            .snapshot()
            .office_path
            .ok_or_else(|| anyhow!("no PDF selected"))?;
        let operation = self.api.pdf_replace_text(&path, search, replace).await?;
        self.preview_pdf(&path).await?;
        let snapshot = self.snapshot();
        self.apply(ControllerAction::SetOffice {
            path: Some(path),
            content_base64: snapshot.office_content_base64,
            editor_text: snapshot.office_editor_text,
            versions: snapshot.office_versions,
            pdf_last_operation: Some(operation),
        });
        Ok(())
    }

    pub async fn save_pdf_version(&self) -> Result<()> {
        let path = self
            .snapshot()
            .office_path
            .ok_or_else(|| anyhow!("no PDF selected"))?;
        let operation = self.api.pdf_save_version(&path).await?;
        let versions = self
            .api
            .office_versions(&path)
            .await
            .map(|payload| payload.versions)
            .unwrap_or_default();
        let snapshot = self.snapshot();
        self.apply(ControllerAction::SetOffice {
            path: Some(path),
            content_base64: snapshot.office_content_base64,
            editor_text: snapshot.office_editor_text,
            versions,
            pdf_last_operation: Some(operation),
        });
        Ok(())
    }

    pub fn load_canvas_for_selected_workflow(&self) -> Result<()> {
        let snapshot = self.snapshot();
        let workflow_id = snapshot
            .selected_workflow_id
            .ok_or_else(|| anyhow!("no workflow selected"))?;
        let workflow = snapshot
            .workflows
            .iter()
            .find(|workflow| workflow.id == workflow_id)
            .cloned()
            .ok_or_else(|| anyhow!("selected workflow not loaded"))?;
        let (nodes, edges) = canvas_from_workflow(&workflow);
        {
            let mut history = self
                .canvas_history
                .write()
                .expect("canvas history write lock");
            history.past.clear();
            history.future.clear();
        }
        self.apply(ControllerAction::SetCanvas {
            nodes,
            edges,
            selected: Vec::new(),
        });
        self.apply(ControllerAction::SetCanvasHistoryDepth {
            undo_depth: 0,
            redo_depth: 0,
        });
        Ok(())
    }

    pub fn canvas_add_node(&self, kind: WorkflowNodeKind) {
        self.capture_canvas_for_undo();
        let mut snapshot = self.snapshot();
        let index = snapshot.canvas_nodes.len() as f32;
        let node_id = format!("{:?}_{}", kind, snapshot.canvas_nodes.len() + 1)
            .to_lowercase()
            .replace(' ', "_");
        snapshot.canvas_nodes.push(CanvasNodeState {
            id: node_id.clone(),
            kind,
            x: 80.0 + (index * 140.0),
            y: 120.0 + ((index as i32 % 3) as f32 * 110.0),
        });
        snapshot.selected_canvas_nodes = vec![node_id];
        self.apply(ControllerAction::SetCanvas {
            nodes: snapshot.canvas_nodes,
            edges: snapshot.canvas_edges,
            selected: snapshot.selected_canvas_nodes,
        });
    }

    pub fn canvas_move_selected(&self, dx: f32, dy: f32) {
        self.capture_canvas_for_undo();
        let mut snapshot = self.snapshot();
        let selected = snapshot.selected_canvas_nodes.clone();
        for node in &mut snapshot.canvas_nodes {
            if selected.iter().any(|item| item == &node.id) {
                node.x += dx;
                node.y += dy;
            }
        }
        self.apply(ControllerAction::SetCanvas {
            nodes: snapshot.canvas_nodes,
            edges: snapshot.canvas_edges,
            selected: snapshot.selected_canvas_nodes,
        });
    }

    pub fn canvas_align_left(&self) {
        let snapshot = self.snapshot();
        let selected: Vec<&CanvasNodeState> = snapshot
            .canvas_nodes
            .iter()
            .filter(|node| snapshot.selected_canvas_nodes.iter().any(|id| id == &node.id))
            .collect();
        if selected.len() < 2 {
            return;
        }
        let target_x = selected
            .iter()
            .map(|node| node.x)
            .fold(f32::INFINITY, f32::min);
        self.capture_canvas_for_undo();
        let mut updated = self.snapshot();
        let selected = updated.selected_canvas_nodes.clone();
        for node in &mut updated.canvas_nodes {
            if selected.iter().any(|id| id == &node.id) {
                node.x = target_x;
            }
        }
        self.apply(ControllerAction::SetCanvas {
            nodes: updated.canvas_nodes,
            edges: updated.canvas_edges,
            selected: updated.selected_canvas_nodes,
        });
    }

    pub fn canvas_undo(&self) {
        let current = self.current_canvas_snapshot();
        let previous = {
            let mut history = self
                .canvas_history
                .write()
                .expect("canvas history write lock");
            let Some(previous) = history.past.pop() else {
                return;
            };
            history.future.push(current);
            previous
        };
        self.apply_canvas_snapshot(previous);
        self.refresh_canvas_depth();
    }

    pub fn canvas_redo(&self) {
        let current = self.current_canvas_snapshot();
        let next = {
            let mut history = self
                .canvas_history
                .write()
                .expect("canvas history write lock");
            let Some(next) = history.future.pop() else {
                return;
            };
            history.past.push(current);
            next
        };
        self.apply_canvas_snapshot(next);
        self.refresh_canvas_depth();
    }

    pub async fn publish_selected_workflow(&self) -> Result<()> {
        let workflow_id = self
            .snapshot()
            .selected_workflow_id
            .ok_or_else(|| anyhow!("no workflow selected"))?;
        let _ = self
            .api
            .update_workflow_status(&workflow_id, WorkflowStatus::Active)
            .await?;
        self.refresh_workflows().await?;
        Ok(())
    }

    fn apply(&self, action: ControllerAction) {
        let mut state = self.state.write().expect("ui state write lock");
        reduce_ui_state(&mut state, action);
    }

    fn sync_diagnostics(&self) {
        let runs = self.snapshot().runs;
        let mut diagnostics = Self::derive_run_diagnostics(&runs);
        diagnostics.extend(
            self.runtime_diagnostics
                .read()
                .expect("runtime diagnostics read lock")
                .values()
                .cloned(),
        );
        diagnostics.sort_by(|a, b| a.code.cmp(&b.code).then(a.message.cmp(&b.message)));
        self.apply(ControllerAction::SetDiagnostics(diagnostics));
    }

    fn derive_run_diagnostics(runs: &[WorkflowRun]) -> Vec<UiDiagnostic> {
        let now = Utc::now();
        runs.iter()
            .filter_map(|run| {
                let queued_too_long = matches!(run.status, workdesk_core::RunStatus::Queued)
                    && (now - run.created_at).num_seconds() >= 90;
                queued_too_long.then(|| UiDiagnostic {
                    code: "RUNNER_UNAVAILABLE".to_string(),
                    message: format!(
                        "Run {} has been queued for over 90 seconds. Check runner process status.",
                        run.run_id
                    ),
                    run_id: Some(run.run_id.clone()),
                })
            })
            .collect()
    }

    fn capture_canvas_for_undo(&self) {
        let current = self.current_canvas_snapshot();
        {
            let mut history = self
                .canvas_history
                .write()
                .expect("canvas history write lock");
            history.past.push(current);
            if history.past.len() > 100 {
                history.past.remove(0);
            }
            history.future.clear();
        }
        self.refresh_canvas_depth();
    }

    fn current_canvas_snapshot(&self) -> CanvasSnapshot {
        let snapshot = self.snapshot();
        CanvasSnapshot {
            nodes: snapshot.canvas_nodes,
            edges: snapshot.canvas_edges,
            selected: snapshot.selected_canvas_nodes,
        }
    }

    fn apply_canvas_snapshot(&self, snapshot: CanvasSnapshot) {
        self.apply(ControllerAction::SetCanvas {
            nodes: snapshot.nodes,
            edges: snapshot.edges,
            selected: snapshot.selected,
        });
    }

    fn refresh_canvas_depth(&self) {
        let (undo_depth, redo_depth) = {
            let history = self
                .canvas_history
                .read()
                .expect("canvas history read lock");
            (history.past.len(), history.future.len())
        };
        self.apply(ControllerAction::SetCanvasHistoryDepth {
            undo_depth,
            redo_depth,
        });
    }

    async fn refresh_run_detail(&self, run_id: &str) -> Result<()> {
        let events = self.api.list_run_events(run_id, 0, 2000).await?;
        let nodes = self.api.list_run_nodes(run_id).await?;
        let skills = self.api.list_run_skills(run_id).await?;
        self.apply(ControllerAction::SetRunDetails {
            events,
            nodes,
            skills,
        });
        Ok(())
    }

    fn workspace_root_from_entries(&self) -> String {
        let snapshot = self.snapshot();
        if snapshot.workspace_entries.is_empty() {
            ".".to_string()
        } else {
            snapshot
                .workspace_entries
                .iter()
                .map(|entry| entry.path.as_str())
                .min()
                .unwrap_or(".")
                .to_string()
        }
    }
}

#[async_trait]
impl CommandDispatcher for DesktopAppController {
    async fn dispatch(&self, command: DesktopCommand) -> Result<()> {
        self.dispatch_command(command).await
    }
}

fn canvas_from_workflow(workflow: &WorkflowDefinition) -> (Vec<CanvasNodeState>, Vec<WorkflowEdge>) {
    let nodes = workflow
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| CanvasNodeState {
            id: node.id.clone(),
            kind: node.kind.clone(),
            x: 80.0 + ((index % 5) as f32 * 180.0),
            y: 90.0 + ((index / 5) as f32 * 140.0),
        })
        .collect();
    (nodes, workflow.edges.clone())
}
