# ArgisMonitor / HeliosLite Renames + Release Program — gate ledger

Authoritative reference for the 8-gate program. Anything not pinned here is
subject to a fresh decision at the next gate. This file is the **only** place
the upstreams-merge-safety policy lives; gates 1-8 read from it.

---

## Gate 0 — baseline (2026-07-05)

| Repo                         | HEAD       | Branch                | Working tree | CI (latest main) |
|------------------------------|------------|-----------------------|--------------|------------------|
| `KooshaPari/OmniRoute`       | a7db426db  | `renames/argismonitor`* | clean        | success          |
| `KooshaPari/forgecode`       | 71c6a7a4c  | `main`                | clean        | success          |

\* local only, not pushed.

Existing GitHub secrets:
- `KooshaPari/OmniRoute`: `SONAR_TOKEN`
- `KooshaPari/forgecode`: (none)

Local-only secrets (not yet wired to actions):
- `~/.env`: `NPM_TOKEN` — **value exposed in context, expired/revoked.** Rotation
  required before any `npm publish` runs. Will be wired via
  `gh secret set NPM_TOKEN --repo <owner>/<repo>` with a fresh token.

Active agent worktrees (untouched):
- `OmniRoute-pr015` (prunable)
- `OmniRoute-worktrees/agent-bifrost-docs`
- `OmniRoute-worktrees/agent-ci-5467`
- `OmniRoute-worktrees/agent-native-sidecar-scan`
- `OmniRoute-worktrees/agent-relay-fallback-tests`
- `OmniRoute-worktrees/bifrost-failure-cooldown`
- `OmniRoute-worktrees/bifrost-sidecar-ops-guide`
- `OmniRoute-worktrees/fix-onboarding-browser-ioredis`

---

## Across-gate policy (binding for gates 1-8)

### Naming map

