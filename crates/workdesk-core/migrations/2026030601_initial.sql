PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS users (
  account_id TEXT PRIMARY KEY,
  password_hash TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS sessions (
  token TEXT PRIMARY KEY,
  account_id TEXT NOT NULL,
  created_at TEXT NOT NULL,
  revoked_at TEXT,
  FOREIGN KEY (account_id) REFERENCES users(account_id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_sessions_account_id ON sessions(account_id);
CREATE INDEX IF NOT EXISTS idx_sessions_active ON sessions(account_id, revoked_at);

CREATE TABLE IF NOT EXISTS workflows (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  timezone TEXT NOT NULL,
  version INTEGER NOT NULL,
  status TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS workflow_nodes (
  workflow_id TEXT NOT NULL,
  id TEXT NOT NULL,
  kind TEXT NOT NULL,
  PRIMARY KEY (workflow_id, id),
  FOREIGN KEY (workflow_id) REFERENCES workflows(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS workflow_edges (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  workflow_id TEXT NOT NULL,
  source_node TEXT NOT NULL,
  target_node TEXT NOT NULL,
  UNIQUE (workflow_id, source_node, target_node),
  FOREIGN KEY (workflow_id) REFERENCES workflows(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_workflow_edges_workflow_id ON workflow_edges(workflow_id);

CREATE TABLE IF NOT EXISTS workflow_proposals (
  proposal_id TEXT PRIMARY KEY,
  workflow_id TEXT NOT NULL,
  diff TEXT NOT NULL,
  created_by_agent TEXT NOT NULL,
  approval_state TEXT NOT NULL,
  approved_by TEXT,
  approved_at TEXT,
  created_at TEXT NOT NULL,
  FOREIGN KEY (workflow_id) REFERENCES workflows(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_workflow_proposals_workflow_id ON workflow_proposals(workflow_id);

CREATE TABLE IF NOT EXISTS skills (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  scope TEXT NOT NULL,
  name TEXT NOT NULL,
  manifest TEXT NOT NULL,
  content_path TEXT NOT NULL,
  version TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  UNIQUE (scope, name)
);

CREATE TABLE IF NOT EXISTS memory_records (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  scope TEXT NOT NULL,
  namespace TEXT NOT NULL,
  key TEXT NOT NULL,
  value TEXT NOT NULL,
  embedding_ref TEXT,
  updated_at TEXT NOT NULL,
  UNIQUE (scope, namespace, key)
);

CREATE TABLE IF NOT EXISTS office_versions (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  path TEXT NOT NULL,
  version_name TEXT NOT NULL,
  content BLOB NOT NULL,
  created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_office_versions_path ON office_versions(path, created_at);
