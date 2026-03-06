use crate::command::DesktopCommand;
use crate::command_bus::CommandBusError;
use crate::controller::{DesktopAppController, UiStateSnapshot};
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use uuid::Uuid;

pub const WINDOWS_AUTOMATION_PIPE: &str = r"\\.\pipe\WorkDeskStudio.Automation";

#[cfg(not(windows))]
pub const FALLBACK_AUTOMATION_SOCKET: &str = "127.0.0.1:45872";

pub fn default_automation_endpoint() -> String {
    #[cfg(windows)]
    {
        return WINDOWS_AUTOMATION_PIPE.to_string();
    }
    #[cfg(not(windows))]
    {
        FALLBACK_AUTOMATION_SOCKET.to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationRequest {
    #[serde(rename = "type")]
    pub request_type: String,
    #[serde(default)]
    pub payload: Value,
    pub request_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationResponse {
    pub ok: bool,
    pub error: Option<CommandBusError>,
    pub state: Option<UiStateSnapshot>,
}

impl AutomationResponse {
    fn ok(state: UiStateSnapshot) -> Self {
        Self {
            ok: true,
            error: None,
            state: Some(state),
        }
    }

    fn fail(code: &str, message: impl Into<String>) -> Self {
        Self {
            ok: false,
            error: Some(CommandBusError {
                code: code.to_string(),
                message: message.into(),
                details: None,
            }),
            state: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AutomationServer {
    endpoint: String,
}

impl AutomationServer {
    pub fn default() -> Self {
        Self::new(default_automation_endpoint())
    }

    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
        }
    }

    pub async fn run(self, controller: std::sync::Arc<DesktopAppController>) -> Result<()> {
        run_server_loop(&self.endpoint, move |line| {
            let controller = controller.clone();
            async move {
                let request: AutomationRequest =
                    serde_json::from_str(&line).context("decode automation request")?;
                let response = handle_request(&controller, request).await;
                Ok(serde_json::to_vec(&response)?)
            }
        })
        .await
    }
}

async fn handle_request(
    controller: &DesktopAppController,
    request: AutomationRequest,
) -> AutomationResponse {
    let outcome = match request.request_type.as_str() {
        "get_state" => Ok(()),
        "refresh_runs" => controller.refresh_runs().await,
        "dispatch_command" => match parse_command(&request.payload) {
            Ok(command) => controller.dispatch_command(command).await,
            Err(error) => Err(error),
        },
        "cancel_selected_run" => controller.cancel_selected_run().await,
        "retry_selected_run" => controller.retry_selected_run().await,
        other => Err(anyhow!("unknown automation request type `{other}`")),
    };

    match outcome {
        Ok(()) => AutomationResponse::ok(controller.snapshot()),
        Err(error) => AutomationResponse::fail("AUTOMATION_REQUEST_FAILED", error.to_string()),
    }
}

fn parse_command(payload: &Value) -> Result<DesktopCommand> {
    let command = payload
        .get("command")
        .ok_or_else(|| anyhow!("payload.command is required"))?;
    serde_json::from_value(command.clone()).context("decode payload.command")
}

#[derive(Debug, Clone)]
pub struct AutomationClient {
    endpoint: String,
}

impl AutomationClient {
    pub fn default() -> Self {
        Self::new(default_automation_endpoint())
    }

    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
        }
    }

    pub async fn get_state(&self) -> Result<UiStateSnapshot> {
        self.send(AutomationRequest {
            request_type: "get_state".into(),
            payload: Value::Object(Default::default()),
            request_id: Uuid::new_v4().to_string(),
        })
        .await
    }

    pub async fn dispatch_command(&self, command: DesktopCommand) -> Result<UiStateSnapshot> {
        self.send(AutomationRequest {
            request_type: "dispatch_command".into(),
            payload: serde_json::json!({
                "command": command
            }),
            request_id: Uuid::new_v4().to_string(),
        })
        .await
    }

    pub async fn cancel_selected_run(&self) -> Result<UiStateSnapshot> {
        self.send(AutomationRequest {
            request_type: "cancel_selected_run".into(),
            payload: Value::Object(Default::default()),
            request_id: Uuid::new_v4().to_string(),
        })
        .await
    }

    pub async fn retry_selected_run(&self) -> Result<UiStateSnapshot> {
        self.send(AutomationRequest {
            request_type: "retry_selected_run".into(),
            payload: Value::Object(Default::default()),
            request_id: Uuid::new_v4().to_string(),
        })
        .await
    }

    async fn send(&self, request: AutomationRequest) -> Result<UiStateSnapshot> {
        let message = format!("{}\n", serde_json::to_string(&request)?);
        let response_raw = write_and_read_response(&self.endpoint, message).await?;
        let response: AutomationResponse =
            serde_json::from_slice(&response_raw).context("decode automation response")?;
        if !response.ok {
            let error = response.error.unwrap_or(CommandBusError {
                code: "AUTOMATION_UNKNOWN".into(),
                message: "unknown automation error".into(),
                details: None,
            });
            return Err(anyhow!("{}: {}", error.code, error.message));
        }
        response
            .state
            .ok_or_else(|| anyhow!("automation response missing state"))
    }
}

#[cfg(windows)]
async fn write_and_read_response(endpoint: &str, message: String) -> Result<Vec<u8>> {
    use tokio::net::windows::named_pipe::ClientOptions;

    let mut client = ClientOptions::new()
        .open(endpoint)
        .with_context(|| format!("open automation pipe: {endpoint}"))?;
    client
        .write_all(message.as_bytes())
        .await
        .context("write automation request")?;
    client.flush().await.context("flush automation request")?;
    let mut response = Vec::new();
    client
        .read_to_end(&mut response)
        .await
        .context("read automation response")?;
    Ok(response)
}

#[cfg(not(windows))]
async fn write_and_read_response(endpoint: &str, message: String) -> Result<Vec<u8>> {
    let mut stream = tokio::net::TcpStream::connect(endpoint)
        .await
        .with_context(|| format!("connect automation socket: {endpoint}"))?;
    stream
        .write_all(message.as_bytes())
        .await
        .context("write automation request")?;
    stream.flush().await.context("flush automation request")?;
    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .await
        .context("read automation response")?;
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
            .with_context(|| format!("create automation pipe: {endpoint}"))?;
        first_pipe_instance = false;

        server
            .connect()
            .await
            .with_context(|| format!("connect automation pipe: {endpoint}"))?;

        let mut line = String::new();
        {
            let mut reader = BufReader::new(&mut server);
            reader
                .read_line(&mut line)
                .await
                .context("read automation request line")?;
        }
        let response = match handler(line).await {
            Ok(raw) => raw,
            Err(error) => serde_json::to_vec(&AutomationResponse::fail(
                "AUTOMATION_BAD_REQUEST",
                error.to_string(),
            ))?,
        };
        server
            .write_all(&response)
            .await
            .context("write automation response")?;
        server.flush().await.context("flush automation response")?;
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
        .with_context(|| format!("bind automation socket: {endpoint}"))?;
    loop {
        let (mut stream, _) = listener.accept().await.context("accept automation")?;
        let mut line = String::new();
        {
            let mut reader = BufReader::new(&mut stream);
            reader
                .read_line(&mut line)
                .await
                .context("read automation request line")?;
        }
        let response = match handler(line).await {
            Ok(raw) => raw,
            Err(error) => serde_json::to_vec(&AutomationResponse::fail(
                "AUTOMATION_BAD_REQUEST",
                error.to_string(),
            ))?,
        };
        stream
            .write_all(&response)
            .await
            .context("write automation response")?;
        stream.flush().await.context("flush automation response")?;
    }
}
