# forgecode OTel Collector - Production Deployment Script (Windows)
# Usage:
#   .\deploy.ps1 [up|down|logs|status|restart]

param(
    [ValidateSet("up","down","logs","status","restart")]
    [string]$Action = "up"
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ComposeFile = Join-Path $ScriptDir "docker-compose.prod.yml"
$EnvFile = Join-Path $ScriptDir ".env"

function Write-Log { param([string]$Message) Write-Host "[forgecode-otel] $Message" -ForegroundColor Green }
function Write-Warn { param([string]$Message) Write-Host "[forgecode-otel] $Message" -ForegroundColor Yellow }
function Write-Err  { param([string]$Message) Write-Host "[forgecode-otel] $Message" -ForegroundColor Red }

function Test-Dependencies {
    if (-not (Get-Command docker -ErrorAction SilentlyContinue)) {
        Write-Err "docker is required but not found on PATH"
        exit 1
    }
    # Check for docker compose v2
    try {
        docker compose version 2>$null | Out-Null
    } catch {
        Write-Err "docker compose (v2 plugin) is required"
        exit 1
    }
}

function Ensure-EnvFile {
    if (-not (Test-Path $EnvFile)) {
        Write-Warn ".env file not found. Copying from .env.example"
        Copy-Item (Join-Path $ScriptDir ".env.example") $EnvFile
        Write-Warn "Edit $EnvFile with your production settings before starting."
        exit 1
    }
}

function Start-Stack {
    Ensure-EnvFile
    Write-Log "Starting forgecode OTel production stack..."
    docker compose -f $ComposeFile --env-file $EnvFile up -d
    Write-Log "Stack started. Services:"
    docker compose -f $ComposeFile ps
}

function Stop-Stack {
    Write-Log "Stopping forgecode OTel production stack..."
    docker compose -f $ComposeFile --env-file $EnvFile down
    Write-Log "Stack stopped."
}

function Show-Logs {
    docker compose -f $ComposeFile --env-file $EnvFile logs -f --tail=100
}

function Show-Status {
    Write-Log "Service status:"
    docker compose -f $ComposeFile ps
}

function Restart-Stack {
    Write-Log "Restarting forgecode OTel production stack..."
    Stop-Stack
    Start-Stack
}

# Main
Test-Dependencies
Set-Location $ScriptDir

switch ($Action) {
    "up"      { Start-Stack }
    "down"    { Stop-Stack }
    "logs"    { Show-Logs }
    "status"  { Show-Status }
    "restart" { Restart-Stack }
}
