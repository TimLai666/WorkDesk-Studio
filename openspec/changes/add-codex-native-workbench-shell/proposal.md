# Change: Add Codex Native Workbench Shell

## Why
WorkDesk Studio still uses a route-based demo desktop even though Phase 1 now needs a Codex-native workbench shell with session persistence, native model/reasoning mapping, choice prompts, and a tighter integration path with the Codex app server.

## What Changes
- Add a native Codex workbench session model, persistence, and API surface.
- Add desktop workbench state, automation hooks, and native choice prompt handling.
- Extend workflow metadata with persisted canvas coordinates and workflow-level agent defaults.
- Tighten the sidecar contract around Codex-native fields instead of app-defined abstractions.

## Impact
- Affected specs: `desktop-shell`, `core-service`, `runner-engine`
- Affected code: `workdesk-domain`, `workdesk-core`, `workdesk-desktop`, `workdesk-runner`
