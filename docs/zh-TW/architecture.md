# WorkDesk Studio 架構

## 執行拓樸

- `apps/workdesk-desktop`：桌面殼層。Local mode 在同一個 app 行程內啟動 core + runner 迴圈；Remote mode 只連遠端 core。
- `crates/workdesk-core`：提供 auth、workflow、proposal、skills、memory、run queue、filesystem、office API。
- `crates/workdesk-runner`：workflow 執行 daemon，負責 claim run、載入 skills snapshot、回寫 run 事件與狀態。

## 持久化策略（目前）

- 本機持久化使用 `sqlx + SQLite`。
- Windows 預設 DB 路徑：
  - `%LOCALAPPDATA%\WorkDeskStudio\data\workdesk.db`
- 可由環境變數覆蓋：
  - `WORKDESK_DB_PATH`
- Core 啟動流程：
  1. 由環境變數載入 `AppConfig`
  2. 建立 DB 上層目錄
  3. 開啟 SQLite pool
  4. 套用 migration 後啟動 API

## 資料模型範圍

- Auth：`users`, `sessions`
- Workflow：`workflows`, `workflow_nodes`, `workflow_edges`, `workflow_proposals`
- 知識：`skills`, `memory_records`
- 執行佇列：`workflow_runs`, `workflow_run_events`, `workflow_run_skill_snapshots`, `runner_leases`
- 辦公版本：`office_versions`

Scope 邊界：

- `user`：使用者私有資料
- `shared`：共享資料

## API 穩定契約

- 全端點統一 envelope：
  - 成功：`{ "data": ..., "error": null, "meta": { "request_id": "...", "timestamp": "..." } }`
  - 失敗：`{ "data": null, "error": { "code": "...", "message": "...", "details": ... }, "meta": {...} }`
- Desktop 端走單一路徑 envelope 解包與錯誤處理。

## Run + Skills Snapshot 流程

1. `POST /workflows/{id}/run` 會建立 run 並入列。
2. Core 會在 run 建立時產生 skills snapshot：
   - 合併 `shared + user`
   - 同名 skill 以 `user` scope 覆寫
3. Runner claim run 後，先把 snapshot materialize 到 run runtime 目錄。
4. Runner 寫入 `skills_loaded` 與完成事件到 `workflow_run_events`。
