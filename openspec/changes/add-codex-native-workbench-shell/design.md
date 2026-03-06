## Context
The current desktop shell is a route-based scaffold. It does not persist Codex-style sessions, does not expose native model configuration, and does not surface Codex request-user-input choice prompts.

## Decisions
- Use Codex-native field names where the app maps directly to Codex app-server semantics: `model`, `model_reasoning_effort`, `speed`, `plan_mode`.
- Persist workbench sessions, messages, and choice prompts in SQLite so local mode survives restart.
- Keep workflow defaults limited to durable agent defaults (`model`, `model_reasoning_effort`) and exclude `speed`.
- Keep diagnostics separate from choice prompts.

## Risks / Trade-offs
- Codex native `speed` support is capability-driven. If the sidecar cannot prove support, the desktop must hide or disable the control rather than guessing.
- The current desktop controller and UI need modularization before the workbench grows further.
