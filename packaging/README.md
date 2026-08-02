# HeliosLite packaging & distribution matrix (Gate 4b)

This directory holds the cross-registry packaging artifacts for HeliosLite,
the renamed binary of the KooshaPari/forgecode fork (upstream
`tailcallhq/forgecode`, MIT).

| Channel | Status | Owner | Location |
|---------|--------|-------|----------|
| crates.io (legacy `forge-dev` + new `helioslite`) | Blocked: workspace crates unpublished | KooshaPari | packaging/crates |
| Homebrew tap (`KooshaPari/tap`) | Deferred: release/checksum gate | KooshaPari | packaging/homebrew |
| Chocolatey (`helioslite`) | Deferred: Windows artifact gate | KooshaPari | packaging/chocolatey |
| winget (`KooshaPari.HeliosLite`) | Deferred: Windows artifact gate | KooshaPari | packaging/winget |
| npm (`forge-dev`, legacy) | Deferred: fork-owned registry gate | KooshaPari | upstream-registry |
| npm (`@helioslite/*`, new) | Pending | KooshaPari | future gate |
| `curl … \| sh` install | Live | KooshaPari | `install.sh` at repo root |
| `irm install.ps1 \| iex` install | In flight | KooshaPari | `install.ps1` |

## Renames matrix

| Old name | New name | Notes |
|----------|----------|-------|
| `forge-dev` (crate) | `helioslite` | additive; crate name flips at first publish |
| `forge-dev` (binary) | `helioslite` | third `[[bin]]` in `forge_main`; legacy kept |
| `KooshaPari/forgecode` (repo) | `helioslite` product alias | canonical source and release path |
| `KooshaPari/forgecode` (releases) | same | release assets live here |
| `forgecode.dev/cli` (update URL) | `helioslite.dev/cli` | additive; old still works |

See `docs/FORK.md` for the fork attribution, `docs/NOTICE.md` for upstream
license carry-over, and `docs/RENAMES-STRATEGY.md` for the additive
rename policy.
