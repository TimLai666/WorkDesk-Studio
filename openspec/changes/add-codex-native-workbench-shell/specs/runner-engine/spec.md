## ADDED Requirements

### Requirement: Codex Native Capability Mapping
The runner and sidecar integration SHALL use Codex-native capability fields when mapping session controls.

#### Scenario: Capabilities enumerate native reasoning values
- **WHEN** capability data is returned from the sidecar
- **THEN** the runtime exposes model names and native reasoning effort values without app-defined aliases

#### Scenario: Speed remains capability-gated
- **WHEN** the sidecar cannot prove speed support for a model
- **THEN** the runtime does not advertise speed as supported
