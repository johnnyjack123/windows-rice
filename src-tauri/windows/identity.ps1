<#
.SYNOPSIS
  Registers (-Install) or removes (-Uninstall) rice's package identity.
  Run by the installer from <install dir>\identity (src-tauri\windows\hooks.nsh).

.DESCRIPTION
  -Install trusts rice's signing certificate for package installs and
  registers the signed sparse package with the install folder as its
  external location. Windows' installation service only accepts
  certificates of the machine store (Cert:\LocalMachine\TrustedPeople), so
  that single step runs elevated: one UAC prompt, and only if the
  certificate is not trusted yet. -Uninstall removes package and
  certificate again (UAC once more for the machine store).
  A failure or a declined prompt leaves rice working without
  notifications; it never stops the installer.
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
$machineCert = "Cert:\LocalMachine\TrustedPeople\$thumbprint"

# Runs one PowerShell command elevated (UAC) and waits for it.
function Invoke-Elevated([string]$command) {
  $encoded = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($command))
  $shell = Join-Path $PSHOME 'powershell.exe'
  Start-Process -FilePath $shell -Verb RunAs -Wait -WindowStyle Hidden `
    -ArgumentList '-NoLogo', '-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass', '-EncodedCommand', $encoded
}

# Updates and repairs start from scratch (the same version cannot be added twice).
Get-AppxPackage -Name $name | ForEach-Object { Remove-AppxPackage -Package $_.PackageFullName }
# Earlier installers trusted the certificate for the user only, which Windows ignores.
Get-ChildItem Cert:\CurrentUser\TrustedPeople | Where-Object { $_.Thumbprint -eq $thumbprint } | Remove-Item

if ($Uninstall) {
  if (Test-Path $machineCert) {
    try {
      Invoke-Elevated "Remove-Item -LiteralPath '$machineCert'"
    } catch {
      Write-Warning "The certificate stays trusted (UAC declined): $_"
    }
  }
  return
}

if ($Install) {
  if (-not (Test-Path $machineCert)) {
    $path = $cer.Replace("'", "''")
    try {
      Invoke-Elevated "Import-Certificate -FilePath '$path' -CertStoreLocation Cert:\LocalMachine\TrustedPeople | Out-Null"
    } catch {
      throw "The certificate was not trusted (UAC declined?): $_ - rice runs without notifications."
    }
  }
  if (-not (Test-Path $machineCert)) {
    throw 'The certificate was not trusted; rice runs without notifications.'
  }
  Add-AppxPackage -Path $msix -ExternalLocation $installDir
}
