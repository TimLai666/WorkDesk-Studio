# WorkDesk Studio Architecture

## Runtime Topology

- `apps/workdesk-desktop`
  - Single binary for GUI + CLI entrypoints.
  - Local mode starts core + runner loops in the same desktop process.
  - Remote mode connects to an external core service.
- `crates/workdesk-core`
  - HTTP API for auth, workflows, proposals, skills, memory, run queue, filesystem, and office endpoints.
- `crates/workdesk-runner`
  - Workflow runner daemon that claims queued runs, materializes skill snapshots, and writes run events/status.

## Desktop Product Layer (Current Milestone)

- `DesktopAppController`
  - Central command/state/view coordinator.
  - Handles CLI/IPC commands, API calls, and run detail synchronization.
- Single-instance (Windows first)
  - Mutex: `Global\WorkDeskStudio.Singleton`
  - Secondary instance forwards command and exits.
- Local command bus
  - Named pipe endpoint: `\\.\pipe\WorkDeskStudio.CommandBus`
  - Request envelope: `{ "type": "...", "payload": { ... }, "request_id": "..." }`
  - Response envelope: `{ "ok": true|false, "error": { ... } }`
- GPUI + `gpui-component`
  - Main view has Run List + Run Detail.
  - Run Detail includes events and run skill snapshot.
  - UI actions include refresh, cancel run, and retry run.
- Automation mode (`--automation`)
  - Test channel endpoint: `\\.\pipe\WorkDeskStudio.Automation`
  - Exposes read-only UI state snapshot plus test actions.

## Persistence Strategy

- Local persistence uses `sqlx + SQLite`.
- Default Windows DB path:
  - `%LOCALAPPDATA%\WorkDeskStudio\data\workdesk.db`
- Override:
  - `WORKDESK_DB_PATH`
- Startup flow:
  1. Resolve `AppConfig`.
  2. Ensure DB parent directory exists.
  3. Open SQLite pool.
  4. Apply migrations.
  5. Start API service.

## Data Model Scope

- Auth: `users`, `sessions`
- Workflow: `workflows`, `workflow_nodes`, `workflow_edges`, `workflow_proposals`
- Knowledge: `skills`, `memory_records`
- Run queue: `workflow_runs`, `workflow_run_events`, `workflow_run_skill_snapshots`, `runner_leases`
- Office history: `office_versions`

Scope boundaries:

- `user`: per-account private records
- `shared`: cross-user shared records

## API Envelope Contract

- Success:
  - `{ "data": ..., "error": null, "meta": { "request_id": "...", "timestamp": "..." } }`
- Failure:
  - `{ "data": null, "error": { "code": "...", "message": "...", "details": ... }, "meta": {...} }`

## Run + Skills Snapshot Flow

1. `POST /workflows/{id}/run` enqueues a run in `workflow_runs`.
2. Core builds run-time skill snapshot from `skills`.
   - Merge order: `shared + user`
   - Same name conflict: `user` scope wins
3. Runner claims queued runs and materializes snapshot paths into run runtime folder.
4. Runner appends `workflow_run_events` and updates run status.
