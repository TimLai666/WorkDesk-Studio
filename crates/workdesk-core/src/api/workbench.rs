use super::{ok, ApiHttpError, ApiState};
use crate::types::{
    AppendAgentWorkspaceMessageInput, ApiEnvelope, ChoicePromptAnswerInput, CodexModelCapability,
    CodexReasoningEffortOption, CreateAgentWorkspaceSessionInput, CreateChoicePromptInput,
    UpdateAgentWorkspaceSessionConfigInput,
};
use anyhow::Context;
use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use serde_json::Value;
use std::path::PathBuf;

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

pub(super) async fn list_agent_capabilities() -> Json<ApiEnvelope<Vec<CodexModelCapability>>> {
    ok(load_cached_capabilities().await.unwrap_or_default())
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
                        item.reasoning_effort_pascal
                            .or(item.effort)
                            .map(|effort| CodexReasoningEffortOption {
                                reasoning_effort: effort,
                                description: item.description,
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
