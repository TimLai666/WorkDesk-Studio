use crate::types::{
    approval_state_from_db, approval_state_to_db, parse_rfc3339_utc, scope_from_db, scope_to_db,
    workflow_kind_from_db, workflow_kind_to_db, workflow_status_from_db, workflow_status_to_db,
    AuthSessionResponse, MemoryRecord, SkillRecord, WorkflowChangeProposal, WorkflowDefinition,
    WorkflowEdge, WorkflowNode,
};
use anyhow::{anyhow, Context, Result};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use async_trait::async_trait;
use chrono::Utc;
use password_hash::SaltString;
use rand::rngs::OsRng;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};
use std::path::Path;
use std::str::FromStr;
use uuid::Uuid;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

#[async_trait]
pub trait CoreRepository: Send + Sync {
    async fn migrate(&self) -> Result<()>;
    async fn verify_or_create_user(&self, account_id: &str, password: &str) -> Result<()>;
    async fn account_exists(&self, account_id: &str) -> Result<bool>;
    async fn create_session(&self, account_id: &str) -> Result<AuthSessionResponse>;
    async fn revoke_sessions(&self, account_id: &str) -> Result<()>;

    async fn create_workflow(&self, workflow: &WorkflowDefinition) -> Result<()>;
    async fn list_workflows(&self) -> Result<Vec<WorkflowDefinition>>;
    async fn get_workflow(&self, workflow_id: &str) -> Result<Option<WorkflowDefinition>>;
    async fn update_workflow_status(
        &self,
        workflow_id: &str,
        status: crate::types::WorkflowStatus,
    ) -> Result<Option<WorkflowDefinition>>;

    async fn create_proposal(&self, proposal: &WorkflowChangeProposal) -> Result<()>;
    async fn get_proposal(&self, proposal_id: &str) -> Result<Option<WorkflowChangeProposal>>;
    async fn update_proposal(&self, proposal: &WorkflowChangeProposal) -> Result<()>;

    async fn upsert_skill(&self, skill: &SkillRecord) -> Result<()>;
    async fn list_skills(&self) -> Result<Vec<SkillRecord>>;
    async fn upsert_memory(&self, memory: &MemoryRecord) -> Result<()>;
    async fn list_memory(&self) -> Result<Vec<MemoryRecord>>;

    async fn insert_office_version(
        &self,
        path: &str,
        version_name: &str,
        content: &[u8],
    ) -> Result<()>;
    async fn list_office_versions(&self, path: &str) -> Result<Vec<String>>;
}

#[derive(Clone)]
pub struct SqliteCoreRepository {
    pool: SqlitePool,
}

