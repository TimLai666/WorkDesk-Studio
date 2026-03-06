param(
  [string]$ToolsRoot = "$env:LOCALAPPDATA\\WorkDeskStudio\\tools"
)

$ErrorActionPreference = "Stop"
New-Item -ItemType Directory -Force -Path $ToolsRoot | Out-Null

function Install-ToolchainPlaceholder {
  param(
    [string]$Name
  )

  $dir = Join-Path $ToolsRoot $Name
  New-Item -ItemType Directory -Force -Path $dir | Out-Null
  Write-Host "Prepared toolchain directory: $dir"
}

Install-ToolchainPlaceholder -Name "codex"
Install-ToolchainPlaceholder -Name "uv"
Install-ToolchainPlaceholder -Name "bun"
Install-ToolchainPlaceholder -Name "go"

Write-Host "Toolchain root prepared at: $ToolsRoot"
