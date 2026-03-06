## MODIFIED Requirements

### Requirement: DAG Node Execution Lifecycle
The runner daemon SHALL claim queued runs and execute workflow nodes in DAG order with persistent node statuses.

#### Scenario: Node status transitions are persisted
- **WHEN** runner executes a workflow node
- **THEN** node status transitions through `pending` -> `running` -> terminal state

#### Scenario: Node failure marks run as failed
- **WHEN** a node execution fails
- **THEN** node status is set to `failed`
- **AND** run status is updated to `failed`
