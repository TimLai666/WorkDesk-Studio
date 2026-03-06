use anyhow::{anyhow, Context, Result};
use reqwest::Url;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::Value;
use workdesk_core::{AuthLoginInput, AuthSessionResponse, WorkflowDefinition};

#[derive(Debug, Clone)]
pub struct ApiClient {
    http: reqwest::Client,
    base_url: Url,
}

#[derive(Debug, Deserialize)]
struct ApiEnvelope<T> {
    data: Option<T>,
    error: Option<ApiError>,
    meta: ApiMeta,
}

#[derive(Debug, Deserialize)]
struct ApiError {
    code: String,
    message: String,
    details: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct ApiMeta {
    request_id: String,
    timestamp: String,
}

impl ApiClient {
    pub fn new(base_url: &str) -> Result<Self> {
        let base_url = Url::parse(base_url).context("invalid WORKDESK_REMOTE_URL")?;
        Ok(Self {
            http: reqwest::Client::new(),
            base_url,
        })
    }

    pub async fn health(&self) -> Result<Value> {
        let url = self.endpoint("/api/v1/health")?;
        let response = self.http.get(url).send().await.context("request health")?;
        parse_envelope(response).await
    }

    pub async fn list_workflows(&self) -> Result<Vec<WorkflowDefinition>> {
        let url = self.endpoint("/api/v1/workflows")?;
        let response = self
            .http
            .get(url)
            .send()
            .await
            .context("request workflows")?;
        parse_envelope(response).await
    }

    pub async fn login(&self, input: &AuthLoginInput) -> Result<AuthSessionResponse> {
        let url = self.endpoint("/api/v1/auth/login")?;
        let response = self
            .http
            .post(url)
            .json(input)
            .send()
            .await
            .context("request auth login")?;
        parse_envelope(response).await
    }

    fn endpoint(&self, path: &str) -> Result<Url> {
        self.base_url
            .join(path)
            .with_context(|| format!("build endpoint for path: {path}"))
    }
}

async fn parse_envelope<T>(response: reqwest::Response) -> Result<T>
where
    T: DeserializeOwned,
{
    let status = response.status();
    let envelope: ApiEnvelope<T> = response.json().await.context("decode API envelope")?;
    let _request_id = &envelope.meta.request_id;
    let _timestamp = &envelope.meta.timestamp;
    if let Some(error) = envelope.error {
        let details = error
            .details
            .map(|d| d.to_string())
            .unwrap_or_else(|| "null".to_string());
        return Err(anyhow!(
            "api_error status={} code={} message={} details={}",
            status.as_u16(),
            error.code,
            error.message,
            details
        ));
    }
    envelope
        .data
        .ok_or_else(|| anyhow!("api response missing data field"))
}
