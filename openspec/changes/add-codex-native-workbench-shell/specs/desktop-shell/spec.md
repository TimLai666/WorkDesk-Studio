## ADDED Requirements

### Requirement: Codex Native Workbench Shell
The desktop SHALL expose a Codex-native workbench shell as the main UI.

#### Scenario: Desktop restores persisted session controls
- **WHEN** the desktop starts in local mode
- **THEN** it loads the last active workbench session from SQLite
- **AND** restores `model`, `model_reasoning_effort`, `speed`, and `plan_mode`

#### Scenario: Desktop uses Codex request-user-input style choice prompts
- **WHEN** a session has a pending choice prompt
- **THEN** the desktop renders the prompt separately from diagnostics
- **AND** allows selecting a recommended option or freeform answer

### Requirement: Desktop automation exposes workbench state
The desktop SHALL expose workbench session and choice prompt state through automation mode.

#### Scenario: Automation reads pending choice prompt
- **WHEN** automation requests the current pending choice prompt
- **THEN** the desktop returns the same prompt data rendered in the UI
