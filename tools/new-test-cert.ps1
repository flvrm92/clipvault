<#
.SYNOPSIS
  Creates a self-signed code-signing certificate for sideloading test MSIX packages (specs/09 §5).

.DESCRIPTION
  The subject must equal Publisher in packaging/msix/identity.json, or Windows refuses the
  package. The certificate goes to Cert:\CurrentUser\My (used by build.ps1 -Sign) and its
  public part is exported to target/msix/ClipVault-test.cer, which must be trusted once on
  each test machine (elevated):

    Import-Certificate -FilePath target\msix\ClipVault-test.cer -CertStoreLocation Cert:\LocalMachine\TrustedPeople

  Never use this certificate for Store submissions; the Store signs the package itself.
#>
[CmdletBinding()]
param([int]$Years = 1)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$root = Resolve-Path (Join-Path $PSScriptRoot '..')
$identity = Get-Content (Join-Path $root 'packaging\msix\identity.json') -Raw | ConvertFrom-Json
$out = Join-Path $root 'target\msix'
New-Item -ItemType Directory -Force $out | Out-Null

$cert = New-SelfSignedCertificate -Type Custom -Subject $identity.Publisher `
    -KeyUsage DigitalSignature -FriendlyName 'ClipVault test signing' `
    -CertStoreLocation Cert:\CurrentUser\My -NotAfter (Get-Date).AddYears($Years) `
    -TextExtension @('2.5.29.37={text}1.3.6.1.5.5.7.3.3', '2.5.29.19={text}')

$cer = Join-Path $out 'ClipVault-test.cer'
Export-Certificate -Cert $cert -FilePath $cer | Out-Null
Write-Host "Certificate $($cert.Thumbprint) for $($identity.Publisher)"
Write-Host "Trust it once per machine (elevated):"
Write-Host "  Import-Certificate -FilePath `"$cer`" -CertStoreLocation Cert:\LocalMachine\TrustedPeople"
