#!/usr/bin/env pwsh
<#
.SYNOPSIS
    archforge workspace 的公开 API 稳定性闸门。

.DESCRIPTION
    对 $TrackedCrates 里的每个 crate, 重新生成 cargo-public-api 快照, 然后跟
    仓库里 <crate>/.public-api/api.txt 的基线对比。

    结果:
      - 基线文件缺失:        FAIL, 打印引导命令。
      - 基线与现状一致:      PASS。
      - 基线与现状不一致:    FAIL, 输出 unified diff。

    带 -Update 运行会覆盖基线 (在你**有意**调整公开 API 时用; 务必把 diff
    跟那次修改放在同一个 PR 里提交)。

.NOTES
    cargo-public-api 在 CI 通过 `cargo install --locked cargo-public-api`
    安装。Windows 本地装需要 C 工具链 (libz-sys); 没装的维护者请走 CI 任务
    或 WSL / Linux。

    为什么按 crate 各自基线: 闸门只关心**冻结层** (kernel、ffi、contract-*)
    的飘移。domain / app / infra 这些 crate 还在快速迭代, 故意还没建基线。
#>

[CmdletBinding()]
param(
    [switch]$Update
)

$ErrorActionPreference = 'Stop'

$repo = Split-Path -Parent $PSScriptRoot
Set-Location $repo

# 已经把公开 API 冻结起来、需要被闸门盯着的 crate。在这里新增是一次性承诺:
# 后续修改它公开面的 PR 必须在同一个 PR 里更新基线。
$TrackedCrates = @('kernel', 'ffi', 'contract-auth')

# 校验 cargo-public-api 是否在 PATH 上。我们故意**不**自动安装 —— 各环境
# 的安装路径不同 (CI: apt+cargo; macOS: cargo; Windows: WSL 或装好的工具链)。
# 直接报错并给出唯一一行修复命令, 好过悄悄给开发者一个意外。
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

    # `--simplified` 把派生的 impl 折叠掉 —— 那些不是我们想 bikeshed 的稳定
    # 标识符。`--color never` 让快照可以拿来做 diff。
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

    # 占位基线: 文件内容只有 bootstrap 哨兵注释时, 视为"尚未建立基线"。
    # 任务会打 warning 让 CI 第一次引入时仍能绿; 后续修过公开面的 PR 会被
    # 明确提醒去跑 -Update。
    if ($baselineText -match '^# pending bootstrap') {
        Write-Host "   ⚠  placeholder baseline — run with -Update to lock the current surface" -ForegroundColor Yellow
        Set-Content -Path (Join-Path $baselineDir 'current.txt') -Value $currentText -NoNewline
        continue
    }

    if ($baselineText -ne $currentText) {
        # 把当前快照落到 CI artifact 里供检视。
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
        # `git --no-pager diff --no-index` 在所有装了 git 的地方都可用。
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
