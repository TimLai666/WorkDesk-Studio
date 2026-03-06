## MODIFIED Requirements

### Requirement: Sidecar Prompt Context Contract
The runner SHALL support sidecar IPC envelope for agent prompt node execution context.

#### Scenario: Agent prompt sends sidecar request
- **WHEN** sidecar endpoint is configured
- **THEN** runner sends `run_prompt` request with run/workflow/node/skills context

#### Scenario: Sidecar envelope roundtrip
- **WHEN** sidecar responds with `ok=true`
- **THEN** runner accepts response as successful prompt dispatch
