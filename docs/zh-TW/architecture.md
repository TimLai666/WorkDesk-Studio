# WorkDesk Studio 架構

## Runtime 拓樸

- `apps/workdesk-desktop`
  - 單一 Windows-first binary，同時承擔 GUI 與 CLI 入口。
  - Local mode 由 desktop process 啟動 core API、runner loop、sidecar supervisor 與 OnlyOffice launcher。
  - Remote mode 保留同一個桌面殼，只把 core 連到外部服務。
- `crates/workdesk-core`
  - 提供 auth、workflow、proposal、run、skills、memory、filesystem、office、updater metadata 與 native workbench session 的 HTTP API。
- `crates/workdesk-runner`
  - Workflow runner daemon，負責 claim queued run、materialize run skill snapshot、執行 DAG 節點並持久化 run/node 狀態。

## Desktop 產品層

- `DesktopAppController`
  - 中央 command/state/view 協調器。
  - 管理 run 監看、canvas 狀態、檔案、office、native workbench session、choice prompt 與 diagnostics。
- 單例殼
  - Mutex：`Global\WorkDeskStudio.Singleton`
  - Command bus：`\\.\pipe\WorkDeskStudio.CommandBus`
  - Automation bus：`\\.\pipe\WorkDeskStudio.Automation`
- GPUI + `gpui-component`
  - 主殼改為 Codex 風格 workbench。
  - 左側顯示 session 與 capability context。
  - 中間顯示 composer 風格控制列與訊息區。
  - 右側保留 run、file、office 等上下文面板。

## 本機 Runtime Supervisor

- Sidecar supervisor
  - 監看 bundled `node.exe + sidecar.js`。
  - runtime 缺失或健康檢查失敗時回報 `SIDECAR_UNAVAILABLE`。
- OnlyOffice launcher
  - 監看設定中的 Document Server binary 與 health endpoint。
  - runtime 缺失或健康檢查失敗時回報 `DOCSERVER_UNAVAILABLE`。

## 持久化與領域狀態

- 本機持久化使用 `sqlx + SQLite`。
- Windows 預設資料庫路徑：
  - `%LOCALAPPDATA%\WorkDeskStudio\data\workdesk.db`

主要持久化區塊：

- Auth：`users`, `sessions`
- Workflow：`workflows`, `workflow_nodes`, `workflow_edges`, `workflow_proposals`
- Workbench：`agent_workspace_sessions`, `agent_workspace_messages`, `agent_workspace_choice_prompts`, `agent_workspace_choice_prompt_options`, `agent_workspace_preferences`
- Knowledge：`skills`, `memory_records`
- Runs：`workflow_runs`, `workflow_run_events`, `workflow_run_nodes`, `workflow_run_skill_snapshots`, `runner_leases`
- Office 歷史：`office_versions`

Workflow 持久化另外保存：

- 每個節點的畫布座標：`x`, `y`
- 節點設定 JSON：`config_json`
- workflow agent 預設 JSON：`agent_defaults_json`

## 原生 Codex 映射

- Session 設定直接使用原生欄位：
  - `model`
  - `model_reasoning_effort`
  - `speed`
  - `plan_mode`
- Workflow 預設只保存：
  - `model`
  - `model_reasoning_effort`
- `speed` 仍然只屬於 session，並且必須由 capability 明確支援。
- Choice prompt 對應 Codex 的 request-user-input 互動模型，不與 diagnostics 混用。

## 診斷訊號

- `RUNNER_UNAVAILABLE`
  - run 超過 90 秒仍停在 queued 且未被 claim。
- `SIDECAR_UNAVAILABLE`
  - sidecar runtime 缺失或不健康。
- `DOCSERVER_UNAVAILABLE`
  - 內嵌文件服務缺失或不健康。

所有 diagnostics 都會出現在 desktop UI 與 automation snapshot。
