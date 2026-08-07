# HeliosLite update strategy (Gate 5b)

This document is the source-of-truth for how the HeliosLite CLI stays
current after install. It cross-references `docs/FORK.md` and
`docs/RENAMES-STRATEGY.md` so the additive-rename policy is honored.

## Channels

HeliosLite source work currently lives in `KooshaPari/forgecode`. A local
build is not a hosted release: no fork-owned GA release, package publication,
or attestation is claimed by this document. When an authorized release is
cut, platform binaries must carry adjacent `.sha256` sidecars and installers
must verify them before executing or replacing a binary. Package-manager
channels remain deferred until fork-owned repositories and signed release
provenance are available.

| Channel  | Source                                  | Use when                                 |
|----------|-----------------------------------------|------------------------------------------|
| stable   | Planned tag/release in `KooshaPari/forgecode` | production users, only after release gates |
| rc       | Planned tag/release in `KooshaPari/forgecode` | QA, only after release gates              |
| nightly  | `helios-lite-nightly` workflow artifact | short-lived; pinned by SHA; not a release |
| legacy   | `forgecode.dev/cli`                     | legacy endpoint; availability unverified  |

Stable and rc go through `cargo-dist`-style release pipelines; nightly
runs via `helios-lite-nightly.yml`.

## Install entrypoints

| Platform  | Command                                             | Source                                    |
|-----------|-----------------------------------------------------|-------------------------------------------|
| curl \\|sh (Linux/macOS) | HeliosLite endpoint (when authorized) | `install.sh` + mandatory `cli.sha256`     |
| irm (Windows PowerShell) | HeliosLite endpoint (when authorized) | `install.ps1`                             |
| Homebrew (macOS/Linux)   | deferred                                | fork-owned tap + signed release required  |
| Chocolatey (Windows)    | deferred                                | fork-owned feed + signed release required  |
| winget (Windows)        | deferred                                | fork-owned manifest + signed release required |
| crates.io (Rust users)  | deferred                                | workspace crates must be publishable first |

## In-app update behaviour

1. On every CLI invocation we may consult `update_informer` against the
   current `KooshaPari/forgecode` repo (`HELIOSLITE_REPO` env var overrides);
   absence of a verified release is not treated as an update.
2. If `frequency = Always` and the process is in a TTY, we ask whether
   to upgrade.
3. If `--apply` was passed or `--yes` was paired with the prompt, the updater
   downloads `$HELIOSLITE_UPDATE_URL` and its adjacent `.sha256` sidecar,
   verifies the exact 64-hex SHA-256 digest, and only then executes the
   installer from a unique temporary directory. Missing or mismatched
   sidecars fail closed. The configured endpoint is `helioslite.dev/cli`, but
   its hosted availability and authorization remain unverified; keep
   `forgecode.dev/cli` only as a legacy fallback until release evidence exists.
4. If the CLI is non-interactive (CI, agent fleet, scripted install),
   the check is skipped.

Legacy `forge-dev` installs still work because `forge_main`'s `[[bin]]`
list keeps `forge-dev` as an alias of the same compiled binary.

## Nightly ratchet

A nightly workflow runs at 06:30 UTC (after the ArgisMonitor nightly so
the cross-fork pair stays consistent). It:

- Reformats and clippy-runs the entire workspace with `-D warnings`.
- Tests the entire workspace.
- Builds the renamed binary `helioslite` plus the legacy alias
  `forge-dev`.
- Uploads both binaries as workflow artifacts under
  `helioslite-nightly-<run-number>`.
- Emits a `phenomonitor://nightly?project=helioslite&date=<date>` event
  into the workspace tracker.

The nightly build does *not* publish; release publishing is gated on a
human tag-pushing a `v*` release.

## Deprecation timeline

- **Pre-GA**: No deprecation clock; legacy Forge names remain for upstream
  compatibility and HeliosLite package publication is deferred.
- **T+0**: Start only after an authorized `helioslite@1.0.0` GA release with
  signed artifacts and verified package receipts.
- **T+3/T+6/T+12 months**: Apply the deprecation policy only after that
  evidenced GA start; record each gate in the release session.
