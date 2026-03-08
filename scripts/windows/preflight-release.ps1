param(
  [string]$Target = "x86_64-pc-windows-msvc",
  [switch]$RequireWix
)

$ErrorActionPreference = "Stop"

function Resolve-RequiredCommand {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Name
  )

  $command = Get-Command $Name -ErrorAction SilentlyContinue
  if ($command) {
    return $command.Source
  }

  throw "Required command not found on PATH: $Name"
}

function Resolve-WindowsSdkBinary {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Name
  )

  $command = Get-Command $Name -ErrorAction SilentlyContinue
  if ($command) {
    return $command.Source
  }

  $candidates = @()
  if ($env:WindowsSdkVerBinPath) {
    $candidates += (Join-Path $env:WindowsSdkVerBinPath $Name)
    $candidates += (Join-Path (Join-Path $env:WindowsSdkVerBinPath "x64") $Name)
  }
  if ($env:WindowsSdkBinPath) {
    $candidates += (Join-Path $env:WindowsSdkBinPath $Name)
    $candidates += (Join-Path (Join-Path $env:WindowsSdkBinPath "x64") $Name)
  }
  if (${env:ProgramFiles(x86)}) {
    $kitRoot = Join-Path ${env:ProgramFiles(x86)} "Windows Kits\10\bin"
    if (Test-Path $kitRoot) {
      $candidates += Get-ChildItem -Path $kitRoot -Directory -ErrorAction SilentlyContinue |
        Sort-Object Name -Descending |
        ForEach-Object { Join-Path $_.FullName "x64\$Name" }
    }
  }

  foreach ($candidate in $candidates) {
    if ($candidate -and (Test-Path $candidate)) {
      return (Resolve-Path $candidate).Path
    }
  }

  throw "Required Windows SDK binary not found: $Name"
}

function Test-RustTargetInstalled {
  param(
    [Parameter(Mandatory = $true)]
    [string]$TargetTriple
  )

  $installed = rustup target list --installed
  return ($installed | Where-Object { $_.Trim() -eq $TargetTriple }) -ne $null
}

$null = Resolve-RequiredCommand -Name "cargo.exe"
$null = Resolve-RequiredCommand -Name "rustup.exe"

if (-not (Test-RustTargetInstalled -TargetTriple $Target)) {
  throw "Rust target not installed: $Target. Run 'rustup target add $Target'."
}

$fxcPath = Resolve-WindowsSdkBinary -Name "fxc.exe"
$toolPaths = [ordered]@{
  fxc = $fxcPath
}

if ($RequireWix) {
  $toolPaths["candle"] = Resolve-RequiredCommand -Name "candle.exe"
  $toolPaths["light"] = Resolve-RequiredCommand -Name "light.exe"
}

[pscustomobject]$toolPaths
