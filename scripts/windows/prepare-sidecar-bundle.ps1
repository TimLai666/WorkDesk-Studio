param(
  [Parameter(Mandatory = $true)]
  [string]$BundleVersion,
  [string]$NodeVersion = "22.22.1",
  [string]$ReleaseTag = "",
  [string]$Repository = "",
  [string]$SidecarScriptUrl = "",
  [string]$OutputDir = "",
  [string]$AssetName = "sidecar-bundle.zip",
  [switch]$UploadToRelease
)

$ErrorActionPreference = "Stop"

function Resolve-RepoRoot {
  return (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
}

function Resolve-SidecarScriptSource {
  param(
    [Parameter(Mandatory = $true)]
    [string]$RepoRoot,
    [string]$Candidate
  )

  if ($Candidate) {
    if (Test-Path $Candidate) {
      return @{
        Kind = "file"
        Value = (Resolve-Path $Candidate).Path
      }
    }
    return @{
      Kind = "url"
      Value = $Candidate
    }
  }

  $knownPaths = @(
    (Join-Path $RepoRoot "apps\workdesk-sidecar\dist\sidecar.js"),
    (Join-Path $RepoRoot "apps\workdesk-sidecar\sidecar.js"),
    (Join-Path $RepoRoot "sidecar\dist\sidecar.js"),
    (Join-Path $RepoRoot "sidecar.js")
  )

  foreach ($path in $knownPaths) {
    if (Test-Path $path) {
      return @{
        Kind = "file"
        Value = (Resolve-Path $path).Path
      }
    }
  }

  throw "No canonical sidecar.js source found in the repository. Provide -SidecarScriptUrl with either a local path or an HTTPS URL."
}

function Find-NodeRoot {
  param(
    [Parameter(Mandatory = $true)]
    [string]$ExtractDir
  )

  $candidates = @((Get-Item $ExtractDir)) + @(Get-ChildItem -Path $ExtractDir -Directory -Recurse -Force | Sort-Object FullName)
  foreach ($candidate in $candidates) {
    if (Test-Path (Join-Path $candidate.FullName "node.exe")) {
      return $candidate.FullName
    }
  }
  throw "Downloaded Node runtime does not contain node.exe under $ExtractDir"
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
  $OutputDir = Join-Path $repoRoot "dist\windows\bundles\sidecar\$BundleVersion"
}
if (-not $ReleaseTag) {
  $ReleaseTag = "bundles/sidecar/$BundleVersion"
}

$scriptSource = Resolve-SidecarScriptSource -RepoRoot $repoRoot -Candidate $SidecarScriptUrl
$bundleRoot = Join-Path $OutputDir "bundle"
$archivePath = Join-Path $OutputDir $AssetName
$tempDir = Join-Path ([System.IO.Path]::GetTempPath()) ("workdesk-sidecar-" + [System.Guid]::NewGuid().ToString("N"))
$nodeArchivePath = Join-Path $tempDir "node-runtime.zip"
$nodeExtractDir = Join-Path $tempDir "node-extracted"
$sidecarScriptPath = Join-Path $tempDir "sidecar.js"

if (Test-Path $OutputDir) {
  Remove-Item $OutputDir -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
New-Item -ItemType Directory -Force -Path $tempDir | Out-Null

$nodeUrl = "https://nodejs.org/dist/v$NodeVersion/node-v$NodeVersion-win-x64.zip"
Invoke-WebRequest -Uri $nodeUrl -OutFile $nodeArchivePath
Expand-Archive -Path $nodeArchivePath -DestinationPath $nodeExtractDir -Force
$nodeRoot = Find-NodeRoot -ExtractDir $nodeExtractDir

if ($scriptSource.Kind -eq "file") {
  Copy-Item $scriptSource.Value $sidecarScriptPath -Force
} else {
  Invoke-WebRequest -Uri $scriptSource.Value -OutFile $sidecarScriptPath
}

Copy-DirectoryContents -SourceDir $nodeRoot -DestinationDir (Join-Path $bundleRoot "node")
Copy-Item $sidecarScriptPath (Join-Path $bundleRoot "sidecar.js") -Force

if (-not (Test-Path (Join-Path $bundleRoot "node\node.exe"))) {
  throw "Prepared sidecar bundle is missing node/node.exe"
}
if (-not (Test-Path (Join-Path $bundleRoot "sidecar.js"))) {
  throw "Prepared sidecar bundle is missing sidecar.js"
}

Compress-Archive -Path (Join-Path $bundleRoot "*") -DestinationPath $archivePath -Force
Write-Host "Prepared sidecar bundle at $archivePath"

if ($UploadToRelease) {
  if (-not $Repository) {
    throw "-Repository is required when -UploadToRelease is used."
  }
  & (Join-Path $PSScriptRoot "upload-release-asset.ps1") `
    -Repository $Repository `
    -ReleaseTag $ReleaseTag `
    -AssetPath $archivePath `
    -AssetName $AssetName `
    -ReleaseTitle "WorkDesk Studio sidecar bundle $BundleVersion" `
    -ReleaseNotes "Automated sidecar bundle containing Node $NodeVersion and sidecar.js."
}

Write-Output $archivePath
