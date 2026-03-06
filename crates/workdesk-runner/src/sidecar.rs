use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;
use workdesk_domain::{CodexIpcRequest, CodexIpcResponse};

pub const WINDOWS_CODEX_SIDECAR_PIPE: &str = r"\\.\pipe\WorkDeskStudio.CodexSidecar";

#[cfg(not(windows))]
pub const FALLBACK_CODEX_SIDECAR_SOCKET: &str = "127.0.0.1:45873";

pub fn default_sidecar_endpoint() -> String {
    #[cfg(windows)]
    {
        return WINDOWS_CODEX_SIDECAR_PIPE.to_string();
    }
    #[cfg(not(windows))]
    {
        FALLBACK_CODEX_SIDECAR_SOCKET.to_string()
    }
}

#[derive(Debug, Clone)]
pub struct CodexSidecarClient {
    endpoint: String,
}

impl CodexSidecarClient {
    pub fn default() -> Self {
        Self::new(default_sidecar_endpoint())
    }

    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
        }
    }

    pub async fn send(&self, request_type: &str, payload: Value) -> Result<CodexIpcResponse> {
        let request = CodexIpcRequest {
            request_type: request_type.to_string(),
            payload,
            request_id: Uuid::new_v4().to_string(),
        };
        let raw = format!("{}\n", serde_json::to_string(&request)?);
        let response_raw = write_and_read_response(&self.endpoint, raw).await?;
        let response: CodexIpcResponse =
            serde_json::from_slice(&response_raw).context("decode sidecar response")?;
        if !response.ok {
            let error = response
                .error
                .as_ref()
                .map(|e| format!("{}: {}", e.code, e.message))
                .unwrap_or_else(|| "unknown sidecar error".to_string());
            return Err(anyhow!(error));
        }
        Ok(response)
    }
}

#[cfg(windows)]
async fn write_and_read_response(endpoint: &str, message: String) -> Result<Vec<u8>> {
    use tokio::net::windows::named_pipe::ClientOptions;

    let mut client = ClientOptions::new()
        .open(endpoint)
        .with_context(|| format!("open sidecar pipe: {endpoint}"))?;
    client
        .write_all(message.as_bytes())
        .await
        .context("write sidecar request")?;
    client.flush().await.context("flush sidecar request")?;
    let mut response = Vec::new();
    client
        .read_to_end(&mut response)
        .await
        .context("read sidecar response")?;
    Ok(response)
}

#[cfg(not(windows))]
async fn write_and_read_response(endpoint: &str, message: String) -> Result<Vec<u8>> {
    let mut stream = tokio::net::TcpStream::connect(endpoint)
        .await
        .with_context(|| format!("connect sidecar socket: {endpoint}"))?;
    stream
        .write_all(message.as_bytes())
        .await
        .context("write sidecar request")?;
    stream.flush().await.context("flush sidecar request")?;
    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .await
        .context("read sidecar response")?;
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::CodexSidecarClient;
    use anyhow::Result;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use workdesk_domain::{CodexIpcError, CodexIpcMeta, CodexIpcResponse};

    #[tokio::test]
    async fn sends_json_envelope_and_parses_response() {
        #[cfg(windows)]
        let endpoint = format!(
            r"\\.\pipe\WorkDeskStudio.CodexSidecar.Test.{}",
            uuid::Uuid::new_v4()
        );
        #[cfg(not(windows))]
        let endpoint = {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind socket");
            let addr = listener.local_addr().expect("addr").to_string();
            drop(listener);
            addr
        };

        let endpoint_for_server = endpoint.clone();
        let server = tokio::spawn(async move { run_test_server(&endpoint_for_server).await });
        tokio::time::sleep(std::time::Duration::from_millis(80)).await;

        let client = CodexSidecarClient::new(endpoint);
        let response = client
            .send("ping", serde_json::json!({"hello":"world"}))
            .await
            .expect("send request");
        assert!(response.ok);
        assert_eq!(
            response.data.expect("response data")["echo_type"],
            "ping"
        );

        server.abort();
    }

    #[cfg(windows)]
    async fn run_test_server(endpoint: &str) -> Result<()> {
        use tokio::net::windows::named_pipe::ServerOptions;
        let mut first = true;
        loop {
            let mut options = ServerOptions::new();
            if first {
                options.first_pipe_instance(true);
            }
            let mut server = options.create(endpoint)?;
            first = false;
            server.connect().await?;
            let mut line = String::new();
            {
                let mut reader = BufReader::new(&mut server);
                reader.read_line(&mut line).await?;
            }
            let request: serde_json::Value = serde_json::from_str(&line)?;
            let response = CodexIpcResponse {
                ok: true,
                data: Some(serde_json::json!({
                    "echo_type": request["type"].as_str().unwrap_or("")
                })),
                error: None::<CodexIpcError>,
                meta: CodexIpcMeta {
                    request_id: request["request_id"].as_str().unwrap_or("").to_string(),
                    timestamp: "2026-03-06T00:00:00Z".to_string(),
                },
            };
            let raw = serde_json::to_vec(&response)?;
            server.write_all(&raw).await?;
            server.flush().await?;
        }
    }

    #[cfg(not(windows))]
    async fn run_test_server(endpoint: &str) -> Result<()> {
        let listener = tokio::net::TcpListener::bind(endpoint).await?;
        loop {
            let (mut stream, _) = listener.accept().await?;
            let mut line = String::new();
            {
                let mut reader = BufReader::new(&mut stream);
                reader.read_line(&mut line).await?;
            }
            let request: serde_json::Value = serde_json::from_str(&line)?;
            let response = CodexIpcResponse {
                ok: true,
                data: Some(serde_json::json!({
                    "echo_type": request["type"].as_str().unwrap_or("")
                })),
                error: None::<CodexIpcError>,
                meta: CodexIpcMeta {
                    request_id: request["request_id"].as_str().unwrap_or("").to_string(),
                    timestamp: "2026-03-06T00:00:00Z".to_string(),
                },
            };
            let raw = serde_json::to_vec(&response)?;
            stream.write_all(&raw).await?;
            stream.flush().await?;
        }
    }
}
