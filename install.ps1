# Pane quick installer
#
#   irm https://trypane.xyz/install.ps1 | iex
#
# Downloads the latest signed release from GitHub, installs per-user
# (no admin), and launches Pane. Works on Windows PowerShell 5.1 and pwsh.

$ErrorActionPreference = 'Stop'

# TLS 1.2 for Windows PowerShell 5.1 (pwsh already defaults to it).
[Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor 3072

Write-Host ''
Write-Host '  Pane - AI usage limits in your system tray' -ForegroundColor Cyan
Write-Host ''

Write-Host '> Finding the latest release...'
$release = Invoke-RestMethod 'https://api.github.com/repos/ItsJazii/pane/releases/latest' `
    -Headers @{ 'User-Agent' = 'pane-installer' } -UseBasicParsing
$asset = $release.assets | Where-Object { $_.name -match '^Pane_.+_x64-setup\.exe$' } | Select-Object -First 1
if (-not $asset) { throw 'No installer found in the latest release - please report this at https://github.com/ItsJazii/pane/issues' }

$dest = Join-Path $env:TEMP $asset.name
Write-Host ("> Downloading {0} ({1:N1} MB)..." -f $asset.name, ($asset.size / 1MB))
Invoke-WebRequest $asset.browser_download_url -OutFile $dest -UseBasicParsing

# Integrity check — MANDATORY, fails closed. Pane's release process writes
# the installer's SHA-256 into latest.json (a field this project adds on
# top of the standard Tauri updater manifest); a release without it is
# malformed and must not be installed. Defends against corrupted or
# swapped downloads (the deeper guarantee is the minisign-verified
# auto-update path once installed).
$manifest = $release.assets | Where-Object { $_.name -eq 'latest.json' } | Select-Object -First 1
if (-not $manifest) {
    Remove-Item $dest -Force -ErrorAction SilentlyContinue
    throw 'Release is missing its latest.json manifest - not installing. Please report this at https://github.com/ItsJazii/pane/issues'
}
$expected = (Invoke-RestMethod $manifest.browser_download_url -UseBasicParsing).sha256
if (-not $expected) {
    Remove-Item $dest -Force -ErrorAction SilentlyContinue
    throw 'Release manifest has no sha256 field - not installing. Please report this at https://github.com/ItsJazii/pane/issues'
}
$actual = (Get-FileHash $dest -Algorithm SHA256).Hash.ToLower()
if ($actual -ne $expected.ToLower()) {
    Remove-Item $dest -Force -ErrorAction SilentlyContinue
    throw 'Integrity check FAILED - download does not match the release manifest SHA-256. Not installing. Please report this at https://github.com/ItsJazii/pane/issues'
}
Write-Host '> SHA-256 verified.'

Write-Host '> Installing (per-user, no admin needed)...'
Get-Process -Name pane -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 500
$proc = Start-Process -FilePath $dest -ArgumentList '/S' -Wait -PassThru
Remove-Item $dest -Force -ErrorAction SilentlyContinue
if ($proc.ExitCode -ne 0) { throw "Installer exited with code $($proc.ExitCode)" }

$exe = Join-Path $env:LOCALAPPDATA 'Pane\pane.exe'
if (-not (Test-Path $exe)) { throw 'Install finished but pane.exe was not found - please report this at https://github.com/ItsJazii/pane/issues' }

Start-Process $exe
Write-Host ''
Write-Host ("  Pane {0} installed - look for the icon in your system tray (next to the clock)." -f $release.tag_name) -ForegroundColor Green
Write-Host '  It auto-detects the AI tools you already use; add API keys via the gear icon.'
Write-Host ''
