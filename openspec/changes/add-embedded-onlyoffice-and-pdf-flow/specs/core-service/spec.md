## ADDED Requirements

### Requirement: Office Versioned Save Baseline
The core service SHALL preserve previous office file revisions during save operations.

#### Scenario: Save creates previous version snapshot
- **WHEN** an office file already exists and is saved
- **THEN** previous file bytes are stored as a version record before overwrite

### Requirement: OnlyOffice Callback Contract
The core service SHALL provide an API extension point for embedded OnlyOffice callbacks.

#### Scenario: Callback route follows envelope contract
- **WHEN** OnlyOffice integration callback is invoked
- **THEN** response follows the unified API envelope shape
