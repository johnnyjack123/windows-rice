<#
.SYNOPSIS
  Registers (or removes) the development identity of rice, so a dev build
  (target\debug\rice.exe) runs with package identity and may read
  notifications (docs/BACKEND.md).

.DESCRIPTION
  Needs Developer Mode (Settings > System > For developers). Writes a
  development variant of src-tauri\package (own name "...rice.dev", shown as
  "rice (dev)") to %LOCALAPPDATA%\rice-dev\package and registers it unsigned,
  with target\debug as the external location where rice.exe lives. The
  installed rice keeps its own identity; nothing is installed or copied
  next to rice.exe.

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts\sparse-package.ps1
  powershell -ExecutionPolicy Bypass -File scripts\sparse-package.ps1 -Remove
#>
param(
  [switch]$Remove
)

$ErrorActionPreference = 'Stop'
$release = 'io.github.johnnyjack123.rice'
$name = "$release.dev"
$root = Split-Path -Parent $PSScriptRoot
$source = Join-Path $root 'src-tauri\package'
$stage = Join-Path $env:LOCALAPPDATA 'rice-dev\package'
$exeDir = Join-Path $root 'target\debug'

# The dev identity, plus a development registration under the release name
# from before the two were separated (the installed, signed one stays).
$old = @(Get-AppxPackage -Name $name) + @(Get-AppxPackage -Name $release | Where-Object { $_.IsDevelopmentMode })
foreach ($package in $old) {
  Write-Host "Removing $($package.PackageFullName)"
  Remove-AppxPackage -Package $package.PackageFullName
}
if ($Remove) {
  Remove-Item -Recurse -Force $stage -ErrorAction SilentlyContinue
  Write-Host 'Done: development builds run without package identity again.'
  return
}

$unlock = Get-ItemProperty -Path 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\AppModelUnlock' -ErrorAction SilentlyContinue
if ($null -eq $unlock -or $unlock.AllowDevelopmentWithoutDevLicense -ne 1) {
  throw 'Developer Mode is off. Turn it on in Settings > System > For developers, then run this again.'
}
if (-not (Test-Path (Join-Path $exeDir 'rice.exe'))) {
  throw "There is no rice.exe in $exeDir yet. Build it first (cargo build -p rice)."
}

# Development variant of the package manifest.
New-Item -ItemType Directory -Force (Join-Path $stage 'Assets') | Out-Null
Copy-Item (Join-Path $source 'Assets\*') (Join-Path $stage 'Assets') -Force
[xml]$manifest = Get-Content -Raw -Encoding UTF8 (Join-Path $source 'AppxManifest.xml')
$manifest.DocumentElement.GetElementsByTagName('Identity')[0].SetAttribute('Name', $name)
$manifest.DocumentElement.GetElementsByTagName('DisplayName')[0].InnerText = 'rice (dev)'
$manifest.DocumentElement.GetElementsByTagName('uap:VisualElements')[0].SetAttribute('DisplayName', 'rice (dev)')
$manifest.Save((Join-Path $stage 'AppxManifest.xml'))

Add-AppxPackage -Register (Join-Path $stage 'AppxManifest.xml') -ExternalLocation $exeDir
Write-Host "Registered $name for $exeDir."
Write-Host 'Start rice as usual (pnpm tauri dev); it now runs with package identity.'
