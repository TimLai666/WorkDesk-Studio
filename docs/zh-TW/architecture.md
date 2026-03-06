# WorkDesk Studio Architecture

## 執行拓樸

- `apps/workdesk-desktop`
  - 單一 binary 同時提供 GUI 與 CLI 入口。
  - Local mode 在同一個 desktop process 內啟動 core + runner loops。
  - Remote mode 連到外部 core service。
- `crates/workdesk-core`
  - 提供 auth、workflow、proposal、skills、memory、run queue、filesystem、office API。
- `crates/workdesk-runner`
  - 透過 queue claim run，載入 skills snapshot，執行 DAG 節點並回寫 run/node 狀態。

## Desktop 產品層（目前里程碑）

- `DesktopAppController`
  - 集中管理 command/state/view。
  - 處理 CLI/IPC 命令、API 呼叫與 run detail 同步。
- 單例（Windows-first）
  - Mutex：`Global\WorkDeskStudio.Singleton`
  - 第二實例只轉送命令，不開新視窗。
- 本機 Command Bus
  - Named pipe：`\\.\pipe\WorkDeskStudio.CommandBus`
  - Request：`{ "type": "...", "payload": { ... }, "request_id": "..." }`
  - Response：`{ "ok": true|false, "error": { ... } }`
- GPUI + `gpui-component`
  - 主畫面含 Run List + Run Detail。
  - Run Detail 包含 events、run nodes、skills snapshot。
  - UI 操作包含 refresh、cancel run、retry run。
- Automation mode（`--automation`）
  - 測試通道：`\\.\pipe\WorkDeskStudio.Automation`
  - 可讀 UI state snapshot，也可觸發測試動作。

## 持久化策略

- 本機持久化採 `sqlx + SQLite`。
- Windows 預設 DB 路徑：
  - `%LOCALAPPDATA%\WorkDeskStudio\data\workdesk.db`
- 覆蓋路徑：
  - `WORKDESK_DB_PATH`
- 啟動流程：
  1. 解析 `AppConfig`
  2. 建立 DB parent directory
  3. 開啟 SQLite pool
  4. 套用 migration
  5. 啟動 API

## 資料模型範圍

- Auth：`users`, `sessions`
- Workflow：`workflows`, `workflow_nodes`, `workflow_edges`, `workflow_proposals`
- Knowledge：`skills`, `memory_records`
- Run queue：`workflow_runs`, `workflow_run_events`, `workflow_run_nodes`, `workflow_run_skill_snapshots`, `runner_leases`
- Office 版本：`office_versions`

Scope：

- `user`：使用者私有資料
- `shared`：共享資料

## API Envelope 契約

- 成功：
  - `{ "data": ..., "error": null, "meta": { "request_id": "...", "timestamp": "..." } }`
- 失敗：
  - `{ "data": null, "error": { "code": "...", "message": "...", "details": ... }, "meta": {...} }`

## Run + Skills Snapshot 流程

1. `POST /workflows/{id}/run` 會在 `workflow_runs` 建立 run。
2. Core 由 `skills` 產生 run-time skill snapshot。
   - 合併順序：`shared + user`
   - 同名衝突：`user` 優先
3. Core 初始化 `workflow_run_nodes`（節點 lifecycle 持久化）。
4. Runner claim queued run，materialize skills 到 run runtime。
5. Runner 依 DAG 順序執行節點並回寫狀態：
   - `pending -> running -> succeeded|failed|canceled`
6. Runner 寫入 `workflow_run_events` 並更新 run 結果。

## 診斷訊號

- Desktop controller 會由 run 狀態推導 diagnostics。
- 若 run `queued` 超過 90 秒，`UiStateSnapshot` 會出現：
  - `RUNNER_UNAVAILABLE`
- 該訊號可在 UI 與 automation mode 觀察到。

## Sidecar 契約基線

- Runner 已提供 sidecar IPC envelope：
  - request：`type`, `payload`, `request_id`
  - response：`ok`, `data`, `error`, `meta`
- 若設定 sidecar endpoint，AgentPrompt 節點會送出 run/workflow/node/skills context。
