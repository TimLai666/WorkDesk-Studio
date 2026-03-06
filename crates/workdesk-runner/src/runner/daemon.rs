use super::node_exec::{CodeExecutionRequest, CodeNodeExecutor, ExecutionLanguage};
use super::scheduler::topological_nodes;
use super::skills::copy_path_recursive;
use super::toolchain::ToolchainManager;
use crate::sidecar::CodexSidecarClient;
use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use serde_json::Value;
use std::path::PathBuf;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use workdesk_core::repository::CoreRepository;
use workdesk_core::{
    RunNodeStatus, RunSkillSnapshot, SqliteCoreRepository, WorkflowDefinition, WorkflowNode,
    WorkflowRun, WorkflowStatus,
};
use workdesk_domain::{CodeNodeSpec, ResourceLimits, WorkflowNodeKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RetryBackoffStrategy {
    Fixed,
    Exponential,
}

#[derive(Debug, Clone, Copy)]
struct RetryPolicy {
    max_attempts: u32,
    backoff_ms: u64,
    max_backoff_ms: Option<u64>,
    strategy: RetryBackoffStrategy,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 1,
            backoff_ms: 0,
            max_backoff_ms: None,
            strategy: RetryBackoffStrategy::Fixed,
        }
    }
}

impl RetryPolicy {
    fn backoff_for_next_attempt(&self, next_attempt: u32) -> u64 {
        if self.backoff_ms == 0 || next_attempt <= 1 {
            return 0;
        }
        let mut delay = self.backoff_ms;
        if matches!(self.strategy, RetryBackoffStrategy::Exponential) {
            for _ in 2..next_attempt {
                delay = delay.saturating_mul(2);
            }
        }
        if let Some(max_backoff) = self.max_backoff_ms {
            delay = delay.min(max_backoff);
        }
        delay
    }
}

#[derive(Debug, Clone)]
struct ResolvedCodeNodeSpec {
    language: ExecutionLanguage,
    entrypoint: PathBuf,
    deps: Vec<String>,
    timeout_sec: u64,
    resource_limits: ResourceLimits,
}

#[derive(Debug, Clone)]
pub struct RunnerConfig {
    pub db_path: PathBuf,
    pub tools_root: PathBuf,
    pub runner_id: String,
    pub poll_interval_ms: u64,
    pub lease_seconds: i64,
}

pub struct WorkflowRunnerDaemon {
    repo: SqliteCoreRepository,
    manager: ToolchainManager,
    config: RunnerConfig,
}

impl WorkflowRunnerDaemon {
    pub async fn new(config: RunnerConfig) -> Result<Self> {
        let repo = SqliteCoreRepository::connect(&config.db_path).await?;
        repo.migrate().await?;
        Ok(Self {
            repo,
            manager: ToolchainManager::new(config.tools_root.clone()),
            config,
        })
    }

    pub async fn run_forever(&self) -> Result<()> {
        loop {
            let had_work = self.run_once().await?;
            if !had_work {
                tokio::time::sleep(Duration::from_millis(self.config.poll_interval_ms)).await;
            }
        }
    }

    pub async fn run_once(&self) -> Result<bool> {
        let Some(run) = self
            .repo
            .claim_queued_run(&self.config.runner_id, self.config.lease_seconds)
            .await?
        else {
            return Ok(false);
        };

        self.process_run(&run).await?;
        Ok(true)
    }

