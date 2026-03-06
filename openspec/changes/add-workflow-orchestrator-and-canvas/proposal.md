# Change: Add Workflow Orchestrator and Canvas

## Why
Phase-1 requires real workflow node execution and an operator-facing canvas flow rather than placeholder run completion.

## What Changes
- Persist run node states and expose node-state API.
- Runner executes nodes in DAG order with lifecycle events.
- Desktop run detail includes node-level execution visibility.
- Keep proposal approval gate semantics for workflow changes.

## Impact
- Affected specs: `runner-engine`, `core-service`, `desktop-shell`
- Affected code: `workdesk-core`, `workdesk-runner`, `workdesk-desktop`
