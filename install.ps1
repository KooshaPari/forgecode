#!/usr/bin/env pwsh
# Install ForgeCode from the canonical GitHub Release archive.
#
# Usage:
#   iwr -useb https://github.com/KooshaPari/forgecode/releases/latest/download/install.ps1 | iex
#   pwsh ./install.ps1 -Local

[CmdletBinding()]
param(
    [string]$Version,
    [switch]$Local
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"
$App = "forge-dev"
$Repository = "KooshaPari/forgecode"
$InstallDir = if ($env:FORGE_DEV_INSTALL_DIR) {
    $env:FORGE_DEV_INSTALL_DIR
} else {
    "$env:LOCALAPPDATA\ForgeCode\bin"
}

function Write-Step($message) { Write-Host "  -> $message" -ForegroundColor Cyan }
function Write-OK($message) { Write-Host "  OK $message" -ForegroundColor Green }
function Write-Err($message) { Write-Host "  ERROR $message" -ForegroundColor Red }

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null

if ($Local) {
    Write-Step "Building $App from source"
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Write-Err "cargo is not on PATH; install Rust from https://rustup.rs/"
        exit 1
    }
    Push-Location $PSScriptRoot
    try {
        cargo build --release --locked -p forge_main --bin $App
        Copy-Item -Force "target\release\$App.exe" "$InstallDir\$App.exe"
    } finally {
        Pop-Location
    }
} else {
    if (-not $Version) {
        $Version = (Invoke-RestMethod -Uri "https://api.github.com/repos/$Repository/releases/latest" `
            -Headers @{ "User-Agent" = "forge-dev-install" }).tag_name
    }
    $Asset = "$App-x86_64-pc-windows-msvc.zip"
    $BaseUrl = "https://github.com/$Repository/releases/download/$Version"
    $TemporaryDirectory = Join-Path $env:TEMP "$App-install-$Version"
    $ZipPath = Join-Path $TemporaryDirectory $Asset
    New-Item -ItemType Directory -Force -Path $TemporaryDirectory | Out-Null
    try {
        Write-Step "Downloading $Asset"
        Invoke-WebRequest -Uri "$BaseUrl/$Asset" -OutFile $ZipPath -UseBasicParsing
        Expand-Archive -Force -Path $ZipPath -DestinationPath $TemporaryDirectory
        Copy-Item -Force (Join-Path $TemporaryDirectory "$App.exe") "$InstallDir\$App.exe"
    } finally {
        Remove-Item -Recurse -Force $TemporaryDirectory -ErrorAction SilentlyContinue
    }
}

$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($UserPath -notlike "*$InstallDir*") {
    Write-Step "Adding $InstallDir to user PATH"
    [Environment]::SetEnvironmentVariable("Path", "$UserPath;$InstallDir", "User")
    $env:Path = "$env:Path;$InstallDir"
}

$ReportedVersion = & "$InstallDir\$App.exe" --version 2>&1 | Select-Object -First 1
if ($LASTEXITCODE -ne 0 -or $ReportedVersion -notmatch "^forge-dev ") {
    Write-Err "$App --version did not report the canonical identity"
    exit 1
}
Write-OK "$ReportedVersion"
Write-Host "ForgeCode installed. Try: forge-dev --help" -ForegroundColor Green
