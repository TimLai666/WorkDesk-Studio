# WorkDesk Studio

WorkDesk Studio is a Rust-first agent workbench for daily automation:

- n8n-style workflow model (`ScheduleTrigger`, `AgentPrompt`, `CodeExec`, `FileOps`, `ApprovalGate`)
- Local desktop mode (Windows-first) and remote/server-ready API shape
- Codex CLI adapter (`AgentProvider`) with login/logout/switch account endpoints
- Isolated runtime path strategy for Python/JS/Go code nodes
- Skills and memory stores with user/shared scope
- Workspace file API and office open/save/version API scaffolding

## Repository Layout

- `apps/workdesk-desktop`: desktop entry binary (local/remote mode bootstrap, i18n loader)
- `crates/workdesk-domain`: public interfaces and core types
- `crates/workdesk-runner`: toolchain manager + code node execution layer + Codex CLI adapter
- `crates/workdesk-core`: HTTP API service, in-memory implementation, workflow/skills/memory/fs/office endpoints
- `deploy/docker-compose.phase2.yml`: phase-2 multi-user deployment baseline (Core + PostgreSQL + OnlyOffice)
- `scripts/windows`: installer/toolchain automation scripts

## Quick Start

### 1) Run tests

```powershell
cargo test
```

### 2) Start local desktop mode (embedded core service)

```powershell
$env:WORKDESK_CORE_BIND="127.0.0.1:4000"
$env:WORKDESK_WORKSPACE_ROOT="C:\path\to\workspace"
cargo run -p workdesk-desktop
```

### 3) Start core service directly

```powershell
$env:WORKDESK_CORE_BIND="127.0.0.1:4000"
$env:WORKDESK_WORKSPACE_ROOT="C:\path\to\workspace"
cargo run -p workdesk-core
```

## Key API Surface (Implemented Scaffold)

- Auth: `POST /api/v1/auth/login`, `POST /api/v1/auth/logout`, `POST /api/v1/auth/switch`
- Workflows: `GET/POST /api/v1/workflows`, `GET /api/v1/workflows/:id`, `PATCH /api/v1/workflows/:id/status`
- Workflow Runtime: `POST /api/v1/workflows/:id/run`
- Proposal Flow: `POST /api/v1/workflows/:id/proposals`, `POST /api/v1/workflows/:id/proposals/:proposal_id/approve`
- Skills: `GET/POST /api/v1/skills`, `GET /api/v1/skills/export`, `POST /api/v1/skills/import`
- Memory: `GET/POST /api/v1/memory`, `GET /api/v1/memory/export`, `POST /api/v1/memory/import`
- Filesystem: `GET /api/v1/fs/tree`, `GET/PUT /api/v1/fs/file`, `POST /api/v1/fs/move`, `DELETE /api/v1/fs/path`
- Office: `POST /api/v1/office/open`, `POST /api/v1/office/save`, `GET /api/v1/office/version`

## Current Scope

This commit provides a production-oriented scaffold with executable APIs, domain contracts, and tests.
Full UI canvas, OnlyOffice embedded editor wiring, and persistent DB backends (SQLite/PostgreSQL) are the next incremental implementation slices.