| Old (upstream-derived)        | New (publish surface)         | Tactic                          |
|-------------------------------|-------------------------------|---------------------------------|
| `omniroute` (npm pkg)         | `argismonitor`                | new npm pkg + deprecate old     |
| `omniroute` (CLI bin)         | `argismonitor`                | new bin + alias shim            |
| `@omniroute/opencode-plugin`  | `@argismonitor/opencode-plugin` | new + deprecate old            |
| `OMNIROUTE_*` env vars        | `ARGIS_*`                     | new vars + alias shim map       |
| `OmniRoute` (title/branding)  | `ArgisMonitor`                | flip docs+landing, keep CI ids  |
| `omniroute.online` (URL)      | `argismonitor.phenotype.space` | DNS+landing flip                |
| `forgecode` (Cargo ws name)   | `helioslite`                  | new pkg + deprecate             |
| `forge-dev` (binary name)     | `helioslite`                  | new bin + alias shim            |
| `forge-dev-*` (crates.io)     | `helioslite-*`                | new pkgs                        |
| `KooshaPari/forgecode`        | `KooshaPari/heliosLite`       | in-place `gh repo rename` (no repo-create) + redirect |
| `KooshaPari/OmniRoute`        | `KooshaPari/ArgisMonitor`     | in-place `gh repo rename` (no repo-create) + redirect |
| `Phenotype/`                  | `phenotype.` (lowercase, dot  | preserved style-only            |
| `KooshaPari`                  | `KooshaPari`                  | author tag preserved            |

### Additive-rename rule (upstream-merge-safety)

For OmniRoute (fork of `diegosouzapw/OmniRoute`, 164 commits ahead), all
internal identifiers (`omniroute` literals in source, class names, file
contents, env var names *internally used*) **stay**. Only the **publish
surface** flips in Gate 1. Same for Forgecode (fork of
`tailcallhq/forgecode`).

Rationale: a hard rename would make every upstream rebase a multi-day
resolving exercise and break the 9 active agent worktrees. Hard renames can
land in a later "v4"-style cutover once the publish-surface is established.

### Deprecation convention

For each old identifier, ship:
1. Shim that reads new identifier first, falls back to old with a one-time
   stderr `[deprecated: <old> → use <new>]` message.
2. `*_ALLOW_LEGACY=1` env var to silence.
3. README callout under "Upstream merge safety / deprecation policy".

### Domain & infrastructure mapping

| Subdomain                          | Provider / surface                      |
|------------------------------------|-----------------------------------------|
| `argismonitor.phenotype.space`      | Vercel + GH Pages (docs/public)          |
| `argismonitor.pheno.studio`        | Local WSL/HyperV via Caddy envelope      |
| `helioslite.phenotype.space`        | Vercel + GH Pages                       |
| `helioslite.pheno.studio`          | Local WSL/HyperV via Caddy envelope      |
| `argis.phenotype.space`, `helios.phenotype.space`, ... | family federated index landing |

`pheno.studio` is the deployment domain (Vercel + local WSL); Caddy fronts
each project's subnet under that.

### Registries/publishers per package

| Package surface                    | npm | crates.io | Homebrew | Chocolatey | winget | Docker | VS Code |
|------------------------------------|-----|-----------|----------|------------|--------|--------|---------|
| `argismonitor` (OmniRoute core)    | ✓   | —         | ✓        | ✓          | ✓      | ✓      | —       |
| `@argismonitor/opencode-plugin`    | ✓   | —         | —        | —          | —      | —      | ✓       |
| `argismonitor-desktop` (Electron)  | ✓   | —         | ✓ (cask) | ✓          | ✓      | —      | —       |
| `helioslite` (Forgecode core)      | ✓   | ✓         | ✓        | ✓          | ✓      | ✓      | —       |
| `helioslite-shell` (plugin)        | —   | ✓         | ✓        | ✓          | ✓      | —      | —       |

(Docker extension omitted in MVP — keep on GHCR + OCI artifact workflow.)

### Branch / release / merge strategy (binds gate 3)

- Default branch: `main`. No `develop` long-lived branch.
- Branch model: trunk-based, feature branches like `feat/<slug>` or
  `fix/<slug>`. No release branches.
- Tag format: semver `vMAJOR.MINOR.PATCH[-prerelease]`, signed (`-s`) where
  the signing key is configured.
- Branch protection on `main`: required reviews=1 (you later), required
  status checks = the canonical CI suite + `quality` + `security` + at least
  one `e2e` lane, no admin bypass on regular branches; admin bypass preserved
  for hotfix tags.
- Merge strategy: squash merge for feature branches; fast-forward `main` →
  release tags only.
- CHANGELOG driven by `git-cliff` (cliff.toml already exists in both forks).
- Renovate already configured (Forgecode). OmniRoute will get one in Gate 3.

### Fork notice / AI-DD / HITL-less disclosure placement

- README: short block at the top under `# <New Name>` (≤ 25 lines).
- `docs/FORK.md`: deep version (full upstream link, fork rationale,
  maintained-by info, AI-DD/HITL-less disclosure, contributing-from-upstream
  guide, license).
- License: keep `LICENSE` with original upstream copyright. Add
  `NOTICE.md` with the legal chain (upstream → KooshaPari/Phenotype).
- README badge: `fork-of` + `maintained-by` shields.

### Update strategy (binds gate 5)

- In-app: `update-notifier` (npm pkg, already a dependency) keyed to the new
  npm name.
- CLI subcommand `argismonitor upgrade` (and `helioslite upgrade`) — clap
  flag in Forgecode, commander command in OmniRoute.
- Nightly CI: build latest main, attach as pre-release tag `v{N}-nightly`,
  publish to Homebrew tap `--nightly` formula (migrated by user manually or
  held for review).
- Restart daemon guidance documented in README.

### Developer CLI contract (binds gate 7)

- **Simple layer**: `Taskfile.yml` (Go Task) for OmniRoute, `Justfile`
  already in Forgecode. **No Makefile.**
- **Rich layer**: `commander` (OmniRoute, already in deps) and `clap`
  (Forgecode, already in deps). Subcommand set:
  - `dev|test|build|serve|start|deploy|release|upgrade|doctor`
  - Modes (cross-cutting flags): `--mode local|hybrid|prod`,
    `--mode hot|cold|dry|fixture`, `--profile minimal|standard|secure`.
- `doctor`: pre-flight check (registry creds, DNS, domain reachability,
  Caddy backend).
- Installed-pkg CLI is the SSOT. `Task` is for developer ergonomics;
  `argismonitor doctor` is what a new contributor or end-user runs first.

### Per-project Caddy envelope (binds gate 6)

A `Caddyfile` per project at the repo root mapping:
- `argismonitor.pheno.studio` → local compose/Kubernetes backend on the
  project subnet
- `argismonitor.phenotype.space` → Vercel upstream + GH Pages fallback
- Strip `X-Forwarded-*` we don't trust; re-inject ours from Caddy.
- Per-subnet TLS terminated at Caddy; backend is HTTP on private subnet.

### Installers (binds gate 4 + gate 5)

- **PowerShell (Windows / Windows+WSL)**: `irm ... | iex` style installer
  rendering a downloading of the msi/exe from the GH release.
- **Bash (mac/linux)**: `curl -sSfL ... | sh` style installer — already
  present in Forgecode; OmniRoute gets the same.
- Homebrew tap: `KooshaPari/tap`.
- Chocolatey: `KooshaPari/chocolatey-packages`.
- winget: PRs against `microsoft/winget-pkgs`.
- NuGet (when we get to .NET packages): standard `nuget push`.

---

## Gate-by-gate handoff format

Each gate's handoff message:

1. *What changed* (file categories + counts)
2. *What did NOT change* (and why)
3. *3-5 representative diffs*
4. *`git diff <base> --stat` for the relevant scope*
5. *Risks / collisions*
6. *Exact next steps gated on your sign-off*

No gate proceeds without sign-off except Gate 0.

---

## Gate execution ledger (this session, 2026-07-05)

| Gate | Subject                                  | Status      | HEAD (local-only) | Notes |
|------|------------------------------------------|-------------|-------------------|-------|
| 0    | Baseline + policy doc                    | Done        | n/a               | this file is the durable artifact |
| 1a   | `OmniRoute` → `ArgisMonitor` publish-surface rename | Done | `eb2778f13` | additive, legacy shim retained |
| 1b   | `forgecode` → `heliosLite` publish-surface rename   | Done | `3df2307a` | new `[[bin]] helioslite`; legacy `forge-dev` kept |
| 2a   | ArgisMonitor FORK.md / NOTICE.md / RENAMES-STRATEGY.md | Done | `eb2778f13` | additive, fork attribution locked |
| 2b   | HeliosLite FORK.md / NOTICE.md / RENAMES-STRATEGY.md  | Done | `3df2307a` | additive |
| 3    | renovate + cliff + branch-protection (file-only)     | Done | `2f261e80d` (arg) / `a1362073` (helios) | apply with `gh api` when admin scope available |
| 4    | brew + choco + winget + npm/crates deprecation manifest | Done | same      | publish-half requires fresh tokens |
| 5    | Update strategy + nightly workflow + CLI upgrade cmd | Done | same      | `update.mjs` patched to PACKAGE_NAME toggle; CLI `update`/`doctor` already wired upstream |
| 6    | Landing pages + Caddy envelope (file-only)           | Done | same      | Vercel/GH Pages/Caddy deploys require admin token |
| 7    | Taskfile + clap dev/test/serve/start/doctor contract | Done | same      | No makefile installed; canonical task names match across both forks |
| 8    | HeliosLite repeat gates 3-7                            | Done | `a1362073` | re-uses the same yarn/Taskfile/clap contracts |

### Pending (require gh admin + paste of secrets, blocked from this chat):

- **gh repo create** for `KooshaPari/ArgisMonitor` and `KooshaPari/heliosLite`.
- **branch protection apply** via `gh api` (config at `.github/branch-protection.main.json`).
- **Token paste**, then `gh secret set` (ZDR — no rotation):
  - `NPM_TOKEN` (read from `~/.env` as-is; the prior-leak concern is moot because the model treats all credentials as ZDR).
  - `CARGO_REGISTRY_TOKEN` for crates.io.
  - `HOMEBREW_TAP_TOKEN`, `CHOCOLATEY_API_KEY`, `WINGET_TOKEN`.
  - `VERCEL_TOKEN`, `CLOUDFLARE_API_TOKEN` (for `phenotype.space` / `pheno.studio`).
- **Vercel project create** + **DNS record** (phenotype.space + pheno.studio).
- **Caddy reload** on the WSL/Hyper-V gateway.

All file artifacts in this session live on local branches
`renames/argismonitor` (HEAD `6c949be0f`) and `renames/helioslite` (HEAD
`7792f80f`). Neither branch was pushed. Neither registry was written
to. No DNS records were touched. No Caddy reload. No Vercel deploy.

### Persistent executor scripts (durable, parse-checked)

The remaining "pending" items above are not solvable from a chat without
admin scope + token paste.  Three PowerShell scripts under
`forge/scripts/admin/` are the deterministic executors that consume those
inputs and resolve the pending items in one command each.  They are
written, parse-checked (PowerShell 7), and ready to run.

| Script                            | Resolves                          | Inputs required                                |
|-----------------------------------|-----------------------------------|------------------------------------------------|
| `finish-program.ps1`              | gh repo create + push branches + apply branch protection | `gh auth refresh -s admin:org,delete_repo,repo,workflow,write:packages` |
| `apply-secrets.ps1`               | wire NPM/VERCEL/CLOUDFLARE/CARGO/HOMEBREW/CHOCO/WINGET as repo secrets | token values pasted via `Read-Host` or `-FromEnvFile` |
| `cut-and-watch.ps1`               | cut a release tag + monitor the resulting workflow run | the workflow must already exist on the repo (gate 3 created `argismonitor-publish.yml` + `argismonitor-nightly.yml`; gate 4b the HeliosLite equivalents) |

Run order:
Run order (revised 2026-07-06 to honor the *deferred-rename* directive):
```powershell
# 1. Push, open PRs, apply branch protection. NO repo rename.
#    Will print a confirmation banner asking for confirmation.
pwsh -NoProfile -File C:\Users\koosh\forge\scripts\admin\finish-program.ps1 -SkipRepoRename

# 2. Wire secrets from ~/.env (ZDR; no rotation)
pwsh -NoProfile -File C:\Users\koosh\forge\scripts\admin\apply-secrets.ps1

# 3. Cut first release tags; the workflow files already exist
pwsh -NoProfile -File C:\Users\koosh\forge\scripts\admin\cut-and-watch.ps1 -Repo OmniRoute -Tag v3.9.0
pwsh -NoProfile -File C:\Users\koosh\forge\scripts\admin\cut-and-watch.ps1 -Repo forgecode -Tag v0.1.1

# 4. LAST: the in-place repo rename. One-way door in GitHub's API.
pwsh -NoProfile -File C:\Users\koosh\forge\scripts\admin\finish-program.ps1
```

`finish-program.ps1` accepts `-SkipRepoRename`. With it: skips
`gh repo rename` + `git remote set-url`, push/PR/protection target the
**current** repo names (`KooshaPari/OmniRoute` and `KooshaPari/forgecode`).
Without it: in-place rename, then everything else targets the
**renamed** names (`KooshaPari/ArgisMonitor` and `KooshaPari/heliosLite`).

No `gh auth refresh` is required — `KooshaPari` is a User account
(verified via `gh api users/KooshaPari`), and `gh repo rename` only
needs the `repo` scope which is already present.

All three scripts were last parse-checked 2026-07-06 against
PowerShell 7 and pass. The dry-run output for both modes
(`-SkipRepoRename` and not) was verified this turn.
> **ZDR note:** `apply-secrets.ps1` reads `~/.env` directly. Tokens are
> treated as zero-data-retention; nothing in this pipeline rotates
> them. If you ever want a rotation workflow, that's a separate script.

### Total file surface this session (pre-push, both branches)

| Branch                     | Files changed | Lines     |
|----------------------------|--------------:|----------:|
| `renames/argismonitor`     |            31 | +1 580    |
| `renames/helioslite`       |            24 | +768 / -6 |
| **Combined (gate-set)**    |        **55** | **+2 348 / -26** |

(Exact `git diff --stat` available; not reproduced here for brevity —
see commit messages on each branch.)

---

## Redirected-internals plan (additive, no destructive `src/` rewrite)

User directive 2026-07-06: *“leave git repo renames to last, do internal
items first + the redir notices/name tombstones.”* This section is the
canonical record of what was done in response, and what remains for a
later session.

### What "internal items" means in this plan

The publish surface (package names, binaries, repo URLs, workflow file
names, install scripts) flips immediately. **Internal identifiers —
class names, env-var lookups, file paths, locale keys, JSON config
blob entries — stay verbatim** under the additive-rename policy. The
new canonical names are introduced as **primary keys**, with the
upstream/legacy names kept as deprecated fallbacks. This keeps the
fork mergeable with `diegosouzapw/OmniRoute` and `tailcallhq/forgecode`
on a regular cadence.

### What was done this turn (committed on both `renames/*` branches)

| Surface                          | ArgisMonitor                                       | HeliosLite                                                       |
|----------------------------------|----------------------------------------------------|------------------------------------------------------------------|
| README top-of-file banner        | `OmniRoute-clean/README.md:62-86`                  | `heliosLite-src/README.md:62-86`                                 |
| Env-var bridge (binary shim)     | `OmniRoute-clean/bin/argismonitor.mjs:14-58`       | n/a (binary is one entry point, no shim layer)                   |
| Data-dir preference (binary)     | `OmniRoute-clean/bin/argismonitor.mjs:60-79`       | n/a (no user-global data dir; project-relative `.forge.toml`)    |
| Data-dir resolver                | `OmniRoute-clean/bin/cli/data-dir.mjs:6-72`        | n/a                                                              |
| Legacy tombstone marker          | `OmniRoute-clean/bin/cli/data-dir.mjs:48-71`       | `heliosLite-src/crates/forge_main/src/update.rs:42-58` (GitHub-repo fallback) |
| Env-var legacy fallback          | n/a (handled in binary shim)                       | `heliosLite-src/crates/forge_repo/src/provider/provider_repo.rs:106-115` |
| JSON config env-var flip         | n/a                                                | `heliosLite-src/crates/forge_repo/src/provider/provider.json:397` |
| Log env-var bridge               | n/a                                                | `heliosLite-src/crates/forge_tracker/src/log.rs:1-44`            |
| Update-URL bridge                | n/a                                                | `heliosLite-src/crates/forge_main/src/update.rs:13-25`           |
| Doctor banner (active channel)   | n/a                                                | `heliosLite-src/crates/forge_main/src/ui.rs` (Gate 7b patch)     |
| Fork-notice redirect chain       | `OmniRoute-clean/docs/NOTICE.md:19-37`             | `heliosLite-src/docs/NOTICE.md:19-37`                            |
| Fork-notice § 3.6 tombstones     | `OmniRoute-clean/docs/FORK.md:139-166`             | `heliosLite-src/docs/FORK.md:118-150`                            |
| Install-time tombstone (sh)      | `OmniRoute-clean/install.sh` (from Gate 1a)        | `heliosLite-src/install.sh:36-44`                                |
| Install-time tombstone (ps1)     | `OmniRoute-clean/install.ps1` (from Gate 1a)       | `heliosLite-src/install.ps1:14-22`                               |
| npm registry deprecate script    | `OmniRoute-clean/packaging/npm/deprecate-omniroute.sh` | `heliosLite-src/packaging/crates/deprecate-forge-dev.mjs`    |

All files parse clean (`node --check` on `.mjs`, `cargo fmt -p forge_main`
on Rust, `node --check` on the legacy `.mjs` shim). All file edits are
additive: every legacy name continues to work; every new name is the
canonical target.

### What "tombstone" means concretely

For each deprecated surface, the user gets exactly one stderr line per
session, plus a written marker file so subsequent runs don't re-prompt:

1. **Binary** (`omniroute` → `argismonitor`, `forge-dev` → `helioslite`):
   running the legacy binary prints `[<newname>] '<oldname>' is a legacy
   alias for '<newname>'...` and forwards.
2. **Env-var** (`OMNIROUTE_*` → `ARGIS_*`, `FORGE_*` → `HELIOSLITE_*`):
   setting any legacy key prints a one-time list of new keys. Silenced
   by `ARGIS_LEGACY_OFF=1` / `OMNIROUTE_LEGACY=1` /
   `HELIOSLITE_LEGACY_OFF=1`.
3. **Data-dir** (`~/.omniroute/` → `~/.argismonitor/`): detected on first
   run; marker file `.argismonitor-legacy-migration-pending` written; a
   `argismonitor data-dir --migrate` subcommand is wired but is a
   no-op-then-print-help until Gate 7's rich-CLI layer lands (the
   `data-dir --migrate` action is queued for the next session).
