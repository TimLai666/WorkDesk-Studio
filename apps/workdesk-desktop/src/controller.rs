use crate::api_client::ApiClient;
use crate::command::DesktopCommand;
use crate::command_bus::CommandDispatcher;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use workdesk_core::{RunSkillSnapshot, WorkflowRun, WorkflowRunEvent, WorkflowRunNodeState};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UiRoute {
    RunList,
    RunDetail,
    WorkflowDetail,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UiDiagnostic {
    pub code: String,
    pub message: String,
    pub run_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiStateSnapshot {
    pub revision: u64,
    pub focus_seq: u64,
    pub route: UiRoute,
    pub selected_run_id: Option<String>,
    pub selected_workflow_id: Option<String>,
    pub runs: Vec<WorkflowRun>,
    pub run_events: Vec<WorkflowRunEvent>,
    pub run_nodes: Vec<WorkflowRunNodeState>,
    pub run_skills: Vec<RunSkillSnapshot>,
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
            runs: Vec::new(),
            run_events: Vec::new(),
            run_nodes: Vec::new(),
            run_skills: Vec::new(),
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
        ControllerAction::SetDiagnostics(diagnostics) => state.diagnostics = diagnostics,
        ControllerAction::SetError(error) => state.last_error = error,
    }
    state.revision += 1;
}

#[async_trait]
pub trait DesktopApi: Send + Sync {
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
}

#[async_trait]
impl DesktopApi for ApiClient {
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
}

#[derive(Clone)]
pub struct DesktopAppController {
    api: Arc<dyn DesktopApi>,
    state: Arc<RwLock<UiStateSnapshot>>,
}

impl DesktopAppController {
    pub fn new(api: Arc<dyn DesktopApi>) -> Self {
        Self {
            api,
            state: Arc::new(RwLock::new(UiStateSnapshot::default())),
        }
    }

    pub fn snapshot(&self) -> UiStateSnapshot {
        self.state.read().expect("ui state read lock").clone()
    }

    pub fn shared_state(&self) -> Arc<RwLock<UiStateSnapshot>> {
        self.state.clone()
    }

    pub async fn bootstrap(&self) -> Result<()> {
        self.refresh_runs().await
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
                self.apply(ControllerAction::SelectWorkflow(Some(workflow_id)));
                Ok(())
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
        let diagnostics = Self::derive_diagnostics(&runs);
        self.apply(ControllerAction::SetRuns(runs));
        self.apply(ControllerAction::SetDiagnostics(diagnostics));
        Ok(())
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

    fn apply(&self, action: ControllerAction) {
        let mut state = self.state.write().expect("ui state write lock");
        reduce_ui_state(&mut state, action);
    }

    fn derive_diagnostics(runs: &[WorkflowRun]) -> Vec<UiDiagnostic> {
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
}

#[async_trait]
impl CommandDispatcher for DesktopAppController {
    async fn dispatch(&self, command: DesktopCommand) -> Result<()> {
        self.dispatch_command(command).await
    }
}

#[cfg(test)]
mod tests {
    use super::{
        reduce_ui_state, ControllerAction, DesktopApi, DesktopAppController, UiRoute,
        UiStateSnapshot,
    };
    use crate::command::DesktopCommand;
    use anyhow::Result;
    use async_trait::async_trait;
    use chrono::Utc;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use workdesk_core::{
        RunSkillSnapshot, RunStatus, Scope, WorkflowRun, WorkflowRunEvent, WorkflowRunNodeState,
    };

    #[derive(Default)]
    struct FakeDesktopApi {
        runs: Mutex<Vec<WorkflowRun>>,
        events: Mutex<HashMap<String, Vec<WorkflowRunEvent>>>,
        skills: Mutex<HashMap<String, Vec<RunSkillSnapshot>>>,
        nodes: Mutex<HashMap<String, Vec<WorkflowRunNodeState>>>,
    }

    impl FakeDesktopApi {
        fn seed_with_one_run() -> Self {
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
                    status: workdesk_core::RunNodeStatus::Succeeded,
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

        async fn cancel_run(
            &self,
            run_id: &str,
            _requested_by: Option<&str>,
        ) -> Result<WorkflowRun> {
            let mut runs = self.runs.lock().expect("runs lock");
            let run = runs
                .iter_mut()
                .find(|run| run.run_id == run_id)
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
                .find(|run| run.run_id == run_id)
                .map(|run| run.workflow_id.clone())
                .unwrap_or_else(|| "wf-unknown".into());
            self.run_workflow(&workflow_id, requested_by).await
        }
    }

    #[test]
    fn reducer_focus_and_route() {
        let mut state = UiStateSnapshot::default();
        reduce_ui_state(&mut state, ControllerAction::FocusWindow);
        reduce_ui_state(&mut state, ControllerAction::SetRoute(UiRoute::RunDetail));
        reduce_ui_state(
            &mut state,
            ControllerAction::SelectRun(Some("run-1".into())),
        );

        assert_eq!(state.focus_seq, 1);
        assert_eq!(state.route, UiRoute::RunDetail);
        assert_eq!(state.selected_run_id.as_deref(), Some("run-1"));
        assert_eq!(state.revision, 3);
    }

    #[tokio::test]
    async fn dispatch_open_run_loads_detail() {
        let api = Arc::new(FakeDesktopApi::seed_with_one_run());
        let controller = DesktopAppController::new(api);

        controller
            .dispatch_command(DesktopCommand::OpenRun {
                run_id: "run-1".into(),
            })
            .await
            .expect("dispatch");

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.route, UiRoute::RunDetail);
        assert_eq!(snapshot.selected_run_id.as_deref(), Some("run-1"));
        assert_eq!(snapshot.run_events.len(), 1);
        assert_eq!(snapshot.run_nodes.len(), 1);
        assert_eq!(snapshot.run_skills.len(), 1);
        assert!(snapshot.last_error.is_none());
    }

    #[tokio::test]
    async fn refresh_runs_generates_runner_unavailable_diagnostic() {
        let api = Arc::new(FakeDesktopApi {
            runs: Mutex::new(vec![WorkflowRun {
                run_id: "run-stuck".into(),
                workflow_id: "wf-1".into(),
                requested_by: Some("tester".into()),
                status: RunStatus::Queued,
                started_at: None,
                finished_at: None,
                cancel_requested: false,
                error_message: None,
                created_at: Utc::now() - chrono::Duration::seconds(95),
                updated_at: Utc::now(),
            }]),
            events: Mutex::new(HashMap::new()),
            skills: Mutex::new(HashMap::new()),
            nodes: Mutex::new(HashMap::new()),
        });
        let controller = DesktopAppController::new(api);
        controller.refresh_runs().await.expect("refresh runs");
        let snapshot = controller.snapshot();
        assert_eq!(snapshot.diagnostics.len(), 1);
        assert_eq!(snapshot.diagnostics[0].code, "RUNNER_UNAVAILABLE");
        assert_eq!(snapshot.diagnostics[0].run_id.as_deref(), Some("run-stuck"));
    }
}
