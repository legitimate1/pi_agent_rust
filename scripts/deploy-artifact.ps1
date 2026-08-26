#Requires -Version 7.0
<#
.SYNOPSIS
    一键从 GitHub Actions 下载最新 Windows 构建产物并部署到本地。

.DESCRIPTION
    - 自动查找 my-build-windows.yml 最新成功的 Run（或按 Tag/RunId 指定）
    - gh run download 下载 artifact (pi-windows-amd64)
    - 兼容历史产物 pi-windows-amd64.exe 自动重命名为 pi.exe
    - 备份现有 pi.exe（时间戳 + .bak.latest，滚动保留 3 份）
    - 杀掉运行中的 pi 进程 → 带重试拷贝 → 验证 --version → cargo sweep

.PARAMETER Tag
    指定 tag 触发的 Run，例如 my-win-v0.1.77。留空则取最新成功的 Run。

.PARAMETER RunId
    直接指定 Run ID，优先级高于 Tag。

.PARAMETER Workflow
    工作流文件名，默认 my-build-windows.yml

.PARAMETER Destination
    本地 pi.exe 目标路径，默认 $env:USERPROFILE\.local\bin\pi.exe

.PARAMETER Repo
    GitHub 仓库，默认 legitimate1/pi_agent_rust

.PARAMETER ArtifactName
    Artifact 名，默认 pi-windows-amd64（zip 名），与 upload-artifact.name 一致

.PARAMETER KeepBackups
    滚动保留的带时间戳备份数，默认 3

.EXAMPLE
    pwsh .\scripts\deploy-artifact.ps1
    pwsh .\scripts\deploy-artifact.ps1 -Tag my-win-v0.1.77
    pwsh .\scripts\deploy-artifact.ps1 -RunId 32967540866
#>
param(
    [string]$Tag = "",
    [string]$RunId = "",
    [string]$Workflow = "my-build-windows.yml",
    [string]$Destination = "$env:USERPROFILE\.local\bin\pi.exe",
    [string]$Repo = "legitimate1/pi_agent_rust",
    [string]$ArtifactName = "pi-windows-amd64",
    [int]$KeepBackups = 3
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Require-Gh {
    if (-not (Get-Command gh -ErrorAction SilentlyContinue)) {
        Write-Error "gh CLI 未安装或不在 PATH。请先安装 GitHub CLI (https://cli.github.com/) 并执行 gh auth login。"
        exit 1
    }
    # 轻量验证认证
    $null = gh auth status 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Warning "gh auth status 非 0，请确认已 gh auth login 且对 $Repo 有读权限。"
    }
}

function Find-LatestSuccessfulRun {
    param([string]$Workflow, [string]$Repo, [string]$Tag)

    # 取最近 20 个 Run，前端过滤 conclusion==success
    $json = gh run list --repo $Repo --workflow $Workflow --limit 20 --json databaseId,conclusion,status,headBranch,displayTitle,createdAt 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Error "gh run list 失败: $json"
        exit 1
    }
    $runs = $json | ConvertFrom-Json
    if (-not $runs -or $runs.Count -eq 0) {
        Write-Error "未找到任何 $Workflow 的 Run（repo=$Repo）。"
        exit 1
    }

    if ($Tag) {
        $hit = $runs | Where-Object { $_.headBranch -eq $Tag -and $_.conclusion -eq "success" } | Select-Object -First 1
        if (-not $hit) {
            # Tag 可能是 push 事件的 head_branch，尝试不 filter conclusion 看是否存在
            $any = $runs | Where-Object { $_.headBranch -eq $Tag } | Select-Object -First 1
            if ($any) {
                Write-Error "Tag $Tag 最近的 Run 状态为 $($any.conclusion)/$($any.status)，非 success。RunId=$($any.databaseId)"
            } else {
                Write-Error "未找到 Tag $Tag 对应的 $Workflow Run。请确认 tag 已推送且 workflow 已完成：gh run list --repo $Repo --limit 20"
            }
            exit 1
        }
        return $hit.databaseId.ToString()
    }

    $hit = $runs | Where-Object { $_.conclusion -eq "success" } | Select-Object -First 1
    if (-not $hit) {
        Write-Error "未找到 $Workflow 最近成功的 Run。最近 5 个：`n$($runs | Select-Object -First 5 | Format-Table | Out-String)"
        exit 1
    }
    return $hit.databaseId.ToString()
}

