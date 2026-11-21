<#
.SYNOPSIS
  Builds the ClipVault release packages end to end (specs/09 §5).

.DESCRIPTION
  1. Sets the workspace version (Cargo.toml [workspace.package]; tauri.conf.json inherits it).
  2. Checks the generated brand assets.
  3. Regenerates THIRD-PARTY-NOTICES.md (ADR-0009), unless -SkipNotices.
  4. Builds ClipVault.exe for each architecture with the Tauri CLI (no bundle).
  5. Packages with packaging/msix/build.ps1.

.EXAMPLE
  tools/release.ps1 -Version 1.0.0
  tools/release.ps1 -Version 1.0.0 -Arch x64 -Sign -SkipNotices
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory)][ValidatePattern('^\d+\.\d+\.\d+$')][string]$Version,
    [ValidateSet('x64', 'arm64')][string[]]$Arch = @('x64', 'arm64'),
    [switch]$Sign,
    [switch]$SkipNotices
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$root = Resolve-Path (Join-Path $PSScriptRoot '..')
$triples = @{ x64 = 'x86_64-pc-windows-msvc'; arm64 = 'aarch64-pc-windows-msvc' }

function Invoke-Step([string]$Command, [string[]]$Arguments) {
    Write-Host "> $Command $($Arguments -join ' ')"
    & $Command @Arguments | Out-Host
    if ($LASTEXITCODE -ne 0) { throw "$Command failed with exit code $LASTEXITCODE" }
}

Push-Location $root
try {
    # 1. Version: only the first `version = "…"` after [workspace.package].
    $cargoToml = Join-Path $root 'Cargo.toml'
    $text = Get-Content $cargoToml -Raw
    $pattern = '(?s)(\[workspace\.package\][^\[]*?version\s*=\s*")[^"]+(")'
    if ($text -notmatch $pattern) { throw 'version not found in [workspace.package]' }
    $updated = [regex]::new($pattern).Replace($text, "`${1}$Version`${2}", 1)
    if ($updated -ne $text) {
        Set-Content $cargoToml $updated -NoNewline -Encoding utf8NoBOM
        Invoke-Step cargo @('update', '--workspace')
    }

    # 2. Brand assets.
    Invoke-Step cargo @('run', '-p', 'gen-assets', '--locked', '--', '--check')

    # 3. Third-party notices.
    if (-not $SkipNotices) {
        & (Join-Path $PSScriptRoot 'third-party-notices.ps1')
    }

    # 4. Binaries (the Tauri beforeBuildCommand builds the UI).
    foreach ($architecture in $Arch) {
        Invoke-Step pnpm @('tauri', 'build', '--no-bundle', '--target', $triples[$architecture])
    }

    # 5. Packages.
    & (Join-Path $root 'packaging\msix\build.ps1') -Version $Version -Arch $Arch -Sign:$Sign
}
finally {
    Pop-Location
}
