param(
  [string]$ToolsRoot = "$env:LOCALAPPDATA\\WorkDeskStudio\\tools",
  [string]$ManifestPath = "$env:LOCALAPPDATA\\WorkDeskStudio\\config\\toolchains.json"
)

$ErrorActionPreference = "Stop"
New-Item -ItemType Directory -Force -Path $ToolsRoot | Out-Null
New-Item -ItemType Directory -Force -Path (Split-Path -Parent $ManifestPath) | Out-Null

function Ensure-ToolchainRecord {
  param(
    [string]$Name,
    [string]$Version = "0.0.0",
    [string]$Source = "manual",
    [string]$Checksum = ""
  )

  $dir = Join-Path $ToolsRoot $Name
  New-Item -ItemType Directory -Force -Path $dir | Out-Null
  $binary = Join-Path $dir "$Name.exe"
  if (-not (Test-Path $binary)) {
    Write-Host "Toolchain binary not found yet: $binary"
  } else {
    Write-Host "Detected managed binary: $binary"
  }
  return @{
    binary = $Name
    version = $Version
    source = $Source
    checksum_sha256 = $Checksum
  }
}

$records = @()
$records += Ensure-ToolchainRecord -Name "codex"
$records += Ensure-ToolchainRecord -Name "uv"
$records += Ensure-ToolchainRecord -Name "bun"
$records += Ensure-ToolchainRecord -Name "go"

$manifest = @{
  records = $records
}
$manifest | ConvertTo-Json -Depth 5 | Set-Content -Encoding UTF8 -Path $ManifestPath

Write-Host "Toolchain root prepared at: $ToolsRoot"
Write-Host "Toolchain manifest prepared at: $ManifestPath"
