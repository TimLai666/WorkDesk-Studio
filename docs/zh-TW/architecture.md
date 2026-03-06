# WorkDesk Studio 架構

## 執行拓樸

- `apps/workdesk-desktop`：桌面殼層。Local mode 啟動內嵌 core；Remote mode 連線遠端 core。
- `crates/workdesk-core`：提供 auth、workflow、proposal、skills、memory、filesystem、office API。
- `crates/workdesk-runner`：程式節點執行與工具鏈管理層，承接 Codex adapter。

## 持久化策略（本里程碑）

- 本機持久化使用 `sqlx + SQLite`。
- Windows 預設資料庫路徑：
  - `%LOCALAPPDATA%\WorkDeskStudio\data\workdesk.db`
- 可由環境變數覆蓋：
  - `WORKDESK_DB_PATH`
- Core 啟動流程：
  1. 由環境變數載入 `AppConfig`。
  2. 建立 DB 上層資料夾。
  3. 開啟 SQLite pool。
  4. 先套用 migration，再啟動 API 服務。

## 資料模型範圍

- Auth：`users`、`sessions`
- Workflow：`workflows`、`workflow_nodes`、`workflow_edges`、`workflow_proposals`
- 知識資料：`skills`、`memory_records`
- 辦公版本：`office_versions`

scope 邊界：

- `user`：單一帳號私有資料。
- `shared`：可共享資料。

## Auth 安全基線

- 密碼使用 Argon2 雜湊（`password_hash`），不存明碼。
- session token 寫入 `sessions`。
- `switch_account` 流程：
  1. 失效舊帳號的有效 session。
  2. 建立新帳號 session。
  3. 回傳新 token。

## API 穩定契約

- 所有端點回傳統一 envelope：
  - 成功：`{ "data": ..., "error": null, "meta": { "request_id": "...", "timestamp": "..." } }`
  - 失敗：`{ "data": null, "error": { "code": "...", "message": "...", "details": ... }, "meta": {...} }`
- 路由 URL 維持既有路徑不變。
- Desktop 端使用單一 envelope 解包與錯誤處理路徑。
