# WorkDesk Studio

WorkDesk Studio is a Windows-first Rust desktop workbench for agent workflows, code execution, workspace editing, and embedded office flows.

## Current Status

- Phase 1 product features are implemented across desktop, core, and runner.
- Local persistence, workflow orchestration, Codex-style workbench shell, files, runs, office callbacks, and diagnostics are in place.
- Windows release packaging now includes:
  - release preflight for `fxc.exe`, Rust target, and optional WiX tools
  - complete MSI payload staging
  - per-user WiX installer authoring
  - bundled runtime bootstrap for sidecar and OnlyOffice
  - app update feed verification primitives wired into the desktop layer

What is still external to this repo:

- actual bundled `Node 22` runtime payload
- actual `sidecar.js` source of truth unless you add a canonical build output into this repo
- actual bundled `OnlyOffice Document Server` runtime payload
- production update feed and signing key material

Those assets are expected at installer build time and are staged into the payload by `scripts/windows/build-installer.ps1`.

## Repository Layout

- `apps/workdesk-desktop`: GPUI desktop shell, single-instance command bus, runtime bootstrap, supervisors
- `crates/workdesk-core`: HTTP API, SQLite persistence, updater manifest verification, filesystem and office endpoints
- `crates/workdesk-runner`: run queue daemon, code execution isolation, toolchain manager
- `crates/workdesk-domain`: shared domain models and contracts
- `deploy/windows`: installer support assets, update feed placeholders, toolchain manifest template
- `scripts/windows`: release preflight, installer build, WiX authoring, toolchain scripts

## Validation

```powershell
cargo test --workspace
python scripts/check_docs_sync.py
openspec validate --changes --strict
openspec validate --specs --strict
```

## Local Run

```powershell
$env:WORKDESK_CORE_BIND="127.0.0.1:4000"
$env:WORKDESK_WORKSPACE_ROOT="C:\path\to\workspace"
cargo run -p workdesk-desktop
```

## Windows Installer Build

Release builds require:

- Rust target `x86_64-pc-windows-msvc`
- Windows SDK `fxc.exe`
- WiX Toolset v3 for MSI generation
- bundled runtime sources for sidecar and OnlyOffice

Payload only:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\windows\build-installer.ps1 `
  -ProductVersion 0.1.0 `
  -SidecarBundleDir C:\bundles\sidecar `
  -OnlyOfficeBundleDir C:\bundles\onlyoffice
```

Payload plus MSI:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\windows\build-installer.ps1 `
  -ProductVersion 0.1.0 `
  -SidecarBundleDir C:\bundles\sidecar `
  -OnlyOfficeBundleDir C:\bundles\onlyoffice `
  -BuildMsi
```

## GitHub Actions MSI Build

The repo now includes manual GitHub Actions workflows:

- `.github/workflows/prepare-sidecar-bundle.yml`
- `.github/workflows/prepare-onlyoffice-bundle.yml`
- `.github/workflows/build-msi.yml`

The intended order is:

1. run `prepare-sidecar-bundle`
2. run `prepare-onlyoffice-bundle`
3. run `build-msi`

Release tag conventions:

- sidecar bundles default to `bundles/sidecar/<bundle_version>`
- OnlyOffice bundles default to `bundles/onlyoffice/<bundle_version>`

Bundle contract:

- sidecar asset must unpack to a directory containing:
  - `node/node.exe`
  - `sidecar.js`
- onlyoffice asset must unpack to a directory containing:
  - `documentserver.exe`

`prepare-sidecar-bundle` inputs:

- `bundle_version`
- optional `node_version`
- optional `release_tag`
- optional `sidecar_script_url`

`prepare-onlyoffice-bundle` inputs:

- `bundle_version`
- optional `source_url`
- optional `release_tag`

`build-msi` inputs:

- `product_version`
- `sidecar_bundle_release_tag`
- `onlyoffice_bundle_release_tag`
- optional `bundle_repository`
- optional `sidecar_bundle_asset_name`
- optional `onlyoffice_bundle_asset_name`

Operational notes:

- `prepare-sidecar-bundle` downloads the official Node 22 Windows x64 zip and pairs it with `sidecar.js`.
- If the repo still does not contain a canonical `sidecar.js` build output, pass `sidecar_script_url`.
- `prepare-onlyoffice-bundle` expects a directory or `.zip` that already contains a runnable `documentserver.exe` runtime tree.
- Do not point `source_url` at the public OnlyOffice web installer unless you have already normalized it into an embedded runtime bundle.
- OnlyOffice distribution remains subject to AGPL and your redistribution obligations.

## Documentation

- English: `docs/en/`
- Traditional Chinese: `docs/zh-TW/`