function Find-BinaryInDownload {
    param([string]$Dir)

    # 新产物已改为 pi.exe，历史产物为 pi-windows-amd64.exe，兼容两者
    $candidates = @(
        (Get-ChildItem -Path $Dir -Recurse -File -Filter "pi.exe" -ErrorAction SilentlyContinue | Select-Object -First 1),
        (Get-ChildItem -Path $Dir -Recurse -File -Filter "pi-windows-amd64.exe" -ErrorAction SilentlyContinue | Select-Object -First 1)
    ) | Where-Object { $_ }

    if ($candidates.Count -eq 0) {
        Write-Error "下载目录未找到 pi.exe / pi-windows-amd64.exe。目录内容：`n$(Get-ChildItem -Path $Dir -Recurse | Format-Table | Out-String)"
        exit 1
    }
    return $candidates[0].FullName
}

Require-Gh

if (-not $RunId) {
    Write-Host "==> 查找 $Workflow 最新成功的 Run (repo=$Repo, Tag=$Tag)..." -ForegroundColor Cyan
    $RunId = Find-LatestSuccessfulRun -Workflow $Workflow -Repo $Repo -Tag $Tag
}
Write-Host "==> 目标 RunId: $RunId" -ForegroundColor Green
Write-Host "    查看: https://github.com/$Repo/actions/runs/$RunId" -ForegroundColor DarkGray

# 下载目录：每次新建临时目录，避免残留
$downloadDir = Join-Path $env:TEMP "pi-artifact-$RunId"
if (Test-Path $downloadDir) { Remove-Item -Recurse -Force $downloadDir -ErrorAction SilentlyContinue }
New-Item -ItemType Directory -Force $downloadDir | Out-Null

Write-Host "==> 下载 artifact $ArtifactName 到 $downloadDir ..." -ForegroundColor Cyan
gh run download $RunId --repo $Repo --name $ArtifactName --dir $downloadDir 2>&1 | Write-Host
if ($LASTEXITCODE -ne 0) {
    Write-Error "gh run download 失败 (exit $LASTEXITCODE)。请确认 RunId=$RunId 存在且 artifact 名为 $ArtifactName。"
    exit 1
}

$sourceExe = Find-BinaryInDownload -Dir $downloadDir
Write-Host "==> 发现产物: $sourceExe" -ForegroundColor Green
Write-Host "    大小: $((Get-Item $sourceExe).Length / 1MB | ForEach-Object { "{0:N2} MB" -f $_ })" -ForegroundColor DarkGray

# 若为历史名，复制一份 pi.exe 供后续统一处理
$normalizedSource = Join-Path $downloadDir "pi.exe"
if ($sourceExe -ne $normalizedSource) {
    Copy-Item -LiteralPath $sourceExe -Destination $normalizedSource -Force
    Write-Host "    已重命名为 pi.exe (兼容历史 pi-windows-amd64.exe)" -ForegroundColor DarkGray
    $sourceExe = $normalizedSource
}

# 备份现有 pi.exe
$destDir = Split-Path $Destination -Parent
if (-not (Test-Path $destDir)) { New-Item -ItemType Directory -Force $destDir | Out-Null }

if (Test-Path $Destination) {
    $timestamp = Get-Date -Format "yyyyMMdd_HHmmss"
    $backupTimestamped = Join-Path $destDir "pi.exe.bak.$timestamp"
    $backupLatest = Join-Path $destDir "pi.exe.bak.latest"

    Write-Host "==> 备份现有 $Destination" -ForegroundColor Cyan
    Copy-Item -LiteralPath $Destination -Destination $backupTimestamped -Force
    Copy-Item -LiteralPath $Destination -Destination $backupLatest -Force
    Write-Host "    -> $backupTimestamped" -ForegroundColor DarkGray
    Write-Host "    -> $backupLatest" -ForegroundColor DarkGray

    # 滚动保留：仅 KeepBackups 份带时间戳备份
    $oldBackups = Get-ChildItem -Path $destDir -File -Filter "pi.exe.bak.*" -ErrorAction SilentlyContinue |
        Where-Object { $_.Name -ne "pi.exe.bak.latest" } |
        Sort-Object LastWriteTime -Descending
    if ($oldBackups.Count -gt $KeepBackups) {
        $toRemove = $oldBackups | Select-Object -Skip $KeepBackups
        foreach ($f in $toRemove) {
            Write-Host "    清理旧备份: $($f.Name)" -ForegroundColor DarkGray
            Remove-Item -LiteralPath $f.FullName -Force -ErrorAction SilentlyContinue
        }
    }
} else {
    Write-Host "==> 目标不存在，跳过备份: $Destination" -ForegroundColor DarkGray
}

