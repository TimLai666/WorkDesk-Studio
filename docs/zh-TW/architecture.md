# WorkDesk Studio 架構

## Runtime 拓樸

- `apps/workdesk-desktop`
  - Windows-first 的 GPUI 桌面殼與 CLI 入口整合在同一個 binary。
  - Local mode 會啟動 core API、runner daemon、sidecar supervisor 與 OnlyOffice launcher。
  - Remote mode 維持同一個 workbench shell，但改連外部 core service。
- `crates/workdesk-core`
  - 提供 auth、workflow、proposal、run、skills、memory、filesystem、office 與 workbench session persistence 的 HTTP API。
- `crates/workdesk-runner`
  - claim queued runs、materialize run skill snapshots、執行 DAG nodes，並持久化 run/node lifecycle state。

## Desktop Runtime Bootstrap

- `AppConfig` 現在會解析：
  - `install_root`
  - `bundled_sidecar_dir`
  - `bundled_onlyoffice_dir`
  - `sidecar_script_path`
  - app update feed / key 路徑
- `RuntimeBootstrapper`
  - 會把安裝目錄中的 sidecar bundle 複製到 `%LOCALAPPDATA%\WorkDeskStudio\sidecar\...`
  - 會把安裝目錄中的 OnlyOffice bundle 複製到 `%LOCALAPPDATA%\WorkDeskStudio\onlyoffice\...`
  - 設計成可重跑、可修復、具冪等性

桌面程式會在 supervisors 啟動前先做 bootstrap。這樣安裝版首啟動與修復流程走同一條路，不會把 installer-time 與 runtime-time 的假設拆開。

## Release Packaging

- `scripts/windows/preflight-release.ps1`
  - 檢查 Rust target
  - 解析 `fxc.exe`
  - 需要 MSI 時再檢查 WiX 工具
- `scripts/windows/build-installer.ps1`
  - 建置 `workdesk-desktop`、`workdesk-core`、`workdesk-runner` 的 release binaries
  - 在 `dist/windows/payload` stage 完整 payload
  - 複製 sidecar、OnlyOffice、update assets、toolchain bootstrap 檔案
  - 需要時再跑 WiX 產生 MSI
- `scripts/windows/wix/Product.wxs`
  - 採 per-user 安裝
  - 安裝到 `%LOCALAPPDATA%\Programs\WorkDesk Studio`
  - 建立 Start Menu shortcut 與升級註冊資訊

## App Update 與 Toolchain Update 分離

- App updates
  - `AppUpdateFeed` 與 `AppUpdateManifest` 會驗證 Ed25519 簽章與 package SHA-256
  - `DesktopAppUpdater` 會讀取指定 channel 並準備已驗證的 installer，交給 Windows 安裝流程
- Toolchain updates
  - 維持在 runner 管理的 app-scoped toolchain root
  - rollback 與 app binary replacement 分離

這樣可以避免 app 升級覆蓋 managed toolchains，也避免 toolchain rollback 影響已安裝的桌面 binaries。

## Diagnostics

- `RUNNER_UNAVAILABLE`
  - queued run 長時間未被 claim
- `SIDECAR_UNAVAILABLE`
  - bundled 或 seeded sidecar runtime 缺失或不健康
- `DOCSERVER_UNAVAILABLE`
  - bundled 或 seeded OnlyOffice runtime 缺失或不健康