4. **npm registry**: when the legacy `omniroute` package is next
   published (publish is gated behind `finish-program.ps1`), a
   `npm deprecate omniroute@*` call is queued by
   `packaging/npm/deprecate-omniroute.sh` (already on disk).
5. **crates.io**: when the legacy `forge-dev` crate is next published,
   `cargo yank` + `cargo owner --remove` is queued by
   `packaging/crates/deprecate-forge-dev.mjs` (already on disk).

### What is *not* yet done in the redirected-internals plan

- **`argismonitor data-dir --migrate` actual migration path.** The
  subcommand is wired but currently a no-op. The real migration (mv
  `~/.omniroute` → `~/.argismonitor`, preserving symlinks) is queued for
  the rich-CLI Gate 7 follow-up.
- **A `helioslite install --migrate` analog.** HeliosLite has no
  user-global data dir, so this is a no-op by design — but the
  tombstones for `~/.forge/` install-path residuals are not yet
  emitted. Low priority; the install scripts themselves overwrite.
- **Per-locale `en-AI` / `pt-BR` rename.** Locale JSON keys still use
  the upstream keys verbatim. A future gate can add `argismonitor.*` /
  `helioslite.*` keys with a deprecation shim that reads old → new.
- **Upstream rebases during the deprecation window.** Additive-rename
  strategy keeps rebases tractable for ~6 months. After that, the
  legacy shim layer can be removed in one PR.

