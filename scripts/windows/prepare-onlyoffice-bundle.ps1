param(
  [Parameter(Mandatory = $true)]
  [string]$BundleVersion,
  [string]$ReleaseTag = "",
  [string]$Repository = "",
  [string]$SourceUrl = "",
  [string]$OutputDir = "",
  [string]$AssetName = "onlyoffice-bundle.zip",
  [switch]$UploadToRelease
)

$ErrorActionPreference = "Stop"

function Resolve-RepoRoot {
  return (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
}

function Resolve-OnlyOfficeSource {
  param(
    [string]$Candidate
  )

  if (-not $Candidate) {
    $Candidate = $env:WORKDESK_ONLYOFFICE_BUNDLE_SOURCE_URL
  }
  if (-not $Candidate) {
    throw "No OnlyOffice bundle source configured. Provide -SourceUrl or WORKDESK_ONLYOFFICE_BUNDLE_SOURCE_URL with either a local directory, a local .zip, or an HTTPS URL to a .zip that contains documentserver.exe."
  }

  if (Test-Path $Candidate) {
    return @{
      Kind = if ((Get-Item $Candidate).PSIsContainer) { "directory" } else { "file" }
      Value = (Resolve-Path $Candidate).Path
    }
  }

  return @{
    Kind = "url"
    Value = $Candidate
  }
}

function Resolve-BundleRoot {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Root
  )

  $candidates = @((Get-Item $Root)) + @(Get-ChildItem -Path $Root -Directory -Recurse -Force | Sort-Object FullName)
  foreach ($candidate in $candidates) {
    if (Test-Path (Join-Path $candidate.FullName "documentserver.exe")) {
      return $candidate.FullName
    }
  }
  throw "OnlyOffice bundle does not contain documentserver.exe under $Root"
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

$repoRoot = Resolve-RepoRoot
if (-not $OutputDir) {
  $OutputDir = Join-Path $repoRoot "dist\windows\bundles\onlyoffice\$BundleVersion"
}
if (-not $ReleaseTag) {
  $ReleaseTag = "bundles/onlyoffice/$BundleVersion"
}

$resolvedSource = Resolve-OnlyOfficeSource -Candidate $SourceUrl
$bundleRoot = Join-Path $OutputDir "bundle"
$archivePath = Join-Path $OutputDir $AssetName
$tempDir = Join-Path ([System.IO.Path]::GetTempPath()) ("workdesk-onlyoffice-" + [System.Guid]::NewGuid().ToString("N"))
$extractDir = Join-Path $tempDir "onlyoffice-extracted"

if (Test-Path $OutputDir) {
  Remove-Item $OutputDir -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
New-Item -ItemType Directory -Force -Path $tempDir | Out-Null

if ($resolvedSource.Kind -eq "directory") {
  $sourceRoot = Resolve-BundleRoot -Root $resolvedSource.Value
} else {
  $archiveInput = if ($resolvedSource.Kind -eq "file") {
    $resolvedSource.Value
  } else {
    $downloadPath = Join-Path $tempDir "onlyoffice-source.zip"
    Invoke-WebRequest -Uri $resolvedSource.Value -OutFile $downloadPath
    $downloadPath
  }

  if ([System.IO.Path]::GetExtension($archiveInput) -ne ".zip") {
    throw "OnlyOffice bundle source must be a directory or a .zip archive."
  }

  Expand-Archive -Path $archiveInput -DestinationPath $extractDir -Force
  $sourceRoot = Resolve-BundleRoot -Root $extractDir
}

Copy-DirectoryContents -SourceDir $sourceRoot -DestinationDir $bundleRoot

if (-not (Test-Path (Join-Path $bundleRoot "documentserver.exe"))) {
  throw "Prepared OnlyOffice bundle is missing documentserver.exe"
}

Compress-Archive -Path (Join-Path $bundleRoot "*") -DestinationPath $archivePath -Force
Write-Host "Prepared OnlyOffice bundle at $archivePath"

if ($UploadToRelease) {
  if (-not $Repository) {
    throw "-Repository is required when -UploadToRelease is used."
  }
  & (Join-Path $PSScriptRoot "upload-release-asset.ps1") `
    -Repository $Repository `
    -ReleaseTag $ReleaseTag `
    -AssetPath $archivePath `
    -AssetName $AssetName `
    -ReleaseTitle "WorkDesk Studio OnlyOffice bundle $BundleVersion" `
    -ReleaseNotes "Automated OnlyOffice bundle for embedded Document Server runtime."
}

Write-Output $archivePath