    async fn process_run(&self, run: &WorkflowRun) -> Result<()> {
        self.repo
            .append_run_event(
                &run.run_id,
                "runner_claimed",
                &format!("runner {} claimed run", self.config.runner_id),
            )
            .await?;

        let workflow = self
            .repo
            .get_workflow(&run.workflow_id)
            .await?
            .ok_or_else(|| anyhow!("workflow disappeared: {}", run.workflow_id))?;
        if !matches!(
            workflow.status,
            WorkflowStatus::Draft | WorkflowStatus::Active | WorkflowStatus::Disabled
        ) {
            return Err(anyhow!("workflow status unsupported"));
        }

        let snapshots = self.repo.list_run_skill_snapshots(&run.run_id).await?;
        let skills_index_path = self.materialize_skills(run, &snapshots).await?;
        let ordered_nodes = topological_nodes(&workflow)?;

        for node in ordered_nodes {
            let latest = self
                .repo
                .get_run(&run.run_id)
                .await?
                .ok_or_else(|| anyhow!("run disappeared after claim: {}", run.run_id))?;
            if latest.cancel_requested {
                self.repo
                    .set_run_node_status(&run.run_id, &node.id, RunNodeStatus::Canceled, None)
                    .await?;
                self.repo
                    .append_run_event(
                        &run.run_id,
                        "run_canceled",
                        "run canceled before finishing all nodes",
                    )
                    .await?;
                self.repo
                    .complete_run_canceled(&run.run_id, "canceled by operator")
                    .await?;
                return Ok(());
            }
            let retry_policy = retry_policy_for_node(&node);
            let mut attempt = 0u32;
            loop {
                attempt += 1;
                self.repo
                    .set_run_node_status(&run.run_id, &node.id, RunNodeStatus::Running, None)
                    .await?;
                self.repo
                    .append_run_event(
                        &run.run_id,
                        "node_started",
                        &serde_json::json!({
                            "node_id": node.id,
                            "kind": format!("{:?}", node.kind),
                            "attempt": attempt,
                        })
                        .to_string(),
                    )
                    .await?;

                match self
                    .execute_node(run, &workflow, &node, &skills_index_path)
                    .await
                {
                    Ok(()) => {
                        self.repo
                            .set_run_node_status(
                                &run.run_id,
                                &node.id,
                                RunNodeStatus::Succeeded,
                                None,
                            )
                            .await?;
                        self.repo
                            .append_run_event(
                                &run.run_id,
                                "node_succeeded",
                                &serde_json::json!({
                                    "node_id": node.id,
                                    "attempt": attempt,
                                })
                                .to_string(),
                            )
                            .await?;
                        break;
                    }
                    Err(error) => {
                        let error_message = error.to_string();
                        if attempt < retry_policy.max_attempts {
                            let next_attempt = attempt + 1;
                            let backoff_ms = retry_policy.backoff_for_next_attempt(next_attempt);
                            self.repo
                                .set_run_node_status(
                                    &run.run_id,
                                    &node.id,
                                    RunNodeStatus::Pending,
                                    Some(&error_message),
                                )
                                .await?;
                            self.repo
                                .append_run_event(
                                    &run.run_id,
                                    "node_retry_scheduled",
                                    &serde_json::json!({
                                        "node_id": node.id,
                                        "attempt": attempt,
                                        "next_attempt": next_attempt,
                                        "backoff_ms": backoff_ms,
                                        "error": error_message,
                                    })
                                    .to_string(),
                                )
                                .await?;
                            if backoff_ms > 0 {
                                tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                            }
                            continue;
                        }

                        self.repo
                            .set_run_node_status(
                                &run.run_id,
                                &node.id,
                                RunNodeStatus::Failed,
                                Some(&error_message),
                            )
                            .await?;
                        self.repo
                            .append_run_event(
                                &run.run_id,
                                "node_failed",
                                &serde_json::json!({
                                    "node_id": node.id,
                                    "attempt": attempt,
                                    "error": error_message
                                })
                                .to_string(),
                            )
                            .await?;
                        self.repo
                            .complete_run_failure(&run.run_id, &error_message)
                            .await?;
                        return Ok(());
                    }
                }
            }
        }

        self.repo
            .append_run_event(
                &run.run_id,
                "run_finished",
                "runner finished workflow DAG execution path",
            )
            .await?;
        self.repo.complete_run_success(&run.run_id).await?;
        Ok(())
    }

    async fn materialize_skills(
        &self,
        run: &WorkflowRun,
        snapshots: &[RunSkillSnapshot],
    ) -> Result<PathBuf> {
        let skills_root = self
            .manager
            .tools_root()
            .join("workflows")
            .join(&run.workflow_id)
            .join("runs")
            .join(&run.run_id)
            .join(".workdesk")
            .join("skills");
        tokio::fs::create_dir_all(&skills_root).await?;

        let mut index = Vec::with_capacity(snapshots.len());
        for snapshot in snapshots {
            let source = PathBuf::from(&snapshot.content_path);
            let target = skills_root.join(&snapshot.name);
            copy_path_recursive(&source, &target)
                .await
                .with_context(|| format!("materialize skill {} failed", snapshot.name))?;
            self.repo
                .update_run_skill_materialized_path(
                    &run.run_id,
                    &snapshot.name,
                    &target.to_string_lossy(),
                )
                .await?;
            index.push(serde_json::json!({
                "name": snapshot.name,
                "scope": snapshot.scope,
                "version": snapshot.version,
                "manifest": snapshot.manifest,
                "path": target.to_string_lossy().to_string()
            }));
        }

        let index_path = skills_root.join("index.json");
        let mut file = tokio::fs::File::create(&index_path).await?;
        file.write_all(serde_json::to_string_pretty(&index)?.as_bytes())
            .await?;
        file.flush().await?;
        self.repo
            .append_run_event(
                &run.run_id,
                "skills_loaded",
                &format!(
                    "loaded {} skills, index={}",
                    snapshots.len(),
                    index_path.to_string_lossy()
                ),
            )
            .await?;
        Ok(index_path)
    }

