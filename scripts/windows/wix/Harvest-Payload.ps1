param(
  [Parameter(Mandatory = $true)]
  [string]$PayloadDir,
  [Parameter(Mandatory = $true)]
  [string]$OutputPath
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path $PayloadDir)) {
  throw "Payload directory not found: $PayloadDir"
}

function New-StableHash {
  param([string]$Value)

  $sha1 = [System.Security.Cryptography.SHA1]::Create()
  try {
    $bytes = [System.Text.Encoding]::UTF8.GetBytes($Value)
    $hash = $sha1.ComputeHash($bytes)
    return ([System.BitConverter]::ToString($hash)).Replace("-", "").Substring(0, 10)
  } finally {
    $sha1.Dispose()
  }
}

function New-WixId {
  param(
    [string]$Prefix,
    [string]$Value
  )

  $leaf = ($Value -replace "[^A-Za-z0-9_]", "_").Trim("_")
  if ([string]::IsNullOrWhiteSpace($leaf)) {
    $leaf = "item"
  }
  if ($leaf.Length -gt 24) {
    $leaf = $leaf.Substring(0, 24)
  }
  "$Prefix$leaf" + "_" + (New-StableHash $Value)
}

function Write-FileComponent {
  param(
    [System.IO.FileInfo]$File,
    [string]$RelativePath,
    [int]$IndentLevel,
    [ref]$XmlLines,
    [ref]$ComponentRefs
  )

  $componentId = New-WixId -Prefix "Cmp_" -Value $RelativePath
  $fileId = New-WixId -Prefix "Fil_" -Value $RelativePath
  $sourcePath = [System.Security.SecurityElement]::Escape($File.FullName)
  $indent = ("  " * $IndentLevel)
  $XmlLines.Value.Add("$indent<Component Id=`"$componentId`" Guid=`"*`">")
  $XmlLines.Value.Add("$indent  <File Id=`"$fileId`" Source=`"$sourcePath`" KeyPath=`"yes`" />")
  $XmlLines.Value.Add("$indent</Component>")
  $ComponentRefs.Value.Add("      <ComponentRef Id=`"$componentId`" />")
}

function Write-DirectoryTree {
  param(
    [string]$CurrentPath,
    [string]$RelativePath,
    [int]$IndentLevel,
    [ref]$XmlLines,
    [ref]$ComponentRefs
  )

  $directories = Get-ChildItem -Path $CurrentPath -Directory | Sort-Object Name
  foreach ($directory in $directories) {
    $childRelative = if ([string]::IsNullOrEmpty($RelativePath)) {
      $directory.Name
    } else {
      Join-Path $RelativePath $directory.Name
    }
    $directoryId = New-WixId -Prefix "Dir_" -Value $childRelative
    $indent = ("  " * $IndentLevel)
    $directoryName = [System.Security.SecurityElement]::Escape($directory.Name)
    $XmlLines.Value.Add("$indent<Directory Id=`"$directoryId`" Name=`"$directoryName`">")
    Write-DirectoryTree -CurrentPath $directory.FullName -RelativePath $childRelative -IndentLevel ($IndentLevel + 1) -XmlLines $XmlLines -ComponentRefs $ComponentRefs
    $XmlLines.Value.Add("$indent</Directory>")
  }

  $files = Get-ChildItem -Path $CurrentPath -File | Sort-Object Name
  foreach ($file in $files) {
    $fileRelative = if ([string]::IsNullOrEmpty($RelativePath)) {
      $file.Name
    } else {
      Join-Path $RelativePath $file.Name
    }
    Write-FileComponent -File $file -RelativePath $fileRelative -IndentLevel $IndentLevel -XmlLines $XmlLines -ComponentRefs $ComponentRefs
  }
}

$xmlLines = New-Object System.Collections.Generic.List[string]
$componentRefs = New-Object System.Collections.Generic.List[string]

$xmlLines.Add('<?xml version="1.0" encoding="utf-8"?>')
$xmlLines.Add('<Wix xmlns="http://schemas.microsoft.com/wix/2006/wi">')
$xmlLines.Add('  <Fragment>')
$xmlLines.Add('    <DirectoryRef Id="INSTALLDIR">')
Write-DirectoryTree -CurrentPath (Resolve-Path $PayloadDir) -RelativePath "" -IndentLevel 3 -XmlLines ([ref]$xmlLines) -ComponentRefs ([ref]$componentRefs)
$xmlLines.Add('    </DirectoryRef>')
$xmlLines.Add('  </Fragment>')
$xmlLines.Add('  <Fragment>')
$xmlLines.Add('    <ComponentGroup Id="PayloadComponents">')
$componentRefs | ForEach-Object { $xmlLines.Add($_) }
$xmlLines.Add('    </ComponentGroup>')
$xmlLines.Add('  </Fragment>')
$xmlLines.Add('</Wix>')

$outputDir = Split-Path -Parent $OutputPath
if ($outputDir) {
  New-Item -ItemType Directory -Force -Path $outputDir | Out-Null
}
$xmlLines | Set-Content -Encoding UTF8 -Path $OutputPath
Write-Host "Generated WiX payload fragment: $OutputPath"
