# WorkDesk Core API

Base path: `/api/v1`

## Envelope

All responses use the same JSON envelope.

Success:

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

Failure:

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

## Error Codes (Stabilized)

- `AUTH_INVALID_CREDENTIALS`
- `AUTH_ACCOUNT_NOT_FOUND`
- `WORKFLOW_NOT_FOUND`
- `PROPOSAL_NOT_FOUND`
- `VALIDATION_FAILED`
- `FS_PATH_TRAVERSAL`
- `BAD_REQUEST`
- `INTERNAL_ERROR`

## Endpoints

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
