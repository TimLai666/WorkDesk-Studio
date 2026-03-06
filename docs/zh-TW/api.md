# WorkDesk Core API

Base path: `/api/v1`

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

## 穩定錯誤碼

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
- `POST /workflows/{id}/run`（enqueue run + 建立 run skill snapshot）
- `POST /workflows/{id}/proposals`
- `POST /workflows/{id}/proposals/{proposal_id}/approve`

### Runs

- `GET /runs?limit=<n>`
- `GET /runs/{run_id}`
- `GET /runs/{run_id}/events?after_seq=<n>&limit=<n>`
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
- `GET /fs/file?path=<relative-path>`
- `PUT /fs/file`
- `POST /fs/move`
- `DELETE /fs/path?path=<relative-path>`

### Office

- `POST /office/open`
- `POST /office/save`
- `GET /office/version?path=<relative-path>`

## Desktop 本機 IPC（Windows 優先）

此段為 desktop 本機行程協調，不是 core HTTP API。

### Command Bus

- Endpoint：`\\.\pipe\WorkDeskStudio.CommandBus`
- Request：

```json
{
  "type": "open_run",
  "payload": { "run_id": "run-123" },
  "request_id": "uuid"
}
```

- Response：

```json
{
  "ok": true,
  "error": null
}
```

### Automation Channel

- Endpoint：`\\.\pipe\WorkDeskStudio.Automation`
- 支援 request type：
  - `get_state`
  - `refresh_runs`
  - `dispatch_command`
  - `cancel_selected_run`
  - `retry_selected_run`