impl SqliteCoreRepository {
    pub async fn connect(db_path: &Path) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .with_context(|| format!("create db parent directory: {}", parent.display()))?;
        }

        let options = SqliteConnectOptions::from_str(&format!("sqlite://{}", db_path.display()))?
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(10)
            .connect_with(options)
            .await
            .context("connect sqlite pool")?;
        Ok(Self { pool })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    async fn load_workflow_by_id(&self, workflow_id: &str) -> Result<Option<WorkflowDefinition>> {
        let row =
            sqlx::query("SELECT id, name, timezone, version, status FROM workflows WHERE id = ?")
                .bind(workflow_id)
                .fetch_optional(&self.pool)
                .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let wf = self.load_workflow_from_row(row).await?;
        Ok(Some(wf))
    }

    async fn load_workflow_from_row(
        &self,
        row: sqlx::sqlite::SqliteRow,
    ) -> Result<WorkflowDefinition> {
        let workflow_id: String = row.try_get("id")?;
        let name: String = row.try_get("name")?;
        let timezone: String = row.try_get("timezone")?;
        let version: i64 = row.try_get("version")?;
        let status: String = row.try_get("status")?;

        let node_rows =
            sqlx::query("SELECT id, kind FROM workflow_nodes WHERE workflow_id = ? ORDER BY id")
                .bind(&workflow_id)
                .fetch_all(&self.pool)
                .await?;
        let nodes = node_rows
            .into_iter()
            .map(|node| {
                Ok(WorkflowNode {
                    id: node.try_get("id")?,
                    kind: workflow_kind_from_db(&node.try_get::<String, _>("kind")?)?,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        let edge_rows = sqlx::query(
            "SELECT source_node, target_node FROM workflow_edges WHERE workflow_id = ? ORDER BY id",
        )
        .bind(&workflow_id)
        .fetch_all(&self.pool)
        .await?;
        let edges = edge_rows
            .into_iter()
            .map(|edge| {
                Ok(WorkflowEdge {
                    from: edge.try_get("source_node")?,
                    to: edge.try_get("target_node")?,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(WorkflowDefinition {
            id: workflow_id,
            name,
            timezone,
            nodes,
            edges,
            version: version as u64,
            status: workflow_status_from_db(&status)?,
        })
    }
}

#[async_trait]
impl CoreRepository for SqliteCoreRepository {
    async fn migrate(&self) -> Result<()> {
        MIGRATOR
            .run(&self.pool)
            .await
            .context("run sqlite migrations")
    }

    async fn verify_or_create_user(&self, account_id: &str, password: &str) -> Result<()> {
        let existing = sqlx::query("SELECT password_hash FROM users WHERE account_id = ?")
            .bind(account_id)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = existing {
            let password_hash: String = row.try_get("password_hash")?;
            let parsed =
                PasswordHash::new(&password_hash).map_err(|e| anyhow!("invalid hash: {e}"))?;
            Argon2::default()
                .verify_password(password.as_bytes(), &parsed)
                .map_err(|_| anyhow!("invalid credentials"))?;
            return Ok(());
        }

        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| anyhow!("hash password failed: {e}"))?
            .to_string();
        sqlx::query("INSERT INTO users (account_id, password_hash, created_at) VALUES (?, ?, ?)")
            .bind(account_id)
            .bind(hash)
            .bind(Utc::now().to_rfc3339())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn account_exists(&self, account_id: &str) -> Result<bool> {
        let row = sqlx::query("SELECT account_id FROM users WHERE account_id = ?")
            .bind(account_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.is_some())
    }

    async fn create_session(&self, account_id: &str) -> Result<AuthSessionResponse> {
        let token = Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO sessions (token, account_id, created_at, revoked_at) VALUES (?, ?, ?, NULL)")
            .bind(&token)
            .bind(account_id)
            .bind(Utc::now().to_rfc3339())
            .execute(&self.pool)
            .await?;
        Ok(AuthSessionResponse {
            session_token: token,
            account_id: account_id.to_string(),
        })
    }

    async fn revoke_sessions(&self, account_id: &str) -> Result<()> {
        sqlx::query(
            "UPDATE sessions SET revoked_at = ? WHERE account_id = ? AND revoked_at IS NULL",
        )
        .bind(Utc::now().to_rfc3339())
        .bind(account_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn create_workflow(&self, workflow: &WorkflowDefinition) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO workflows (id, name, timezone, version, status, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&workflow.id)
        .bind(&workflow.name)
        .bind(&workflow.timezone)
        .bind(workflow.version as i64)
        .bind(workflow_status_to_db(&workflow.status))
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await?;

        for node in &workflow.nodes {
            sqlx::query("INSERT INTO workflow_nodes (workflow_id, id, kind) VALUES (?, ?, ?)")
                .bind(&workflow.id)
                .bind(&node.id)
                .bind(workflow_kind_to_db(&node.kind))
                .execute(&mut *tx)
                .await?;
        }

        for edge in &workflow.edges {
            sqlx::query(
                "INSERT INTO workflow_edges (workflow_id, source_node, target_node) VALUES (?, ?, ?)",
            )
            .bind(&workflow.id)
            .bind(&edge.from)
            .bind(&edge.to)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn list_workflows(&self) -> Result<Vec<WorkflowDefinition>> {
        let rows = sqlx::query(
            "SELECT id, name, timezone, version, status FROM workflows ORDER BY updated_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            out.push(self.load_workflow_from_row(row).await?);
        }
        Ok(out)
    }

    async fn get_workflow(&self, workflow_id: &str) -> Result<Option<WorkflowDefinition>> {
        self.load_workflow_by_id(workflow_id).await
    }

    async fn update_workflow_status(
        &self,
        workflow_id: &str,
        status: crate::types::WorkflowStatus,
    ) -> Result<Option<WorkflowDefinition>> {
        let result = sqlx::query(
            "UPDATE workflows
             SET status = ?, version = version + 1, updated_at = ?
             WHERE id = ?",
        )
        .bind(workflow_status_to_db(&status))
        .bind(Utc::now().to_rfc3339())
        .bind(workflow_id)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Ok(None);
        }
        self.load_workflow_by_id(workflow_id).await
    }

    async fn create_proposal(&self, proposal: &WorkflowChangeProposal) -> Result<()> {
        sqlx::query(
            "INSERT INTO workflow_proposals
            (proposal_id, workflow_id, diff, created_by_agent, approval_state, approved_by, approved_at, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&proposal.proposal_id)
        .bind(&proposal.workflow_id)
        .bind(&proposal.diff)
        .bind(&proposal.created_by_agent)
        .bind(approval_state_to_db(&proposal.approval_state))
        .bind(&proposal.approved_by)
        .bind(proposal.approved_at.map(|x| x.to_rfc3339()))
        .bind(Utc::now().to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_proposal(&self, proposal_id: &str) -> Result<Option<WorkflowChangeProposal>> {
        let row = sqlx::query(
            "SELECT proposal_id, workflow_id, diff, created_by_agent, approval_state, approved_by, approved_at
             FROM workflow_proposals WHERE proposal_id = ?",
        )
        .bind(proposal_id)
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let approved_at: Option<String> = row.try_get("approved_at")?;
        Ok(Some(WorkflowChangeProposal {
            proposal_id: row.try_get("proposal_id")?,
            workflow_id: row.try_get("workflow_id")?,
            diff: row.try_get("diff")?,
            created_by_agent: row.try_get("created_by_agent")?,
            approval_state: approval_state_from_db(&row.try_get::<String, _>("approval_state")?)?,
            approved_by: row.try_get("approved_by")?,
            approved_at: approved_at.as_deref().map(parse_rfc3339_utc).transpose()?,
        }))
    }

    async fn update_proposal(&self, proposal: &WorkflowChangeProposal) -> Result<()> {
        sqlx::query(
            "UPDATE workflow_proposals
             SET approval_state = ?, approved_by = ?, approved_at = ?
             WHERE proposal_id = ?",
        )
        .bind(approval_state_to_db(&proposal.approval_state))
        .bind(&proposal.approved_by)
        .bind(proposal.approved_at.map(|x| x.to_rfc3339()))
        .bind(&proposal.proposal_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn upsert_skill(&self, skill: &SkillRecord) -> Result<()> {
        sqlx::query(
            "INSERT INTO skills (scope, name, manifest, content_path, version, updated_at)
             VALUES (?, ?, ?, ?, ?, ?)
             ON CONFLICT(scope, name) DO UPDATE SET
               manifest = excluded.manifest,
               content_path = excluded.content_path,
               version = excluded.version,
               updated_at = excluded.updated_at",
        )
        .bind(scope_to_db(&skill.scope))
        .bind(&skill.name)
        .bind(&skill.manifest)
        .bind(&skill.content_path)
        .bind(&skill.version)
        .bind(Utc::now().to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn list_skills(&self) -> Result<Vec<SkillRecord>> {
        let rows = sqlx::query(
            "SELECT scope, name, manifest, content_path, version FROM skills ORDER BY scope, name",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|row| {
                Ok(SkillRecord {
                    scope: scope_from_db(&row.try_get::<String, _>("scope")?)?,
                    name: row.try_get("name")?,
                    manifest: row.try_get("manifest")?,
                    content_path: row.try_get("content_path")?,
                    version: row.try_get("version")?,
                })
            })
            .collect()
    }

    async fn upsert_memory(&self, memory: &MemoryRecord) -> Result<()> {
        sqlx::query(
            "INSERT INTO memory_records (scope, namespace, key, value, embedding_ref, updated_at)
             VALUES (?, ?, ?, ?, ?, ?)
             ON CONFLICT(scope, namespace, key) DO UPDATE SET
               value = excluded.value,
               embedding_ref = excluded.embedding_ref,
               updated_at = excluded.updated_at",
        )
        .bind(scope_to_db(&memory.scope))
        .bind(&memory.namespace)
        .bind(&memory.key)
        .bind(&memory.value)
        .bind(&memory.embedding_ref)
        .bind(memory.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn list_memory(&self) -> Result<Vec<MemoryRecord>> {
        let rows = sqlx::query(
            "SELECT scope, namespace, key, value, embedding_ref, updated_at
             FROM memory_records ORDER BY scope, namespace, key",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|row| {
                Ok(MemoryRecord {
                    scope: scope_from_db(&row.try_get::<String, _>("scope")?)?,
                    namespace: row.try_get("namespace")?,
                    key: row.try_get("key")?,
                    value: row.try_get("value")?,
                    embedding_ref: row.try_get("embedding_ref")?,
                    updated_at: parse_rfc3339_utc(&row.try_get::<String, _>("updated_at")?)?,
                })
            })
            .collect()
    }

    async fn insert_office_version(
        &self,
        path: &str,
        version_name: &str,
        content: &[u8],
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO office_versions (path, version_name, content, created_at) VALUES (?, ?, ?, ?)",
        )
        .bind(path)
        .bind(version_name)
        .bind(content)
        .bind(Utc::now().to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn list_office_versions(&self, path: &str) -> Result<Vec<String>> {
        let rows = sqlx::query(
            "SELECT version_name FROM office_versions WHERE path = ? ORDER BY created_at DESC",
        )
        .bind(path)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|row| row.try_get("version_name").map_err(Into::into))
            .collect()
    }
}
