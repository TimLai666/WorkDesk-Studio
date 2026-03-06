use super::{ok, ApiHttpError, ApiState};
use crate::types::{
    ApiEnvelope, AppendAgentWorkspaceMessageInput, ChoicePromptAnswerInput, CodexModelCapability,
    CodexReasoningEffortOption, CreateAgentWorkspaceSessionInput, CreateChoicePromptInput,
    UpdateAgentWorkspaceSessionConfigInput,
};
use anyhow::{anyhow, Context};
use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use serde_json::Value;
use std::path::PathBuf;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::warn;
use uuid::Uuid;
use workdesk_domain::{CodexIpcRequest, CodexIpcResponse};

#[derive(Debug, Deserialize)]
struct CachedModelsFile {
    #[serde(default)]
    models: Vec<CachedModel>,
}

#[derive(Debug, Deserialize)]
struct CachedModel {
    #[serde(default)]
    slug: Option<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default, rename = "displayName")]
    display_name_pascal: Option<String>,
    #[serde(default)]
    default_reasoning_level: Option<String>,
    #[serde(default, rename = "defaultReasoningEffort")]
    default_reasoning_effort_pascal: Option<String>,
    #[serde(default)]
    supported_reasoning_levels: Vec<CachedReasoningLevel>,
    #[serde(default, rename = "supportedReasoningEfforts")]
    supported_reasoning_efforts_pascal: Vec<CachedReasoningLevel>,
}

#[derive(Debug, Deserialize)]
struct CachedReasoningLevel {
    #[serde(default)]
    effort: Option<String>,
    #[serde(default, rename = "reasoningEffort")]
    reasoning_effort_pascal: Option<String>,
    description: String,
}

#[derive(Debug, Deserialize)]
struct SidecarCapabilityEnvelope {
    #[serde(default)]
    capabilities: Vec<SidecarCapability>,
    #[serde(default)]
    models: Vec<SidecarCapability>,
}

