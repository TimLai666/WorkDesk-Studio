use super::*;
use anyhow::{anyhow, Result};
use chrono::Utc;
use workdesk_core::{
    AgentWorkspaceMessageRole, AppendAgentWorkspaceMessageInput, AuthLoginInput, AuthLogoutInput,
    AuthSwitchInput, CreateAgentWorkspaceSessionInput,
};

impl DesktopAppController {
    pub async fn login_local_account(&self, account_id: &str, password: &str) -> Result<()> {
        let session = self
            .api
            .login(&AuthLoginInput {
                account_id: account_id.to_string(),
                password: password.to_string(),
            })
            .await?;
        self.apply(ControllerAction::SetAuthSession(Some(session)));
        Ok(())
    }

    pub async fn logout_active_account(&self) -> Result<()> {
        let account_id = self
            .snapshot()
            .auth_account_id
            .ok_or_else(|| anyhow!("no authenticated account"))?;
        let _ = self
            .api
            .logout(&AuthLogoutInput {
                account_id: account_id.clone(),
            })
            .await?;
        self.apply(ControllerAction::SetAuthSession(None));
        Ok(())
    }

    pub async fn switch_local_account(&self, from_account: &str, to_account: &str) -> Result<()> {
        let session = self
            .api
            .switch_account(&AuthSwitchInput {
                from_account: from_account.to_string(),
                to_account: to_account.to_string(),
            })
            .await?;
        self.apply(ControllerAction::SetAuthSession(Some(session)));
        Ok(())
    }

    pub async fn refresh_agent_capabilities(&self) -> Result<()> {
        let capabilities = self.api.list_agent_capabilities().await?;
        self.apply(ControllerAction::SetModelCapabilities(capabilities));
        Ok(())
    }

    pub async fn refresh_agent_sessions(&self) -> Result<()> {
        let sessions = self.api.list_agent_workspace_sessions().await?;
        self.apply(ControllerAction::SetAgentSessions(sessions));
        Ok(())
    }

    pub async fn refresh_active_agent_workspace(&self) -> Result<()> {
        let session_id = self
            .snapshot()
            .active_agent_session_id
            .ok_or_else(|| anyhow!("no agent session selected"))?;
        let messages = self.api.list_agent_workspace_messages(&session_id).await?;
        let prompts = self.api.list_choice_prompts(&session_id).await?;
        self.apply(ControllerAction::SetAgentMessages(messages));
        self.apply(ControllerAction::SetChoicePrompts(prompts));
        Ok(())
    }

    pub fn select_agent_session(&self, session_id: Option<String>) {
        self.apply(ControllerAction::SelectAgentSession(session_id));
    }

    pub async fn create_agent_session(&self, title: &str) -> Result<()> {
        let input = CreateAgentWorkspaceSessionInput {
            title: title.to_string(),
            config: None,
            last_active_panel: Some("workbench".to_string()),
        };
        let created = self.api.create_agent_workspace_session(&input).await?;
        self.refresh_agent_sessions().await?;
        self.apply(ControllerAction::SelectAgentSession(Some(
            created.session_id.clone(),
        )));
        self.refresh_active_agent_workspace().await?;
        Ok(())
    }

    pub async fn activate_agent_session(&self, session_id: &str) -> Result<()> {
        self.apply(ControllerAction::SelectAgentSession(Some(
            session_id.to_string(),
        )));
        self.refresh_active_agent_workspace().await
    }

    pub async fn send_prompt(&self, content: &str) -> Result<()> {
        let session_id = self
            .snapshot()
            .active_agent_session_id
            .ok_or_else(|| anyhow!("no agent session selected"))?;
        let input = AppendAgentWorkspaceMessageInput {
            role: AgentWorkspaceMessageRole::User,
            content: content.to_string(),
        };
        let _ = self
            .api
            .append_agent_workspace_message(&session_id, &input)
            .await?;
        self.refresh_active_agent_workspace().await
    }

