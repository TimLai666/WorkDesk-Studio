# Quickstart

## Prerequisites

- Rust toolchain
- Windows recommended for phase-1 local mode

## Run Tests

```powershell
cargo test --workspace
```

## Start Core Service

```powershell
$env:WORKDESK_CORE_BIND="127.0.0.1:4000"
$env:WORKDESK_WORKSPACE_ROOT="C:\path\to\workspace"
# optional DB override:
# $env:WORKDESK_DB_PATH="C:\custom\workdesk.db"
cargo run -p workdesk-core
```

Default DB path on Windows:

`%LOCALAPPDATA%\WorkDeskStudio\data\workdesk.db`

## Start Desktop (Local Mode)

```powershell
$env:WORKDESK_CORE_BIND="127.0.0.1:4000"
$env:WORKDESK_WORKSPACE_ROOT="C:\path\to\workspace"
cargo run -p workdesk-desktop
```

## Start Desktop (Remote Mode)

```powershell
$env:WORKDESK_REMOTE_URL="http://127.0.0.1:4000"
cargo run -p workdesk-desktop -- --remote
```

Optional remote login smoke check:

```powershell
$env:WORKDESK_LOGIN_ACCOUNT="demo"
$env:WORKDESK_LOGIN_PASSWORD="demo-pass"
cargo run -p workdesk-desktop -- --remote
```
