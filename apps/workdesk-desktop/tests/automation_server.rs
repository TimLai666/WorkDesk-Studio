use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use workdesk_core::{
    RunNodeStatus, RunSkillSnapshot, RunStatus, Scope, WorkflowRun, WorkflowRunEvent,
    WorkflowRunNodeState,
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
