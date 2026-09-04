#Requires -Version 7.0
<#
.SYNOPSIS
    一键递增版本号并触发云端构建（A 方案）。

.DESCRIPTION
    A 方案：Cargo.toml 即构建版本，云构建/部署脚本均只读不 bump。
    本脚本是版本递增的唯一入口：
      1. cargo set-version --bump <Level> -p pi_agent_rust
      2. git add Cargo.toml Cargo.lock && git commit && git push origin <Branch>
      3. gh workflow run <Workflow> --ref <Branch>

    默认 bump patch，推送到 custom，触发 my-build-windows.yml。

.PARAMETER Level
    递增级别：patch / minor / major，默认 patch。

.PARAMETER Branch
    推送并触发的分支，默认 custom。

.PARAMETER Workflow
    触发的工作流文件名，默认 my-build-windows.yml。

.PARAMETER Repo
    GitHub 仓库，默认 legitimate1/pi_agent_rust。

.PARAMETER NoPush
    仅本地 bump，不 push 也不触发构建（用于本地验证）。

.PARAMETER NoTrigger
    bump + push，但不触发 workflow（仅更新远端版本）。

.EXAMPLE
    pwsh .\scripts\bump-and-build.ps1
    pwsh .\scripts\bump-and-build.ps1 -Level minor
    pwsh .\scripts\bump-and-build.ps1 -NoPush
#>
param(
    [ValidateSet("patch", "minor", "major")]
    [string]$Level = "patch",
    [string]$Branch = "custom",
    [string]$Workflow = "my-build-windows.yml",
    [string]$Repo = "legitimate1/pi_agent_rust",
    [switch]$NoPush,
    [switch]$NoTrigger
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$projectRoot = Resolve-Path (Join-Path $PSScriptRoot "..") -ErrorAction Stop
Push-Location $projectRoot
try {
    # 前置检查
    if (-not (Get-Command cargo.exe -ErrorAction SilentlyContinue)) {
        Write-Error "cargo.exe 不可用（需 MSVC 环境，请在 pwsh 中运行，`$PROFILE` 已注入 vcvars64）"
        exit 1
    }
    if (-not $NoPush -and -not (Get-Command gh -ErrorAction SilentlyContinue)) {
        Write-Error "gh CLI 未安装或不在 PATH，请先安装 https://cli.github.com/ 并 gh auth login"
        exit 1
    }

    $cargoToml = Join-Path $projectRoot "Cargo.toml"
    $beforeRaw = Get-Content $cargoToml -Raw
    $beforeVer = $null
    if ($beforeRaw -match '(?m)^\[package\][\s\S]*?^\s*version\s*=\s*"([^"]+)"') { $beforeVer = $Matches[1] }
    Write-Host "==> 当前版本: $beforeVer" -ForegroundColor Cyan
    Write-Host "    递增级别: $Level  |  分支: $Branch  |  工作流: $Workflow" -ForegroundColor DarkGray

    # 检查工作区是否干净（除 Cargo.toml/lock 外有未提交改动则提醒）
    $porcelain = git status --porcelain 2>&1 | Out-String
    $dirty = $porcelain.Trim()
    if ($dirty) {
        # 允许仅 Cargo.toml/lock 干净的情况继续；否则警告
        $nonVersionDirty = $porcelain -split "`n" | Where-Object { $_ -and ($_ -notmatch "Cargo\.toml") -and ($_ -notmatch "Cargo\.lock") }
        if ($nonVersionDirty) {
            Write-Warning "工作区有未提交的非版本文件改动，建议先提交：`n$($nonVersionDirty -join "`n")"
            Write-Host "    继续执行（仅 bump 版本文件）..." -ForegroundColor DarkGray
        }
    }

    # bump
    Write-Host "==> cargo set-version --bump $Level -p pi_agent_rust ..." -ForegroundColor Cyan
    $null = cargo.exe set-version --bump $Level -p pi_agent_rust 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Error "cargo set-version 失败 (exit $LASTEXITCODE)。请确认已安装 cargo-edit: cargo install cargo-edit"
        exit 1
    }

    $afterRaw = Get-Content $cargoToml -Raw
    $afterVer = $null
    if ($afterRaw -match '(?m)^\[package\][\s\S]*?^\s*version\s*=\s*"([^"]+)"') { $afterVer = $Matches[1] }
    if (-not $afterVer) { Write-Error "无法从 Cargo.toml 解析递增后版本号"; exit 1 }
    if ($afterVer -eq $beforeVer) { Write-Warning "版本号未变化 ($beforeVer)，可能 set-version 未生效"; exit 1 }
    Write-Host "==> 版本已递增: $beforeVer -> $afterVer" -ForegroundColor Green

    if ($NoPush) {
        Write-Host "==> -NoPush 已指定，跳过 push/trigger。本地已 bump 到 $afterVer，记得手动 git push" -ForegroundColor Yellow
        exit 0
    }

    # commit + push
    git add Cargo.toml Cargo.lock 2>&1 | Out-Null
    git diff --cached --quiet 2>&1 | Out-Null
    if ($LASTEXITCODE -eq 0) {
        Write-Warning "版本文件无变化，跳过提交（可能已是最新）"
    } else {
        git commit -m "chore: bump version to $afterVer [skip ci]" 2>&1 | Write-Host
        if ($LASTEXITCODE -ne 0) { Write-Error "git commit 失败 (exit $LASTEXITCODE)"; exit 1 }
        Write-Host "    已提交 chore: bump version to $afterVer [skip ci]" -ForegroundColor Green
    }

    # 确保在 Branch 上
    $curBranch = (git branch --show-current 2>&1).Trim()
    if ($curBranch -ne $Branch) {
        Write-Warning "当前分支 $curBranch != $Branch，将推送到 origin/$Branch（本地分支不变）"
    }

    Write-Host "==> git push origin $Branch ..." -ForegroundColor Cyan
    git push origin "HEAD:$Branch" 2>&1 | Write-Host
    if ($LASTEXITCODE -ne 0) {
        Write-Error "git push 失败 (exit $LASTEXITCODE)，请手动 push 后再触发构建"
        exit 1
    }
    Write-Host "    已推送 $afterVer 到 origin/$Branch" -ForegroundColor Green

    if ($NoTrigger) {
        Write-Host "==> -NoTrigger 已指定，跳过 workflow 触发" -ForegroundColor Yellow
        exit 0
    }

    # trigger workflow
    Write-Host "==> gh workflow run $Workflow --ref $Branch --repo $Repo ..." -ForegroundColor Cyan
    gh workflow run $Workflow --ref $Branch --repo $Repo 2>&1 | Write-Host
    if ($LASTEXITCODE -ne 0) {
        Write-Error "gh workflow run 失败 (exit $LASTEXITCODE)"
        exit 1
    }
    Write-Host ""
    Write-Host "OK  已触发 $Workflow @ $Branch (版本 $afterVer)" -ForegroundColor Green
    Write-Host "   查看进度: gh run list --workflow=$Workflow --limit 5 --repo $Repo" -ForegroundColor DarkGray
    Write-Host "   跟踪日志: gh run watch <run-id> --repo $Repo" -ForegroundColor DarkGray
    Write-Host "   构建完成后部署: pwsh .\scripts\deploy-artifact.ps1" -ForegroundColor DarkGray
} finally {
    Pop-Location
}
