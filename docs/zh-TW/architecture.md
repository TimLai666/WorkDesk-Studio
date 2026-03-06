# WorkDesk Studio 架構說明

## 執行拓樸

- `apps/workdesk-desktop`
  - Windows-first 的單一 binary，同時承擔 GUI 與 CLI 入口。
  - Local mode 會在 desktop process 內啟動 core API、runner loop、sidecar supervisor 與 OnlyOffice launcher。
  - Remote mode 連到外部 core service，但保留相同桌面 UI 殼層。
- `crates/workdesk-core`
  - 提供 auth、workflow、proposal、run、skills、memory、filesystem、office 與 update metadata 的 HTTP API。
- `crates/workdesk-runner`
  - 負責 claim queued runs、materialize run skill snapshots、執行 DAG 節點，並持久化 run/node 狀態。

## Desktop 產品層

- `DesktopAppController`
  - 中央 command/state/view 協調器。
  - 處理單例 IPC、API 呼叫、diagnostics、導覽，以及 automation 所需的 UI state snapshot。
- 單例 shell
  - Mutex: `Global\WorkDeskStudio.Singleton`
  - Command bus: `\\.\pipe\WorkDeskStudio.CommandBus`
  - Automation bus: `\\.\pipe\WorkDeskStudio.Automation`
- GPUI + `gpui-component`
  - 主要路由為 Run monitor、Workflow canvas、File manager、Office/PDF desk。
  - Run detail 顯示 events、node lifecycle、diagnostics 與 run skills snapshot。
  - File desk 顯示 workspace tree、文字編輯器、搜尋結果、diff 視圖與 terminal 輸出。
  - Office/PDF desk 顯示文件開啟/儲存、版本歷史、PDF 標註與文字替換流程，以及 OnlyOffice callback 狀態。

## 本機 Runtime Supervisor

- Sidecar supervisor
  - 監看內建 `node.exe + sidecar.js`。
  - 透過 HTTP、TCP 或 named pipe 檢查 sidecar endpoint。
  - 當 runtime 檔案缺失或健康檢查失敗時，發出 `SIDECAR_UNAVAILABLE`。
- OnlyOffice launcher
  - 監看設定好的 Document Server binary 與 health endpoint。
  - 當檔案存在但服務不健康時，嘗試啟動內嵌 runtime。
  - 當 runtime 檔案缺失或健康檢查失敗時，發出 `DOCSERVER_UNAVAILABLE`。

## 持久化與資料狀態

- 本機持久化採 `sqlx + SQLite`。
- Windows 預設資料庫路徑：
  - `%LOCALAPPDATA%\WorkDeskStudio\data\workdesk.db`
- Toolchain manifest 路徑：
  - `%LOCALAPPDATA%\WorkDeskStudio\config\toolchains.json`
- 啟動流程：
  1. 解析 `AppConfig`
  2. 確保 AppData 目錄存在
  3. 開啟 SQLite 並套用 migrations
  4. 啟動 API service 與背景 supervisor

主要持久化範圍：

- Auth: `users`, `sessions`
- Workflow: `workflows`, `workflow_nodes`, `workflow_edges`, `workflow_proposals`
- Knowledge: `skills`, `memory_records`
- Runs: `workflow_runs`, `workflow_run_events`, `workflow_run_nodes`, `workflow_run_skill_snapshots`, `runner_leases`
- Office 歷史：`office_versions`

## Workflow 與 Run 執行流程

1. `POST /api/v1/workflows/{id}/run` 建立 run record。
2. Core 依 `shared + user` skills 建立 run-time skill snapshot，若名稱衝突則 `user` scope 優先。
3. Core 在執行前先持久化 run-node lifecycle rows。
4. Runner claim queued runs，將 skill paths materialize 到 workflow runtime root，並依 DAG 順序執行節點。
5. Node 狀態轉移會持久化為 `pending -> running -> succeeded|failed|canceled`。
6. Core 與 runner 持續追加 run events，並將狀態暴露給 desktop UI 與 automation snapshot。

## 更新與封裝基線

- Managed toolchains
  - `ToolchainManager` 管理 app-scoped 的 `codex`、`uv`、`bun`、`go` binaries。
  - `ToolchainReleaseFeed` 支援從本機路徑或 HTTP 載入 feed。
  - 下載到的 release asset 會先驗證 SHA-256，再更新 manifest；若失敗則回滾到 `.previous` snapshot。
- App updates
  - `AppUpdateFeed` 與 `AppUpdateManifest` 定義依 channel 分流的 update metadata。
  - signed manifest 驗證採固定 Ed25519 public key，加上 package SHA-256 驗證。
  - app update channel 與 toolchain update channel 明確分離。
- Windows installer
  - `scripts/windows/build-installer.ps1` 產出 desktop/core/runner 的 release payload。
  - `scripts/windows/wix/Harvest-Payload.ps1` 將 payload 目錄轉成 `Payload.wxs`。
  - `scripts/windows/wix/Product.wxs` 定義 MSI product、固定 `UpgradeCode`、`MajorUpgrade` 與具交易性的 upgrade scheduling。

## Diagnostics 與降級訊號

- `RUNNER_UNAVAILABLE`
  - run 超過 90 秒仍停留在 queued 且未被 claim。
- `SIDECAR_UNAVAILABLE`
  - sidecar runtime 缺失或不健康。
- `DOCSERVER_UNAVAILABLE`
  - 內嵌 document server 缺失或不健康。

所有 diagnostics 都會透過 desktop UI 與 automation snapshot 暴露。

## API Envelope 契約

- 成功：
  - `{ "data": ..., "error": null, "meta": { "request_id": "...", "timestamp": "..." } }`
- 失敗：
  - `{ "data": null, "error": { "code": "...", "message": "...", "details": ... }, "meta": { ... } }`
