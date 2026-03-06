## Purpose
Define phase-1 runner daemon behavior for queue execution, skill materialization, sidecar prompt context, and managed toolchain safety.

## Requirements
### Requirement: DAG Node Execution Lifecycle
The runner daemon SHALL claim queued runs and execute workflow nodes in DAG order with persistent node statuses.

#### Scenario: Node status transitions are persisted
- **WHEN** runner executes a workflow node
- **THEN** node status transitions through `pending` -> `running` -> terminal state

### Requirement: Skill Snapshot Materialization
The runner daemon SHALL materialize run skill snapshots into run-scoped runtime paths before node execution.

#### Scenario: Materialized path is recorded
- **WHEN** run snapshot skills are copied
- **THEN** snapshot entries include materialized paths

### Requirement: Sidecar Prompt Context Contract
The runner SHALL support sidecar IPC envelope for agent prompt node execution context.

#### Scenario: Agent prompt sends sidecar request
- **WHEN** sidecar endpoint is configured
- **THEN** runner sends `run_prompt` request with run/workflow/node/skills context

### Requirement: Managed Toolchain Metadata and Rollback
The runner SHALL support managed toolchain manifest persistence and binary rollback mechanics.

#### Scenario: Rollback restores previous binary
- **WHEN** rollback is requested for a managed binary
- **THEN** previous binary snapshot is restored
