param(
  [string]$Target = "x86_64-pc-windows-msvc",
  [switch]$BuildMsi
)

$ErrorActionPreference = "Stop"

Write-Host "Building WorkDesk Studio desktop binary..."
cargo build -p workdesk-desktop --release --target $Target

Write-Host "Building WorkDesk Core binary..."
cargo build -p workdesk-core --release --target $Target

Write-Host "Building WorkDesk Runner binary..."
cargo build -p workdesk-runner --release --target $Target

Write-Host "Preparing installer payload..."
$distRoot = Join-Path $PSScriptRoot "..\..\dist\windows"
$payloadDir = Join-Path $distRoot "payload"
New-Item -ItemType Directory -Force -Path $payloadDir | Out-Null

Copy-Item "target\$Target\release\workdesk-desktop.exe" (Join-Path $payloadDir "workdesk-desktop.exe") -Force
Copy-Item "target\$Target\release\workdesk-core.exe" (Join-Path $payloadDir "workdesk-core.exe") -Force
Copy-Item "target\$Target\release\workdesk-runner.exe" (Join-Path $payloadDir "workdesk-runner.exe") -Force

Write-Host "Installer payload prepared at $payloadDir"

if ($BuildMsi) {
  $wixObj = Join-Path $distRoot "WorkDeskStudio.wixobj"
  $wixMsi = Join-Path $distRoot "WorkDeskStudio.msi"
  if (Get-Command candle.exe -ErrorAction SilentlyContinue) {
    candle.exe -o $wixObj "$PSScriptRoot\\wix\\Product.wxs"
    light.exe -o $wixMsi $wixObj
    Write-Host "MSI generated: $wixMsi"
  } else {
    Write-Warning "WiX not found (candle.exe/light.exe). Install WiX Toolset to build MSI."
  }
} else {
  Write-Host "Skip MSI build. Re-run with -BuildMsi after WiX authoring is ready."
}
