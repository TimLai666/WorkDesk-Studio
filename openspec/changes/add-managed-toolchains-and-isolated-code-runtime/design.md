## Context
Toolchain lifecycle operations need deterministic local metadata and failure recovery.

## Decisions
- Persist toolchain metadata in JSON manifest under app-owned path.
- Use `.previous` binary snapshots for rollback.
- Keep runtime path isolation as `tools/workflows/<workflow_id>/<language>`.

## Risks / Trade-offs
- Download orchestration is intentionally staged after baseline rollback mechanics.