    pub async fn answer_choice_prompt_option(
        &self,
        session_id: &str,
        prompt_id: &str,
        option_id: &str,
    ) -> Result<()> {
        let _ = self
            .api
            .answer_choice_prompt(session_id, prompt_id, Some(option_id), None)
            .await?;
        if self.snapshot().active_agent_session_id.as_deref() == Some(session_id) {
            self.refresh_active_agent_workspace().await?;
        }
        Ok(())
    }

    pub async fn answer_choice_prompt_text(
        &self,
        session_id: &str,
        prompt_id: &str,
        text: &str,
    ) -> Result<()> {
        let _ = self
            .api
            .answer_choice_prompt(session_id, prompt_id, None, Some(text))
            .await?;
        if self.snapshot().active_agent_session_id.as_deref() == Some(session_id) {
            self.refresh_active_agent_workspace().await?;
        }
        Ok(())
    }

    pub async fn set_active_model(&self, model: &str) -> Result<()> {
        let snapshot = self.snapshot();
        let session = self
            .active_agent_session(&snapshot)
            .ok_or_else(|| anyhow!("no agent session selected"))?;
        let capability = snapshot
            .model_capabilities
            .iter()
            .find(|capability| capability.model == model)
            .ok_or_else(|| anyhow!("model not supported by capabilities: {model}"))?;
        let mut config = session.config.clone();
        config.model = Some(capability.model.clone());
        if !capability.reasoning_values.iter().any(|value| {
            session.config.model_reasoning_effort.as_deref()
                == Some(value.reasoning_effort.as_str())
        }) {
            config.model_reasoning_effort = capability.default_reasoning_effort.clone();
        }
        if !capability.supports_speed {
            config.speed = Some(false);
        }
        self.persist_active_session_config(
            &session.session_id,
            config,
            session.last_active_panel.clone(),
        )
        .await
    }

    pub async fn set_active_reasoning_effort(&self, reasoning_effort: &str) -> Result<()> {
        let snapshot = self.snapshot();
        let session = self
            .active_agent_session(&snapshot)
            .ok_or_else(|| anyhow!("no agent session selected"))?;
        let capability = self
            .active_model_capability(&snapshot, &session)
            .ok_or_else(|| anyhow!("no model capability available"))?;
        let supported = capability
            .reasoning_values
            .iter()
            .any(|value| value.reasoning_effort == reasoning_effort);
        if !supported {
            return Err(anyhow!(
                "reasoning effort not supported for model {}: {}",
                capability.model,
                reasoning_effort
            ));
        }
        let mut config = session.config.clone();
        config.model_reasoning_effort = Some(reasoning_effort.to_string());
        self.persist_active_session_config(
            &session.session_id,
            config,
            session.last_active_panel.clone(),
        )
        .await
    }

    pub async fn set_active_speed(&self, enabled: bool) -> Result<()> {
        let snapshot = self.snapshot();
        let session = self
            .active_agent_session(&snapshot)
            .ok_or_else(|| anyhow!("no agent session selected"))?;
        let capability = self
            .active_model_capability(&snapshot, &session)
            .ok_or_else(|| anyhow!("no model capability available"))?;
        if !capability.supports_speed {
            return Err(anyhow!("active model does not support speed"));
        }
        let mut config = session.config.clone();
        config.speed = Some(enabled);
        self.persist_active_session_config(
            &session.session_id,
            config,
            session.last_active_panel.clone(),
        )
        .await
    }

    pub async fn set_plan_mode(&self, enabled: bool) -> Result<()> {
        let snapshot = self.snapshot();
        let session = self
            .active_agent_session(&snapshot)
            .ok_or_else(|| anyhow!("no agent session selected"))?;
        if let Some(capability) = self.active_model_capability(&snapshot, &session) {
            if !capability.supports_plan_mode {
                return Err(anyhow!("active model does not support plan mode"));
            }
        }
        let mut config = session.config.clone();
        config.plan_mode = enabled;
        self.persist_active_session_config(
            &session.session_id,
            config,
            session.last_active_panel.clone(),
        )
        .await
    }

