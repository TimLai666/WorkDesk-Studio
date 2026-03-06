use super::*;
use crate::types::{AgentWorkspaceMessageRole, ChoicePromptOption, ChoicePromptStatus, CodexNativeSessionConfig};

impl SqliteCoreRepository {
    fn map_agent_workspace_session_row(
        row: sqlx::sqlite::SqliteRow,
    ) -> Result<AgentWorkspaceSession> {
        Ok(AgentWorkspaceSession {
            session_id: row.try_get("session_id")?,
            title: row.try_get("title")?,
            config: CodexNativeSessionConfig {
                model: row.try_get("model")?,
                model_reasoning_effort: row.try_get("model_reasoning_effort")?,
                speed: row.try_get("speed")?,
                plan_mode: row.try_get::<i64, _>("plan_mode")? != 0,
            },
            last_active_panel: row.try_get("last_active_panel")?,
            created_at: parse_rfc3339_utc(&row.try_get::<String, _>("created_at")?)?,
            updated_at: parse_rfc3339_utc(&row.try_get::<String, _>("updated_at")?)?,
        })
    }

    fn map_agent_workspace_message_role(role: &str) -> Result<AgentWorkspaceMessageRole> {
        match role {
            "user" => Ok(AgentWorkspaceMessageRole::User),
            "assistant" => Ok(AgentWorkspaceMessageRole::Assistant),
            "system" => Ok(AgentWorkspaceMessageRole::System),
            "tool" => Ok(AgentWorkspaceMessageRole::Tool),
            other => Err(anyhow!("unknown agent workspace message role `{other}`")),
        }
    }