#[derive(Debug, Deserialize)]
struct SidecarCapability {
    model: String,
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    reasoning_values: Vec<SidecarReasoningEffortOption>,
    #[serde(default)]
    default_reasoning_effort: Option<String>,
    #[serde(default)]
    supports_speed: Option<bool>,
    #[serde(default)]
    supports_plan_mode: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct SidecarReasoningEffortOption {
    reasoning_effort: String,
    #[serde(default)]
    description: String,
}

pub(super) async fn list_agent_capabilities(
    State(_state): State<ApiState>,
) -> Json<ApiEnvelope<Vec<CodexModelCapability>>> {
    match load_sidecar_capabilities().await {
        Ok(Some(capabilities)) => ok(capabilities),
        Ok(None) => ok(load_cached_capabilities().await.unwrap_or_default()),
        Err(error) => {
            warn!("load capabilities from sidecar failed, fallback to cache: {error:#}");
            ok(load_cached_capabilities().await.unwrap_or_default())
        }
    }
}

pub(super) async fn list_agent_sessions(
    State(state): State<ApiState>,
) -> Result<Json<ApiEnvelope<Vec<crate::types::AgentWorkspaceSession>>>, ApiHttpError> {
    let sessions = state
        .service
        .list_agent_workspace_sessions()
        .await
        .map_err(ApiHttpError::from)?;
    Ok(ok(sessions))
}

pub(super) async fn create_agent_session(
    State(state): State<ApiState>,
    Json(input): Json<CreateAgentWorkspaceSessionInput>,
) -> Result<Json<ApiEnvelope<crate::types::AgentWorkspaceSession>>, ApiHttpError> {
    let session = state
        .service
        .create_agent_workspace_session(input)
        .await
        .map_err(ApiHttpError::from)?;
    Ok(ok(session))
}

pub(super) async fn update_agent_session_config(
    Path(session_id): Path<String>,
    State(state): State<ApiState>,
    Json(input): Json<UpdateAgentWorkspaceSessionConfigInput>,
) -> Result<Json<ApiEnvelope<crate::types::AgentWorkspaceSession>>, ApiHttpError> {
    let session = state
        .service
        .update_agent_workspace_session_config(&session_id, input)
        .await
        .map_err(ApiHttpError::from)?;
    Ok(ok(session))
}

pub(super) async fn list_session_messages(
    Path(session_id): Path<String>,
    State(state): State<ApiState>,
) -> Result<Json<ApiEnvelope<Vec<crate::types::AgentWorkspaceMessage>>>, ApiHttpError> {
    let messages = state
        .service
        .list_agent_workspace_messages(&session_id)
        .await
        .map_err(ApiHttpError::from)?;
    Ok(ok(messages))
}

pub(super) async fn post_session_message(
    Path(session_id): Path<String>,
    State(state): State<ApiState>,
    Json(input): Json<AppendAgentWorkspaceMessageInput>,
) -> Result<Json<ApiEnvelope<crate::types::AgentWorkspaceMessage>>, ApiHttpError> {
    let message = state
        .service
        .append_agent_workspace_message_input(&session_id, input)
        .await
        .map_err(ApiHttpError::from)?;
    Ok(ok(message))
}

pub(super) async fn list_choice_prompts(
    Path(session_id): Path<String>,
    State(state): State<ApiState>,
) -> Result<Json<ApiEnvelope<Vec<crate::types::ChoicePrompt>>>, ApiHttpError> {
    let prompts = state
        .service
        .list_choice_prompts(&session_id)
        .await
        .map_err(ApiHttpError::from)?;
    Ok(ok(prompts))
}

pub(super) async fn create_choice_prompt(
    Path(session_id): Path<String>,
    State(state): State<ApiState>,
    Json(input): Json<CreateChoicePromptInput>,
) -> Result<Json<ApiEnvelope<crate::types::ChoicePrompt>>, ApiHttpError> {
    let prompt = state
        .service
        .create_choice_prompt(&session_id, input)
        .await
        .map_err(ApiHttpError::from)?;
    Ok(ok(prompt))
}

pub(super) async fn answer_choice_prompt(
    Path((session_id, prompt_id)): Path<(String, String)>,
    State(state): State<ApiState>,
    Json(input): Json<ChoicePromptAnswerInput>,
) -> Result<Json<ApiEnvelope<crate::types::ChoicePrompt>>, ApiHttpError> {
    let prompt = state
        .service
        .answer_choice_prompt(&session_id, &prompt_id, input)
        .await
        .map_err(ApiHttpError::from)?;
    Ok(ok(prompt))
}

async fn load_cached_capabilities() -> anyhow::Result<Vec<CodexModelCapability>> {
    let path = codex_models_cache_path();
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = tokio::fs::read_to_string(&path)
        .await
        .with_context(|| format!("read models cache {}", path.display()))?;
    let payload: CachedModelsFile = serde_json::from_str(&raw)
        .or_else(|_| {
            let value: Value = serde_json::from_str(&raw)?;
            let models = value
                .get("models")
                .cloned()
                .unwrap_or(Value::Array(Vec::new()));
            serde_json::from_value(Value::Object(
                [("models".to_string(), models)].into_iter().collect(),
            ))
        })
        .with_context(|| format!("parse models cache {}", path.display()))?;
    Ok(payload
        .models
        .into_iter()
        .map(|model| CodexModelCapability {
            model: model
                .model
                .or(model.slug)
                .unwrap_or_else(|| "unknown".into()),
            display_name: model
                .display_name_pascal
                .or(model.display_name)
                .unwrap_or_else(|| "unknown".into()),
            reasoning_values: {
                let mut values = model.supported_reasoning_efforts_pascal;
                values.extend(model.supported_reasoning_levels);
                values
                    .into_iter()
                    .filter_map(|item| {
                        item.reasoning_effort_pascal.or(item.effort).map(|effort| {
                            CodexReasoningEffortOption {
                                reasoning_effort: effort,
                                description: item.description,
                            }
                        })
                    })
                    .collect()
            },
            default_reasoning_effort: model
                .default_reasoning_effort_pascal
                .or(model.default_reasoning_level),
            supports_speed: false,
            supports_plan_mode: true,
        })
        .collect())
}

async fn load_sidecar_capabilities() -> anyhow::Result<Option<Vec<CodexModelCapability>>> {
    let endpoint = std::env::var("WORKDESK_SIDECAR_ENDPOINT")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let Some(endpoint) = endpoint else {
        return Ok(None);
    };

    let request = CodexIpcRequest {
        request_type: "get_capabilities".into(),
        payload: Value::Object(Default::default()),
        request_id: Uuid::new_v4().to_string(),
    };
    let response = send_sidecar_request(&endpoint, &request).await?;
    if !response.ok {
        let message = response
            .error
            .map(|error| format!("{}: {}", error.code, error.message))
            .unwrap_or_else(|| "unknown sidecar error".to_string());
        return Err(anyhow!(
            "sidecar returned error for get_capabilities: {message}"
        ));
    }

    let payload = response.data.unwrap_or(Value::Array(Vec::new()));
    let capabilities = parse_sidecar_capabilities(payload)?;
    Ok(Some(capabilities))
}

fn parse_sidecar_capabilities(payload: Value) -> anyhow::Result<Vec<CodexModelCapability>> {
    let raw_capabilities = if payload.is_array() {
        serde_json::from_value::<Vec<SidecarCapability>>(payload)?
    } else if payload.is_object() {
        let envelope: SidecarCapabilityEnvelope = serde_json::from_value(payload)?;
        if !envelope.capabilities.is_empty() {
            envelope.capabilities
        } else {
            envelope.models
        }
    } else {
        return Err(anyhow!("unexpected sidecar capability payload shape"));
    };

    Ok(raw_capabilities
        .into_iter()
        .map(|capability| CodexModelCapability {
            model: capability.model.clone(),
            display_name: capability
                .display_name
                .unwrap_or_else(|| capability.model.clone()),
            reasoning_values: capability
                .reasoning_values
                .into_iter()
                .map(|value| CodexReasoningEffortOption {
                    reasoning_effort: value.reasoning_effort,
                    description: value.description,
                })
                .collect(),
            default_reasoning_effort: capability.default_reasoning_effort,
            supports_speed: capability.supports_speed.unwrap_or(false),
            supports_plan_mode: capability.supports_plan_mode.unwrap_or(false),
        })
        .collect())
}

#[cfg(windows)]
async fn send_sidecar_request(
    endpoint: &str,
    request: &CodexIpcRequest,
) -> anyhow::Result<CodexIpcResponse> {
    use tokio::net::windows::named_pipe::ClientOptions;

    let mut client = ClientOptions::new()
        .open(endpoint)
        .with_context(|| format!("open sidecar pipe: {endpoint}"))?;
    let raw = format!("{}\n", serde_json::to_string(request)?);
    client
        .write_all(raw.as_bytes())
        .await
        .context("write sidecar capability request")?;
    client
        .flush()
        .await
        .context("flush sidecar capability request")?;

    let mut response = Vec::new();
    client
        .read_to_end(&mut response)
        .await
        .context("read sidecar capability response")?;
    serde_json::from_slice::<CodexIpcResponse>(&response)
        .context("decode sidecar capability response")
}

#[cfg(not(windows))]
async fn send_sidecar_request(
    endpoint: &str,
    request: &CodexIpcRequest,
) -> anyhow::Result<CodexIpcResponse> {
    let mut stream = tokio::net::TcpStream::connect(endpoint)
        .await
        .with_context(|| format!("connect sidecar socket: {endpoint}"))?;
    let raw = format!("{}\n", serde_json::to_string(request)?);
    stream
        .write_all(raw.as_bytes())
        .await
        .context("write sidecar capability request")?;
    stream
        .flush()
        .await
        .context("flush sidecar capability request")?;

    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .await
        .context("read sidecar capability response")?;
    serde_json::from_slice::<CodexIpcResponse>(&response)
        .context("decode sidecar capability response")
}

fn codex_models_cache_path() -> PathBuf {
    if let Ok(codex_home) = std::env::var("CODEX_HOME") {
        return PathBuf::from(codex_home).join("models_cache.json");
    }
    if cfg!(windows) {
        if let Ok(user_profile) = std::env::var("USERPROFILE") {
            return PathBuf::from(user_profile)
                .join(".codex")
                .join("models_cache.json");
        }
    }
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".codex")
        .join("models_cache.json")
}