    async fn execute_node(
        &self,
        run: &WorkflowRun,
        workflow: &WorkflowDefinition,
        node: &WorkflowNode,
        skills_index_path: &PathBuf,
    ) -> Result<()> {
        match node.kind {
            WorkflowNodeKind::ScheduleTrigger => {
                let wait = schedule_trigger_wait(node)?;
                if let Some(wait) = wait {
                    if wait > Duration::from_millis(0) {
                        self.repo
                            .append_run_event(
                                &run.run_id,
                                "schedule_wait_started",
                                &serde_json::json!({
                                    "node_id": node.id,
                                    "wait_ms": wait.as_millis(),
                                    "timezone": workflow.timezone
                                })
                                .to_string(),
                            )
                            .await?;
                        tokio::time::sleep(wait).await;
                    }
                }
                self.repo
                    .append_run_event(
                        &run.run_id,
                        "schedule_trigger_fired",
                        &serde_json::json!({
                            "node_id": node.id,
                            "timezone": workflow.timezone,
                            "fired_at": Utc::now().to_rfc3339()
                        })
                        .to_string(),
                    )
                    .await?;
                Ok(())
            }
            WorkflowNodeKind::ApprovalGate => Ok(()),
            WorkflowNodeKind::FileOps => Ok(()),
            WorkflowNodeKind::AgentPrompt => {
                let sidecar_response = if let Ok(endpoint) =
                    std::env::var("WORKDESK_SIDECAR_ENDPOINT")
                {
                    let client = CodexSidecarClient::new(endpoint);
                    Some(
                        client
                            .send(
                                "run_prompt",
                                serde_json::json!({
                                    "run_id": run.run_id,
                                    "workflow_id": run.workflow_id,
                                    "node_id": node.id,
                                    "skills_index_path": skills_index_path.to_string_lossy().to_string()
                                }),
                            )
                            .await?,
                    )
                } else {
                    None
                };
                self.repo
                    .append_run_event(
                        &run.run_id,
                        "agent_prompt",
                        &serde_json::json!({
                            "node_id": node.id,
                            "skills_index_path": skills_index_path.to_string_lossy().to_string(),
                            "sidecar_attached": sidecar_response.is_some()
                        })
                        .to_string(),
                    )
                    .await?;
                Ok(())
            }
            WorkflowNodeKind::CodeExec => {
                let spec = resolve_code_node_spec(&run.workflow_id, node, &self.manager)?;
                if !spec.entrypoint.exists() {
                    self.repo
                        .append_run_event(
                            &run.run_id,
                            "code_exec_skipped",
                            &serde_json::json!({
                                "node_id": node.id,
                                "reason": "missing entrypoint",
                                "entry": spec.entrypoint.to_string_lossy().to_string(),
                                "language": format!("{:?}", spec.language),
                            })
                            .to_string(),
                        )
                        .await?;
                    return Ok(());
                }

                let result = CodeNodeExecutor::new(self.manager.clone())
                    .execute(CodeExecutionRequest {
                        workflow_id: run.workflow_id.clone(),
                        language: spec.language.clone(),
                        entrypoint: spec.entrypoint.clone(),
                        args: vec![],
                        deps: spec.deps.clone(),
                        timeout_sec: spec.timeout_sec,
                        resource_limits: Some(spec.resource_limits.clone()),
                    })
                    .await?;

                if result.exit_code != 0 {
                    return Err(anyhow!(
                        "code node failed with exit code {}: {}",
                        result.exit_code,
                        result.stderr
                    ));
                }
                self.repo
                    .append_run_event(
                        &run.run_id,
                        "code_exec_succeeded",
                        &serde_json::json!({
                            "node_id": node.id,
                            "entry": spec.entrypoint.to_string_lossy().to_string(),
                            "language": format!("{:?}", spec.language),
                            "stdout": result.stdout,
                            "stderr": result.stderr,
                        })
                        .to_string(),
                    )
                    .await?;
                Ok(())
            }
        }
    }
}

fn retry_policy_for_node(node: &WorkflowNode) -> RetryPolicy {
    let mut policy = RetryPolicy::default();
    let Some(config) = node.config.as_ref() else {
        return policy;
    };

    if let Some(retry) = config.get("retry") {
        apply_retry_policy_overrides(&mut policy, retry);
    } else {
        apply_retry_policy_overrides(&mut policy, config);
    }

    policy.max_attempts = policy.max_attempts.max(1);
    policy
}

