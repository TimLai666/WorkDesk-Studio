ALTER TABLE workflows ADD COLUMN agent_defaults_json TEXT;
ALTER TABLE workflow_nodes ADD COLUMN x REAL;
ALTER TABLE workflow_nodes ADD COLUMN y REAL;
ALTER TABLE workflow_nodes ADD COLUMN config_json TEXT;

CREATE TABLE IF NOT EXISTS agent_workspace_sessions (
  session_id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  model TEXT,
  model_reasoning_effort TEXT,
  speed INTEGER,
  plan_mode INTEGER NOT NULL DEFAULT 0,
  last_active_panel TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_agent_workspace_sessions_updated_at
  ON agent_workspace_sessions(updated_at DESC);

CREATE TABLE IF NOT EXISTS agent_workspace_messages (
  message_id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  role TEXT NOT NULL,
  content TEXT NOT NULL,
  created_at TEXT NOT NULL,
  FOREIGN KEY (session_id) REFERENCES agent_workspace_sessions(session_id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_agent_workspace_messages_session_created_at
  ON agent_workspace_messages(session_id, created_at ASC);

CREATE TABLE IF NOT EXISTS agent_workspace_choice_prompts (
  prompt_id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  question TEXT NOT NULL,
  recommended_option_id TEXT,
  allow_freeform INTEGER NOT NULL DEFAULT 0,
  status TEXT NOT NULL,
  selected_option_id TEXT,
  freeform_answer TEXT,
  created_at TEXT NOT NULL,
  answered_at TEXT,
  FOREIGN KEY (session_id) REFERENCES agent_workspace_sessions(session_id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_agent_workspace_choice_prompts_session_created_at
  ON agent_workspace_choice_prompts(session_id, created_at ASC);

CREATE TABLE IF NOT EXISTS agent_workspace_choice_prompt_options (
  prompt_id TEXT NOT NULL,
  option_id TEXT NOT NULL,
  label TEXT NOT NULL,
  description TEXT NOT NULL,
  PRIMARY KEY (prompt_id, option_id),
  FOREIGN KEY (prompt_id) REFERENCES agent_workspace_choice_prompts(prompt_id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS agent_workspace_preferences (
  preference_key TEXT PRIMARY KEY,
  preference_value TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
