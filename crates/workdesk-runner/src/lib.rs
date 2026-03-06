use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use uuid::Uuid;
use workdesk_core::repository::CoreRepository;
use workdesk_core::{RunSkillSnapshot, SqliteCoreRepository, WorkflowRun};

pub use workdesk_domain::ExecutionLanguage;
use workdesk_domain::{AgentEvent, AgentProvider, AgentSession};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolchainBinary {
    Codex,
    Uv,
    Bun,
    Go,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Semver {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

#[derive(Debug, Clone)]
pub struct ToolchainManager {
    tools_root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolchainStatus {
    pub binary: ToolchainBinary,
    pub installed: bool,
    pub version: Option<Semver>,
}

impl ToolchainManager {
    pub fn new(tools_root: PathBuf) -> Self {
        Self { tools_root }
    }

    pub fn tools_root(&self) -> &PathBuf {
        &self.tools_root
    }

    pub fn binary_dir(&self, binary: ToolchainBinary) -> PathBuf {
        let name = match binary {
            ToolchainBinary::Codex => "codex",
            ToolchainBinary::Uv => "uv",
            ToolchainBinary::Bun => "bun",
            ToolchainBinary::Go => "go",
        };
        self.tools_root.join(name)
    }

    pub fn binary_path(&self, binary: ToolchainBinary) -> PathBuf {
        let executable = match binary {
            ToolchainBinary::Codex => "codex.exe",
            ToolchainBinary::Uv => "uv.exe",
            ToolchainBinary::Bun => "bun.exe",
            ToolchainBinary::Go => "go.exe",
        };
        self.binary_dir(binary).join(executable)
    }

    pub fn workflow_runtime_root(&self, workflow_id: &str, language: ExecutionLanguage) -> PathBuf {
        let lang_dir = match language {
            ExecutionLanguage::Python => "python",
            ExecutionLanguage::Javascript => "javascript",
            ExecutionLanguage::Go => "go",
        };
        self.tools_root
            .join("workflows")
            .join(workflow_id)
            .join(lang_dir)
    }

    pub fn parse_version_output(&self, output: &str, _binary: ToolchainBinary) -> Result<Semver> {
        let token = output
            .split_whitespace()
            .find(|item| item.chars().any(|ch| ch.is_ascii_digit()))
            .ok_or_else(|| anyhow!("no semver token found in output"))?;

        let version = token.trim_start_matches('v');
        let mut parts = version.split('.');
        let major = parts
            .next()
            .ok_or_else(|| anyhow!("missing major version"))?
            .parse::<u64>()
            .context("invalid major version")?;
        let minor = parts
            .next()
            .ok_or_else(|| anyhow!("missing minor version"))?
            .parse::<u64>()
            .context("invalid minor version")?;
        let patch = parts
            .next()
            .ok_or_else(|| anyhow!("missing patch version"))?
            .chars()
            .take_while(|ch| ch.is_ascii_digit())
            .collect::<String>()
            .parse::<u64>()
            .context("invalid patch version")?;

        Ok(Semver {
            major,
            minor,
            patch,
        })
    }

    pub async fn detect_installed_version(
        &self,
        binary: ToolchainBinary,
    ) -> Result<Option<Semver>> {
        let binary_path = self.binary_path(binary);
        if !binary_path.exists() {
            return Ok(None);
        }

        let output = Command::new(binary_path).arg("--version").output().await?;
        if !output.status.success() {
            return Ok(None);
        }
        let parsed = self.parse_version_output(&String::from_utf8_lossy(&output.stdout), binary)?;
        Ok(Some(parsed))
    }

    pub async fn toolchain_status(&self, binary: ToolchainBinary) -> Result<ToolchainStatus> {
        let version = self.detect_installed_version(binary).await?;
        Ok(ToolchainStatus {
            binary,
            installed: version.is_some(),
            version,
        })
    }
}

#[derive(Debug, Clone)]
pub struct CodeExecutionRequest {
    pub workflow_id: String,
    pub language: ExecutionLanguage,
    pub entrypoint: PathBuf,
    pub args: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CodeExecutionResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

pub struct CodeNodeExecutor {
    manager: ToolchainManager,
}

impl CodeNodeExecutor {
    pub fn new(manager: ToolchainManager) -> Self {
        Self { manager }
    }

    pub async fn execute(&self, request: CodeExecutionRequest) -> Result<CodeExecutionResult> {
        let runtime_root = self
            .manager
            .workflow_runtime_root(&request.workflow_id, request.language.clone());
        tokio::fs::create_dir_all(&runtime_root)
            .await
            .context("create workflow runtime root")?;

        let mut command = match request.language {
            ExecutionLanguage::Python => {
                let mut cmd = Command::new(self.binary_name(ToolchainBinary::Uv));
                cmd.arg("run").arg(request.entrypoint.as_os_str());
                cmd
            }
            ExecutionLanguage::Javascript => {
                let mut cmd = Command::new(self.binary_name(ToolchainBinary::Bun));
                cmd.arg("run").arg(request.entrypoint.as_os_str());
                cmd
            }
            ExecutionLanguage::Go => {
                let mut cmd = Command::new(self.binary_name(ToolchainBinary::Go));
                cmd.arg("run").arg(request.entrypoint.as_os_str());
                cmd
            }
        };

        command.current_dir(&runtime_root);
        for arg in &request.args {
            command.arg(arg);
        }

        let output = command
            .output()
            .await
            .context("execute code node command")?;
        Ok(CodeExecutionResult {
            exit_code: output.status.code().unwrap_or_default(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }

    fn binary_name(&self, binary: ToolchainBinary) -> &OsStr {
        match binary {
            ToolchainBinary::Codex => OsStr::new("codex"),
            ToolchainBinary::Uv => OsStr::new("uv"),
            ToolchainBinary::Bun => OsStr::new("bun"),
            ToolchainBinary::Go => OsStr::new("go"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CodexCliAgentProvider {
    codex_binary: PathBuf,
}

impl Default for CodexCliAgentProvider {
    fn default() -> Self {
        Self {
            codex_binary: PathBuf::from("codex"),
        }
    }
}

impl CodexCliAgentProvider {
    pub fn new(codex_binary: PathBuf) -> Self {
        Self { codex_binary }
    }

    fn command(&self) -> Command {
        Command::new(&self.codex_binary)
    }
}

#[async_trait]
impl AgentProvider for CodexCliAgentProvider {
    async fn start_session(&self, account_id: &str) -> Result<AgentSession> {
        let version = self.command().arg("--version").output().await;
        if let Ok(output) = version {
            if !output.status.success() {
                return Err(anyhow!(
                    "codex CLI is installed but unavailable: {}",
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
        }

        Ok(AgentSession {
            session_id: Uuid::new_v4().to_string(),
            account_id: account_id.to_string(),
        })
    }

    async fn run_prompt(&self, _session: &AgentSession, prompt: &str) -> Result<String> {
        let output = self
            .command()
            .arg("exec")
            .arg(prompt)
            .output()
            .await
            .context("invoke codex exec")?;

        if !output.status.success() {
            return Err(anyhow!(
                "codex execution failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    async fn stream_events(&self, _session: &AgentSession) -> Result<Vec<AgentEvent>> {
        Ok(vec![AgentEvent {
            kind: "info".into(),
            payload: "streaming from codex CLI is not wired in this scaffold".into(),
        }])
    }

    async fn logout(&self, _account_id: &str) -> Result<()> {
        let _ = self.command().arg("logout").output().await;
        Ok(())
    }

    async fn switch_account(&self, from_account: &str, to_account: &str) -> Result<AgentSession> {
        self.logout(from_account).await?;
        self.start_session(to_account).await
    }
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

        if let Err(error) = self.process_run(&run).await {
            let _ = self
                .repo
                .complete_run_failure(&run.run_id, &error.to_string())
                .await;
            return Err(error);
        }

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

        let snapshots = self.repo.list_run_skill_snapshots(&run.run_id).await?;
        self.materialize_skills(&run, &snapshots).await?;

        let latest = self
            .repo
            .get_run(&run.run_id)
            .await?
            .ok_or_else(|| anyhow!("run disappeared after claim: {}", run.run_id))?;
        if latest.cancel_requested {
            self.repo
                .append_run_event(
                    &run.run_id,
                    "run_canceled",
                    "run canceled before node execution started",
                )
                .await?;
            self.repo
                .complete_run_canceled(&run.run_id, "canceled before execution")
                .await?;
            return Ok(());
        }

        self.repo
            .append_run_event(
                &run.run_id,
                "run_finished",
                "runner finished placeholder execution path",
            )
            .await?;
        self.repo.complete_run_success(&run.run_id).await?;
        Ok(())
    }

    async fn materialize_skills(
        &self,
        run: &WorkflowRun,
        snapshots: &[RunSkillSnapshot],
    ) -> Result<()> {
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
        Ok(())
    }
}

async fn copy_path_recursive(source: &PathBuf, target: &PathBuf) -> Result<()> {
    let source = source.clone();
    let target = target.clone();
    tokio::task::spawn_blocking(move || copy_path_recursive_sync(&source, &target))
        .await
        .context("join skill copy task")??;
    Ok(())
}

fn copy_path_recursive_sync(source: &PathBuf, target: &PathBuf) -> Result<()> {
    let metadata = std::fs::metadata(source)
        .with_context(|| format!("skill source path not found: {}", source.display()))?;
    if metadata.is_file() {
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(source, target)?;
        return Ok(());
    }

    std::fs::create_dir_all(target)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let path = entry.path();
        let nested_target = target.join(entry.file_name());
        copy_path_recursive_sync(&path, &nested_target)?;
    }
    Ok(())
}
