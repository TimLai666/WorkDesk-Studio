use serde::{Deserialize, Serialize};
use workdesk_core::{
    AgentWorkspaceMessage, AgentWorkspaceSession, AuthSessionResponse, ChoicePrompt,
    CodexModelCapability, FsDiffResponse, FsSearchMatch, FsTreeEntry, PdfOperationResponse,
    RunSkillSnapshot, TerminalSessionResponse, WorkflowDefinition, WorkflowEdge, WorkflowNodeKind,
    WorkflowRun, WorkflowRunEvent, WorkflowRunNodeState,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UiRoute {
    Workbench,
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
    pub office_embed_url: Option<String>,
    pub office_versions: Vec<String>,
    pub pdf_last_operation: Option<PdfOperationResponse>,
    pub diagnostics: Vec<UiDiagnostic>,
    pub auth_account_id: Option<String>,
    pub auth_session_token: Option<String>,
    pub agent_sessions: Vec<AgentWorkspaceSession>,
    pub active_agent_session_id: Option<String>,
    pub agent_messages: Vec<AgentWorkspaceMessage>,
    pub choice_prompts: Vec<ChoicePrompt>,
    pub pending_choice_prompt: Option<ChoicePrompt>,
    pub model_capabilities: Vec<CodexModelCapability>,
    pub last_error: Option<String>,
}

impl Default for UiStateSnapshot {
    fn default() -> Self {
        Self {
            revision: 0,
            focus_seq: 0,
            route: UiRoute::Workbench,
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
            office_embed_url: None,
            office_versions: Vec::new(),
            pdf_last_operation: None,
            diagnostics: Vec::new(),
            auth_account_id: None,
            auth_session_token: None,
            agent_sessions: Vec::new(),
            active_agent_session_id: None,
            agent_messages: Vec::new(),
            choice_prompts: Vec::new(),
            pending_choice_prompt: None,
            model_capabilities: Vec::new(),
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
        embed_url: Option<String>,
        versions: Vec<String>,
        pdf_last_operation: Option<PdfOperationResponse>,
    },
    SetDiagnostics(Vec<UiDiagnostic>),
    SetAuthSession(Option<AuthSessionResponse>),
    SetAgentSessions(Vec<AgentWorkspaceSession>),
    SelectAgentSession(Option<String>),
    SetAgentMessages(Vec<AgentWorkspaceMessage>),
    SetChoicePrompts(Vec<ChoicePrompt>),
    SetModelCapabilities(Vec<CodexModelCapability>),
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
            embed_url,
            versions,
            pdf_last_operation,
        } => {
            state.office_path = path;
            state.office_content_base64 = content_base64;
            state.office_editor_text = editor_text;
            state.office_embed_url = embed_url;
            state.office_versions = versions;
            state.pdf_last_operation = pdf_last_operation;
        }
        ControllerAction::SetDiagnostics(diagnostics) => state.diagnostics = diagnostics,
        ControllerAction::SetAuthSession(session) => {
            state.auth_account_id = session.as_ref().map(|value| value.account_id.clone());
            state.auth_session_token = session.map(|value| value.session_token);
        }
        ControllerAction::SetAgentSessions(sessions) => {
            state.agent_sessions = sessions;
            if state.active_agent_session_id.is_none() {
                state.active_agent_session_id = state
                    .agent_sessions
                    .first()
                    .map(|session| session.session_id.clone());
            }
        }
        ControllerAction::SelectAgentSession(session_id) => {
            state.active_agent_session_id = session_id
        }
        ControllerAction::SetAgentMessages(messages) => state.agent_messages = messages,
        ControllerAction::SetChoicePrompts(prompts) => {
            state.pending_choice_prompt = prompts
                .iter()
                .find(|prompt| matches!(prompt.status, workdesk_core::ChoicePromptStatus::Pending))
                .cloned();
            state.choice_prompts = prompts;
        }
        ControllerAction::SetModelCapabilities(capabilities) => {
            state.model_capabilities = capabilities
        }
        ControllerAction::SetError(error) => state.last_error = error,
    }
    state.revision += 1;
}
