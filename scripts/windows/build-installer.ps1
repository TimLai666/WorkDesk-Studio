param(
  [string]$Target = "x86_64-pc-windows-msvc",
  [string]$ProductVersion = "0.1.0",
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
  $wixDir = Join-Path $PSScriptRoot "wix"
  $payloadWxs = Join-Path $distRoot "Payload.wxs"
  $productWixObj = Join-Path $distRoot "Product.wixobj"
  $payloadWixObj = Join-Path $distRoot "Payload.wixobj"
  $wixMsi = Join-Path $distRoot "WorkDeskStudio.msi"
  & (Join-Path $wixDir "Harvest-Payload.ps1") -PayloadDir $payloadDir -OutputPath $payloadWxs
  if (Get-Command candle.exe -ErrorAction SilentlyContinue) {
    candle.exe -dProductVersion=$ProductVersion -out $productWixObj (Join-Path $wixDir "Product.wxs")
    candle.exe -dProductVersion=$ProductVersion -out $payloadWixObj $payloadWxs
    light.exe -o $wixMsi $productWixObj $payloadWixObj
    Write-Host "MSI generated: $wixMsi"
  } else {
    Write-Warning "WiX not found (candle.exe/light.exe). Install WiX Toolset to build MSI."
  }
} else {
  Write-Host "Skip MSI build. Re-run with -BuildMsi after WiX authoring is ready."
}
