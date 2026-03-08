param(
  [Parameter(Mandatory = $true)]
  [string]$Repository,
  [Parameter(Mandatory = $true)]
  [string]$ReleaseTag,
  [Parameter(Mandatory = $true)]
  [string]$AssetPath,
  [string]$AssetName = "",
  [string]$ReleaseTitle = "",
  [string]$ReleaseNotes = ""
)

$ErrorActionPreference = "Stop"

function Get-ResolvedPath {
  param(
    [Parameter(Mandatory = $true)]
    [string]$PathValue,
    [Parameter(Mandatory = $true)]
    [string]$Description
  )

  if (-not (Test-Path $PathValue)) {
    throw "$Description not found: $PathValue"
  }
  return (Resolve-Path $PathValue).Path
}

function Test-ReleaseExists {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Repo,
    [Parameter(Mandatory = $true)]
    [string]$Tag
  )

  $null = gh release view $Tag --repo $Repo 2>$null
  return $LASTEXITCODE -eq 0
}

$null = Get-Command gh -ErrorAction Stop
$resolvedAssetPath = Get-ResolvedPath -PathValue $AssetPath -Description "Release asset"
$leafName = Split-Path -Leaf $resolvedAssetPath
$targetName = if ($AssetName) { $AssetName } else { $leafName }
$uploadPath = $resolvedAssetPath
$tempDir = $null

if ($leafName -ne $targetName) {
  $tempDir = Join-Path ([System.IO.Path]::GetTempPath()) ([System.Guid]::NewGuid().ToString("N"))
  New-Item -ItemType Directory -Force -Path $tempDir | Out-Null
  $uploadPath = Join-Path $tempDir $targetName
  Copy-Item $resolvedAssetPath $uploadPath -Force
}

try {
  if (-not (Test-ReleaseExists -Repo $Repository -Tag $ReleaseTag)) {
    $title = if ($ReleaseTitle) { $ReleaseTitle } else { $ReleaseTag }
    $notes = if ($ReleaseNotes) { $ReleaseNotes } else { "Automated WorkDesk Studio release asset." }
    gh release create $ReleaseTag $uploadPath --repo $Repository --title $title --notes $notes | Out-Null
    return
  }

  gh release delete-asset $ReleaseTag $targetName --repo $Repository --yes 2>$null | Out-Null
  gh release upload $ReleaseTag $uploadPath --repo $Repository --clobber | Out-Null
} finally {
  if ($tempDir -and (Test-Path $tempDir)) {
    Remove-Item $tempDir -Recurse -Force
  }
}
