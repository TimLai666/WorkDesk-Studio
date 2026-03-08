# Quickstart

## Prerequisites

- Rust toolchain
- Windows recommended for Phase 1 desktop usage
- Windows SDK with `fxc.exe`
- WiX Toolset v3 if you want to build MSI packages
- Bundled sidecar and OnlyOffice runtime sources for release packaging

## Validation

```powershell
cargo test --workspace
python scripts/check_docs_sync.py
openspec validate --changes --strict
openspec validate --specs --strict
```

## Start Desktop in Local Mode

```powershell
$env:WORKDESK_CORE_BIND="127.0.0.1:4000"
$env:WORKDESK_WORKSPACE_ROOT="C:\path\to\workspace"
cargo run -p workdesk-desktop
```

Local mode will:

- acquire the single-instance lock
- start the core API and runner daemon
- seed bundled sidecar and OnlyOffice runtimes when available
- start sidecar and OnlyOffice supervisors
- open the GPUI workbench shell

Useful overrides:

```powershell
$env:WORKDESK_DB_PATH="C:\custom\workdesk.db"
$env:WORKDESK_INSTALL_ROOT="C:\Users\you\AppData\Local\Programs\WorkDesk Studio"
$env:WORKDESK_SIDECAR_PATH="$env:LOCALAPPDATA\\WorkDeskStudio\\sidecar\\node\\node.exe"
$env:WORKDESK_SIDECAR_SCRIPT="$env:LOCALAPPDATA\\WorkDeskStudio\\sidecar\\sidecar.js"
$env:WORKDESK_ONLYOFFICE_BIN="$env:LOCALAPPDATA\\WorkDeskStudio\\onlyoffice\\documentserver\\documentserver.exe"
$env:WORKDESK_BUNDLED_SIDECAR_DIR="C:\bundles\sidecar"
$env:WORKDESK_BUNDLED_ONLYOFFICE_DIR="C:\bundles\onlyoffice"
$env:WORKDESK_APP_UPDATE_FEED="https://updates.example.com/workdesk/stable.json"
```

## Start Desktop in Remote Mode

```powershell
$env:WORKDESK_REMOTE_URL="http://127.0.0.1:4000"
cargo run -p workdesk-desktop -- --remote
```

## Forward Commands to the Primary Window

```powershell
cargo run -p workdesk-desktop -- open
cargo run -p workdesk-desktop -- open-run --run-id run-123
cargo run -p workdesk-desktop -- open-workflow --workflow-id wf-123
cargo run -p workdesk-desktop -- run-workflow --workflow-id wf-123
```

## Build the Windows Payload

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\windows\build-installer.ps1 `
  -ProductVersion 0.1.0 `
  -SidecarBundleDir C:\bundles\sidecar `
  -OnlyOfficeBundleDir C:\bundles\onlyoffice
```

The build script will:

- run release preflight
- resolve `fxc.exe`
- build release binaries
- stage installer resources into `dist/windows/payload`

## Build the MSI

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\windows\build-installer.ps1 `
  -ProductVersion 0.1.0 `
  -SidecarBundleDir C:\bundles\sidecar `
  -OnlyOfficeBundleDir C:\bundles\onlyoffice `
  -BuildMsi
```

Expected extra inputs:

- `deploy/windows/updates/app-update-feed.json`
- `deploy/windows/updates/app-update-public-key.txt`
- `deploy/windows/toolchains/toolchains.json`

Bundle contract:

- sidecar bundle must unpack to a directory containing:
  - `node/node.exe`
  - `sidecar.js`
- OnlyOffice bundle must unpack to a directory containing:
  - `documentserver.exe`

## Build the MSI in GitHub Actions

Use the manual workflows in this order:

- `.github/workflows/prepare-sidecar-bundle.yml`
- `.github/workflows/prepare-onlyoffice-bundle.yml`
- `.github/workflows/build-msi.yml`

Default release tags:

- sidecar: `bundles/sidecar/<bundle_version>`
- onlyoffice: `bundles/onlyoffice/<bundle_version>`

`prepare-sidecar-bundle` inputs:

- `bundle_version`
- optional `node_version`
  - defaults to `22.22.1`
- optional `release_tag`
- optional `sidecar_script_url`
  - required in practice until the repo contains a canonical `sidecar.js` build output

`prepare-onlyoffice-bundle` inputs:

- `bundle_version`
- optional `source_url`
  - use a local path or an HTTPS URL to a `.zip` or directory that contains a runnable `documentserver.exe` runtime tree
- optional `release_tag`

Required workflow inputs:

- `product_version`
- `sidecar_bundle_release_tag`
- `onlyoffice_bundle_release_tag`

Optional workflow inputs:

- `bundle_repository`
  - defaults to the current repository
- `sidecar_bundle_asset_name`
  - defaults to `sidecar-bundle.zip`
- `onlyoffice_bundle_asset_name`
  - defaults to `onlyoffice-bundle.zip`

The workflow will:

- install WiX Toolset v3
- install OpenSpec CLI
- run release preflight and regression checks
- download and normalize sidecar / OnlyOffice bundles from GitHub Releases
- build `dist/windows/WorkDeskStudio-<version>.msi`
- upload the MSI and payload as workflow artifacts

Source guidance:

- `prepare-sidecar-bundle` downloads the official Node 22 Windows x64 runtime zip and pairs it with `sidecar.js`.
- `prepare-onlyoffice-bundle` does not convert the public OnlyOffice web installer into an embedded runtime automatically.
- For strict embedded mode, feed `prepare-onlyoffice-bundle` a pre-normalized runtime directory or zip that already contains `documentserver.exe`.
- Review AGPL obligations before redistributing OnlyOffice assets.

## Diagnostics

- `RUNNER_UNAVAILABLE`
- `SIDECAR_UNAVAILABLE`
- `DOCSERVER_UNAVAILABLE`
