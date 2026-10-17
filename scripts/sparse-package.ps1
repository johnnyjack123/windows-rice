<#
.SYNOPSIS
  Registers (or removes) rice's sparse package for development, so rice.exe
  runs with package identity and may read notifications (docs/BACKEND.md).

.DESCRIPTION
  Needs Developer Mode (Settings > System > For developers). Registers
  src-tauri\package unsigned, with target\debug (or target\release) as the
  external location where rice.exe lives. Nothing is copied or installed.
  Releases use a signed package registered by the installer instead.

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts\sparse-package.ps1
  powershell -ExecutionPolicy Bypass -File scripts\sparse-package.ps1 -Release
  powershell -ExecutionPolicy Bypass -File scripts\sparse-package.ps1 -Remove
#>
param(
  [switch]$Release,
  [switch]$Remove
)

$ErrorActionPreference = 'Stop'
$name = 'io.github.johnnyjack123.rice'
$root = Split-Path -Parent $PSScriptRoot
$manifest = Join-Path $root 'src-tauri\package\AppxManifest.xml'
$exeDir = Join-Path $root ('target\' + $(if ($Release) { 'release' } else { 'debug' }))

$existing = Get-AppxPackage -Name $name
if ($existing) {
  Write-Host "Removing $($existing.PackageFullName)"
  Remove-AppxPackage -Package $existing.PackageFullName
}
if ($Remove) {
  Write-Host 'Done: rice runs without package identity again.'
  return
}

$unlock = Get-ItemProperty -Path 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\AppModelUnlock' -ErrorAction SilentlyContinue
if ($null -eq $unlock -or $unlock.AllowDevelopmentWithoutDevLicense -ne 1) {
  throw 'Developer Mode is off. Turn it on in Settings > System > For developers, then run this again.'
}
if (-not (Test-Path (Join-Path $exeDir 'rice.exe'))) {
  throw "There is no rice.exe in $exeDir yet. Build it first (cargo build -p rice$(if ($Release) { ' --release' }))."
}

Add-AppxPackage -Register $manifest -ExternalLocation $exeDir
Write-Host "Registered $name for $exeDir."
Write-Host 'Start rice from there as usual; it now runs with package identity.'
