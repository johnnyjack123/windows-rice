<#
.SYNOPSIS
  Registers (-Install) or removes (-Uninstall) rice's package identity.
  Run by the installer from <install dir>\identity (src-tauri\windows\hooks.nsh).

.DESCRIPTION
  -Install trusts rice's signing certificate for the current user (Trusted
  People) and registers the signed sparse package with the install folder as
  its external location. -Uninstall removes both again. A failure leaves rice
  working without notifications; it never stops the installer.
#>
param(
  [switch]$Install,
  [switch]$Uninstall
)

$ErrorActionPreference = 'Stop'
$name = 'io.github.johnnyjack123.rice'
$installDir = Split-Path -Parent $PSScriptRoot
$cer = Join-Path $PSScriptRoot 'rice.cer'
$msix = Join-Path $PSScriptRoot 'rice.msix'
$thumbprint = (New-Object System.Security.Cryptography.X509Certificates.X509Certificate2 $cer).Thumbprint

# Updates and repairs start from scratch (the same version cannot be added twice).
Get-AppxPackage -Name $name | ForEach-Object { Remove-AppxPackage -Package $_.PackageFullName }

if ($Uninstall) {
  Get-ChildItem Cert:\CurrentUser\TrustedPeople | Where-Object { $_.Thumbprint -eq $thumbprint } | Remove-Item
  return
}
if ($Install) {
  Import-Certificate -FilePath $cer -CertStoreLocation Cert:\CurrentUser\TrustedPeople | Out-Null
  Add-AppxPackage -Path $msix -ExternalLocation $installDir
}
