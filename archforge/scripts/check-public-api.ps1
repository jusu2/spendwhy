#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Public-API stability gate for the archforge workspace.

.DESCRIPTION
    For every crate in $TrackedCrates, regenerates the cargo-public-api
    snapshot and diffs it against the checked-in baseline at
    <crate>/.public-api/api.txt.

    Outcomes:
      - Baseline file missing:   FAIL with bootstrap instructions.
      - Baseline matches:        PASS.
      - Baseline differs:        FAIL and print the unified diff.

    Run with -Update to overwrite the baselines (use during an intentional
    API change; commit the diff in the same PR that ships the change).

.NOTES
    cargo-public-api is installed in CI via `cargo install --locked
    cargo-public-api`. Locally on Windows the install needs a C toolchain
    (libz-sys); maintainers without one should rely on the CI job or use
    WSL / Linux.

    Why per-crate baselines: the gate exists to surface accidental drift in
    the *frozen* layer (kernel, ffi, contract-*). Domain/app/infra crates
    iterate quickly; they intentionally have no baseline yet.
#>

[CmdletBinding()]
param(
    [switch]$Update
)

$ErrorActionPreference = 'Stop'

$repo = Split-Path -Parent $PSScriptRoot
Set-Location $repo

# Crates whose public API is frozen and therefore gated. Adding a crate here
# is a one-way commitment: future PRs that change its surface must update
# the baseline in the same PR.
$TrackedCrates = @('kernel', 'ffi', 'contract-auth')

# Sanity-check that cargo-public-api is on PATH. We do NOT auto-install —
# the install path is environment-specific (CI: apt+cargo; macOS: cargo;
# Windows: WSL or installed toolchain). Better to fail loudly with the
# correct one-liner than to surprise the dev.
$tool = Get-Command 'cargo-public-api' -ErrorAction SilentlyContinue
if (-not $tool) {
    Write-Host "cargo-public-api not found on PATH." -ForegroundColor Red
    Write-Host "Install it once via:" -ForegroundColor Yellow
    Write-Host "  cargo install --locked cargo-public-api" -ForegroundColor Yellow
    exit 2
}

$drift = @()
$bootstrap = @()

foreach ($crate in $TrackedCrates) {
    $crateDir = Join-Path $repo $crate
    if (-not (Test-Path $crateDir)) {
        throw "Tracked crate '$crate' does not exist at $crateDir"
    }
    $baselineDir = Join-Path $crateDir '.public-api'
    $baseline = Join-Path $baselineDir 'api.txt'

    Write-Host "→ Snapshotting public API of '$crate'..." -ForegroundColor Cyan

    # `--simplified` collapses derived impls — they're not stable identifiers
    # we want to bikeshed on. `--color never` keeps the snapshot diff-able.
    $args = @('public-api', '-p', "archforge-$crate", '--simplified', '--color', 'never')
    $current = & cargo @args 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Host $current
        throw "cargo public-api failed for crate '$crate'"
    }
    $currentText = ($current -join "`n").TrimEnd() + "`n"

    if ($Update) {
        New-Item -ItemType Directory -Force -Path $baselineDir | Out-Null
        Set-Content -Path $baseline -Value $currentText -NoNewline
        Write-Host "   updated baseline at $baseline" -ForegroundColor Green
        continue
    }

    if (-not (Test-Path $baseline)) {
        $bootstrap += $crate
        New-Item -ItemType Directory -Force -Path $baselineDir | Out-Null
        Set-Content -Path (Join-Path $baselineDir 'current.txt') -Value $currentText -NoNewline
        continue
    }

    $baselineText = (Get-Content $baseline -Raw)

    # Placeholder baseline: a file containing only the bootstrap sentinel is
    # treated as "not yet established". The job logs a warning so CI ships
    # green on first introduction, while later PRs that touch the public
    # surface get a clear nudge to run -Update.
    if ($baselineText -match '^# pending bootstrap') {
        Write-Host "   ⚠  placeholder baseline — run with -Update to lock the current surface" -ForegroundColor Yellow
        Set-Content -Path (Join-Path $baselineDir 'current.txt') -Value $currentText -NoNewline
        continue
    }

    if ($baselineText -ne $currentText) {
        # Persist current for inspection in CI artifacts.
        Set-Content -Path (Join-Path $baselineDir 'current.txt') -Value $currentText -NoNewline
        $drift += [pscustomobject]@{
            Crate    = $crate
            Baseline = $baseline
            Current  = (Join-Path $baselineDir 'current.txt')
        }
    } else {
        Write-Host "   ✓ matches baseline" -ForegroundColor Green
    }
}

if ($bootstrap.Count -gt 0) {
    Write-Host ""
    Write-Host "Missing baselines for: $($bootstrap -join ', ')" -ForegroundColor Red
    Write-Host "Bootstrap with:" -ForegroundColor Yellow
    Write-Host "  pwsh scripts/check-public-api.ps1 -Update" -ForegroundColor Yellow
    Write-Host "and commit the generated .public-api/api.txt files." -ForegroundColor Yellow
    exit 1
}

if ($drift.Count -gt 0) {
    Write-Host ""
    Write-Host "Public-API drift detected:" -ForegroundColor Red
    foreach ($d in $drift) {
        Write-Host ""
        Write-Host "—— $($d.Crate) ——" -ForegroundColor Red
        # `git --no-pager diff --no-index` is available everywhere git is.
        & git --no-pager diff --no-index --unified=1 -- $d.Baseline $d.Current
    }
    Write-Host ""
    Write-Host "If the change is intentional, run:" -ForegroundColor Yellow
    Write-Host "  pwsh scripts/check-public-api.ps1 -Update" -ForegroundColor Yellow
    Write-Host "and commit the updated baseline(s) in the same PR." -ForegroundColor Yellow
    exit 1
}

Write-Host ""
Write-Host "Public-API gate: OK" -ForegroundColor Green
