# 快速開始

## 前置需求

- Rust toolchain
- 第一階段桌面使用建議在 Windows 上執行
- Windows SDK，且必須能取得 `fxc.exe`
- 若要產生 MSI，需安裝 WiX Toolset v3
- 若要做 release packaging，需準備 sidecar 與 OnlyOffice 的 bundled runtime 來源

## 驗證

```powershell
cargo test --workspace
python scripts/check_docs_sync.py
openspec validate --changes --strict
openspec validate --specs --strict
```

## 以 Local Mode 啟動 Desktop

```powershell
$env:WORKDESK_CORE_BIND="127.0.0.1:4000"
$env:WORKDESK_WORKSPACE_ROOT="C:\path\to\workspace"
cargo run -p workdesk-desktop
```

Local mode 會：

- 取得 single-instance lock
- 啟動 core API 與 runner daemon
- 若安裝目錄有 bundled runtime，先 seed sidecar 與 OnlyOffice
- 啟動 sidecar 與 OnlyOffice supervisors
- 開啟 GPUI workbench shell

常用 override：

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

## 以 Remote Mode 啟動 Desktop

```powershell
$env:WORKDESK_REMOTE_URL="http://127.0.0.1:4000"
cargo run -p workdesk-desktop -- --remote
```

## 將命令轉送到既有主視窗

```powershell
cargo run -p workdesk-desktop -- open
cargo run -p workdesk-desktop -- open-run --run-id run-123
cargo run -p workdesk-desktop -- open-workflow --workflow-id wf-123
cargo run -p workdesk-desktop -- run-workflow --workflow-id wf-123
```

## 建立 Windows Payload

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\windows\build-installer.ps1 `
  -ProductVersion 0.1.0 `
  -SidecarBundleDir C:\bundles\sidecar `
  -OnlyOfficeBundleDir C:\bundles\onlyoffice
```

這個 script 會：

- 執行 release preflight
- 解析 `fxc.exe`
- 建置 release binaries
- 在 `dist/windows/payload` stage installer 所需資源

## 建立 MSI

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\windows\build-installer.ps1 `
  -ProductVersion 0.1.0 `
  -SidecarBundleDir C:\bundles\sidecar `
  -OnlyOfficeBundleDir C:\bundles\onlyoffice `
  -BuildMsi
```

額外需要的檔案：

- `deploy/windows/updates/app-update-feed.json`
- `deploy/windows/updates/app-update-public-key.txt`
- `deploy/windows/toolchains/toolchains.json`

Bundle 結構需求：

- sidecar bundle 解壓後必須包含：
  - `node/node.exe`
  - `sidecar.js`
- OnlyOffice bundle 解壓後必須包含：
  - `documentserver.exe`

## 在 GitHub Actions 建立 MSI

依序使用這三個手動 workflow：

- `.github/workflows/prepare-sidecar-bundle.yml`
- `.github/workflows/prepare-onlyoffice-bundle.yml`
- `.github/workflows/build-msi.yml`

預設 release tag 規則：

- sidecar: `bundles/sidecar/<bundle_version>`
- onlyoffice: `bundles/onlyoffice/<bundle_version>`

`prepare-sidecar-bundle` 輸入：

- `bundle_version`
- 可選 `node_version`
  - 預設為 `22.22.1`
- 可選 `release_tag`
- 可選 `sidecar_script_url`
  - 在 repo 還沒有 canonical `sidecar.js` build output 前，實務上需要提供

`prepare-onlyoffice-bundle` 輸入：

- `bundle_version`
- 可選 `source_url`
  - 可填本機路徑或 HTTPS URL，來源必須是已包含可執行 `documentserver.exe` runtime tree 的資料夾或 `.zip`
- 可選 `release_tag`

必要輸入：

- `product_version`
- `sidecar_bundle_release_tag`
- `onlyoffice_bundle_release_tag`

可選輸入：

- `bundle_repository`
  - 預設為目前 repo
- `sidecar_bundle_asset_name`
  - 預設為 `sidecar-bundle.zip`
- `onlyoffice_bundle_asset_name`
  - 預設為 `onlyoffice-bundle.zip`

workflow 會：

- 安裝 WiX Toolset v3
- 安裝 OpenSpec CLI
- 執行 release preflight 與 regression checks
- 從 GitHub Releases 下載並 normalize sidecar / OnlyOffice bundles
- 建立 `dist/windows/WorkDeskStudio-<version>.msi`
- 上傳 MSI 與 payload 作為 workflow artifacts

來源說明：

- `prepare-sidecar-bundle` 會下載官方 Node 22 Windows x64 runtime zip，並搭配 `sidecar.js` 打包。
- `prepare-onlyoffice-bundle` 不會自動把公開 OnlyOffice web installer 轉成可內嵌 runtime。
- 若要符合嚴格內嵌模式，請提供已經 normalize 完成、且內含 `documentserver.exe` 的 runtime 目錄或 zip。
- 重新散佈 OnlyOffice 資產前，請先確認 AGPL 與相應開源義務。

## Diagnostics

- `RUNNER_UNAVAILABLE`
- `SIDECAR_UNAVAILABLE`
- `DOCSERVER_UNAVAILABLE`
