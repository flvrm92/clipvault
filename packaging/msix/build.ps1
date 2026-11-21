<#
.SYNOPSIS
  Packages already-built ClipVault binaries as MSIX (specs/09 §5, ADR-0007).

.DESCRIPTION
  For each architecture: renders the manifest from identity.json, lays out the package
  (exe + Assets + resources.pri), runs MakePri and MakeAppx. Then bundles every .msix into a
  .msixbundle and zips it with the symbols into the .msixupload sent to Partner Center.

  Expects target/<triple>/release/ClipVault.exe for every requested architecture
  (built by tools/release.ps1 or `pnpm tauri build --no-bundle --target <triple>`).

.PARAMETER Version
  Release version X.Y.Z. The package version is X.Y.Z.0 (the Store requires revision 0).

.PARAMETER Arch
  Architectures to package: x64, arm64 (default: both).

.PARAMETER Sign
  Also signs the .msix and .msixbundle with the test certificate created by
  tools/new-test-cert.ps1 (sideload/QA only; the Store signs on publication).
  The .msixupload is always left unsigned.

.EXAMPLE
  packaging/msix/build.ps1 -Version 1.0.0 -Sign
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory)][ValidatePattern('^\d+\.\d+\.\d+$')][string]$Version,
    [ValidateSet('x64', 'arm64')][string[]]$Arch = @('x64', 'arm64'),
    [switch]$Sign
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$Root = Resolve-Path (Join-Path $PSScriptRoot '..\..')
$Msix = $PSScriptRoot
$Out = Join-Path $Root 'target\msix'
$PackageVersion = "$Version.0"
$Triples = @{ x64 = 'x86_64-pc-windows-msvc'; arm64 = 'aarch64-pc-windows-msvc' }

function Find-SdkTool([string]$Name) {
    $bin = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
    $tool = Get-ChildItem $bin -Directory -Filter '10.*' -ErrorAction SilentlyContinue |
        Sort-Object { [version]$_.Name } -Descending |
        ForEach-Object { Join-Path $_.FullName "x64\$Name" } |
        Where-Object { Test-Path $_ } |
        Select-Object -First 1
    if (-not $tool) { throw "$Name not found under $bin; install the Windows 10/11 SDK." }
    $tool
}

function Invoke-Tool([string]$Tool, [string[]]$Arguments) {
    # The SDK tools are verbose; their output only matters when they fail.
    $output = & $Tool @Arguments 2>&1
    if ($LASTEXITCODE -ne 0) {
        $output | Out-Host
        throw "$(Split-Path $Tool -Leaf) failed with exit code $LASTEXITCODE"
    }
}

function Read-Identity {
    $identity = Get-Content (Join-Path $Msix 'identity.json') -Raw | ConvertFrom-Json
    foreach ($field in 'Name', 'Publisher', 'PublisherDisplayName') {
        if ([string]::IsNullOrWhiteSpace($identity.$field)) { throw "identity.json: $field is empty" }
    }
    if ($identity.Publisher -notmatch '^CN=') { throw 'identity.json: Publisher must start with CN=' }
    $identity
}

function New-Manifest($Identity, [string]$Architecture) {
    $escape = { param($value) [System.Security.SecurityElement]::Escape($value) }
    (Get-Content (Join-Path $Msix 'AppxManifest.template.xml') -Raw).
        Replace('{{NAME}}', (& $escape $Identity.Name)).
        Replace('{{PUBLISHER_DISPLAY_NAME}}', (& $escape $Identity.PublisherDisplayName)).
        Replace('{{PUBLISHER}}', (& $escape $Identity.Publisher)).
        Replace('{{VERSION}}', $PackageVersion).
        Replace('{{ARCH}}', $Architecture)
}

function Compress-Zip([string[]]$Paths, [string]$Destination) {
    $zip = "$Destination.zip"
    Compress-Archive -Path $Paths -DestinationPath $zip -Force
    Move-Item $zip $Destination -Force
}

$makePri = Find-SdkTool 'makepri.exe'
$makeAppx = Find-SdkTool 'makeappx.exe'
$identity = Read-Identity

New-Item -ItemType Directory -Force $Out | Out-Null
$bundleInput = Join-Path $Out 'bundle'
Remove-Item $bundleInput -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $bundleInput | Out-Null
$symbols = @()

foreach ($architecture in $Arch) {
    $release = Join-Path $Root "target\$($Triples[$architecture])\release"
    $exe = Join-Path $release 'ClipVault.exe'
    if (-not (Test-Path $exe)) { throw "$exe not found; build it first (tools/release.ps1)." }

    Write-Host "== $architecture"
    $layout = Join-Path $Out "layout-$architecture"
    Remove-Item $layout -Recurse -Force -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force $layout | Out-Null
    Copy-Item $exe $layout
    Copy-Item (Join-Path $Msix 'Assets') $layout -Recurse
    Copy-Item (Join-Path $Msix 'Strings') $layout -Recurse
    Set-Content (Join-Path $layout 'AppxManifest.xml') (New-Manifest $identity $architecture) -Encoding utf8NoBOM

    # Written outside the indexed folder, then moved in, so MakePri never re-indexes it.
    $pri = Join-Path $Out "resources-$architecture.pri"
    Invoke-Tool $makePri @('new', '/pr', $layout, '/cf', (Join-Path $Msix 'priconfig.xml'),
        '/mn', (Join-Path $layout 'AppxManifest.xml'), '/of', $pri, '/o')
    Move-Item $pri (Join-Path $layout 'resources.pri') -Force
    # The strings now live in resources.pri.
    Remove-Item (Join-Path $layout 'Strings') -Recurse -Force

    $package = Join-Path $bundleInput "ClipVault_${PackageVersion}_$architecture.msix"
    Invoke-Tool $makeAppx @('pack', '/d', $layout, '/p', $package, '/o')
    Write-Host "  $package"

    $pdb = Join-Path $release 'ClipVault.pdb'
    if (Test-Path $pdb) {
        $appxsym = Join-Path $Out "ClipVault_${PackageVersion}_$architecture.appxsym"
        Compress-Zip @($pdb) $appxsym
        $symbols += $appxsym
    }
}

$bundleName = "ClipVault_${PackageVersion}_$($Arch -join '_')"
$bundle = Join-Path $Out "$bundleName.msixbundle"
Invoke-Tool $makeAppx @('bundle', '/d', $bundleInput, '/p', $bundle, '/bv', $PackageVersion, '/o')

$upload = Join-Path $Out "$bundleName.msixupload"
Compress-Zip (@($bundle) + $symbols) $upload
Write-Host "Store upload: $upload"

if ($Sign) {
    $cert = Get-ChildItem Cert:\CurrentUser\My |
        Where-Object { $_.Subject -eq $identity.Publisher -and $_.HasPrivateKey } |
        Sort-Object NotAfter -Descending |
        Select-Object -First 1
    if (-not $cert) { throw "No certificate for $($identity.Publisher); run tools/new-test-cert.ps1." }
    $signTool = Find-SdkTool 'signtool.exe'
    $packages = @(Get-ChildItem $bundleInput -Filter '*.msix' | ForEach-Object FullName) + $bundle
    Invoke-Tool $signTool (@('sign', '/fd', 'SHA256', '/sha1', $cert.Thumbprint) + $packages)
    Write-Host "Signed for sideloading: $bundle"
}
