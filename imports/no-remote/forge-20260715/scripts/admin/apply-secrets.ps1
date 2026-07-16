#requires -Version 7
<#
.SYNOPSIS
    Wire the existing tokens from the user's ~/.env (treated as ZDR; no
    rotation) as GitHub repository secrets on KooshaPari/ArgisMonitor and
    KooshaPari/heliosLite.  This is the deterministic executor for the
    gate-3.1 "Token paste" line in ARGIS-HELIOS-PROGRAM.md.

.DESCRIPTION
    Pre-conditions:
      - `gh auth refresh -s repo,workflow,write:packages`
      - The user's `~/.env` contains (at least) NPM_TOKEN.  Other tokens
        are read on demand: CARGO_REGISTRY_TOKEN, HOMEBREW_TAP_TOKEN,
        CHOCOLATEY_API_KEY, WINGET_TOKEN, VERCEL_TOKEN,
        CLOUDFLARE_API_TOKEN.

    Tokens are assumed to be ZDR (zero data retention) and are NOT rotated
    by this script.

    Run:
        pwsh -NoProfile -File forge/scripts/admin/apply-secrets.ps1

    Or specify a secrets file:
        pwsh -NoProfile -File forge/scripts/admin/apply-secrets.ps1 -FromEnvFile ~/.env

.PARAMETER FromEnvFile
    Optional path to a local file.  Defaults to ~/.env.
    Lines of form KEY=VALUE (no quotes).

.PARAMETER DryRun
    Print the `gh secret set` commands but don't actually set any secret.
#>
[CmdletBinding(SupportsShouldProcess)]
param(
    [string]$FromEnvFile = (Join-Path $HOME '.env'),
    [switch]$DryRun
)

$ErrorActionPreference = "Stop"

if (-not (Get-Command gh -ErrorAction SilentlyContinue)) { throw "gh not on PATH" }
if (Get-Command gh -ErrorAction SilentlyContinue) {
    $status = gh auth status 2>&1 | Out-String
    if ($status -notmatch 'Logged in to') {
        throw "gh not authenticated.  Run:  gh auth refresh -s repo,workflow,write:packages"
    }
}

function Env-Or-Prompt([string]$name) {
    $v = [Environment]::GetEnvironmentVariable($name)
    if ($v) { return $v }
    if ($FromEnvFile -and (Test-Path $FromEnvFile)) {
        Get-Content $FromEnvFile | ForEach-Object {
            if ($_ -match "^$name=(.+)$") { $v = $Matches[1]; return }
        }
        if ($v) { return $v }
    }
    $sec = Read-Host "Enter value for $name"
    return $sec
}

function Set-Repo-Secret([string]$repo, [string]$secretName) {
    $val = Env-Or-Prompt $secretName
    if (-not $val) {
        Write-Host "  ! skipping $repo/$secretName (empty)" -ForegroundColor Yellow
        return
    }
    if ($DryRun) {
        Write-Host "  [dry-run] would: gh secret set $secretName --repo KooshaPari/$repo"
        return
    }
    Write-Host "  → setting $secretName on KooshaPari/$repo"
    $val | gh secret set $secretName --repo "KooshaPari/$repo" 2>&1 | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "gh secret set failed for $repo/$secretName" }
    Write-Host "  ✓ $secretName set"
}

$argiSecrets  = @('NPM_TOKEN', 'VERCEL_TOKEN', 'CLOUDFLARE_API_TOKEN')
$heliosSecrets = @('CARGO_REGISTRY_TOKEN', 'HOMEBREW_TAP_TOKEN', 'CHOCOLATEY_API_KEY', 'WINGET_TOKEN', 'VERCEL_TOKEN', 'CLOUDFLARE_API_TOKEN')

Write-Host ""
Write-Host "==> Wiring secrets to KooshaPari/ArgisMonitor" -ForegroundColor Cyan
foreach ($s in $argiSecrets) {
    try { Set-Repo-Secret 'ArgisMonitor' $s } catch { Write-Host "  ! ${s}: $($_.Exception.Message)" -ForegroundColor Yellow }
}

Write-Host ""
Write-Host "==> Wiring secrets to KooshaPari/heliosLite" -ForegroundColor Cyan
foreach ($s in $heliosSecrets) {
    try { Set-Repo-Secret 'heliosLite' $s } catch { Write-Host "  ! ${s}: $($_.Exception.Message)" -ForegroundColor Yellow }
}

Write-Host ""
Write-Host "==> Done." -ForegroundColor Green
