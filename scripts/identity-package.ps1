<#
.SYNOPSIS
  Builds the signed identity package for the installer (`pnpm release`
  runs this before compiling).

.DESCRIPTION
  Packs src-tauri\package with the app version from tauri.conf.json, signs it
  and exports the public part of the certificate:
  target\identity\rice.msix and target\identity\rice.cer.

  The signing certificate ("CN=rice", self-signed) is created once in your
  personal certificate store (Cert:\CurrentUser\My) with a private key that
  cannot be exported; it never lands in a file or in the repository.
  Creating it trusts nothing: only the installer adds the public part to
  "Trusted People" of the user who installs rice.

  Needs the Windows SDK (MakeAppx, SignTool).
#>
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$out = Join-Path $root 'target\identity'
$stage = Join-Path $out 'stage'
$subject = 'CN=rice'
$friendlyName = 'rice package signing'

# Newest Windows SDK tool for this machine's architecture.
function Find-SdkTool([string]$name) {
  $arch = if ($env:PROCESSOR_ARCHITECTURE -eq 'ARM64') { 'arm64' } else { 'x64' }
  $kits = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
  $tool = Get-ChildItem $kits -Directory -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -match '^\d+(\.\d+){3}$' } |
    Sort-Object { [version]$_.Name } -Descending |
    ForEach-Object { Join-Path $_.FullName "$arch\$name" } |
    Where-Object { Test-Path $_ } |
    Select-Object -First 1
  if (-not $tool) {
    throw "$name not found. Install the Windows SDK (Visual Studio Installer > Individual components > Windows 11 SDK)."
  }
  return $tool
}
$makeAppx = Find-SdkTool 'makeappx.exe'
$signTool = Find-SdkTool 'signtool.exe'

# Package version = app version plus a fourth part (MSIX wants four).
$conf = Get-Content -Raw -Encoding UTF8 (Join-Path $root 'src-tauri\tauri.conf.json') | ConvertFrom-Json
$version = "$($conf.version).0"

$cert = Get-ChildItem Cert:\CurrentUser\My |
  Where-Object { $_.Subject -eq $subject -and $_.FriendlyName -eq $friendlyName -and $_.NotAfter -gt (Get-Date).AddDays(30) } |
  Sort-Object NotAfter -Descending |
  Select-Object -First 1
if (-not $cert) {
  # Code signing (EKU 1.3.6.1.5.5.7.3.3), no CA (basic constraints empty), as MSIX requires.
  $cert = New-SelfSignedCertificate -Type Custom -Subject $subject -FriendlyName $friendlyName `
    -KeyUsage DigitalSignature -KeyExportPolicy NonExportable -CertStoreLocation Cert:\CurrentUser\My `
    -NotAfter (Get-Date).AddYears(10) -TextExtension @('2.5.29.37={text}1.3.6.1.5.5.7.3.3', '2.5.29.19={text}')
  Write-Host "Created the signing certificate $($cert.Thumbprint) in Cert:\CurrentUser\My."
}

Remove-Item -Recurse -Force $out -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force (Join-Path $stage 'Assets') | Out-Null
Copy-Item (Join-Path $root 'src-tauri\package\Assets\*') (Join-Path $stage 'Assets')
[xml]$manifest = Get-Content -Raw -Encoding UTF8 (Join-Path $root 'src-tauri\package\AppxManifest.xml')
$manifest.DocumentElement.GetElementsByTagName('Identity')[0].SetAttribute('Version', $version)
$manifest.Save((Join-Path $stage 'AppxManifest.xml'))

$msix = Join-Path $out 'rice.msix'
& $makeAppx pack /o /nv /d $stage /p $msix | Out-Null
if ($LASTEXITCODE -ne 0) { throw "MakeAppx failed ($LASTEXITCODE)." }
& $signTool sign /q /fd SHA256 /s My /sha1 $cert.Thumbprint $msix
if ($LASTEXITCODE -ne 0) { throw "SignTool failed ($LASTEXITCODE)." }
Export-Certificate -Cert $cert -FilePath (Join-Path $out 'rice.cer') | Out-Null
Remove-Item -Recurse -Force $stage
Write-Host "Identity package $version signed: $msix"
