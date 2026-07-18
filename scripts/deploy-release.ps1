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
        exit 0
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