fn apply_retry_policy_overrides(policy: &mut RetryPolicy, payload: &Value) {
    if !payload.is_object() {
        return;
    }

    if let Some(max_attempts) = payload.get("max_attempts").and_then(Value::as_u64) {
        policy.max_attempts = max_attempts.clamp(1, u32::MAX as u64) as u32;
    }
    if let Some(backoff_ms) = payload.get("backoff_ms").and_then(Value::as_u64) {
        policy.backoff_ms = backoff_ms;
    }
    if let Some(max_backoff_ms) = payload.get("max_backoff_ms").and_then(Value::as_u64) {
        policy.max_backoff_ms = Some(max_backoff_ms);
    }
    let strategy = payload
        .get("strategy")
        .and_then(Value::as_str)
        .or_else(|| payload.get("backoff").and_then(Value::as_str))
        .map(|value| value.trim().to_ascii_lowercase());
    if matches!(strategy.as_deref(), Some("exponential")) {
        policy.strategy = RetryBackoffStrategy::Exponential;
    } else if matches!(strategy.as_deref(), Some("fixed")) {
        policy.strategy = RetryBackoffStrategy::Fixed;
    }
}

fn resolve_code_node_spec(
    workflow_id: &str,
    node: &WorkflowNode,
    manager: &ToolchainManager,
) -> Result<ResolvedCodeNodeSpec> {
    let default_timeout_sec = 60u64;
    let mut language = ExecutionLanguage::Python;
    let mut entry = String::from("main.py");
    let mut deps = Vec::<String>::new();
    let mut timeout_sec = default_timeout_sec;
    let mut max_memory_mb = 512u64;

    if let Some(config) = node.config.as_ref() {
        let parsed = serde_json::from_value::<CodeNodeSpec>(config.clone())
            .ok()
            .or_else(|| {
                config
                    .get("code_spec")
                    .cloned()
                    .and_then(|value| serde_json::from_value::<CodeNodeSpec>(value).ok())
            })
            .or_else(|| {
                config
                    .get("spec")
                    .cloned()
                    .and_then(|value| serde_json::from_value::<CodeNodeSpec>(value).ok())
            });

        if let Some(spec) = parsed {
            language = spec.language;
            entry = spec.entry;
            deps = spec.deps;
            timeout_sec = spec.timeout_sec.max(1);
            max_memory_mb = spec.resource_limits.max_memory_mb.max(64);
            timeout_sec = timeout_sec.max(spec.resource_limits.timeout_sec.max(1));
        } else {
            if let Some(raw_language) = config.get("language").and_then(Value::as_str) {
                language =
                    parse_execution_language(raw_language).unwrap_or(ExecutionLanguage::Python);
            }
            if let Some(raw_entry) = config.get("entry").and_then(Value::as_str) {
                entry = raw_entry.to_string();
            }
            if let Some(raw_deps) = config.get("deps").and_then(Value::as_array) {
                deps = raw_deps
                    .iter()
                    .filter_map(Value::as_str)
                    .map(ToString::to_string)
                    .collect();
            }
            if let Some(raw_timeout) = config.get("timeout_sec").and_then(Value::as_u64) {
                timeout_sec = raw_timeout.max(1);
            }
            if let Some(resource_limits) = config.get("resource_limits").and_then(Value::as_object)
            {
                if let Some(limit_timeout) =
                    resource_limits.get("timeout_sec").and_then(Value::as_u64)
                {
                    timeout_sec = timeout_sec.max(limit_timeout.max(1));
                }
                if let Some(limit_memory) =
                    resource_limits.get("max_memory_mb").and_then(Value::as_u64)
                {
                    max_memory_mb = limit_memory.max(64);
                }
            }
        }
    }

    let runtime_root = manager.workflow_runtime_root(workflow_id, language.clone());
    let entrypoint = {
        let path = PathBuf::from(&entry);
        if path.is_absolute() {
            path
        } else {
            runtime_root.join(path)
        }
    };

    Ok(ResolvedCodeNodeSpec {
        language,
        entrypoint,
        deps,
        timeout_sec: timeout_sec.max(1),
        resource_limits: ResourceLimits {
            timeout_sec: timeout_sec.max(1),
            max_memory_mb: max_memory_mb.max(64),
        },
    })
}

