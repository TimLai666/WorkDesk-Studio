use super::lib::ToolchainBinary;
use super::toolchain::ToolchainManager;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::time::Duration;
use tokio::process::Command;
use uuid::Uuid;

pub use workdesk_domain::ExecutionLanguage;
use workdesk_domain::{AgentEvent, AgentProvider, AgentSession, ResourceLimits};

#[derive(Debug, Clone)]
pub struct CodeExecutionRequest {
    pub workflow_id: String,
    pub language: ExecutionLanguage,
    pub entrypoint: PathBuf,
    pub args: Vec<String>,
    pub deps: Vec<String>,
    pub timeout_sec: u64,
    pub resource_limits: Option<ResourceLimits>,
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

        self.prepare_dependencies(&request, &runtime_root).await?;
        let mut command = match request.language {
            ExecutionLanguage::Python => {
                let mut cmd = self.command_for_binary(ToolchainBinary::Uv);
                cmd.arg("run");
                for dep in &request.deps {
                    cmd.arg("--with").arg(dep);
                }
                cmd.arg(request.entrypoint.as_os_str());
                cmd.env("UV_CACHE_DIR", runtime_root.join(".uv-cache"));
                cmd
            }
            ExecutionLanguage::Javascript => {
                let mut cmd = self.command_for_binary(ToolchainBinary::Bun);
                cmd.arg("run").arg(request.entrypoint.as_os_str());
                cmd.env("BUN_INSTALL_CACHE_DIR", runtime_root.join(".bun-cache"));
                cmd
            }
            ExecutionLanguage::Go => {
                let mut cmd = self.command_for_binary(ToolchainBinary::Go);
                cmd.arg("run").arg(request.entrypoint.as_os_str());
                cmd.env("GOMODCACHE", runtime_root.join(".go-mod-cache"));
                cmd.env("GOCACHE", runtime_root.join(".go-build-cache"));
                cmd
            }
        };

        command.current_dir(&runtime_root);
        for arg in &request.args {
            command.arg(arg);
        }

        if let Some(limit) = request.resource_limits.as_ref() {
            command.env("WORKDESK_MAX_MEMORY_MB", limit.max_memory_mb.to_string());
        }

        let timeout = Duration::from_secs(request.timeout_sec.max(1));
        let output = tokio::time::timeout(timeout, command.output())
            .await
            .map_err(|_| anyhow!("code node timeout after {} seconds", timeout.as_secs()))?
            .context("execute code node command")?;
        Ok(CodeExecutionResult {
            exit_code: output.status.code().unwrap_or_default(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }

    fn binary_name(binary: ToolchainBinary) -> &'static OsStr {
        match binary {
            ToolchainBinary::Codex => OsStr::new("codex"),
            ToolchainBinary::Uv => OsStr::new("uv"),
            ToolchainBinary::Bun => OsStr::new("bun"),
            ToolchainBinary::Go => OsStr::new("go"),
        }
    }

    fn command_for_binary(&self, binary: ToolchainBinary) -> Command {
        let managed = self.manager.binary_path(binary);
        if managed.exists() {
            Command::new(managed)
        } else {
            Command::new(Self::binary_name(binary))
        }
    }

    async fn prepare_dependencies(
        &self,
        request: &CodeExecutionRequest,
        runtime_root: &PathBuf,
    ) -> Result<()> {
        if request.deps.is_empty() {
            return Ok(());
        }
        match request.language {
            ExecutionLanguage::Python => Ok(()),
            ExecutionLanguage::Javascript => {
                let package_json = runtime_root.join("package.json");
                if !package_json.exists() {
                    tokio::fs::write(
                        &package_json,
                        r#"{"name":"workdesk-workflow","private":true}"#,
                    )
                    .await
                    .context("create workflow package.json")?;
                }
                let mut command = self.command_for_binary(ToolchainBinary::Bun);
                command.arg("add").arg("--no-save");
                for dep in &request.deps {
                    command.arg(dep);
                }
                command.current_dir(runtime_root);
                let output = command
                    .output()
                    .await
                    .context("install bun dependencies for code node")?;
                if !output.status.success() {
                    return Err(anyhow!(
                        "bun dependency install failed: {}",
                        String::from_utf8_lossy(&output.stderr)
                    ));
                }
                Ok(())
            }
            ExecutionLanguage::Go => {
                let go_mod = runtime_root.join("go.mod");
                if !go_mod.exists() {
                    let mut init = self.command_for_binary(ToolchainBinary::Go);
                    init.arg("mod").arg("init").arg("workdesk/workflow");
                    init.current_dir(runtime_root);
                    let output = init.output().await.context("initialize workflow go.mod")?;
                    if !output.status.success() {
                        return Err(anyhow!(
                            "go mod init failed: {}",
                            String::from_utf8_lossy(&output.stderr)
                        ));
                    }
                }

                let mut get = self.command_for_binary(ToolchainBinary::Go);
                get.arg("get");
                for dep in &request.deps {
                    get.arg(dep);
                }
                get.current_dir(runtime_root);
                let output = get
                    .output()
                    .await
                    .context("install go dependencies for code node")?;
                if !output.status.success() {
                    return Err(anyhow!(
                        "go get failed: {}",
                        String::from_utf8_lossy(&output.stderr)
                    ));
                }
                Ok(())
            }
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
