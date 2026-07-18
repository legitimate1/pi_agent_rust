# Deploy release build: notify running pi.exe to exit gracefully, then copy new binary
param(
    [string]$Source = "target\release\pi.exe",
    [string]$Destination = "$env:USERPROFILE\.local\bin\pi.exe"
)

$sourcePath = Join-Path $PSScriptRoot ".." $Source
if (-not (Test-Path $sourcePath)) {
    Write-Error "Source not found: $sourcePath"
    exit 1
}

# Step 1: Signal any running pi process to exit gracefully
$piProcess = Get-Process -Name pi -ErrorAction SilentlyContinue
if ($piProcess) {
    Write-Host "Found running pi (PID $($piProcess.Id)), requesting graceful shutdown..."

    $signalDir = "$env:USERPROFILE\.pi\agent"
    $signalFile = Join-Path $signalDir "graceful-shutdown"
    New-Item -Path $signalDir -ItemType Directory -Force | Out-Null
    New-Item -Path $signalFile -ItemType File -Force | Out-Null

    Write-Host "Waiting up to 10 seconds for pi to exit..."
    $exited = $piProcess.WaitForExit(10000)
    if ($exited) {
        Write-Host "pi exited gracefully."
    } else {
        Write-Warning "pi did not exit within timeout, force killing..."
        $piProcess | Stop-Process -Force
    }
}

# Step 2: Copy new binary with retry (process may still hold handle after exit)
$maxRetries = 5
$retryDelay = 1
for ($attempt = 1; $attempt -le $maxRetries; $attempt++) {
    try {
        Copy-Item -LiteralPath $sourcePath -Destination $Destination -Force -ErrorAction Stop
        Write-Output "Deployed $sourcePath -> $Destination"
        return
    } catch {
        if ($attempt -lt $maxRetries) {
            Write-Warning "Copy failed (attempt $attempt/$maxRetries): $($_.Exception.Message)"
            Start-Sleep $retryDelay
        } else {
            Write-Error "Copy failed after $maxRetries attempts: $($_.Exception.Message)"
            exit 1
        }
    }
}
