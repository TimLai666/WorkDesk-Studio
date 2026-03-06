# Quickstart

## Prerequisites

- Rust toolchain
- Windows recommended for phase-1 local desktop mode
- Optional: WiX Toolset (`candle.exe`, `light.exe`) for MSI builds

## Run the Main Validation Set

```powershell
cargo test --workspace
python scripts/check_docs_sync.py
openspec validate --changes --strict
openspec validate --specs --strict
```

## Start Desktop in Local Mode

```powershell
$env:WORKDESK_CORE_BIND="127.0.0.1:4000"
$env:WORKDESK_WORKSPACE_ROOT="C:\path\to\workspace"
cargo run -p workdesk-desktop
```

Local mode does the following:

- acquires the single-instance lock
- starts core API and runner loops
- starts sidecar and OnlyOffice supervisors
- opens the GPUI shell with runs, canvas, files, and office/PDF views

Useful local overrides:

```powershell
$env:WORKDESK_DB_PATH="C:\custom\workdesk.db"
$env:WORKDESK_SIDECAR_PATH="$env:LOCALAPPDATA\\WorkDeskStudio\\sidecar\\node\\node.exe"
$env:WORKDESK_SIDECAR_SCRIPT="$env:LOCALAPPDATA\\WorkDeskStudio\\sidecar\\sidecar.js"
$env:WORKDESK_ONLYOFFICE_BIN="$env:LOCALAPPDATA\\WorkDeskStudio\\onlyoffice\\documentserver\\documentserver.exe"
$env:WORKDESK_APP_UPDATE_CHANNEL="stable"
$env:WORKDESK_TOOLCHAIN_UPDATE_CHANNEL="stable"
```

## Start Desktop in Remote Mode

```powershell
$env:WORKDESK_REMOTE_URL="http://127.0.0.1:4000"
cargo run -p workdesk-desktop -- --remote
```

## Forward CLI Commands to the Primary Window

```powershell
cargo run -p workdesk-desktop -- open
cargo run -p workdesk-desktop -- open-run --run-id run-123
cargo run -p workdesk-desktop -- open-workflow --workflow-id wf-123
cargo run -p workdesk-desktop -- run-workflow --workflow-id wf-123
```

If a primary instance already exists, the secondary process forwards the command through the named-pipe command bus and exits.

## Automation Mode

```powershell
$env:WORKDESK_ENABLE_AUTOMATION="1"
cargo run -p workdesk-desktop -- --automation
```

Automation mode enables a local test channel for:

- reading `UiStateSnapshot`
- dispatching desktop commands
- triggering run cancel/retry actions

## Build Windows Payload and MSI

Build the release payload only:

```powershell
powershell -File .\scripts\windows\build-installer.ps1 -ProductVersion 0.1.0
```

Build the release payload and attempt MSI authoring:

```powershell
powershell -File .\scripts\windows\build-installer.ps1 -ProductVersion 0.1.0 -BuildMsi
```

Generate or inspect the harvested WiX payload fragment directly:

```powershell
powershell -File .\scripts\windows\wix\Harvest-Payload.ps1 -PayloadDir .\dist\windows\payload -OutputPath .\dist\windows\Payload.wxs
```

## Toolchain and App Update Baseline

- Toolchain feeds
  - `ToolchainManager` accepts local-path or HTTP JSON feeds and verifies SHA-256 before replacing managed binaries.
- App update feeds
  - `AppUpdateFeed` groups manifests by channel.
  - `AppUpdateManifest` verifies an Ed25519 signature plus package SHA-256 before an installer can be trusted.

## Filesystem and Office API Surface

- `GET /api/v1/fs/search?path=<path>&query=<text>&limit=<n>`
- `POST /api/v1/fs/diff`
- `POST /api/v1/fs/terminal/start`
- `GET /api/v1/fs/terminal/session/{session_id}`
- `POST /api/v1/office/open`
- `POST /api/v1/office/save`
- `GET /api/v1/office/version?path=<path>`
- `POST /api/v1/office/onlyoffice/callback`
- `POST /api/v1/office/pdf/preview`
- `POST /api/v1/office/pdf/annotate`
- `POST /api/v1/office/pdf/replace`
- `POST /api/v1/office/pdf/save-version`

## Diagnostics You Should Expect

- `RUNNER_UNAVAILABLE`
- `SIDECAR_UNAVAILABLE`
- `DOCSERVER_UNAVAILABLE`
