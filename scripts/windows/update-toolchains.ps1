param(
  [string]$ToolsRoot = "$env:LOCALAPPDATA\\WorkDeskStudio\\tools",
  [string]$ManifestPath = "$env:LOCALAPPDATA\\WorkDeskStudio\\config\\toolchains.json"
)

$ErrorActionPreference = "Stop"

Write-Host "Updating managed toolchains under $ToolsRoot"
if (-not (Test-Path $ManifestPath)) {
  throw "Manifest not found: $ManifestPath"
}

$manifest = Get-Content -Raw $ManifestPath | ConvertFrom-Json
if (-not $manifest.records) {
  throw "Invalid manifest: missing records[]"
}

foreach ($record in $manifest.records) {
  $binaryDir = Join-Path $ToolsRoot $record.binary
  $binaryPath = Join-Path $binaryDir "$($record.binary).exe"
  $backupPath = "$binaryPath.previous"
  if (Test-Path $binaryPath) {
    if (Test-Path $backupPath) {
      Remove-Item $backupPath -Force
    }
    Move-Item $binaryPath $backupPath
    Write-Host "Staged backup: $backupPath"
    Write-Host "Place new binary at: $binaryPath"
  } else {
    Write-Host "Binary not installed yet: $binaryPath"
  }
}

Write-Host "Update staging complete."
Write-Host "If validation fails, restore .previous file back to .exe to rollback."
