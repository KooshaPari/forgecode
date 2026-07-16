param(
    [string]$OmniRoute = 'C:\Users\koosh\OmniRoute-clean\.github\branch-protection.main.json',
    [string]$Helios   = 'C:\Users\koosh\heliosLite-src\.github\branch-protection.main.json'
)

# Load JSON and strip the keys that a personal-account repo rejects.
foreach ($pair in @(@{p=$OmniRoute;r='KooshaPari/OmniRoute'},@{p=$Helios;r='KooshaPari/forgecode'})) {
    $path = $pair.p; $repo = $pair.r
    if (-not (Test-Path -LiteralPath $path)) { Write-Warning "missing: $path"; continue }
    $obj = Get-Content -Raw -LiteralPath $path | ConvertFrom-Json -AsHashtable

    # Drop the org-only fields for personal-account repos.
    if ($obj.ContainsKey('restrictions'))           { $null = $obj.Remove('restrictions') }
    if ($obj.ContainsKey('enforce_admins')) {
        # For personal repos, use plain admins (no team bypass list).
        $obj['enforce_admins'] = $true
    }
    if ($obj.ContainsKey('required_pull_request_reviews')) {
        $rpr = $obj['required_pull_request_reviews']
        if ($rpr -is [hashtable]) {
            if ($rpr.ContainsKey('bypass_pull_request_allowances')) {
                $bp = $rpr['bypass_pull_request_allowances']
                if ($bp -is [hashtable]) {
                    if ($bp.ContainsKey('teams')) { $null = $bp.Remove('teams') }
                    # Keep 'users' (KooshaPari) only. Drop 'apps' on personal repos.
                    if ($bp.ContainsKey('apps')) { $null = $bp.Remove('apps') }
                }
            }
        }
    }

    $body = $obj | ConvertTo-Json -Depth 20
    $tmp  = [System.IO.Path]::GetTempFileName()
    try {
        Set-Content -LiteralPath $tmp -Value $body -Encoding UTF8
        Write-Host "==> PUT $repo"
        $out = gh api "repos/$repo/branches/main/protection" -X PUT --input $tmp 2>&1
        $code = $LASTEXITCODE
        if ($code -eq 0) {
            Write-Host "  branch protection applied"
        } else {
            Write-Host "  FAILED ($code):"
            Write-Host $out
        }
    } finally {
        Remove-Item -LiteralPath $tmp -Force -ErrorAction SilentlyContinue
    }
}
