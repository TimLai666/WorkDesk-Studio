## ADDED Requirements

### Requirement: Windows Installer Channel Separation
The desktop distribution SHALL separate application updates from toolchain updates.

#### Scenario: Toolchain rollback does not alter app binary
- **WHEN** a toolchain rollback is executed
- **THEN** desktop application binary remains unchanged

### Requirement: MSI Packaging Baseline
The release pipeline SHALL target MSI (WiX) for phase-1 Windows installation.

#### Scenario: Release payload is installer-ready
- **WHEN** release build scripts run
- **THEN** desktop/core binaries are produced in installer payload directory
