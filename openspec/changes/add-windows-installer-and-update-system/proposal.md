# Change: Add Windows Installer and Update System

## Why
Phase-1 requires production install/upgrade flow and separated app/toolchain update channels.

## What Changes
- Define MSI (WiX) packaging contract and payload boundaries.
- Define app update vs toolchain update separation.
- Add implementation tasks for rollback and diagnostics logging.

## Impact
- Affected specs: `desktop-shell`, `runner-engine`
- Affected code: `scripts/windows`, release packaging pipeline
