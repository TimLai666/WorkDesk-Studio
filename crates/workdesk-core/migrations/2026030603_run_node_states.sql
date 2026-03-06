CREATE TABLE IF NOT EXISTS workflow_run_nodes (
  run_id TEXT NOT NULL,
  node_id TEXT NOT NULL,
  kind TEXT NOT NULL,
  status TEXT NOT NULL,
  attempt INTEGER NOT NULL DEFAULT 0,
  error_message TEXT,
  started_at TEXT,
  finished_at TEXT,
  updated_at TEXT NOT NULL,
  PRIMARY KEY (run_id, node_id),
  FOREIGN KEY (run_id) REFERENCES workflow_runs(run_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_workflow_run_nodes_run
  ON workflow_run_nodes(run_id, status, node_id);
