# 快速開始

## 前置需求

- Rust toolchain
- 第一階段 desktop local mode 建議在 Windows 執行

## 執行完整測試

```powershell
cargo test --workspace
```

## 啟動 Desktop（Local Mode）

```powershell
$env:WORKDESK_CORE_BIND="127.0.0.1:4000"
$env:WORKDESK_WORKSPACE_ROOT="C:\path\to\workspace"
cargo run -p workdesk-desktop
```

Local mode 會：

- 取得單例鎖（single instance）。
- 啟動 core API 與 runner daemon loops。
- 開啟 GPUI 視窗（Run List + Run Detail）。

Windows 預設 SQLite 路徑：

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

## CLI 對主視窗命令轉送

同一個 binary 支援把命令轉送到主實例。

```powershell
cargo run -p workdesk-desktop -- open
cargo run -p workdesk-desktop -- open-run --run-id run-123
cargo run -p workdesk-desktop -- open-workflow --workflow-id wf-123
cargo run -p workdesk-desktop -- run-workflow --workflow-id wf-123
```

若主實例已存在，第二實例會透過 command bus 轉送命令後立即退出。

## Automation Mode（可 Headless 回歸）

```powershell
$env:WORKDESK_ENABLE_AUTOMATION="1"
cargo run -p workdesk-desktop -- --automation
```

Automation mode 會開啟測試 IPC channel，可用於：

- 讀取 `UiStateSnapshot`
- 派發 desktop command
- 觸發 cancel/retry 動作
