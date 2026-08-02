#!/usr/bin/env pwsh
# install.ps1 — Install HeliosLite (formerly Forgecode) on Windows / PowerShell
#
# Usage:
#   iwr -useb https://helioslite.phenotype.space/install.ps1 | iex
#
#   # Pin a specific version:
#   iwr -useb https://helioslite.phenotype.space/install.ps1 | iex - -Version 1.2.3
#
#   # Local install (no download): run from repo root
#   pwsh ./install.ps1 -Local
#
# Installs the HeliosLite CLI as a single-binary `helioslite` on PATH.
# On Windows we download the matching raw `forge-*.exe` release binary from
# GitHub Releases and install it as `helioslite.exe`.

[CmdletBinding()]
param(
    [string]$Version,
    [switch]$Local,
    [switch]$SkipForgeAlias,
    [switch]$SkipUpdateCheck
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

function Write-Step($msg) { Write-Host "  → $msg" -ForegroundColor Cyan }
function Write-OK($msg)   { Write-Host "  ✓ $msg" -ForegroundColor Green }
function Write-Warn($msg) { Write-Host "  ⚠ $msg" -ForegroundColor Yellow }
function Write-Err($msg)  { Write-Host "  ✖ $msg" -ForegroundColor Red }

# 1) Resolve target version
$ReleaseRepo = if ($env:HELIOSLITE_RELEASE_REPO) { $env:HELIOSLITE_RELEASE_REPO } else { "KooshaPari/forgecode" }
$ReleasesApi = "https://api.github.com/repos/$ReleaseRepo/releases"
if (-not $Version -and -not $Local) {
    try {
        $relJson = Invoke-RestMethod -Uri "$ReleasesApi/latest" -Headers @{ "User-Agent" = "helioslite-install" }
        $Version = $relJson.tag_name.TrimStart("v")
    } catch {
        Write-Warn "Could not determine latest version from GitHub — falling back to v0.1.0."
        $Version = "0.1.0"
    }
}
$Version = $Version.TrimStart("v")
Write-Step "Target version: $Version"

# 2) Pick install location
$InstallDir = if ($env:HELIOSLITE_INSTALL_DIR) { $env:HELIOSLITE_INSTALL_DIR } else { "$env:LOCALAPPDATA\helioslite\bin" }
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null

if ($Local) {
    Write-Step "Local install — building from source via cargo..."
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Write-Err "cargo not on PATH — install rustup: https://rustup.rs/"
        exit 1
    }
    Push-Location (Resolve-Path "$PSScriptRoot")
    try {
        cargo build --release --bin helioslite
        Copy-Item -Force "target\release\helioslite.exe" "$InstallDir\helioslite.exe"
    } finally {
        Pop-Location
    }
} else {
    $Asset = "forge-x86_64-pc-windows-msvc.exe"
    $Url   = "https://github.com/$ReleaseRepo/releases/download/v$Version/$Asset"
    $Tmp   = Join-Path $env:TEMP "helioslite-install-$Version"
    New-Item -ItemType Directory -Force -Path $Tmp | Out-Null

    Write-Step "Downloading $Url"
    $BinaryPath = Join-Path $Tmp "helioslite.exe"
    try {
        Invoke-WebRequest -Uri $Url -OutFile $BinaryPath -UseBasicParsing
    } catch {
        Write-Err "Download failed: $_"
        exit 1
    }

    $ChecksumPath = Join-Path $Tmp "helioslite.exe.sha256"
    try {
        Invoke-WebRequest -Uri "$Url.sha256" -OutFile $ChecksumPath -UseBasicParsing
    } catch {
        Write-Err "Release checksum is unavailable; refusing an unverified binary."
        exit 1
    }
    $ExpectedSha = (Get-Content $ChecksumPath | ForEach-Object { ($_ -split '\s+')[0] } | Where-Object { $_ } | Select-Object -First 1)
    if ($ExpectedSha -notmatch '^[0-9a-fA-F]{64}$') {
        Write-Err "Invalid SHA-256 checksum format."
        exit 1
    }
    $ActualSha = (Get-FileHash -Algorithm SHA256 -Path $BinaryPath).Hash
    if ($ExpectedSha.ToLowerInvariant() -ne $ActualSha.ToLowerInvariant()) {
        Write-Err "SHA-256 verification failed."
        exit 1
    }
    Write-OK "SHA-256 verified"

    Copy-Item -Force $BinaryPath "$InstallDir\helioslite.exe"
    Remove-Item -Recurse -Force $Tmp
}

# 3) PATH
$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($UserPath -notlike "*$InstallDir*") {
    Write-Step "Adding $InstallDir to user PATH"
    [Environment]::SetEnvironmentVariable("Path", "$UserPath;$InstallDir", "User")
    $env:Path = "$env:Path;$InstallDir"
}

# 4) Optional: legacy `forge`/`forge-dev` alias
if (-not $SkipForgeAlias) {
    foreach ($old in @("forge", "forge-dev")) {
        $oldPath = Join-Path $InstallDir "$old.exe"
        $newPath = Join-Path $InstallDir "helioslite.exe"
        if (-not (Test-Path $oldPath)) {
            Copy-Item -Force $newPath $oldPath
            Write-OK "Created legacy alias $oldPath"
        }
    }
}

# 5) Verify
$Ver = & "$InstallDir\helioslite.exe" --version 2>&1 | Select-Object -First 1
if ($LASTEXITCODE -ne 0) {
    Write-Err "helioslite --version failed."
    exit 1
}
Write-OK "helioslite reports: $Ver"

Write-Host ""
Write-Host "  🎉 HeliosLite installed." -ForegroundColor Green
Write-Host "     Try:  helioslite --help" -ForegroundColor Green
Write-Host "     Docs: https://helioslite.phenotype.space" -ForegroundColor Green
Write-Host "     Old commands still work: forge / forge-dev (deprecated)" -ForegroundColor DarkGray
