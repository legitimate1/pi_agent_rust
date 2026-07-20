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

# Cleanup: stamp current build timestamp for cargo-sweep
# Next `cargo sweep --file` will remove artifacts older than this deployment
$projectRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Push-Location $projectRoot
try {
    if (Get-Command cargo-sweep -ErrorAction SilentlyContinue) {
        cargo sweep --stamp
        Write-Output "cargo-sweep: timestamp updated (run 'cargo sweep --file' to clean old artifacts)"
    } else {
        Write-Warning "cargo-sweep not installed. Install with: cargo install cargo-sweep"
    }
} finally {
    Pop-Location
}
