# 快速開始

## 先決條件

- Rust toolchain
- 第一階段建議在 Windows 執行 local mode

## 執行測試

```powershell
cargo test --workspace
```

## 啟動 Core Service

```powershell
$env:WORKDESK_CORE_BIND="127.0.0.1:4000"
$env:WORKDESK_WORKSPACE_ROOT="C:\path\to\workspace"
# 可選：覆蓋 DB 路徑
# $env:WORKDESK_DB_PATH="C:\custom\workdesk.db"
cargo run -p workdesk-core
```

Windows 預設 DB 路徑：

`%LOCALAPPDATA%\WorkDeskStudio\data\workdesk.db`

## 啟動 Runner Daemon

```powershell
$env:WORKDESK_DB_PATH="$env:LOCALAPPDATA\WorkDeskStudio\data\workdesk.db"
$env:WORKDESK_TOOLS_ROOT="$env:LOCALAPPDATA\WorkDeskStudio\tools"
cargo run -p workdesk-runner
```

## 啟動 Desktop（Local Mode）

```powershell
$env:WORKDESK_CORE_BIND="127.0.0.1:4000"
$env:WORKDESK_WORKSPACE_ROOT="C:\path\to\workspace"
cargo run -p workdesk-desktop
```

Local mode 會同時啟動 core 與 runner 迴圈，自動處理 queued runs。

## 啟動 Desktop（Remote Mode）

```powershell
$env:WORKDESK_REMOTE_URL="http://127.0.0.1:4000"
cargo run -p workdesk-desktop -- --remote
```

可選的 remote 登入 smoke check：

```powershell
$env:WORKDESK_LOGIN_ACCOUNT="demo"
$env:WORKDESK_LOGIN_PASSWORD="demo-pass"
cargo run -p workdesk-desktop -- --remote
```
