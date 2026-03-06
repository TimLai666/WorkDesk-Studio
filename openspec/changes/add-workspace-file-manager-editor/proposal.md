# Change: Add Workspace File Manager and Editor APIs

## Why
Phase-1 desktop file workflows require search/diff/terminal support beyond basic tree/read/write/move/delete.

## What Changes
- Add FS API routes for full-text search, text diff, and terminal session output.
- Keep workspace path traversal protections.
- Extend desktop API client and run detail state synchronization.

## Impact
- Affected specs: `core-service`, `desktop-shell`
- Affected code: `workdesk-core`, `workdesk-desktop`
