# WorkDesk Core API

Base path: `/api/v1`

## Envelope

所有回應統一使用 JSON envelope。

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

## 錯誤碼（固定）

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

## 端點

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
- `POST /workflows/{id}/run`（入列 run + 建立 skills snapshot）
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
