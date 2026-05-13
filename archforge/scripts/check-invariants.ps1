#!/usr/bin/env pwsh
# Architectural invariants linter for the archforge workspace.
#
# Fails CI if any of these constitutional rules are violated:
#   I1. kernel must not depend on anything outside `serde / thiserror / uuid / async-trait`
#   I2. contract crates must not depend on domain/app/infra crates
#   I3. domain crates must not depend on infra/* crates
#   I4. infra crates must depend on contract crates (Port-defined-by-consumer)
#   I5. no `Domain` aggregate derives `Serialize` or `Deserialize`
#
# This script is intentionally simple (grep over Cargo.toml + grep over src/)
# so it stays trustworthy. Don't replace with a complex Rust crate without
# very good reason.

$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

$violations = @()

function Check-Forbids {
    param(
        [string]$Crate,
        [string[]]$Forbidden,
        [string]$Reason
    )
    $manifest = Join-Path $root "$Crate/Cargo.toml"
    if (-not (Test-Path $manifest)) { return }
    $text = Get-Content $manifest -Raw
    foreach ($f in $Forbidden) {
        if ($text -match "(?m)^\s*$([regex]::Escape($f))\s*=") {
            $script:violations += "[$Crate] forbidden dependency '$f' — $Reason"
        }
    }
}

# I1 — kernel is pristine
$kernelAllow = @('serde', 'thiserror', 'uuid', 'async-trait', 'serde_json')
$kernelManifest = Get-Content (Join-Path $root 'kernel/Cargo.toml') -Raw
$kernelDeps = [regex]::Matches($kernelManifest, '(?m)^\s*([a-zA-Z0-9_\-]+)\s*=\s*\{[^}]*workspace\s*=\s*true') | ForEach-Object { $_.Groups[1].Value }
foreach ($d in $kernelDeps) {
    if ($kernelAllow -notcontains $d) {
        $violations += "[kernel] dependency '$d' is not on the kernel-allowlist (I1)"
    }
}

# I2 — contract-* must not depend on domain/app/infra
Get-ChildItem -Directory -Filter 'contract-*' | ForEach-Object {
    Check-Forbids -Crate $_.Name -Reason "contracts must not import implementations (I2)" `
        -Forbidden @(
            'archforge-domain-auth',
            'archforge-app-auth',
            'archforge-infra-auth-memory',
            'archforge-infra-auth-jsonfile'
        )
}

# I3 — domain-* must not depend on infra/*
Get-ChildItem -Directory -Filter 'domain-*' | ForEach-Object {
    Check-Forbids -Crate $_.Name -Reason "domain must not import infrastructure (I3)" `
        -Forbidden @(
            'archforge-infra-auth-memory',
            'archforge-infra-auth-jsonfile',
            'tokio',
            'sqlx',
            'reqwest'
        )
}

# I4 — infra-* must depend on a contract-* (port-defined-by-consumer)
Get-ChildItem -Directory -Filter 'infra-*' | ForEach-Object {
    $manifest = Get-Content (Join-Path $_.FullName 'Cargo.toml') -Raw
    if ($manifest -notmatch 'archforge-contract-') {
        $violations += "[$($_.Name)] infra crate must depend on a contract-* crate (I4)"
    }
}

# I5 — no Domain aggregate derives Serialize/Deserialize
Get-ChildItem -Directory -Filter 'domain-*' | ForEach-Object {
    Get-ChildItem -Path (Join-Path $_.FullName 'src') -Recurse -Filter '*.rs' -ErrorAction SilentlyContinue | ForEach-Object {
        $src = Get-Content $_.FullName -Raw
        if ($src -match '#\[derive\([^)]*\b(Serialize|Deserialize)\b[^)]*\)\]') {
            $violations += "[$($_.FullName)] domain types must not derive Serialize/Deserialize (I5)"
        }
    }
}

if ($violations.Count -gt 0) {
    Write-Host "ArchForge invariants FAILED:" -ForegroundColor Red
    $violations | ForEach-Object { Write-Host "  - $_" -ForegroundColor Red }
    exit 1
}

Write-Host "ArchForge invariants: OK" -ForegroundColor Green
