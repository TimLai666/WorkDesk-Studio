## Context
A stable IPC contract is required before sidecar runtime process management can be productionized.

## Decisions
- Use JSON envelope with `type`, `payload`, `request_id` request shape.
- Treat non-OK sidecar responses as execution errors.
- Keep endpoint configurable through environment.

## Risks / Trade-offs
- Sidecar runtime packaging and lifecycle manager remains follow-up implementation.
