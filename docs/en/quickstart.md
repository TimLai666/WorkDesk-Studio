# Quickstart

## Prerequisites

- Rust toolchain
- Windows recommended for phase-1 local desktop mode

## Run Full Test Suite

```powershell
cargo test --workspace
```

## Validate OpenSpec Baseline + Changes

```powershell
openspec validate --changes --strict
openspec validate --specs --strict
```

## Start Desktop in Local Mode

```powershell
$env:WORKDESK_CORE_BIND="127.0.0.1:4000"
$env:WORKDESK_WORKSPACE_ROOT="C:\path\to\workspace"
cargo run -p workdesk-desktop
```

What local mode does:

- Acquires single-instance lock.
- Starts core API + runner daemon loops.
- Opens GPUI window (Run List + Run Detail).

Default SQLite DB path (Windows):

- `%LOCALAPPDATA%\WorkDeskStudio\data\workdesk.db`

Override DB path:

```powershell
$env:WORKDESK_DB_PATH="C:\custom\workdesk.db"
```

## Start Desktop in Remote Mode

```powershell
$env:WORKDESK_REMOTE_URL="http://127.0.0.1:4000"
cargo run -p workdesk-desktop -- --remote
```

## CLI to Primary Window Commands

The same binary supports command forwarding to the primary instance.

```powershell
cargo run -p workdesk-desktop -- open
cargo run -p workdesk-desktop -- open-run --run-id run-123
cargo run -p workdesk-desktop -- open-workflow --workflow-id wf-123
cargo run -p workdesk-desktop -- run-workflow --workflow-id wf-123
```

If a primary instance already exists, the secondary process forwards the command through command bus and exits.

## Automation Mode (Headless-Friendly Regression)

```powershell
$env:WORKDESK_ENABLE_AUTOMATION="1"
cargo run -p workdesk-desktop -- --automation
```

Automation mode enables test IPC channel for:

- reading `UiStateSnapshot`
- dispatching desktop commands
- triggering cancel/retry actions

## New FS Utility Endpoints (Local/Remote)

- `GET /api/v1/fs/search?path=<path>&query=<text>&limit=<n>`
- `POST /api/v1/fs/diff`
- `POST /api/v1/fs/terminal/start`
- `GET /api/v1/fs/terminal/session/{session_id}`
