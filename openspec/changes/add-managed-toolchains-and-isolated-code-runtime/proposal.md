# Change: Add Managed Toolchains and Isolated Code Runtime

## Why
Phase-1 requires app-scoped toolchains and recoverable update mechanics instead of placeholder directory setup.

## What Changes
- Add manifest read/write model for managed toolchains.
- Add binary update staging and rollback mechanics.
- Keep per-workflow runtime isolation paths for Python/JS/Go.

## Impact
- Affected specs: `runner-engine`
- Affected code: `workdesk-runner`, `scripts/windows`
