## Purpose
Define phase-1 core HTTP service behavior for persistence, envelope contract, workflow run introspection, and workspace operations.

## Requirements
### Requirement: Unified API Envelope
The core HTTP service SHALL return a unified envelope for all success and failure responses.

#### Scenario: Success response uses envelope
- **WHEN** a core endpoint succeeds
- **THEN** response body includes `data`, `error`, and `meta`
- **AND** `error` is `null`

#### Scenario: Failure response uses envelope
- **WHEN** a core endpoint fails
- **THEN** response body includes `data`, `error`, and `meta`
- **AND** `data` is `null`

### Requirement: Persistent Local Data
The core service SHALL persist auth, workflow, run, skills, memory, and office version data in SQLite.

#### Scenario: Data survives restart
- **WHEN** core service is restarted
- **THEN** previously created records remain queryable

### Requirement: Workflow Run Introspection
The core service SHALL expose run events, run node states, and run skill snapshots.

#### Scenario: Run node states are available
- **WHEN** a run is queued
- **THEN** node state rows are created for workflow nodes
- **AND** node status can transition through execution lifecycle

### Requirement: Workspace File Operations
The core service SHALL support tree/read/write/move/delete and search/diff/terminal file operations with path safety checks.

#### Scenario: Search returns text matches
- **WHEN** a user searches a workspace path with a query
- **THEN** matched file path and line preview are returned

#### Scenario: Path traversal is rejected
- **WHEN** a request includes parent traversal outside workspace root
- **THEN** the API returns a path traversal error
