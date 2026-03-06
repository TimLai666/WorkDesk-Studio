## Purpose
Define phase-1 desktop shell behavior for single-instance orchestration, command routing, diagnostics, and automation test control.

## Requirements
### Requirement: Single Instance Desktop
The desktop app SHALL enforce single-instance behavior and forward secondary commands to the primary instance.

#### Scenario: Secondary instance forwards command
- **WHEN** a second desktop process starts
- **THEN** it sends command payload to primary process via command bus
- **AND** exits without opening a second window

### Requirement: Controller-Based UI State
The desktop app SHALL centralize UI command/state management in `DesktopAppController`.

#### Scenario: Command updates route and selection
- **WHEN** `open-run` command is dispatched
- **THEN** UI route changes to run detail
- **AND** selected run context is refreshed from API

### Requirement: Diagnostics Visibility
The desktop app SHALL surface diagnostic entries for degraded local runtime conditions.

#### Scenario: Runner unavailable diagnostic appears
- **WHEN** a run remains queued beyond threshold
- **THEN** UI snapshot contains `RUNNER_UNAVAILABLE` diagnostic

### Requirement: Automation Test Channel
The desktop app SHALL provide automation mode IPC for state snapshot and test actions.

#### Scenario: Automation client reads state
- **WHEN** automation channel receives `get_state`
- **THEN** current `UiStateSnapshot` is returned
