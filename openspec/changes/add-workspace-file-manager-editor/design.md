## Context
Workspace operations must remain local-root constrained while exposing practical editing diagnostics.

## Decisions
- Search and diff run server-side and return concise envelope payloads.
- Terminal execution captures stdout/stderr and exit code for session lookup.
- Existing path traversal checks are reused for all new fs routes.

## Risks / Trade-offs
- Terminal API currently returns completed command output, not fully interactive PTY streaming.
