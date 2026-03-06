# WorkDesk Studio Wave 1 Resume Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Restore the interrupted Wave 1 desktop work and finish the remaining desktop runtime pieces needed for a usable Phase-1 local product.

**Architecture:** Keep the existing split between `workdesk-core`, `workdesk-runner`, and `workdesk-desktop`, but finish the desktop layer by adding UI state for canvas/files/office and local supervisors for sidecar and embedded document services. Reuse existing envelope APIs and diagnostics plumbing instead of adding parallel code paths.

**Tech Stack:** Rust, GPUI, gpui-component, Tokio, sqlx SQLite, named pipes/socket fallback.

---

### Task 1: Restore Desktop Build Baseline

**Files:**
- Modify: `apps/workdesk-desktop/src/controller.rs`
- Modify: `apps/workdesk-desktop/src/ui.rs`
- Modify: `apps/workdesk-desktop/src/api_client.rs`
- Modify: `apps/workdesk-desktop/tests/automation_server.rs`
- Modify: `apps/workdesk-desktop/tests/command_bus_forwarding.rs`

**Step 1: Write/refresh failing test signal**

Run: `cargo test --workspace`
Expected: desktop compile/test failures after interrupted edit state.

**Step 2: Restore minimal compile-safe implementation**

- Finish `DesktopApi` extensions and fake test implementations.
- Finish GPUI render helpers for runs, canvas, files, and office/PDF panels.
- Keep canvas/file/office UI functional but narrow in scope until runtime supervisors are wired.

**Step 3: Re-run verification**

Run: `cargo test --workspace`
Expected: full workspace green.

### Task 2: Add Sidecar Supervisor

**Files:**
- Create: `apps/workdesk-desktop/src/sidecar_supervisor.rs`
- Modify: `apps/workdesk-desktop/src/lib.rs`
- Modify: `apps/workdesk-desktop/src/main.rs`
- Test: `apps/workdesk-desktop/src/sidecar_supervisor.rs`

**Step 1: Write failing unit tests**

Cover:
- health check reports unavailable when sidecar binary/script missing
- supervisor publishes `SIDECAR_UNAVAILABLE`
- healthy stub process clears the diagnostic

**Step 2: Implement minimal supervisor**

- Launch configured `node.exe` + `sidecar.js`
- expose periodic health probe and diagnostic callback
- store recent log lines for later UI surfacing

**Step 3: Re-run targeted tests**

Run: `cargo test -p workdesk-desktop sidecar_supervisor -- --nocapture`

### Task 3: Add OnlyOffice Launcher

**Files:**
- Create: `apps/workdesk-desktop/src/onlyoffice.rs`
- Modify: `apps/workdesk-desktop/src/main.rs`
- Modify: `docs/en/architecture.md`
- Modify: `docs/zh-TW/architecture.md`

**Step 1: Write failing tests**

Cover:
- missing package/binary reports `DOCSERVER_UNAVAILABLE`
- launcher health URL success clears diagnostic

**Step 2: Implement launcher**

- compute AppData extraction/runtime paths
- support health-only mode if bundle missing
- feed diagnostics into `DesktopAppController`

**Step 3: Verify**

Run: `cargo test -p workdesk-desktop onlyoffice -- --nocapture`

### Task 4: Sync OpenSpec and Docs

**Files:**
- Modify: `openspec/changes/add-codex-sdk-sidecar-auth-and-session/tasks.md`
- Modify: `openspec/changes/add-embedded-onlyoffice-and-pdf-flow/tasks.md`
- Modify: `docs/en/quickstart.md`
- Modify: `docs/zh-TW/quickstart.md`

**Step 1: Update task checkboxes only for implemented items**

**Step 2: Document local runtime expectations**

- sidecar path
- document server path
- diagnostics behavior

**Step 3: Final verification**

Run: `cargo test --workspace`
Run: `python scripts/check_docs_sync.py`
Run: `openspec validate --changes --strict`
Run: `openspec validate --specs --strict`
