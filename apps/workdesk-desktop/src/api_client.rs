use anyhow::{anyhow, Context, Result};
use reqwest::Url;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::Value;
use workdesk_core::{
    AgentWorkspaceMessage, AgentWorkspaceSession, AppendAgentWorkspaceMessageInput,
    AuthLoginInput, AuthSessionResponse, CancelRunInput, ChoicePrompt,
    ChoicePromptAnswerInput, CodexModelCapability, CodexNativeSessionConfig, RetryRunInput,
    RunSkillSnapshot, RunWorkflowInput, TerminalSessionResponse, TerminalStartInput,
    UpdateAgentWorkspaceSessionConfigInput, WorkflowDefinition, WorkflowRun, WorkflowRunEvent,
    WorkflowRunNodeState, WorkflowStatus, UpdateWorkflowStatusInput,
};

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

    pub async fn update_workflow_status(
        &self,
        workflow_id: &str,
        status: WorkflowStatus,
    ) -> Result<WorkflowDefinition> {
        let url = self.endpoint(&format!("/api/v1/workflows/{workflow_id}/status"))?;
        let response = self
            .http
            .patch(url)
            .json(&UpdateWorkflowStatusInput { status })
            .send()
            .await
            .with_context(|| format!("update workflow status for {workflow_id}"))?;
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

    pub async fn list_runs(&self, limit: usize) -> Result<Vec<WorkflowRun>> {
        let url = self.endpoint(&format!("/api/v1/runs?limit={}", limit.clamp(1, 500)))?;
        let response = self.http.get(url).send().await.context("request runs")?;
        parse_envelope(response).await
    }

    pub async fn list_run_events(
        &self,
        run_id: &str,
        after_seq: i64,
        limit: usize,
    ) -> Result<Vec<WorkflowRunEvent>> {
        let url = self.endpoint(&format!(
            "/api/v1/runs/{run_id}/events?after_seq={after_seq}&limit={}",
            limit.clamp(1, 2000)
        ))?;
        let response = self
            .http
            .get(url)
            .send()
            .await
            .with_context(|| format!("request run events for {run_id}"))?;
        parse_envelope(response).await
    }

    pub async fn list_run_skills(&self, run_id: &str) -> Result<Vec<RunSkillSnapshot>> {
        let url = self.endpoint(&format!("/api/v1/runs/{run_id}/skills"))?;
        let response = self
            .http
            .get(url)
            .send()
            .await
            .with_context(|| format!("request run skills for {run_id}"))?;
        parse_envelope(response).await
    }

    pub async fn list_run_nodes(&self, run_id: &str) -> Result<Vec<WorkflowRunNodeState>> {
        let url = self.endpoint(&format!("/api/v1/runs/{run_id}/nodes"))?;
        let response = self
            .http
            .get(url)
            .send()
            .await
            .with_context(|| format!("request run nodes for {run_id}"))?;
        parse_envelope(response).await
    }

    pub async fn run_workflow(
        &self,
        workflow_id: &str,
        requested_by: Option<&str>,
    ) -> Result<WorkflowRun> {
        let url = self.endpoint(&format!("/api/v1/workflows/{workflow_id}/run"))?;
        let response = self
            .http
            .post(url)
            .json(&RunWorkflowInput {
                requested_by: requested_by.map(ToString::to_string),
            })
            .send()
            .await
            .with_context(|| format!("request run workflow {workflow_id}"))?;
        parse_envelope(response).await
    }

    pub async fn cancel_run(
        &self,
        run_id: &str,
        requested_by: Option<&str>,
    ) -> Result<WorkflowRun> {
        let url = self.endpoint(&format!("/api/v1/runs/{run_id}/cancel"))?;
        let response = self
            .http
            .post(url)
            .json(&CancelRunInput {
                requested_by: requested_by.map(ToString::to_string),
            })
            .send()
            .await
            .with_context(|| format!("request cancel run {run_id}"))?;
        parse_envelope(response).await
    }

    pub async fn retry_run(&self, run_id: &str, requested_by: Option<&str>) -> Result<WorkflowRun> {
        let url = self.endpoint(&format!("/api/v1/runs/{run_id}/retry"))?;
        let response = self
            .http
            .post(url)
            .json(&RetryRunInput {
                requested_by: requested_by.map(ToString::to_string),
            })
            .send()
            .await
            .with_context(|| format!("request retry run {run_id}"))?;
        parse_envelope(response).await
    }

    pub async fn list_agent_capabilities(&self) -> Result<Vec<CodexModelCapability>> {
        let url = self.endpoint("/api/v1/agent/capabilities")?;
        let response = self
            .http
            .get(url)
            .send()
            .await
            .context("request agent capabilities")?;
        parse_envelope(response).await
    }

    pub async fn list_agent_workspace_sessions(&self) -> Result<Vec<AgentWorkspaceSession>> {
        let url = self.endpoint("/api/v1/agent/sessions")?;
        let response = self
            .http
            .get(url)
            .send()
            .await
            .context("request agent sessions")?;
        parse_envelope(response).await
    }

    pub async fn update_agent_workspace_session_config(
        &self,
        session_id: &str,
        config: CodexNativeSessionConfig,
        last_active_panel: Option<&str>,
    ) -> Result<AgentWorkspaceSession> {
        let url = self.endpoint(&format!("/api/v1/agent/sessions/{session_id}/config"))?;
        let response = self
            .http
            .patch(url)
            .json(&UpdateAgentWorkspaceSessionConfigInput {
                model: config.model,
                model_reasoning_effort: config.model_reasoning_effort,
                speed: config.speed,
                plan_mode: Some(config.plan_mode),
                last_active_panel: last_active_panel.map(ToString::to_string),
            })
            .send()
            .await
            .with_context(|| format!("update agent session {session_id}"))?;
        parse_envelope(response).await
    }

    pub async fn list_agent_workspace_messages(
        &self,
        session_id: &str,
    ) -> Result<Vec<AgentWorkspaceMessage>> {
        let url = self.endpoint(&format!("/api/v1/agent/sessions/{session_id}/messages"))?;
        let response = self
            .http
            .get(url)
            .send()
            .await
            .with_context(|| format!("request session messages for {session_id}"))?;
        parse_envelope(response).await
    }

    pub async fn append_agent_workspace_message(
        &self,
        session_id: &str,
        input: &AppendAgentWorkspaceMessageInput,
    ) -> Result<AgentWorkspaceMessage> {
        let url = self.endpoint(&format!("/api/v1/agent/sessions/{session_id}/messages"))?;
        let response = self
            .http
            .post(url)
            .json(input)
            .send()
            .await
            .with_context(|| format!("append session message for {session_id}"))?;
        parse_envelope(response).await
    }

    pub async fn list_choice_prompts(&self, session_id: &str) -> Result<Vec<ChoicePrompt>> {
        let url = self.endpoint(&format!(
            "/api/v1/agent/sessions/{session_id}/choice-prompts"
        ))?;
        let response = self
            .http
            .get(url)
            .send()
            .await
            .with_context(|| format!("request choice prompts for {session_id}"))?;
        parse_envelope(response).await
    }

    pub async fn answer_choice_prompt(
        &self,
        session_id: &str,
        prompt_id: &str,
        selected_option_id: Option<&str>,
        freeform_answer: Option<&str>,
    ) -> Result<ChoicePrompt> {
        let url = self.endpoint(&format!(
            "/api/v1/agent/sessions/{session_id}/choice-prompts/{prompt_id}/answer"
        ))?;
        let response = self
            .http
            .post(url)
            .json(&ChoicePromptAnswerInput {
                selected_option_id: selected_option_id.map(ToString::to_string),
                freeform_answer: freeform_answer.map(ToString::to_string),
            })
            .send()
            .await
            .with_context(|| format!("answer choice prompt {prompt_id}"))?;
        parse_envelope(response).await
    }

    pub async fn fs_search(
        &self,
        path: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<workdesk_core::FsSearchMatch>> {
        let url = self.endpoint(&format!(
            "/api/v1/fs/search?path={}&query={}&limit={}",
            urlencoding::encode(path),
            urlencoding::encode(query),
            limit.clamp(1, 2000)
        ))?;
        let response = self
            .http
            .get(url)
            .send()
            .await
            .context("request fs search")?;
        parse_envelope(response).await
    }

    pub async fn fs_tree(&self, path: &str) -> Result<Vec<workdesk_core::FsTreeEntry>> {
        let url = self.endpoint(&format!(
            "/api/v1/fs/tree?path={}",
            urlencoding::encode(path)
        ))?;
        let response = self
            .http
            .get(url)
            .send()
            .await
            .context("request fs tree")?;
        parse_envelope(response).await
    }

    pub async fn fs_read(&self, path: &str) -> Result<workdesk_core::FsReadResponse> {
        let url = self.endpoint(&format!(
            "/api/v1/fs/file?path={}",
            urlencoding::encode(path)
        ))?;
        let response = self
            .http
            .get(url)
            .send()
            .await
            .context("request fs read")?;
        parse_envelope(response).await
    }

    pub async fn fs_write(&self, path: &str, content_base64: String) -> Result<Value> {
        let url = self.endpoint("/api/v1/fs/file")?;
        let response = self
            .http
            .put(url)
            .json(&workdesk_core::FsWriteInput {
                path: path.to_string(),
                content_base64,
            })
            .send()
            .await
            .context("request fs write")?;
        parse_envelope(response).await
    }

    pub async fn fs_move(&self, from: &str, to: &str) -> Result<Value> {
        let url = self.endpoint("/api/v1/fs/move")?;
        let response = self
            .http
            .post(url)
            .json(&workdesk_core::FsMoveInput {
                from: from.to_string(),
                to: to.to_string(),
            })
            .send()
            .await
            .context("request fs move")?;
        parse_envelope(response).await
    }

    pub async fn fs_delete(&self, path: &str) -> Result<Value> {
        let url = self.endpoint(&format!(
            "/api/v1/fs/path?path={}",
            urlencoding::encode(path)
        ))?;
        let response = self
            .http
            .delete(url)
            .send()
            .await
            .context("request fs delete")?;
        parse_envelope(response).await
    }

    pub async fn fs_diff(&self, left_path: &str, right_path: &str) -> Result<workdesk_core::FsDiffResponse> {
        let url = self.endpoint("/api/v1/fs/diff")?;
        let response = self
            .http
            .post(url)
            .json(&workdesk_core::FsDiffInput {
                left_path: left_path.to_string(),
                right_path: right_path.to_string(),
            })
            .send()
            .await
            .context("request fs diff")?;
        parse_envelope(response).await
    }

    pub async fn terminal_start(&self, input: &TerminalStartInput) -> Result<TerminalSessionResponse> {
        let url = self.endpoint("/api/v1/fs/terminal/start")?;
        let response = self
            .http
            .post(url)
            .json(input)
            .send()
            .await
            .context("request terminal start")?;
        parse_envelope(response).await
    }

    pub async fn terminal_session(&self, session_id: &str) -> Result<TerminalSessionResponse> {
        let url = self.endpoint(&format!("/api/v1/fs/terminal/session/{session_id}"))?;
        let response = self
            .http
            .get(url)
            .send()
            .await
            .context("request terminal session")?;
        parse_envelope(response).await
    }

    pub async fn office_open(&self, path: &str) -> Result<workdesk_core::FsReadResponse> {
        let url = self.endpoint("/api/v1/office/open")?;
        let response = self
            .http
            .post(url)
            .json(&workdesk_core::OfficeOpenInput {
                path: path.to_string(),
            })
            .send()
            .await
            .context("request office open")?;
        parse_envelope(response).await
    }

    pub async fn office_save(&self, path: &str, content_base64: String) -> Result<Value> {
        let url = self.endpoint("/api/v1/office/save")?;
        let response = self
            .http
            .post(url)
            .json(&workdesk_core::OfficeSaveInput {
                path: path.to_string(),
                content_base64,
            })
            .send()
            .await
            .context("request office save")?;
        parse_envelope(response).await
    }

    pub async fn office_versions(&self, path: &str) -> Result<workdesk_core::OfficeVersionResponse> {
        let url = self.endpoint(&format!(
            "/api/v1/office/version?path={}",
            urlencoding::encode(path)
        ))?;
        let response = self
            .http
            .get(url)
            .send()
            .await
            .context("request office versions")?;
        parse_envelope(response).await
    }

    pub async fn pdf_preview(&self, path: &str) -> Result<workdesk_core::FsReadResponse> {
        let url = self.endpoint("/api/v1/office/pdf/preview")?;
        let response = self
            .http
            .post(url)
            .json(&workdesk_core::PdfPreviewInput {
                path: path.to_string(),
            })
            .send()
            .await
            .context("request pdf preview")?;
        parse_envelope(response).await
    }

    pub async fn pdf_annotate(
        &self,
        path: &str,
        annotation: &str,
    ) -> Result<workdesk_core::PdfOperationResponse> {
        let url = self.endpoint("/api/v1/office/pdf/annotate")?;
        let response = self
            .http
            .post(url)
            .json(&workdesk_core::PdfAnnotateInput {
                path: path.to_string(),
                annotation: annotation.to_string(),
            })
            .send()
            .await
            .context("request pdf annotate")?;
        parse_envelope(response).await
    }

    pub async fn pdf_replace_text(
        &self,
        path: &str,
        search: &str,
        replace: &str,
    ) -> Result<workdesk_core::PdfOperationResponse> {
        let url = self.endpoint("/api/v1/office/pdf/replace")?;
        let response = self
            .http
            .post(url)
            .json(&workdesk_core::PdfReplaceTextInput {
                path: path.to_string(),
                search: search.to_string(),
                replace: replace.to_string(),
            })
            .send()
            .await
            .context("request pdf replace text")?;
        parse_envelope(response).await
    }

    pub async fn pdf_save_version(&self, path: &str) -> Result<workdesk_core::PdfOperationResponse> {
        let url = self.endpoint("/api/v1/office/pdf/save-version")?;
        let response = self
            .http
            .post(url)
            .json(&workdesk_core::PdfSaveVersionInput {
                path: path.to_string(),
            })
            .send()
            .await
            .context("request pdf save version")?;
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