fn parse_execution_language(value: &str) -> Option<ExecutionLanguage> {
    match value.trim().to_ascii_lowercase().as_str() {
        "python" | "py" => Some(ExecutionLanguage::Python),
        "javascript" | "js" | "node" => Some(ExecutionLanguage::Javascript),
        "go" | "golang" => Some(ExecutionLanguage::Go),
        _ => None,
    }
}

fn schedule_trigger_wait(node: &WorkflowNode) -> Result<Option<Duration>> {
    let Some(config) = node.config.as_ref() else {
        return Ok(None);
    };
    if let Some(delay_ms) = config.get("delay_ms").and_then(Value::as_u64) {
        return Ok(Some(Duration::from_millis(delay_ms)));
    }
    if let Some(delay_sec) = config.get("delay_sec").and_then(Value::as_u64) {
        return Ok(Some(Duration::from_secs(delay_sec)));
    }
    if let Some(run_at) = config.get("run_at").and_then(Value::as_str) {
        let target = chrono::DateTime::parse_from_rfc3339(run_at)
            .with_context(|| format!("invalid schedule trigger run_at: {run_at}"))?
            .with_timezone(&Utc);
        let now = Utc::now();
        if target > now {
            let millis = (target - now).num_milliseconds().max(0) as u64;
            return Ok(Some(Duration::from_millis(millis)));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use workdesk_core::WorkflowNodeKind;

    #[test]
    fn retry_policy_defaults_to_single_attempt() {
        let node = WorkflowNode {
            id: "node-1".into(),
            kind: WorkflowNodeKind::AgentPrompt,
            x: None,
            y: None,
            config: None,
        };
        let policy = retry_policy_for_node(&node);
        assert_eq!(policy.max_attempts, 1);
        assert_eq!(policy.backoff_ms, 0);
    }

    #[test]
    fn retry_policy_parses_exponential_backoff() {
        let node = WorkflowNode {
            id: "node-2".into(),
            kind: WorkflowNodeKind::CodeExec,
            x: None,
            y: None,
            config: Some(json!({
                "retry": {
                    "max_attempts": 4,
                    "backoff_ms": 100,
                    "strategy": "exponential",
                    "max_backoff_ms": 350
                }
            })),
        };
        let policy = retry_policy_for_node(&node);
        assert_eq!(policy.max_attempts, 4);
        assert_eq!(policy.backoff_for_next_attempt(2), 100);
        assert_eq!(policy.backoff_for_next_attempt(3), 200);
        assert_eq!(policy.backoff_for_next_attempt(4), 350);
    }

    #[test]
    fn code_spec_accepts_embedded_spec_payload() {
        let manager = ToolchainManager::new(PathBuf::from("C:/workdesk/tools"));
        let node = WorkflowNode {
            id: "code-1".into(),
            kind: WorkflowNodeKind::CodeExec,
            x: None,
            y: None,
            config: Some(json!({
                "code_spec": {
                    "language": "javascript",
                    "entry": "src/index.ts",
                    "deps": ["zod"],
                    "timeout_sec": 45,
                    "resource_limits": {
                        "timeout_sec": 45,
                        "max_memory_mb": 768
                    }
                }
            })),
        };
        let resolved = resolve_code_node_spec("wf-1", &node, &manager).expect("resolve");
        assert_eq!(resolved.language, ExecutionLanguage::Javascript);
        assert_eq!(resolved.timeout_sec, 45);
        assert_eq!(resolved.deps, vec!["zod"]);
        let entry = resolved.entrypoint.to_string_lossy();
        assert!(entry.contains("workflows"));
        assert!(entry.contains("wf-1"));
        assert!(entry.contains("javascript"));
        assert!(entry.contains("src"));
        assert!(entry.ends_with("index.ts"));
        assert_eq!(resolved.resource_limits.max_memory_mb, 768);
    }

    #[test]
    fn schedule_trigger_wait_supports_delay_and_run_at() {
        let node = WorkflowNode {
            id: "schedule-1".into(),
            kind: WorkflowNodeKind::ScheduleTrigger,
            x: None,
            y: None,
            config: Some(json!({
                "delay_sec": 3
            })),
        };
        let wait = schedule_trigger_wait(&node).expect("wait");
        assert_eq!(wait, Some(Duration::from_secs(3)));

        let run_at = (Utc::now() + chrono::Duration::seconds(2)).to_rfc3339();
        let node = WorkflowNode {
            id: "schedule-2".into(),
            kind: WorkflowNodeKind::ScheduleTrigger,
            x: None,
            y: None,
            config: Some(json!({
                "run_at": run_at
            })),
        };
        let wait = schedule_trigger_wait(&node).expect("wait");
        assert!(wait.expect("duration") >= Duration::from_millis(500));
    }
}
