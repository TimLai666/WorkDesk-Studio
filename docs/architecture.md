# WorkDesk Studio Architecture (Phase 1 -> Phase 2)

## Runtime Topology

- `workdesk-desktop`: desktop shell process, starts local core in local mode, or points to remote core in remote mode.
- `workdesk-core`: API process for auth, workflows, proposals, skills, memory, filesystem, and office endpoints.
- `workdesk-runner`: execution layer for toolchain/runtime control and agent/code execution adapters.

## Data Ownership

- Phase 1: in-memory store in `workdesk-core` plus workspace files on disk.
- Phase 2 target: SQLite for local mode, PostgreSQL for server mode.
- Scope boundaries:
  - `user` scope: private skills/memory per account.
  - `shared` scope: organization-level skills.

## Workflow Safety Model

- Workflows are DAGs validated before persistence.
- Agent modifications are represented as `WorkflowChangeProposal`.
- Proposal must be `pending` before approval and application.
- Human approval is mandatory before proposal application.

## Toolchain & Runtime Isolation

- Toolchains are app-scoped (non-global) under app-controlled root.
- Runtime paths are workflow-scoped and language-scoped:
  - `<tools>/workflows/<workflow-id>/python`
  - `<tools>/workflows/<workflow-id>/javascript`
  - `<tools>/workflows/<workflow-id>/go`

## Office Integration Path

- Current implementation exposes office open/save/version API shape.
- Phase-2 deployment includes `onlyoffice/documentserver` in Docker Compose.
- Desktop embedding of OnlyOffice editor panel is tracked as next UI milestone.
