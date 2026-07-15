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

Get-Process -Name pi -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep 1
Copy-Item -LiteralPath $sourcePath -Destination $Destination -Force
Write-Output "Deployed $sourcePath -> $Destination"
