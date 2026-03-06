# WorkDesk Studio Project

## Scope
WorkDesk Studio is a Windows-first Rust desktop product with local-first execution for phase 1.

## Stack
- Rust workspace: `workdesk-core`, `workdesk-runner`, `workdesk-domain`, `workdesk-desktop`
- UI: GPUI + gpui-component
- API: Axum + sqlx (SQLite)
- Runner: Tokio daemon, managed toolchains (`codex`, `uv`, `bun`, `go`)

## Delivery Rules
- Phase 1 completeness before phase 2 multi-user server scope.
- Windows single-instance desktop behavior is mandatory.
- Local and remote API shape must stay compatible.
- API responses use a unified envelope.

## Documentation Rules
- English docs are source of truth under `docs/en`.
- Traditional Chinese docs under `docs/zh-TW` must stay synchronized.
- CI docs sync gate must pass on every PR.

## Quality Rules
- Every behavior change requires tests (unit/integration/e2e where practical).
- OpenSpec changes require `proposal.md`, `tasks.md`, `design.md`, and spec deltas.
- `openspec validate --strict` must pass before completion claims.
