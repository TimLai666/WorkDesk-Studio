# WorkDesk Studio Architecture

## Runtime Topology

- `apps/workdesk-desktop`
  - Windows-first GPUI shell and CLI entrypoint in one binary.
  - Local mode starts the core API, runner daemon, sidecar supervisor, and OnlyOffice launcher.
  - Remote mode keeps the same shell while targeting an external core service.
- `crates/workdesk-core`
  - HTTP API for auth, workflows, proposals, runs, skills, memory, filesystem, office flows, and workbench session persistence.
- `crates/workdesk-runner`
  - Claims queued runs, materializes run skill snapshots, executes DAG nodes, and records run/node lifecycle state.

## Desktop Runtime Bootstrap

- `AppConfig` now resolves:
  - `install_root`
  - `bundled_sidecar_dir`
  - `bundled_onlyoffice_dir`
  - `sidecar_script_path`
  - app update feed/key locations
- `RuntimeBootstrapper`
  - Seeds bundled sidecar assets from the install directory into `%LOCALAPPDATA%\WorkDeskStudio\sidecar\...`
  - Seeds bundled OnlyOffice assets from the install directory into `%LOCALAPPDATA%\WorkDeskStudio\onlyoffice\...`
  - Is idempotent and safe to re-run on startup

The desktop process performs bootstrap before supervisors start. This keeps release installs and first-run recovery on one path instead of splitting behavior between installer-time and runtime-only assumptions.

## Release Packaging

- `scripts/windows/preflight-release.ps1`
  - Checks Rust target availability
  - Resolves `fxc.exe`
  - Resolves WiX tools when MSI authoring is requested
- `scripts/windows/build-installer.ps1`
  - Builds `workdesk-desktop`, `workdesk-core`, and `workdesk-runner` in release mode
  - Stages a full payload under `dist/windows/payload`
  - Copies bundled sidecar, OnlyOffice, update assets, and toolchain bootstrap files
  - Optionally runs WiX authoring to produce the MSI
- `scripts/windows/wix/Product.wxs`
  - Uses per-user installation
  - Installs under `%LOCALAPPDATA%\Programs\WorkDesk Studio`
  - Adds Start Menu shortcut and upgrade registration

## Update and Toolchain Separation

- App updates
  - `AppUpdateFeed` and `AppUpdateManifest` verify Ed25519 signatures and package SHA-256
  - `DesktopAppUpdater` loads the selected channel and prepares verified installers for handoff to Windows install flow
- Toolchain updates
  - Stay under the runner-managed app-scoped toolchain root
  - Keep rollback separate from app binary replacement

This separation avoids app upgrades overwriting managed toolchains and avoids toolchain rollbacks affecting the installed desktop binaries.

## Diagnostics

- `RUNNER_UNAVAILABLE`
  - A queued run is not claimed in time
- `SIDECAR_UNAVAILABLE`
  - Bundled or seeded sidecar runtime is missing or unhealthy
- `DOCSERVER_UNAVAILABLE`
  - Bundled or seeded OnlyOffice runtime is missing or unhealthy
