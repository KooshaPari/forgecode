#requires -Version 7
<#
.SYNOPSIS
    Trigger the publish half: cuts v0.1.0-canary.1 release tags on both
    rename branches and pushes them.  GitHub Actions picks up the release
    workflows (argismonitor-publish.yml and helios-lite-nightly.yml)
    which are wired in the renames branches from this session.

.DESCRIPTION
    Pre-conditions:
      - finish-program.ps1 has run successfully (repos exist, branches pushed).
      - apply-secrets.ps1 has wired NPM_TOKEN, CARGO_REGISTRY_TOKEN,
        HOMEBREW_TAP_TOKEN etc.

    This script is the deterministic executor for "dry-run a release"
    described in the gate-4 / gate-5 hand-offs.  It does NOT publish to
    any registry directly; it tags + pushes, then watches the actions
    runs to completion.  Run:

        pwsh -NoProfile -File forge/scripts/admin/cut-and-watch.ps1

.PARAMETER Tag
    The release tag prefix.  Defaults to v0.1.0-canary.1.

.PARAMETER Watch
    After tagging, follow the GitHub Actions runs for each repo until
    they complete (max 20 minutes).  Default: $true.
#>
[CmdletBinding(SupportsShouldProcess)]
param(
    [string]$Tag = 'v0.1.0-canary.1',
    [bool]$Watch = $true,
    [int]$TimeoutSeconds = 1200
)

$ErrorActionPreference = "Stop"
if (-not (Get-Command gh -ErrorAction SilentlyContinue)) { throw "gh not on PATH" }

function Cut-Tag([string]$repoPath, [string]$remoteRepo, [string]$branch, [string]$tag) {
    Push-Location $repoPath
    try {
        # Sanity: tag must not already exist on this remote.
        if (git tag --list | Where-Object { $_ -eq $tag }) {
            Write-Host "  ✓ local tag $tag already exists in $repoPath, skipping creation." -ForegroundColor Yellow
        } else {
            Write-Host "  → cutting local tag $tag on $branch ..."
            git tag -a $tag -m "$tag (rename release)" 2>&1 | Out-Null
            if ($LASTEXITCODE -ne 0) { throw "git tag failed in $repoPath" }
        }
        if (-not (git remote get-url $remoteRepo 2>$null)) {
            git remote add $remoteRepo "https://github.com/$remoteRepo.git"
        }
        Write-Host "  → pushing tag to $remoteRepo ..."
        git push $remoteRepo $tag 2>&1 | Out-Host
        if ($LASTEXITCODE -ne 0) { throw "git push tag failed" }
    } finally {
        Pop-Location
    }
}

Write-Host ""
Write-Host "==> Cutting $Tag on renames branches" -ForegroundColor Cyan

Cut-Tag 'OmniRoute-clean' 'KooshaPari/ArgisMonitor' 'renames/argismonitor' $Tag
Cut-Tag 'heliosLite-src'   'KooshaPari/heliosLite'  'renames/helioslite'   $Tag

if (-not $Watch) {
    Write-Host ""
    Write-Host "==> Tag pushed.  Skipping watch." -ForegroundColor Green
    return
}

Write-Host ""
Write-Host "==> Watching GitHub Actions runs" -ForegroundColor Cyan

$deadline = (Get-Date).AddSeconds($TimeoutSeconds)
$repos    = @('KooshaPari/ArgisMonitor', 'KooshaPari/heliosLite')
$seen     = @{}

while ((Get-Date) -lt $deadline) {
    $allDone = $true
    foreach ($r in $repos) {
        $runsJson = gh run list --repo $r --limit 1 --json databaseId,headBranch,name,conclusion,status,createdAt 2>$null
        if (-not $runsJson) { $allDone = $false; continue }
        $runs = $runsJson | ConvertFrom-Json
        foreach ($run in $runs) {
            $key = "$r/$($run.databaseId)"
            $last = $seen[$key]
            $state = "$($run.status)/$($run.conclusion)"
            if ($state -ne $last) {
                Write-Host ("  · {0,-32} {1,-22} {2}" -f $r, $state, $run.name)
                $seen[$key] = $state
            }
            if ($run.status -ne 'completed') { $allDone = $false }
        }
    }
    if ($allDone) { break }
    Start-Sleep 15
}

if ($allDone) {
    Write-Host ""
    Write-Host "==> All actions runs completed." -ForegroundColor Green
} else {
    Write-Host ""
    Write-Host "==> Watch timeout; check GitHub Actions manually." -ForegroundColor Yellow
}