    fn agent_workspace_message_role_to_db(role: &AgentWorkspaceMessageRole) -> &'static str {
        match role {
            AgentWorkspaceMessageRole::User => "user",
            AgentWorkspaceMessageRole::Assistant => "assistant",
            AgentWorkspaceMessageRole::System => "system",
            AgentWorkspaceMessageRole::Tool => "tool",
        }
    }

    fn choice_prompt_status_from_db(status: &str) -> Result<ChoicePromptStatus> {
        match status {
            "pending" => Ok(ChoicePromptStatus::Pending),
            "answered" => Ok(ChoicePromptStatus::Answered),
            other => Err(anyhow!("unknown choice prompt status `{other}`")),
        }
    }

    fn choice_prompt_status_to_db(status: &ChoicePromptStatus) -> &'static str {
        match status {
            ChoicePromptStatus::Pending => "pending",
            ChoicePromptStatus::Answered => "answered",
        }
    }

    pub(crate) async fn create_agent_workspace_session_impl(
        &self,
        session: &AgentWorkspaceSession,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO agent_workspace_sessions
             (session_id, title, model, model_reasoning_effort, speed, plan_mode, last_active_panel, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&session.session_id)
        .bind(&session.title)
        .bind(&session.config.model)
        .bind(&session.config.model_reasoning_effort)
        .bind(session.config.speed)
        .bind(if session.config.plan_mode { 1 } else { 0 })
        .bind(&session.last_active_panel)
        .bind(session.created_at.to_rfc3339())
        .bind(session.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn list_agent_workspace_sessions_impl(
        &self,
    ) -> Result<Vec<AgentWorkspaceSession>> {
        let rows = sqlx::query(
            "SELECT session_id, title, model, model_reasoning_effort, speed, plan_mode, last_active_panel, created_at, updated_at
             FROM agent_workspace_sessions
             ORDER BY updated_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(Self::map_agent_workspace_session_row)
            .collect()
    }

    pub(crate) async fn get_agent_workspace_session_impl(
        &self,
        session_id: &str,
    ) -> Result<Option<AgentWorkspaceSession>> {
        let row = sqlx::query(
            "SELECT session_id, title, model, model_reasoning_effort, speed, plan_mode, last_active_panel, created_at, updated_at
             FROM agent_workspace_sessions
             WHERE session_id = ?",
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(Self::map_agent_workspace_session_row).transpose()
    }

    pub(crate) async fn update_agent_workspace_session_impl(
        &self,
        session: &AgentWorkspaceSession,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE agent_workspace_sessions
             SET title = ?, model = ?, model_reasoning_effort = ?, speed = ?, plan_mode = ?, last_active_panel = ?, updated_at = ?
             WHERE session_id = ?",
        )
        .bind(&session.title)
        .bind(&session.config.model)
        .bind(&session.config.model_reasoning_effort)
        .bind(session.config.speed)
        .bind(if session.config.plan_mode { 1 } else { 0 })
        .bind(&session.last_active_panel)
        .bind(session.updated_at.to_rfc3339())
        .bind(&session.session_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn append_agent_workspace_message_impl(
        &self,
        message: &AgentWorkspaceMessage,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO agent_workspace_messages (message_id, session_id, role, content, created_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&message.message_id)
        .bind(&message.session_id)
        .bind(Self::agent_workspace_message_role_to_db(&message.role))
        .bind(&message.content)
        .bind(message.created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn list_agent_workspace_messages_impl(
        &self,
        session_id: &str,
    ) -> Result<Vec<AgentWorkspaceMessage>> {
        let rows = sqlx::query(
            "SELECT message_id, session_id, role, content, created_at
             FROM agent_workspace_messages
             WHERE session_id = ?
             ORDER BY created_at ASC",
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|row| {
                Ok(AgentWorkspaceMessage {
                    message_id: row.try_get("message_id")?,
                    session_id: row.try_get("session_id")?,
                    role: Self::map_agent_workspace_message_role(&row.try_get::<String, _>("role")?)?,
                    content: row.try_get("content")?,
                    created_at: parse_rfc3339_utc(&row.try_get::<String, _>("created_at")?)?,
                })
            })
            .collect()
    }

    pub(crate) async fn create_choice_prompt_impl(&self, prompt: &ChoicePrompt) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO agent_workspace_choice_prompts
             (prompt_id, session_id, question, recommended_option_id, allow_freeform, status, selected_option_id, freeform_answer, created_at, answered_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&prompt.prompt_id)
        .bind(&prompt.session_id)
        .bind(&prompt.question)
        .bind(&prompt.recommended_option_id)
        .bind(if prompt.allow_freeform { 1 } else { 0 })
        .bind(Self::choice_prompt_status_to_db(&prompt.status))
        .bind(&prompt.selected_option_id)
        .bind(&prompt.freeform_answer)
        .bind(prompt.created_at.to_rfc3339())
        .bind(prompt.answered_at.map(|value| value.to_rfc3339()))
        .execute(&mut *tx)
        .await?;
        for option in &prompt.options {
            sqlx::query(
                "INSERT INTO agent_workspace_choice_prompt_options (prompt_id, option_id, label, description)
                 VALUES (?, ?, ?, ?)",
            )
            .bind(&prompt.prompt_id)
            .bind(&option.option_id)
            .bind(&option.label)
            .bind(&option.description)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn list_choice_prompt_options(&self, prompt_id: &str) -> Result<Vec<ChoicePromptOption>> {
        let rows = sqlx::query(
            "SELECT option_id, label, description
             FROM agent_workspace_choice_prompt_options
             WHERE prompt_id = ?
             ORDER BY option_id ASC",
        )
        .bind(prompt_id)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|row| {
                Ok(ChoicePromptOption {
                    option_id: row.try_get("option_id")?,
                    label: row.try_get("label")?,
                    description: row.try_get("description")?,
                })
            })
            .collect()
    }

    fn map_choice_prompt_row<'a>(
        &'a self,
        row: sqlx::sqlite::SqliteRow,
    ) -> impl std::future::Future<Output = Result<ChoicePrompt>> + 'a {
        async move {
            let prompt_id: String = row.try_get("prompt_id")?;
            Ok(ChoicePrompt {
                prompt_id: prompt_id.clone(),
                session_id: row.try_get("session_id")?,
                question: row.try_get("question")?,
                options: self.list_choice_prompt_options(&prompt_id).await?,
                recommended_option_id: row.try_get("recommended_option_id")?,
                allow_freeform: row.try_get::<i64, _>("allow_freeform")? != 0,
                status: Self::choice_prompt_status_from_db(&row.try_get::<String, _>("status")?)?,
                selected_option_id: row.try_get("selected_option_id")?,
                freeform_answer: row.try_get("freeform_answer")?,
                created_at: parse_rfc3339_utc(&row.try_get::<String, _>("created_at")?)?,
                answered_at: row
                    .try_get::<Option<String>, _>("answered_at")?
                    .as_deref()
                    .map(parse_rfc3339_utc)
                    .transpose()?,
            })
        }
    }

    pub(crate) async fn list_choice_prompts_impl(&self, session_id: &str) -> Result<Vec<ChoicePrompt>> {
        let rows = sqlx::query(
            "SELECT prompt_id, session_id, question, recommended_option_id, allow_freeform, status, selected_option_id, freeform_answer, created_at, answered_at
             FROM agent_workspace_choice_prompts
             WHERE session_id = ?
             ORDER BY created_at ASC",
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await?;
        let mut prompts = Vec::with_capacity(rows.len());
        for row in rows {
            prompts.push(self.map_choice_prompt_row(row).await?);
        }
        Ok(prompts)
    }

    pub(crate) async fn get_choice_prompt_impl(
        &self,
        session_id: &str,
        prompt_id: &str,
    ) -> Result<Option<ChoicePrompt>> {
        let row = sqlx::query(
            "SELECT prompt_id, session_id, question, recommended_option_id, allow_freeform, status, selected_option_id, freeform_answer, created_at, answered_at
             FROM agent_workspace_choice_prompts
             WHERE session_id = ? AND prompt_id = ?",
        )
        .bind(session_id)
        .bind(prompt_id)
        .fetch_optional(&self.pool)
        .await?;
        match row {
            Some(row) => Ok(Some(self.map_choice_prompt_row(row).await?)),
            None => Ok(None),
        }
    }

    pub(crate) async fn update_choice_prompt_impl(&self, prompt: &ChoicePrompt) -> Result<()> {
        sqlx::query(
            "UPDATE agent_workspace_choice_prompts
             SET recommended_option_id = ?, allow_freeform = ?, status = ?, selected_option_id = ?, freeform_answer = ?, answered_at = ?
             WHERE prompt_id = ?",
        )
        .bind(&prompt.recommended_option_id)
        .bind(if prompt.allow_freeform { 1 } else { 0 })
        .bind(Self::choice_prompt_status_to_db(&prompt.status))
        .bind(&prompt.selected_option_id)
        .bind(&prompt.freeform_answer)
        .bind(prompt.answered_at.map(|value| value.to_rfc3339()))
        .bind(&prompt.prompt_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
