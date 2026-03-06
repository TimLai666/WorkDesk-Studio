# WorkDesk Core API

基底路徑：`/api/v1`

## Envelope

所有 HTTP 回應都使用同一個 envelope 結構。

成功：

```json
{
  "data": {},
  "error": null,
  "meta": {
    "request_id": "uuid",
    "timestamp": "2026-03-06T12:00:00Z"
  }
}
```

失敗：

```json
{
  "data": null,
  "error": {
    "code": "WORKFLOW_NOT_FOUND",
    "message": "workflow not found",
    "details": null
  },
  "meta": {
    "request_id": "uuid",
    "timestamp": "2026-03-06T12:00:00Z"
  }
}
```

## 固定錯誤碼

- `AUTH_INVALID_CREDENTIALS`
- `AUTH_ACCOUNT_NOT_FOUND`
- `WORKFLOW_NOT_FOUND`
- `PROPOSAL_NOT_FOUND`
- `RUN_NOT_FOUND`
- `RUN_NOT_CANCELABLE`
- `VALIDATION_FAILED`
- `FS_PATH_TRAVERSAL`
- `BAD_REQUEST`
- `INTERNAL_ERROR`

## HTTP 端點

### Health

- `GET /health`

### Auth

- `POST /auth/login`
- `POST /auth/logout`
- `POST /auth/switch`

### Workflows

- `GET /workflows`
- `POST /workflows`
- `GET /workflows/{id}`
- `PATCH /workflows/{id}/status`
- `POST /workflows/{id}/run`
- `POST /workflows/{id}/proposals`
- `POST /workflows/{id}/proposals/{proposal_id}/approve`

Workflow 定義現在會持久化以下欄位：

- 節點畫布座標：`x`、`y`
- 節點設定 JSON：`config`
- workflow agent 預設：`agent_defaults.model`、`agent_defaults.model_reasoning_effort`

### Agent Workbench

- `GET /agent/capabilities`
- `GET /agent/sessions`
- `POST /agent/sessions`
- `PATCH /agent/sessions/{session_id}/config`
- `GET /agent/sessions/{session_id}/messages`
- `POST /agent/sessions/{session_id}/messages`
- `GET /agent/sessions/{session_id}/choice-prompts`
- `POST /agent/sessions/{session_id}/choice-prompts`
- `POST /agent/sessions/{session_id}/choice-prompts/{prompt_id}/answer`

Workbench 設定直接對應 Codex 原生欄位：

- `model`
- `model_reasoning_effort`
- `speed`
- `plan_mode`

`speed` 只屬於互動 session，不會寫進 workflow 預設。

### Runs

- `GET /runs?limit=<n>`
- `GET /runs/{run_id}`
- `GET /runs/{run_id}/events?after_seq=<n>&limit=<n>`
- `GET /runs/{run_id}/nodes`
- `GET /runs/{run_id}/skills`
- `POST /runs/{run_id}/cancel`
- `POST /runs/{run_id}/retry`

### Skills

- `GET /skills`
- `POST /skills`
- `GET /skills/export`
- `POST /skills/import`

### Memory

- `GET /memory`
- `POST /memory`
- `GET /memory/export`
- `POST /memory/import`

### File System

- `GET /fs/tree?path=<relative-path>`
- `GET /fs/search?path=<relative-path>&query=<text>&limit=<n>`
- `GET /fs/file?path=<relative-path>`
- `PUT /fs/file`
- `POST /fs/move`
- `POST /fs/diff`
- `POST /fs/terminal/start`
- `GET /fs/terminal/session/{session_id}`
- `DELETE /fs/path?path=<relative-path>`

### Office

- `POST /office/open`
- `POST /office/save`
- `GET /office/version?path=<relative-path>`
- `POST /office/onlyoffice/callback`
- `POST /office/pdf/preview`
- `POST /office/pdf/annotate`
- `POST /office/pdf/replace`
- `POST /office/pdf/save-version`

## Desktop 本機 IPC（Windows First）

這一節描述的是桌面程式本機協調，不是 core HTTP。

### Command Bus

- 端點：`\\.\pipe\WorkDeskStudio.CommandBus`
- 支援命令：
  - `open`
  - `open_run`
  - `open_workflow`
  - `run_workflow`

### Automation Channel

- 端點：`\\.\pipe\WorkDeskStudio.Automation`
- 支援 request type：
  - `get_state`
  - `get_pending_choice_prompt`
  - `refresh_runs`
  - `dispatch_command`
  - `cancel_selected_run`
  - `retry_selected_run`
  - `submit_choice_prompt_option`
  - `submit_choice_prompt_text`