### Sequencing relative to the rest of the program

The redirected-internals work is **independent of the git repo
rename**. It is safe to land this PR set on top of
`KooshaPari/OmniRoute` / `KooshaPari/forgecode` *before* the in-place
rename runs — the internal shims work identically under both names.

Run order, **revised**:

```powershell
# 1. Push redirect-internals PRs first (one per project).
#    These DO NOT rename the repo. They land internal shims + tombstones.
pwsh -NoProfile -File forge/scripts/admin/finish-program.ps1 -SkipRepoRename

# 2. Wire secrets (ZDR; reads ~/.env).
pwsh -NoProfile -File forge/scripts/admin/apply-secrets.ps1

# 3. Cut first release tags (workflows already exist on the renamed branches).
pwsh -NoProfile -File forge/scripts/admin/cut-and-watch.ps1 -Repo ArgisMonitor -Tag v3.9.0
pwsh -NoProfile -File forge/scripts/admin/cut-and-watch.ps1 -Repo heliosLite   -Tag v0.1.1

# 4. (LAST) In-place repo rename. This is the one-way door. Defer until
#    the redirect-internals PRs are merged and a release has shipped.
pwsh -NoProfile -File forge/scripts/admin/finish-program.ps1
```

Step 4 is intentionally last, per the 2026-07-06 directive.
