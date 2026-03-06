# WorkDesk Studio 架構

## 執行拓樸

- `apps/workdesk-desktop`
  - 單一 binary，同時提供 GUI 與 CLI 入口。
  - Local mode 在同一個 desktop process 內啟動 core + runner loops。
  - Remote mode 連線到外部 core service。
- `crates/workdesk-core`
  - 提供 auth、workflow、proposal、skills、memory、run queue、filesystem、office 的 HTTP API。
- `crates/workdesk-runner`
  - Workflow runner daemon，負責 claim queued runs、materialize skills snapshot、寫入 run events/status。

## 桌面產品層（本里程碑）

- `DesktopAppController`
  - 集中處理 command/state/view。
  - 統一處理 CLI/IPC 命令、API 呼叫與 run detail 同步。
- 單例機制（Windows 優先）
  - Mutex：`Global\WorkDeskStudio.Singleton`
  - 第二實例只轉送命令到主實例後退出。
- 本機 command bus
  - Named pipe：`\\.\pipe\WorkDeskStudio.CommandBus`
  - Request envelope：`{ "type": "...", "payload": { ... }, "request_id": "..." }`
  - Response envelope：`{ "ok": true|false, "error": { ... } }`
- GPUI + `gpui-component`
  - 主畫面包含 Run List + Run Detail。
  - Run Detail 顯示 events 與本次 run 的 skills snapshot。
  - UI 動作含 refresh、cancel run、retry run。
- Automation mode（`--automation`）
  - 測試 channel：`\\.\pipe\WorkDeskStudio.Automation`
  - 可讀取 UI state snapshot，並觸發測試動作。

## 持久化策略

- 本機持久化採 `sqlx + SQLite`。
- Windows 預設 DB 路徑：
  - `%LOCALAPPDATA%\WorkDeskStudio\data\workdesk.db`
- 覆蓋設定：
  - `WORKDESK_DB_PATH`
- 啟動流程：
  1. 解析 `AppConfig`
  2. 確保 DB 父目錄存在
  3. 開啟 SQLite pool
  4. 套用 migrations
  5. 啟動 API service

## 資料模型範圍

- Auth：`users`, `sessions`
- Workflow：`workflows`, `workflow_nodes`, `workflow_edges`, `workflow_proposals`
- 知識資料：`skills`, `memory_records`
- Run queue：`workflow_runs`, `workflow_run_events`, `workflow_run_skill_snapshots`, `runner_leases`
- Office 版本：`office_versions`

Scope 邊界：

- `user`：使用者私有資料
- `shared`：可共用資料

## API Envelope 契約

- 成功：
  - `{ "data": ..., "error": null, "meta": { "request_id": "...", "timestamp": "..." } }`
- 失敗：
  - `{ "data": null, "error": { "code": "...", "message": "...", "details": ... }, "meta": {...} }`

## Run + Skills Snapshot 流程

1. `POST /workflows/{id}/run` 在 `workflow_runs` 建立排隊 run。
2. Core 從 `skills` 建立該 run 的 snapshot。
   - 合併順序：`shared + user`
   - 同名衝突：`user` scope 優先
3. Runner claim queued run，並把 snapshot materialize 到 run runtime 目錄。
4. Runner 寫入 `workflow_run_events` 並更新 run status。
