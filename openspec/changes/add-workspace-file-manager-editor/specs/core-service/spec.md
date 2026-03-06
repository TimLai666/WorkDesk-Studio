## MODIFIED Requirements

### Requirement: Workspace File Operations
The core service SHALL support tree/read/write/move/delete and search/diff/terminal file operations with path safety checks.

#### Scenario: Search returns text matches
- **WHEN** a user searches a workspace path with a query
- **THEN** matched file path and line preview are returned

#### Scenario: Diff returns insert/delete hunks
- **WHEN** a user compares two workspace files
- **THEN** response includes line-level insert/delete hunks

#### Scenario: Terminal command session output is retrievable
- **WHEN** a terminal command is started for a workspace directory
- **THEN** API returns a session id and stored command output metadata
