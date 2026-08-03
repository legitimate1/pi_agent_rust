# Deploy release build: stop running pi.exe and copy new binary
param(
    [string]$Source = "target\release\pi.exe",
    [string]$Destination = "$env:USERPROFILE\.local\bin\pi.exe"
)

$sourcePath = Join-Path $PSScriptRoot ".." $Source
if (-not (Test-Path $sourcePath)) {
    Write-Error "Source not found: $sourcePath"
    exit 1
}

# Force kill running pi
Get-Process -Name pi -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep 1

# Copy with retry (file handle may linger after kill)
$maxRetries = 5
$retryDelay = 1
for ($attempt = 1; $attempt -le $maxRetries; $attempt++) {
    try {
        Copy-Item -LiteralPath $sourcePath -Destination $Destination -Force -ErrorAction Stop
        Write-Output "Deployed $sourcePath -> $Destination"
        break
    } catch {
        if ($attempt -lt $maxRetries) {
            Write-Warning "Copy failed (attempt $attempt/$maxRetries): $($_.Exception.Message)"
            Start-Sleep $retryDelay
            continue
        } else {
            Write-Error "Copy failed after $maxRetries attempts: $($_.Exception.Message)"
            exit 1
        }
    }
}

# Cleanup: first remove artifacts older than the previous deployment's stamp
# (keeps only what was built since then), then stamp now so the next
# deployment's sweep has a fresh baseline. Order matters: file-before-stamp.
$projectRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Push-Location $projectRoot
try {
    if (Get-Command cargo-sweep -ErrorAction SilentlyContinue) {
        if (Test-Path "sweep.timestamp") {
            cargo sweep --file
            if ($LASTEXITCODE -eq 0) {
                Write-Output "cargo-sweep: cleaned artifacts older than previous stamp"
            } else {
                Write-Warning "cargo sweep --file failed (exit $LASTEXITCODE); keeping old stamp for retry"
                exit 1
            }
        } else {
            Write-Output "cargo-sweep: no previous stamp, skipping cleanup"
        }
        cargo sweep --stamp
        Write-Output "cargo-sweep: timestamp updated"
    } else {
        Write-Warning "cargo-sweep not installed. Install with: cargo install cargo-sweep"
    }
} finally {
    Pop-Location
}
