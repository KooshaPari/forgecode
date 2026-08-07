# Renames strategy — `HeliosLite` (this fork)

> **Status (Gate 1b, additive-only):** new `helioslite` binary added;
> legacy `forge` and `forge-dev` binaries preserved; internal crate
> names verbatim. Hosted rename, package, and deprecation gates are not
> complete; no GA window has started. This document is the
> binding reference for every identifier category and its migration
> order.
>
> Companion docs: [`FORK.md`](./FORK.md) (provenance, AI-DD notice),
> [`packaging/FORGE_DEV_PACKAGING.md`](./packaging/FORGE_DEV_PACKAGING.md) (registry matrix),
> [`UPDATE-STRATEGY.md`](./UPDATE-STRATEGY.md) (upgrade flow),
> [`DEV-CLI.md`](./DEV-CLI.md) (developer CLI).

---

## 1. Why additive, not hard

A hard `forge_* -> helioslite_*` find-replace across 33 crates would
generate hundreds of merge conflicts on every upstream rebase from
`tailcallhq/forgecode`. This fork stays mergeable by adding a new
`helioslite` binary that shares the same entry point as `forge` /
`forge-dev`, and keeping every internal crate name verbatim.

## 2. Identifier categories

| # | Category | Old name | New name | Mapped in gate | Status |
|---|----------|----------|----------|----------------|--------|
| 1 | Cargo crate `forge_main` (lib name) | `forge_main` | preserved | Future | Untouched (binary added) |
| 2 | Cargo crate `forge_app`, `forge_api`, `forge_domain`, ... | preserved | unchanged | Future | Untouched |
| 3 | Bin `forge` (release) | `forge` | preserved | Gate 1b | Done (legacy bin kept) |
| 4 | Bin `forge-dev` (dev channel) | `forge-dev` | preserved | Gate 1b | Done (legacy bin kept) |
| 5 | Bin `helioslite` (canonical) | new | added | Gate 1b | Done |
| 6 | Default data dir | `~/.forge/` | `~/.helioslite/`; Forge source is read-only for import | Gate 1b | Implemented locally; importer gate pending |
| 7 | Workflow file | `release.yml` | `release.yml` remains authoritative; nightly workflow is artifact-only | Gate 4 | Pending |
| 8 | Env vars (`FORGE_*`) | preserved | unchanged | Future | Untouched |
| 9 | Domain `helioslite.phenotype.space` | new | added | Gate 6 | Pending |
| 10 | Domain `helioslite.pheno.studio` | new | added | Gate 6 | Pending |
| 11 | crates.io package (binary) | `forge-dev` (upstream naming) | `helioslite` (planned) | Gate 4 | Deferred; no publication receipt |
| 12 | Homebrew formula | n/a | `helioslite` (planned) | Gate 4 | Deferred; no authorized tap |
| 13 | Chocolatey package | n/a | `helioslite` (planned) | Gate 4 | Deferred; no authorized feed |
| 14 | winget manifest | n/a | `KooshaPari.helioslite` (planned) | Gate 4 | Deferred; no authorized manifest |
| 15 | Cargo internal identifiers (in `src/`) | preserved | unchanged | Future | Untouched |
| 16 | DB schema names | preserved | unchanged | Future | Untouched |
| 17 | Internal config keys | preserved | unchanged | Future | Untouched |

## 3. Legacy shim layer (active from Gate 1b)

### 3.1 Binary

- **Canonical**: `helioslite` (in `crates/forge_main/src/main.rs`,
  exposed as `[[bin]] name = "helioslite"` in `crates/forge_main/Cargo.toml`).
- **Legacy**: `forge`, `forge-dev` (kept in the same crate, same entry
  point). The first invocation of either prints a one-time deprecation
  notice (silence with `HELIOSLITE_LEGACY=1`).

### 3.2 Data dir

- **Canonical**: `~/.helioslite/`.
- **Legacy source**: `~/.forge/` remains owned by the standard Forge CLI.
  HeliosLite may open it only through a dedicated read-only snapshot/import
  boundary; it must not share Forge locks, WAL files, or write paths.
- **Migration**: an explicit snapshot importer (Gate 5) writes only into
  HeliosLite-owned folders. There is no implicit move or in-place migration.

## 4. New artifacts added in Gate 1b

| Path | Purpose |
|------|---------|
| `crates/forge_main/Cargo.toml` `[[bin]] name = "helioslite"` | New canonical binary. |
| `Cargo.toml` `[workspace.package]` `authors`/`homepage`/`repository`/`documentation` | Publish-surface metadata. |
| `docs/FORK.md` | Provenance, AI-DD notice, fork differences. |
| `docs/NOTICE.md` | License + trademark attributions. |
| `docs/RENAMES-STRATEGY.md` | This document. |

## 5. Diff stats (Gate 1b)

Counts of files modified / added in the `renames/helioslite` branch
versus `origin/main`:

```
modified:  Cargo.toml                                       (publish-surface metadata)
modified:  crates/forge_main/Cargo.toml                     (added helioslite [[bin]])
added:     docs/FORK.md                                     (provenance, AI-DD)
added:     docs/NOTICE.md                                   (license/trademark)
added:     docs/RENAMES-STRATEGY.md                         (this doc)
```

(Internal `crates/` source files are intentionally NOT touched in
Gate 1b — this is the additive-rename policy.)

## 6. Future gates — internal-identifier migration plan

When this fork stops pulling from upstream (decision criterion: when
the AI-DD divergence produces >50% non-trivial fork surface), the
internal identifiers flip in this order:

1. **Internal env vars** (`FORGE_*` → `HELIOSLITE_*`). Add read-side
   aliases; emit warning when legacy value is set.
2. **Cargo crate names** (`forge_*` → `helioslite_*`). All in one
   PR; CI checks that no `forge_` references remain in `Cargo.toml`
   `dependencies` blocks.
3. **Bin names** (`forge`, `forge-dev` → `helioslite` only). Remove
   the legacy `[[bin]]` entries from `Cargo.toml`.
4. **File paths under `crates/forge_*/src/`** → `crates/helioslite_*/src/`.
5. **DB schema names** (`forge_*` tables → `helioslite_*`). New columns
   added; legacy columns mirrored; read-side decides which to prefer
   via feature flag.
6. **Internal package names** (`forge_*` → `helioslite_*`) only after
   the deprecation window closes.

This sequence keeps every gate independently shippable and reversible.

## 7. End-of-life criteria for legacy aliases

The `forge` and `forge-dev` binaries, the `~/.forge/` data-dir alias,
and the `FORGE_*` env-var read-aliase are removed when **all** of:

- a verified `helioslite@1.0.0` GA release exists and its six-month window
  has elapsed,
  AND
- upstream `tailcallhq/forgecode` has not received a meaningful rebase
  in that window (decision criterion: <5 merges from upstream per
  month), AND
- no open issue on `KooshaPari/forgecode` references a blocking
  legacy-identifier problem, AND
- telemetry shows <1% of invocations use legacy identifiers.

Tracked in [`TECH_DEBT.md`](./TECH_DEBT.md) § "Legacy aliases".

## 8. Reference

- Upstream: <https://github.com/tailcallhq/forgecode>
- This fork: <https://github.com/KooshaPari/forgecode> (HeliosLite product name)
- Domain: <https://helioslite.phenotype.space>
