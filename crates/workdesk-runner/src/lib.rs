use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use std::ffi::OsStr;
use std::path::PathBuf;
use tokio::process::Command;
use uuid::Uuid;

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