# 杀掉运行中的 pi
$procs = Get-Process -Name pi -ErrorAction SilentlyContinue
if ($procs) {
    Write-Host "==> 停止运行中的 pi 进程 (PID: $($procs.Id -join ', '))..." -ForegroundColor Yellow
    $procs | Stop-Process -Force -ErrorAction SilentlyContinue
    Start-Sleep 1
}

# 带重试拷贝
$maxRetries = 5
$retryDelay = 1
$deployed = $false
for ($attempt = 1; $attempt -le $maxRetries; $attempt++) {
    try {
        Copy-Item -LiteralPath $sourceExe -Destination $Destination -Force -ErrorAction Stop
        Write-Host "==> 已部署 $sourceExe -> $Destination (attempt $attempt)" -ForegroundColor Green
        $deployed = $true
        break
    } catch {
        if ($attempt -lt $maxRetries) {
            Write-Warning "拷贝失败 (attempt $attempt/$maxRetries): $($_.Exception.Message)，${retryDelay}s 后重试..."
            Start-Sleep $retryDelay
        } else {
            Write-Error "拷贝失败，已重试 $maxRetries 次: $($_.Exception.Message)"
            Write-Host "    已备份到 $destDir\pi.exe.bak.latest，可手动恢复：Copy-Item $destDir\pi.exe.bak.latest $Destination -Force" -ForegroundColor Yellow
            exit 1
        }
    }
}

if (-not $deployed) { exit 1 }

# 校验
Write-Host "==> 校验部署结果..." -ForegroundColor Cyan
Get-Item $Destination | Format-List Name, Length, LastWriteTime | Out-String | Write-Host
try {
    $ver = & $Destination --version 2>&1 | Out-String
    Write-Host $ver.Trim() -ForegroundColor Green
} catch {
    Write-Warning "执行 $Destination --version 失败: $($_.Exception.Message)"
}

# 清理 cargo 产物（复用 deploy-release 逻辑：先 --file 再 --stamp）
$projectRoot = Resolve-Path (Join-Path $PSScriptRoot "..") -ErrorAction SilentlyContinue
if ($projectRoot) {
    Push-Location $projectRoot
    try {
        if (Get-Command cargo-sweep -ErrorAction SilentlyContinue) {
            if (Test-Path "sweep.timestamp") {
                cargo sweep --file 2>&1 | Write-Host
                if ($LASTEXITCODE -ne 0) {
                    Write-Warning "cargo sweep --file 失败 (exit $LASTEXITCODE)"
                } else {
                    Write-Host "cargo-sweep: cleaned artifacts older than previous stamp" -ForegroundColor DarkGray
                }
            } else {
                Write-Host "cargo-sweep: no previous stamp, skipping --file" -ForegroundColor DarkGray
            }
            cargo sweep --stamp 2>&1 | Write-Host
            Write-Host "cargo-sweep: timestamp updated" -ForegroundColor DarkGray
        }
    } finally {
        Pop-Location
    }
}

# 清理临时下载目录
Remove-Item -Recurse -Force $downloadDir -ErrorAction SilentlyContinue

Write-Host ""
Write-Host "✅ 部署完成: $Destination" -ForegroundColor Green
Write-Host "   回滚: Copy-Item `"$destDir\pi.exe.bak.latest`" `"$Destination`" -Force" -ForegroundColor DarkGray
Write-Host "   查看备份: Get-ChildItem `"$destDir\pi.exe.bak.*`" | Sort LastWriteTime" -ForegroundColor DarkGray
