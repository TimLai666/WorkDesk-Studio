# 快速開始

## 前置需求

- Rust toolchain
- 第一階段本機 desktop mode 建議使用 Windows

## 執行完整測試

```powershell
cargo test --workspace
```

## 驗證 OpenSpec 基線與 Change

```powershell
openspec validate --changes --strict
openspec validate --specs --strict
```

## 啟動 Desktop（Local Mode）

```powershell
$env:WORKDESK_CORE_BIND="127.0.0.1:4000"
$env:WORKDESK_WORKSPACE_ROOT="C:\path\to\workspace"
cargo run -p workdesk-desktop
```

Local mode 會做：

- 取得 single-instance lock
- 啟動 core API + runner daemon loops
- 開啟 GPUI 視窗（Run List + Run Detail）

Windows 預設 SQLite DB 路徑：

- `%LOCALAPPDATA%\WorkDeskStudio\data\workdesk.db`

覆蓋 DB 路徑：

```powershell
$env:WORKDESK_DB_PATH="C:\custom\workdesk.db"
```

## 啟動 Desktop（Remote Mode）

```powershell
$env:WORKDESK_REMOTE_URL="http://127.0.0.1:4000"
cargo run -p workdesk-desktop -- --remote
```

## CLI 對 Primary 視窗轉送命令

同一個 binary 支援轉送到 primary instance：

```powershell
cargo run -p workdesk-desktop -- open
cargo run -p workdesk-desktop -- open-run --run-id run-123
cargo run -p workdesk-desktop -- open-workflow --workflow-id wf-123
cargo run -p workdesk-desktop -- run-workflow --workflow-id wf-123
```

若 primary 已存在，secondary process 會透過 command bus 轉送後結束。

## Automation Mode（CI Headless 友善）

```powershell
$env:WORKDESK_ENABLE_AUTOMATION="1"
cargo run -p workdesk-desktop -- --automation
```

Automation mode 會開啟測試 IPC，可用於：

- 讀取 `UiStateSnapshot`
- 轉送 desktop command
- 觸發 cancel/retry

## 新增 FS 實用端點（Local/Remote）

- `GET /api/v1/fs/search?path=<path>&query=<text>&limit=<n>`
- `POST /api/v1/fs/diff`
- `POST /api/v1/fs/terminal/start`
- `GET /api/v1/fs/terminal/session/{session_id}`
