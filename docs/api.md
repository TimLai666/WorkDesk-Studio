# WorkDesk Core API (Scaffold)

Base path: `/api/v1`

## Auth

- `POST /auth/login`
- `POST /auth/logout`
- `POST /auth/switch`

## Workflows

- `GET /workflows`
- `POST /workflows`
- `GET /workflows/:id`
- `PATCH /workflows/:id/status`
- `POST /workflows/:id/run`
- `POST /workflows/:id/proposals`
- `POST /workflows/:id/proposals/:proposal_id/approve`

## Skills and Memory

- `GET/POST /skills`
- `GET /skills/export`
- `POST /skills/import`
- `GET/POST /memory`
- `GET /memory/export`
- `POST /memory/import`

## Filesystem

- `GET /fs/tree?path=<relative-path>`
- `GET /fs/file?path=<relative-path>`
- `PUT /fs/file`
- `POST /fs/move`
- `DELETE /fs/path?path=<relative-path>`

## Office

- `POST /office/open`
- `POST /office/save`
- `GET /office/version?path=<relative-path>`
