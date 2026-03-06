CREATE TABLE IF NOT EXISTS workflow_runs (
  run_id TEXT PRIMARY KEY,
  workflow_id TEXT NOT NULL,
  requested_by TEXT,
  status TEXT NOT NULL,
  started_at TEXT,
  finished_at TEXT,
  cancel_requested INTEGER NOT NULL DEFAULT 0,
  error_message TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (workflow_id) REFERENCES workflows(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_workflow_runs_workflow_status
  ON workflow_runs(workflow_id, status, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_workflow_runs_status
  ON workflow_runs(status, created_at DESC);

CREATE TABLE IF NOT EXISTS workflow_run_events (
  run_id TEXT NOT NULL,
  seq INTEGER NOT NULL,
  event_type TEXT NOT NULL,
  payload TEXT NOT NULL,
  created_at TEXT NOT NULL,
  PRIMARY KEY (run_id, seq),
  FOREIGN KEY (run_id) REFERENCES workflow_runs(run_id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_workflow_run_events_run_seq
  ON workflow_run_events(run_id, seq);

CREATE TABLE IF NOT EXISTS workflow_run_skill_snapshots (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  run_id TEXT NOT NULL,
  scope TEXT NOT NULL,
  name TEXT NOT NULL,
  manifest TEXT NOT NULL,
  content_path TEXT NOT NULL,
  version TEXT NOT NULL,
  materialized_path TEXT,
  created_at TEXT NOT NULL,
  UNIQUE (run_id, name),
  FOREIGN KEY (run_id) REFERENCES workflow_runs(run_id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_run_skill_snapshots_run
  ON workflow_run_skill_snapshots(run_id);

CREATE TABLE IF NOT EXISTS runner_leases (
  run_id TEXT PRIMARY KEY,
  runner_id TEXT NOT NULL,
  lease_until TEXT NOT NULL,
  heartbeat_at TEXT NOT NULL,
  FOREIGN KEY (run_id) REFERENCES workflow_runs(run_id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_runner_leases_lease_until
  ON runner_leases(lease_until);
