# WorkDesk Studio Architecture

## Runtime Topology

- `apps/workdesk-desktop`: desktop shell. Local mode starts core + runner loops in the same app process; remote mode connects to a server core.
- `crates/workdesk-core`: HTTP API for auth, workflows, proposals, skills, memory, run queue, filesystem, and office endpoints.
- `crates/workdesk-runner`: workflow runner daemon. Claims queued runs, materializes skill snapshots, writes run events/status.

## Persistence Strategy (Current Milestone)

- Local persistence uses `sqlx + SQLite`.
- Default database path on Windows:
  - `%LOCALAPPDATA%\WorkDeskStudio\data\workdesk.db`
- Override path:
  - `WORKDESK_DB_PATH`
- Core startup flow:
  1. Resolve `AppConfig` from env.
  2. Ensure DB parent directory exists.
  3. Open SQLite pool.
  4. Apply migrations before serving API.

## Data Model Scope

- Auth: `users`, `sessions`
- Workflow: `workflows`, `workflow_nodes`, `workflow_edges`, `workflow_proposals`
- Knowledge: `skills`, `memory_records`
- Run queue: `workflow_runs`, `workflow_run_events`, `workflow_run_skill_snapshots`, `runner_leases`
- Office history: `office_versions`

Scope boundaries:

- `user` scope: per-account private records.
- `shared` scope: cross-user shared records.

## Auth Baseline

- Passwords are stored as Argon2 hashes (`password_hash`), never plaintext.
- Session tokens are persisted in `sessions`.
- `switch_account` flow:
  1. Revoke old account active sessions.
  2. Create new session for target account.
  3. Return new token.

## API Stability Contract

- All endpoints return a single envelope format:
  - Success: `{ "data": ..., "error": null, "meta": { "request_id": "...", "timestamp": "..." } }`
  - Failure: `{ "data": null, "error": { "code": "...", "message": "...", "details": ... }, "meta": {...} }`
- Route paths remain unchanged from previous scaffold.
- Desktop API client uses one shared envelope decoder and one error-handling path.

## Run + Skills Snapshot Flow

1. `POST /workflows/{id}/run` enqueues a run in `workflow_runs`.
2. Core creates a run-time skill snapshot from `skills`:
   - merges `shared + user`
   - same skill name uses `user` scope as winner
3. Runner claims queued runs and materializes snapshot content into run runtime folder.
4. Runner records `skills_loaded` and completion events in `workflow_run_events`.
