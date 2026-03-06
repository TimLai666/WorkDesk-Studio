use crate::command::DesktopCommand;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use uuid::Uuid;

pub const WINDOWS_COMMAND_PIPE: &str = r"\\.\pipe\WorkDeskStudio.CommandBus";

#[cfg(not(windows))]
pub const FALLBACK_COMMAND_SOCKET: &str = "127.0.0.1:45871";

pub fn default_command_endpoint() -> String {
    #[cfg(windows)]
    {
        return WINDOWS_COMMAND_PIPE.to_string();
    }
    #[cfg(not(windows))]
    {
        FALLBACK_COMMAND_SOCKET.to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandBusRequest {
    #[serde(rename = "type")]
    pub command_type: String,
    #[serde(default)]
    pub payload: Value,
    pub request_id: String,
}

impl CommandBusRequest {
    pub fn from_command(command: &DesktopCommand) -> Result<Self> {
        let (command_type, payload) = match command {
            DesktopCommand::Open => ("open".to_string(), Value::Object(Default::default())),
            DesktopCommand::OpenRun { run_id } => (
                "open_run".to_string(),
                json!({
                    "run_id": run_id
                }),
            ),
            DesktopCommand::OpenWorkflow { workflow_id } => (
                "open_workflow".to_string(),
                json!({
                    "workflow_id": workflow_id
                }),
            ),
            DesktopCommand::RunWorkflow { workflow_id } => (
                "run_workflow".to_string(),
                json!({
                    "workflow_id": workflow_id
                }),
            ),
        };
        Ok(Self {
            command_type,
            payload,
            request_id: Uuid::new_v4().to_string(),
        })
    }

    pub fn into_command(self) -> Result<DesktopCommand> {
        match self.command_type.as_str() {
            "open" => Ok(DesktopCommand::Open),
            "open_run" => {
                let run_id = self
                    .payload
                    .get("run_id")
                    .and_then(Value::as_str)
                    .ok_or_else(|| anyhow!("missing payload.run_id"))?;
                Ok(DesktopCommand::OpenRun {
                    run_id: run_id.to_string(),
                })
            }
            "open_workflow" => {
                let workflow_id = self
                    .payload
                    .get("workflow_id")
                    .and_then(Value::as_str)
                    .ok_or_else(|| anyhow!("missing payload.workflow_id"))?;
                Ok(DesktopCommand::OpenWorkflow {
                    workflow_id: workflow_id.to_string(),
                })
            }
            "run_workflow" => {
                let workflow_id = self
                    .payload
                    .get("workflow_id")
                    .and_then(Value::as_str)
                    .ok_or_else(|| anyhow!("missing payload.workflow_id"))?;
                Ok(DesktopCommand::RunWorkflow {
                    workflow_id: workflow_id.to_string(),
                })
            }
            other => Err(anyhow!("unsupported command type: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandBusError {
    pub code: String,
    pub message: String,
    pub details: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandBusResponse {
    pub ok: bool,
    pub error: Option<CommandBusError>,
}

impl CommandBusResponse {
    pub fn ok() -> Self {
        Self {
            ok: true,
            error: None,
        }
    }

    pub fn fail(code: &str, message: impl Into<String>, details: Option<Value>) -> Self {
        Self {
            ok: false,
            error: Some(CommandBusError {
                code: code.to_string(),
                message: message.into(),
                details,
            }),
        }
    }
}

#[async_trait]
pub trait CommandDispatcher: Send + Sync {
    async fn dispatch(&self, command: DesktopCommand) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct CommandBusClient {
    endpoint: String,
}

impl CommandBusClient {
    pub fn default() -> Self {
        Self::new(default_command_endpoint())
    }

    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
        }
    }

    pub async fn send(&self, command: &DesktopCommand) -> Result<CommandBusResponse> {
        let request = CommandBusRequest::from_command(command)?;
        self.send_request(&request).await
    }

    pub async fn send_request(&self, request: &CommandBusRequest) -> Result<CommandBusResponse> {
        let message = format!("{}\n", serde_json::to_string(request)?);
        let response_raw = write_and_read_response(&self.endpoint, message).await?;
        let response: CommandBusResponse =
            serde_json::from_slice(&response_raw).context("decode command bus response")?;
        Ok(response)
    }
}

#[derive(Debug, Clone)]
pub struct CommandBusServer {
    endpoint: String,
}

impl CommandBusServer {
    pub fn default() -> Self {
        Self::new(default_command_endpoint())
    }

    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
        }
    }

    pub async fn run(self, dispatcher: std::sync::Arc<dyn CommandDispatcher>) -> Result<()> {
        run_server_loop(&self.endpoint, move |line| {
            let dispatcher = dispatcher.clone();
            async move {
                let request: CommandBusRequest =
                    serde_json::from_str(&line).context("decode command bus request")?;
                let command = request
                    .into_command()
                    .context("parse desktop command from command bus request")?;
                let response = match dispatcher.dispatch(command).await {
                    Ok(()) => CommandBusResponse::ok(),
                    Err(error) => CommandBusResponse::fail(
                        "COMMAND_DISPATCH_FAILED",
                        error.to_string(),
                        Some(json!({ "cause": format!("{error:#}") })),
                    ),
                };
                Ok(serde_json::to_vec(&response)?)
            }
        })
        .await
    }
}

#[cfg(windows)]
async fn write_and_read_response(endpoint: &str, message: String) -> Result<Vec<u8>> {
    use tokio::net::windows::named_pipe::ClientOptions;

    let mut client = ClientOptions::new()
        .open(endpoint)
        .with_context(|| format!("open named pipe client: {endpoint}"))?;
    client
        .write_all(message.as_bytes())
        .await
        .context("write command bus request")?;
    client.flush().await.context("flush command bus request")?;
    let mut response = Vec::new();
    client
        .read_to_end(&mut response)
        .await
        .context("read command bus response")?;
    Ok(response)
}

#[cfg(not(windows))]
async fn write_and_read_response(endpoint: &str, message: String) -> Result<Vec<u8>> {
    let mut stream = tokio::net::TcpStream::connect(endpoint)
        .await
        .with_context(|| format!("connect command bus socket: {endpoint}"))?;
    stream
        .write_all(message.as_bytes())
        .await
        .context("write command bus request")?;
    stream.flush().await.context("flush command bus request")?;
    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .await
        .context("read command bus response")?;
    Ok(response)
}

#[cfg(windows)]
async fn run_server_loop<H, Fut>(endpoint: &str, handler: H) -> Result<()>
where
    H: Fn(String) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<Vec<u8>>> + Send,
{
    use tokio::net::windows::named_pipe::ServerOptions;
    let mut first_pipe_instance = true;
    loop {
        let mut options = ServerOptions::new();
        if first_pipe_instance {
            options.first_pipe_instance(true);
        }
        let mut server = options
            .create(endpoint)
            .with_context(|| format!("create named pipe server: {endpoint}"))?;
        first_pipe_instance = false;
        server
            .connect()
            .await
            .with_context(|| format!("connect named pipe server: {endpoint}"))?;

        let mut line = String::new();
        {
            let mut reader = BufReader::new(&mut server);
            reader
                .read_line(&mut line)
                .await
                .context("read named pipe request line")?;
        }

        let response = match handler(line).await {
            Ok(raw) => raw,
            Err(error) => serde_json::to_vec(&CommandBusResponse::fail(
                "COMMAND_BUS_BAD_REQUEST",
                error.to_string(),
                None,
            ))?,
        };

        server
            .write_all(&response)
            .await
            .context("write named pipe response")?;
        server.flush().await.context("flush named pipe response")?;
    }
}

#[cfg(not(windows))]
async fn run_server_loop<H, Fut>(endpoint: &str, handler: H) -> Result<()>
where
    H: Fn(String) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<Vec<u8>>> + Send,
{
    let listener = tokio::net::TcpListener::bind(endpoint)
        .await
        .with_context(|| format!("bind command bus socket: {endpoint}"))?;
    loop {
        let (mut stream, _) = listener.accept().await.context("accept command bus")?;
        let mut line = String::new();
        {
            let mut reader = BufReader::new(&mut stream);
            reader
                .read_line(&mut line)
                .await
                .context("read command bus request line")?;
        }
        let response = match handler(line).await {
            Ok(raw) => raw,
            Err(error) => serde_json::to_vec(&CommandBusResponse::fail(
                "COMMAND_BUS_BAD_REQUEST",
                error.to_string(),
                None,
            ))?,
        };
        stream
            .write_all(&response)
            .await
            .context("write command bus response")?;
        stream.flush().await.context("flush command bus response")?;
    }
}

#[cfg(test)]
mod tests {
    use super::{CommandBusRequest, CommandBusResponse};
    use crate::command::DesktopCommand;

    #[test]
    fn command_request_roundtrip() {
        let request = CommandBusRequest::from_command(&DesktopCommand::OpenRun {
            run_id: "run-a".into(),
        })
        .expect("request");
        let json = serde_json::to_string(&request).expect("encode");
        let decoded: CommandBusRequest = serde_json::from_str(&json).expect("decode");
        let command = decoded.into_command().expect("into command");
        assert_eq!(
            command,
            DesktopCommand::OpenRun {
                run_id: "run-a".into()
            }
        );
    }

    #[test]
    fn response_error_serialization() {
        let response = CommandBusResponse::fail("E_TEST", "boom", None);
        let encoded = serde_json::to_string(&response).expect("encode");
        let decoded: CommandBusResponse = serde_json::from_str(&encoded).expect("decode");
        assert!(!decoded.ok);
        let error = decoded.error.expect("error");
        assert_eq!(error.code, "E_TEST");
    }
}
