# Change: Add Codex SDK Sidecar Auth and Session Contract

## Why
Phase-1 requires a stable sidecar IPC contract so desktop/core/runner can decouple from direct CLI-only flows.

## What Changes
- Add sidecar request/response envelope types in domain model.
- Add runner sidecar IPC client for local command transport.
- Wire agent prompt node path to include run context and skills snapshot path.

## Impact
- Affected specs: `runner-engine`
- Affected code: `workdesk-domain`, `workdesk-runner`
