param(
  [string]$Target = "x86_64-pc-windows-msvc"
)

$ErrorActionPreference = "Stop"

Write-Host "Building WorkDesk Studio desktop binary..."
cargo build -p workdesk-desktop --release --target $Target

Write-Host "Building WorkDesk Core binary..."
cargo build -p workdesk-core --release --target $Target

Write-Host "Preparing installer payload..."
$outDir = Join-Path $PSScriptRoot "..\..\dist\windows"
New-Item -ItemType Directory -Force -Path $outDir | Out-Null

Copy-Item "target\$Target\release\workdesk-desktop.exe" (Join-Path $outDir "workdesk-desktop.exe") -Force
Copy-Item "target\$Target\release\workdesk-core.exe" (Join-Path $outDir "workdesk-core.exe") -Force

Write-Host "Installer payload prepared at $outDir"
Write-Host "Next: package with WiX/NSIS in CI pipeline."
