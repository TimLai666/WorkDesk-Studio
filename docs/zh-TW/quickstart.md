# 快速開始

## 前置需求

- Rust toolchain
- 第一階段本機桌面模式建議使用 Windows
- 若要產出 MSI，需額外安裝 WiX Toolset（`candle.exe`、`light.exe`）

## 執行主要驗證流程

```powershell
cargo test --workspace
python scripts/check_docs_sync.py
openspec validate --changes --strict
openspec validate --specs --strict
```

## 以 Local Mode 啟動 Desktop

```powershell
$env:WORKDESK_CORE_BIND="127.0.0.1:4000"
$env:WORKDESK_WORKSPACE_ROOT="C:\path\to\workspace"
cargo run -p workdesk-desktop
```

Local mode 會做這些事：

- 取得 single-instance lock
- 啟動 core API 與 runner loops
- 啟動 sidecar 與 OnlyOffice supervisors
- 開啟包含 runs、canvas、files、office/PDF 視圖的 GPUI shell

常用本機覆寫：

```powershell
$env:WORKDESK_DB_PATH="C:\custom\workdesk.db"
$env:WORKDESK_SIDECAR_PATH="$env:LOCALAPPDATA\\WorkDeskStudio\\sidecar\\node\\node.exe"
$env:WORKDESK_SIDECAR_SCRIPT="$env:LOCALAPPDATA\\WorkDeskStudio\\sidecar\\sidecar.js"
$env:WORKDESK_ONLYOFFICE_BIN="$env:LOCALAPPDATA\\WorkDeskStudio\\onlyoffice\\documentserver\\documentserver.exe"
$env:WORKDESK_APP_UPDATE_CHANNEL="stable"
$env:WORKDESK_TOOLCHAIN_UPDATE_CHANNEL="stable"
```

## 以 Remote Mode 啟動 Desktop

```powershell
$env:WORKDESK_REMOTE_URL="http://127.0.0.1:4000"
cargo run -p workdesk-desktop -- --remote
```

## 將 CLI 指令轉送到主視窗

```powershell
cargo run -p workdesk-desktop -- open
cargo run -p workdesk-desktop -- open-run --run-id run-123
cargo run -p workdesk-desktop -- open-workflow --workflow-id wf-123
cargo run -p workdesk-desktop -- run-workflow --workflow-id wf-123
```

若已有 primary instance，secondary process 會透過 named-pipe command bus 轉送命令後退出。

## Automation Mode

```powershell
$env:WORKDESK_ENABLE_AUTOMATION="1"
cargo run -p workdesk-desktop -- --automation
```

Automation mode 會開啟本機測試通道，用於：

- 讀取 `UiStateSnapshot`
- 派送 desktop commands
- 觸發 run cancel/retry 操作

## 建立 Windows Payload 與 MSI

只建立 release payload：

```powershell
powershell -File .\scripts\windows\build-installer.ps1 -ProductVersion 0.1.0
```

建立 release payload 並嘗試產出 MSI：

```powershell
powershell -File .\scripts\windows\build-installer.ps1 -ProductVersion 0.1.0 -BuildMsi
```

若要直接產生或檢查 harvested WiX payload fragment：

```powershell
powershell -File .\scripts\windows\wix\Harvest-Payload.ps1 -PayloadDir .\dist\windows\payload -OutputPath .\dist\windows\Payload.wxs
```

## Toolchain 與 App Update 基線

- Toolchain feeds
  - `ToolchainManager` 可接受本機路徑或 HTTP JSON feed，並在替換 managed binaries 前驗證 SHA-256。
- App update feeds
  - `AppUpdateFeed` 依 channel 管理 manifests。
  - `AppUpdateManifest` 會先驗證 Ed25519 signature，再驗證 package SHA-256，確認 installer 可被信任。

## Filesystem 與 Office API

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

## 常見 Diagnostics

- `RUNNER_UNAVAILABLE`
- `SIDECAR_UNAVAILABLE`
- `DOCSERVER_UNAVAILABLE`
