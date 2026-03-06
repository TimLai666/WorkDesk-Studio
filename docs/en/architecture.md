# WorkDesk Studio Architecture

## Runtime Topology

- `apps/workdesk-desktop`
  - Single Windows-first binary for GUI and CLI entrypoints.
  - Local mode starts core API, runner loop, sidecar supervisor, and OnlyOffice launcher from the desktop process.
  - Remote mode keeps the same shell while connecting to an external core service.
- `crates/workdesk-core`
  - HTTP API for auth, workflows, proposals, runs, skills, memory, filesystem, office, updater metadata, and the native workbench session surface.
- `crates/workdesk-runner`
  - Workflow runner daemon that claims queued runs, materializes run skill snapshots, executes DAG nodes, and records run/node state.

## Desktop Product Layer

- `DesktopAppController`
  - Central command/state/view coordinator.
  - Owns run monitoring, canvas state, files, office state, native workbench session state, choice prompts, and diagnostics.
- Single-instance shell
  - Mutex: `Global\WorkDeskStudio.Singleton`
  - Command bus: `\\.\pipe\WorkDeskStudio.CommandBus`
  - Automation bus: `\\.\pipe\WorkDeskStudio.Automation`
- GPUI + `gpui-component`
  - Main shell is a Codex-style workbench.
  - Left side shows sessions and capability context.
  - Center shows composer-style controls and session messages.
  - Right side keeps run, file, and office context panels available from the same shell.

## Local Runtime Supervisors

- Sidecar supervisor
  - Watches bundled `node.exe + sidecar.js`.
  - Emits `SIDECAR_UNAVAILABLE` when runtime files are missing or health checks fail.
- OnlyOffice launcher
  - Watches the configured Document Server binary and health endpoint.
  - Emits `DOCSERVER_UNAVAILABLE` when runtime files are missing or health checks fail.

## Persistence and Domain State

- Local persistence uses `sqlx + SQLite`.
- Default Windows database path:
  - `%LOCALAPPDATA%\WorkDeskStudio\data\workdesk.db`

Primary persisted areas:

- Auth: `users`, `sessions`
- Workflow: `workflows`, `workflow_nodes`, `workflow_edges`, `workflow_proposals`
- Workbench: `agent_workspace_sessions`, `agent_workspace_messages`, `agent_workspace_choice_prompts`, `agent_workspace_choice_prompt_options`, `agent_workspace_preferences`
- Knowledge: `skills`, `memory_records`
- Runs: `workflow_runs`, `workflow_run_events`, `workflow_run_nodes`, `workflow_run_skill_snapshots`, `runner_leases`
- Office history: `office_versions`

Workflow persistence also stores:

- canvas coordinates per node: `x`, `y`
- node config JSON: `config_json`
- workflow agent defaults JSON: `agent_defaults_json`

## Native Codex Mapping

- Session config uses native field names:
  - `model`
  - `model_reasoning_effort`
  - `speed`
  - `plan_mode`
- Workflow defaults only persist:
  - `model`
  - `model_reasoning_effort`
- `speed` remains session-scoped and capability-gated.
- Choice prompts map to the Codex request-user-input interaction model rather than diagnostics.

## Diagnostics

- `RUNNER_UNAVAILABLE`
  - A run remains queued for more than 90 seconds without being claimed.
- `SIDECAR_UNAVAILABLE`
  - Sidecar runtime is missing or unhealthy.
- `DOCSERVER_UNAVAILABLE`
  - Embedded document server is missing or unhealthy.

All diagnostics are surfaced through the desktop UI and automation snapshot.
