# WorkDesk Core API

Base path: `/api/v1`

## Envelope

All HTTP responses use one envelope schema.

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

## Stabilized Error Codes

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

## HTTP Endpoints

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
- `PATCH /workflows/{id}`
- `PATCH /workflows/{id}/status`
- `POST /workflows/{id}/run`
- `POST /workflows/{id}/proposals`
- `POST /workflows/{id}/proposals/{proposal_id}/approve`

`POST /workflows/{id}/proposals/{proposal_id}/approve` now enforces proposal apply semantics:

- proposal `diff` must be a JSON workflow patch payload
- approval applies the patch and creates a new workflow version
- proposal state is persisted as `applied` only after patch success

Workflow definitions now persist:

- node canvas coordinates: `x`, `y`
- node config JSON: `config`
- workflow agent defaults: `agent_defaults.model`, `agent_defaults.model_reasoning_effort`

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

Capability resolution order:

1. query sidecar IPC `get_capabilities`
2. fallback to local models cache

Fallback only infers `model` and `model_reasoning_effort`. It never infers `speed`.

Native workbench config fields map directly to Codex-native names:

- `model`
- `model_reasoning_effort`
- `speed`
- `plan_mode`

`speed` is session-scoped. It is not persisted in workflow defaults.

### Runs

- `GET /runs?limit=<n>`
- `GET /runs/{run_id}`
- `GET /runs/{run_id}/events?after_seq=<n>&limit=<n>`
- `GET /runs/{run_id}/nodes`
- `GET /runs/{run_id}/skills`
- `POST /runs/{run_id}/cancel`
- `POST /runs/{run_id}/retry`

Run node execution evidence now includes retry scheduling events when node retry policy is configured in node `config`:

- `node_started` includes `attempt`
- `node_retry_scheduled` includes `attempt`, `next_attempt`, and `backoff_ms`
- `node_succeeded` / `node_failed` include `attempt`

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

## Desktop Local IPC (Windows First)

This section is for desktop local process coordination, not core HTTP.

### Command Bus

- Endpoint: `\\.\pipe\WorkDeskStudio.CommandBus`
- Commands:
  - `open`
  - `open_run`
  - `open_workflow`
  - `run_workflow`

### Automation Channel

- Endpoint: `\\.\pipe\WorkDeskStudio.Automation`
- Supported request types:
  - `get_state`
  - `get_pending_choice_prompt`
  - `refresh_runs`
  - `dispatch_command`
  - `cancel_selected_run`
  - `retry_selected_run`
  - `submit_choice_prompt_option`
  - `submit_choice_prompt_text`
