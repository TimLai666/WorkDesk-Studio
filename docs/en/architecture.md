# WorkDesk Studio Architecture

## Runtime Topology

- `apps/workdesk-desktop`
  - Single Windows-first binary for GUI and CLI entrypoints.
  - Local mode starts core API, runner loop, sidecar supervisor, and OnlyOffice launcher from the desktop process.
  - Remote mode connects to an external core service while keeping the same UI shell.
- `crates/workdesk-core`
  - HTTP API for auth, workflows, proposals, runs, skills, memory, filesystem, office, and update metadata.
- `crates/workdesk-runner`
  - Workflow runner daemon that claims queued runs, materializes run skill snapshots, executes DAG nodes, and records run/node state.

## Desktop Product Layer

- `DesktopAppController`
  - Central command/state/view coordinator.
  - Handles single-instance IPC, API calls, diagnostics, navigation, and UI state snapshots for automation.
- Single-instance shell
  - Mutex: `Global\WorkDeskStudio.Singleton`
  - Command bus: `\\.\pipe\WorkDeskStudio.CommandBus`
  - Automation bus: `\\.\pipe\WorkDeskStudio.Automation`
- GPUI + `gpui-component`
  - Main routes: Run monitor, Workflow canvas, File manager, and Office/PDF desk.
  - Run detail shows events, node lifecycle state, diagnostics, and run skill snapshot.
  - File desk shows workspace tree, text editor, search results, diff view, and terminal output.
  - Office/PDF desk shows document open/save, version history, PDF annotate/replace flow, and OnlyOffice callback state.

## Local Runtime Supervisors

- Sidecar supervisor
  - Watches bundled `node.exe + sidecar.js`.
  - Checks configured sidecar endpoint over HTTP, TCP, or named pipe.
  - Emits `SIDECAR_UNAVAILABLE` when runtime files are missing or health checks fail.
- OnlyOffice launcher
  - Watches the configured Document Server binary and health endpoint.
  - Starts the embedded runtime when files exist but the service is not healthy.
  - Emits `DOCSERVER_UNAVAILABLE` when runtime files are missing or health checks fail.

## Persistence and Domain State

- Local persistence uses `sqlx + SQLite`.
- Default Windows database path:
  - `%LOCALAPPDATA%\WorkDeskStudio\data\workdesk.db`
- Toolchain manifest path:
  - `%LOCALAPPDATA%\WorkDeskStudio\config\toolchains.json`
- Startup flow:
  1. Resolve `AppConfig`.
  2. Ensure AppData directories exist.
  3. Open SQLite and apply migrations.
  4. Start API service and background supervisors.

Primary persisted areas:

- Auth: `users`, `sessions`
- Workflow: `workflows`, `workflow_nodes`, `workflow_edges`, `workflow_proposals`
- Knowledge: `skills`, `memory_records`
- Runs: `workflow_runs`, `workflow_run_events`, `workflow_run_nodes`, `workflow_run_skill_snapshots`, `runner_leases`
- Office history: `office_versions`

## Workflow and Run Execution

1. `POST /api/v1/workflows/{id}/run` creates a run record.
2. Core builds a run-time skill snapshot from `shared + user` skills, with `user` scope taking precedence on conflicts.
3. Core persists run-node lifecycle rows before execution starts.
4. Runner claims queued runs, materializes skill paths into the workflow runtime root, and executes nodes in DAG order.
5. Node state transitions are persisted as `pending -> running -> succeeded|failed|canceled`.
6. Core and runner append run events and expose status through the desktop UI and automation snapshot.

## Update and Packaging Baseline

- Managed toolchains
  - `ToolchainManager` owns app-scoped binaries for `codex`, `uv`, `bun`, and `go`.
  - `ToolchainReleaseFeed` supports local-path or HTTP feed loading.
  - Release assets are verified with SHA-256 before manifest upsert; failed installs roll back to `.previous` snapshots.
- App updates
  - `AppUpdateFeed` and `AppUpdateManifest` define channel-aware update metadata.
  - Signed manifest verification uses a pinned Ed25519 public key plus package SHA-256 verification.
  - App update channel is intentionally separate from toolchain update channel.
- Windows installer
  - `scripts/windows/build-installer.ps1` builds desktop/core/runner release payloads.
  - `scripts/windows/wix/Harvest-Payload.ps1` converts the payload directory into `Payload.wxs`.
  - `scripts/windows/wix/Product.wxs` defines the MSI product, fixed `UpgradeCode`, `MajorUpgrade`, and transactional upgrade scheduling.

## Diagnostics and Degraded Runtime Signals

- `RUNNER_UNAVAILABLE`
  - A run remains queued for more than 90 seconds without being claimed.
- `SIDECAR_UNAVAILABLE`
  - Sidecar runtime is missing or unhealthy.
- `DOCSERVER_UNAVAILABLE`
  - Embedded document server is missing or unhealthy.

All diagnostics are surfaced through the desktop UI and automation snapshot.

## API Envelope Contract

- Success:
  - `{ "data": ..., "error": null, "meta": { "request_id": "...", "timestamp": "..." } }`
- Failure:
  - `{ "data": null, "error": { "code": "...", "message": "...", "details": ... }, "meta": { ... } }`
