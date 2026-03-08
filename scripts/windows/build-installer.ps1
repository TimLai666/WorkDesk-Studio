param(
  [string]$Target = "x86_64-pc-windows-msvc",
  [string]$ProductVersion = "0.1.0",
  [switch]$BuildMsi,
  [string]$SidecarBundleDir = "",
  [string]$OnlyOfficeBundleDir = "",
  [string]$UpdateFeedPath = "",
  [string]$UpdatePublicKeyPath = "",
  [string]$ToolchainManifestTemplate = ""
)

$ErrorActionPreference = "Stop"

function Resolve-ExistingPath {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Candidate,
    [Parameter(Mandatory = $true)]
    [string]$Description
  )

  if (-not $Candidate) {
    throw "$Description was not provided."
  }
  if (-not (Test-Path $Candidate)) {
    throw "$Description not found: $Candidate"
  }
  return (Resolve-Path $Candidate).Path
}

function Copy-DirectoryContents {
  param(
    [Parameter(Mandatory = $true)]
    [string]$SourceDir,
    [Parameter(Mandatory = $true)]
    [string]$DestinationDir
  )

  New-Item -ItemType Directory -Force -Path $DestinationDir | Out-Null
  Copy-Item (Join-Path $SourceDir "*") $DestinationDir -Recurse -Force
}

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$distRoot = Join-Path $repoRoot "dist\windows"
$payloadDir = Join-Path $distRoot "payload"
$resourcesDir = Join-Path $payloadDir "resources"
$updatesDir = Join-Path $resourcesDir "updates"
$scriptsDir = Join-Path $resourcesDir "scripts"
$toolchainsDir = Join-Path $resourcesDir "toolchains"

if (-not $SidecarBundleDir) {
  if ($env:WORKDESK_SIDECAR_BUNDLE_DIR) {
    $SidecarBundleDir = $env:WORKDESK_SIDECAR_BUNDLE_DIR
  } else {
    $SidecarBundleDir = Join-Path $repoRoot "vendor\sidecar"
  }
}
if (-not $OnlyOfficeBundleDir) {
  if ($env:WORKDESK_ONLYOFFICE_BUNDLE_DIR) {
    $OnlyOfficeBundleDir = $env:WORKDESK_ONLYOFFICE_BUNDLE_DIR
  } else {
    $OnlyOfficeBundleDir = Join-Path $repoRoot "vendor\onlyoffice"
  }
}
if (-not $UpdateFeedPath) {
  $UpdateFeedPath = Join-Path $repoRoot "deploy\windows\updates\app-update-feed.json"
}
if (-not $UpdatePublicKeyPath) {
  $UpdatePublicKeyPath = Join-Path $repoRoot "deploy\windows\updates\app-update-public-key.txt"
}
if (-not $ToolchainManifestTemplate) {
  $ToolchainManifestTemplate = Join-Path $repoRoot "deploy\windows\toolchains\toolchains.json"
}

$toolPaths = & (Join-Path $PSScriptRoot "preflight-release.ps1") -Target $Target -RequireWix:$BuildMsi
$pathEntries = @(
  (Split-Path -Parent $toolPaths.fxc)
)
if ($BuildMsi) {
  $pathEntries += (Split-Path -Parent $toolPaths.candle)
  $pathEntries += (Split-Path -Parent $toolPaths.light)
}
$env:PATH = (($pathEntries + @($env:PATH.Split(';'))) | Select-Object -Unique) -join ';'

$resolvedSidecarBundle = Resolve-ExistingPath -Candidate $SidecarBundleDir -Description "Sidecar bundle directory"
$resolvedOnlyOfficeBundle = Resolve-ExistingPath -Candidate $OnlyOfficeBundleDir -Description "OnlyOffice bundle directory"
$resolvedUpdateFeed = Resolve-ExistingPath -Candidate $UpdateFeedPath -Description "App update feed file"
$resolvedUpdatePublicKey = Resolve-ExistingPath -Candidate $UpdatePublicKeyPath -Description "App update public key file"
$resolvedToolchainManifest = Resolve-ExistingPath -Candidate $ToolchainManifestTemplate -Description "Toolchain manifest template"

Write-Host "Building WorkDesk Studio desktop binary..."
cargo build -p workdesk-desktop --release --target $Target

Write-Host "Building WorkDesk Core binary..."
cargo build -p workdesk-core --release --target $Target

Write-Host "Building WorkDesk Runner binary..."
cargo build -p workdesk-runner --release --target $Target

Write-Host "Preparing installer payload..."
if (Test-Path $payloadDir) {
  Remove-Item $payloadDir -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $payloadDir | Out-Null
New-Item -ItemType Directory -Force -Path $resourcesDir | Out-Null
New-Item -ItemType Directory -Force -Path $updatesDir | Out-Null
New-Item -ItemType Directory -Force -Path $scriptsDir | Out-Null
New-Item -ItemType Directory -Force -Path $toolchainsDir | Out-Null

Copy-Item "target\$Target\release\workdesk-desktop.exe" (Join-Path $payloadDir "workdesk-desktop.exe") -Force
Copy-Item "target\$Target\release\workdesk-core.exe" (Join-Path $payloadDir "workdesk-core.exe") -Force
Copy-Item "target\$Target\release\workdesk-runner.exe" (Join-Path $payloadDir "workdesk-runner.exe") -Force

Copy-DirectoryContents -SourceDir $resolvedSidecarBundle -DestinationDir (Join-Path $resourcesDir "sidecar")
Copy-DirectoryContents -SourceDir $resolvedOnlyOfficeBundle -DestinationDir (Join-Path $resourcesDir "onlyoffice")
Copy-Item $resolvedUpdateFeed (Join-Path $updatesDir "app-update-feed.json") -Force
Copy-Item $resolvedUpdatePublicKey (Join-Path $updatesDir "app-update-public-key.txt") -Force
Copy-Item $resolvedToolchainManifest (Join-Path $toolchainsDir "toolchains.json") -Force
Copy-Item (Join-Path $PSScriptRoot "install-toolchains.ps1") (Join-Path $scriptsDir "install-toolchains.ps1") -Force
Copy-Item (Join-Path $PSScriptRoot "update-toolchains.ps1") (Join-Path $scriptsDir "update-toolchains.ps1") -Force

Write-Host "Installer payload prepared at $payloadDir"

if ($BuildMsi) {
  $wixDir = Join-Path $PSScriptRoot "wix"
  $payloadWxs = Join-Path $distRoot "Payload.wxs"
  $productWixObj = Join-Path $distRoot "Product.wixobj"
  $payloadWixObj = Join-Path $distRoot "Payload.wixobj"
  $wixMsi = Join-Path $distRoot "WorkDeskStudio-$ProductVersion.msi"
  & (Join-Path $wixDir "Harvest-Payload.ps1") -PayloadDir $payloadDir -OutputPath $payloadWxs
  candle.exe -dProductVersion=$ProductVersion -out $productWixObj (Join-Path $wixDir "Product.wxs")
  candle.exe -dProductVersion=$ProductVersion -out $payloadWixObj $payloadWxs
  light.exe -o $wixMsi $productWixObj $payloadWixObj
  Write-Host "MSI generated: $wixMsi"
} else {
  Write-Host "Skip MSI build. Re-run with -BuildMsi to generate installer after payload verification."
}