    pub async fn create_new_file_from_workbench(&self) -> Result<()> {
        let filename = format!("workbench-{}.md", Utc::now().format("%Y%m%d-%H%M%S"));
        self.create_file(&filename, "").await?;
        self.open_file(&filename).await
    }

    pub fn navigate(&self, route: UiRoute) {
        self.apply(ControllerAction::SetRoute(route));
    }

    pub async fn cycle_active_model(&self) -> Result<()> {
        let snapshot = self.snapshot();
        let session = self
            .active_agent_session(&snapshot)
            .ok_or_else(|| anyhow!("no agent session selected"))?;
        if snapshot.model_capabilities.is_empty() {
            return Ok(());
        }
        let current_index = snapshot
            .model_capabilities
            .iter()
            .position(|capability| {
                session.config.model.as_deref() == Some(capability.model.as_str())
            })
            .unwrap_or(usize::MAX);
        let next_index = if current_index == usize::MAX {
            0
        } else {
            (current_index + 1) % snapshot.model_capabilities.len()
        };
        self.set_active_model(&snapshot.model_capabilities[next_index].model)
            .await
    }

    pub async fn cycle_active_reasoning_effort(&self) -> Result<()> {
        let snapshot = self.snapshot();
        let session = self
            .active_agent_session(&snapshot)
            .ok_or_else(|| anyhow!("no agent session selected"))?;
        let capability = self
            .active_model_capability(&snapshot, &session)
            .ok_or_else(|| anyhow!("no model capability available"))?;
        if capability.reasoning_values.is_empty() {
            return Ok(());
        }
        let current_index = capability
            .reasoning_values
            .iter()
            .position(|value| {
                session.config.model_reasoning_effort.as_deref()
                    == Some(value.reasoning_effort.as_str())
            })
            .unwrap_or(usize::MAX);
        let next_index = if current_index == usize::MAX {
            0
        } else {
            (current_index + 1) % capability.reasoning_values.len()
        };
        self.set_active_reasoning_effort(&capability.reasoning_values[next_index].reasoning_effort)
            .await
    }

    pub async fn toggle_active_speed(&self) -> Result<()> {
        let snapshot = self.snapshot();
        let session = self
            .active_agent_session(&snapshot)
            .ok_or_else(|| anyhow!("no agent session selected"))?;
        self.set_active_speed(!session.config.speed.unwrap_or(false))
            .await
    }

    pub async fn toggle_plan_mode(&self) -> Result<()> {
        let snapshot = self.snapshot();
        let session = self
            .active_agent_session(&snapshot)
            .ok_or_else(|| anyhow!("no agent session selected"))?;
        self.set_plan_mode(!session.config.plan_mode).await
    }

    fn active_agent_session(&self, snapshot: &UiStateSnapshot) -> Option<AgentWorkspaceSession> {
        snapshot
            .active_agent_session_id
            .as_ref()
            .and_then(|session_id| {
                snapshot
                    .agent_sessions
                    .iter()
                    .find(|session| session.session_id == *session_id)
                    .cloned()
            })
    }

    fn active_model_capability(
        &self,
        snapshot: &UiStateSnapshot,
        session: &AgentWorkspaceSession,
    ) -> Option<CodexModelCapability> {
        let model = session.config.model.as_deref()?;
        snapshot
            .model_capabilities
            .iter()
            .find(|capability| capability.model == model)
            .cloned()
    }

    async fn persist_active_session_config(
        &self,
        session_id: &str,
        config: CodexNativeSessionConfig,
        last_active_panel: Option<String>,
    ) -> Result<()> {
        let _ = self
            .api
            .update_agent_workspace_session_config(session_id, config, last_active_panel.as_deref())
            .await?;
        self.refresh_agent_sessions().await?;
        self.apply(ControllerAction::SelectAgentSession(Some(
            session_id.to_string(),
        )));
        self.refresh_active_agent_workspace().await?;
        Ok(())
    }
}
