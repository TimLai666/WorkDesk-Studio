## MODIFIED Requirements

### Requirement: Managed Toolchain Metadata and Rollback
The runner SHALL support managed toolchain manifest persistence and binary rollback mechanics.

#### Scenario: Manifest roundtrip
- **WHEN** managed toolchain records are saved
- **THEN** manifest can be loaded without data loss

#### Scenario: Rollback restores previous binary
- **WHEN** rollback is requested for a managed binary
- **THEN** previous binary snapshot is restored
