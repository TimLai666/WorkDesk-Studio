## ADDED Requirements

### Requirement: Native Workbench Session Persistence
The core service SHALL persist Codex-native workbench session state in SQLite.

#### Scenario: Session survives restart
- **WHEN** a workbench session is created with native Codex config and pending choice prompts
- **THEN** the core service reloads the same session, config, messages, and prompts after restart

#### Scenario: Workflow defaults exclude speed
- **WHEN** workflow agent defaults are stored
- **THEN** the core service persists `model` and `model_reasoning_effort`
- **AND** does not persist `speed`

### Requirement: Native Workbench API
The core service SHALL expose workbench session and choice prompt routes with the standard API envelope.

#### Scenario: Session config updates use native field names
- **WHEN** a client updates session config
- **THEN** the request and response use `model`, `model_reasoning_effort`, `speed`, and `plan_mode`
