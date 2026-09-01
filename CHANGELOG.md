# Changelog

All notable changes to **helioslite** (formerly `forgecode` / `forge-dev`),
the AI-enhanced terminal development environment, are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html)
on the **v2.13.x-h.0.1.x** axis for fork releases, and **v2.x.y** for upstream-synced
line releases of the `KooshaPari/forgecode` fork of `tailcallhq/forgecode`.

> **Fork lineage.** This is the Phenotype fork of
> [`tailcallhq/forgecode`](https://github.com/tailcallhq/forgecode) (MIT/Apache-2.0),
> now published under the canonical name `helioslite`. Legacy aliases (`forge-dev`,
> `forgecode` workspace, `FORGE_*` env vars) remain supported for the duration of
> the 6-month migration window described in `docs/RENAMES-STRATEGY.md`.

## [Unreleased]

### Added
- **F3 episodic semantic-memory contract** for the `forge_domain` layer, defining
  long-term recall semantics across agent sessions (#216).
- **New P2/P3 crates:** `forge_sandbox` (Landlock-runtime sandbox hook),
  `helios-bot` (CLI scaffolding with webhook hooks), and an `E2E` test harness
  crate driving real agent loops against mock providers.
- **Agent SDK integration module** wiring sandbox hook + E2E scenarios into
  the public `forge-domain` SDK surface.
- **E2E agent-loop test** with mock LLM provider verifying the full
  request → tool-call → response cycle.

### Changed
- **README badges:** added the `sladge.net` "AI slop inside" badge and the
  GitHub downloads (all assets, all releases) counter badge (#225).
- **Benchmarks CI:** upgraded `actions/cache` to `v4` to match the rest of
  the workflow stack (#224).

### Fixed
- `helioslite_helper` now **inspects parent wait errors** instead of
  discarding them (#221).
- `TaskPriority` switched to `#[derive(Default)]`, eliminating a manual
  `Default` impl that triggered nightly clippy (#220).
- **Multi-file E2E scenario step count** corrected to match the actual
  number of agent-loop iterations (#217).
- `forge-sdk` doctests updated to match the real public API signatures (#214).
- `forge-repo` daemon-spawn test made portable across Unix and Windows (#213).
- Replaced a nonexistent `scenario.run()` call in the E2E doctest with the
  real `into_mock_llm()` constructor.
- `helios-bot` no longer uses unchecked `Command::args` for CLI invocation
  and avoids `as` slices on user-supplied counts (#211).
- `helios-bot` webhook module annotated `#[allow(dead_code)]` to keep the
  `trunk check` gate green.
- Clippy `-D warnings` cleared in the new `helios-bot` and `forge_sandbox`
  crates, including declaration of the `landlock-runtime` feature flag.
- Test items in `forge_dbd::client`, `forge_repo::daemon_repo`, and
  `forge_services::fd` are now gated with `#[cfg(unix)]` so the workspace
  builds on Windows again.

## [v2.13.21-h.0.1.4] - 2026-08-27

### Added
- **CycloneDX SBOM publishing** integrated into the release pipeline via
  `softprops/action-gh-release` (pinned to SHA `3bb12739` / `v2.6.2`) and
  `anchore/sbom-action` (#180).
- **Manual re-run capability** (`workflow_dispatch`) added to the
  `scorecard` workflow.
- **DORA metrics, ADRs, Docker dev env, and incident-response playbook**
  to formalise the SRE/DevEx pillar (`feat(devex)` #747fd4e).
- **Helioslite repo synthax benchmarks** for similarity, syntax, and config,
  surfaced via `cargo bench` on the criterion harness.
- **DAEMON write replay guard** prevents silent re-application of an ack-lost
  MutationV2 upsert (#163, hardened further in #165).
- **Workspace-id stamping** on every daemon-routed upsert to provide
  per-tenant isolation across multiple `heliosdoctor` clients.

### Changed
- **CI Scorecard hardening:** every Scorecard workflow now has top-level
  `permissions: contents: read`, pinned dependencies, and least-privilege
  bindings to close the `PinnedDependencies` and `TokenPermissions` alerts.
- **Dockerfile.dev base image** pinned to `python:3.12-slim-bookworm@sha256:0f5b…` for
  reproducible Scorecard builds; RUN packages are also pinned by SHA.
- **Python dependency pinning:** all `scorecard` extras now live in
  `requirements/dev.txt` with `--hash=sha256` entries.
- **Scorecard Scorecard workflow** ignores `__pycache__/`, `*.pyc`, `*.pyo`,
  `*.pyd`, and `.pytest_cache/`; tracked `*.pyc` artefacts were removed with
  `git rm --cached`.
- **CHANGELOG/security notes** document the documented branch-protection
  exception for `h.0.1.4` Scorecard bypasses (#207).
- **BackgroundTasks** API hardened: `pub(crate)` constructor, drop-before-sleep
  shutdown ordering, and per-task timeout to eliminate the
  "future dropped before poll" panics seen on slow provider responses.

### Fixed
- **Test code** for `forge_dbd::client`, `forge_repo::daemon_repo`, and
  `forge_services::fd` re-gated with `#[cfg(unix)]`; the `super::*` import in
  `forge_services::fd` got the same treatment.
- `helioslite_helper::update` no longer produces indexing-slicing lints and
  uses `derive(Default)` for `TaskPriority`.
- `helioslite_helper::update` long-line formatting brought under
  `rustfmt::stable`.
- `helioslite_helper::update` conversation_repo formatting brought under
  `rustfmt::stable` (a single-line `UpsertConversation` was previously
  mis-wrapped).

### Security
- `h2` crate bumped from `0.4.13` to `0.4.16` to address **RUSTSEC-2026-0258**;
  `deny.toml` ignore for `RUSTSEC-2026-0258` was reversed on the new advisory
  version line and is no longer required for `cargo-deny 0.19`.

## [v2.13.21-h.0.1.3] - 2026-08-25  (swap-14)

### Added
- **Release drafter** regenerated for the v2.13.21-h.0.1.3 / `ci.yml`
  release model so the `swap-14` automated PR ships with the right
  `tag_name` and `overwrite_files` parameters.

### Changed
- Release artefacts now flow through the **single-source `forge_ci` model**,
  replacing the previous xresloader-based publisher.

### Fixed
- Release workflow no longer passes an unused `release_id` binding.
- `tag` → `tag_name` and `overwrite` → `overwrite_files` parameter renames
  applied to match `softprops/action-gh-release`'s v2 schema.
- `heliosdoctor`/`helioslite_helper` clippy `-D warnings` cleared.

## [m4-20260821] - 2026-08-21  (interim milestone)

### Fixed
- Removed an invalid `bench` section from the Cargo virtual manifest that
  caused `cargo metadata` to fail under newer toolchains.

## [v2.13.21-h.0.1.1] - 2026-08-19

### Added
- **Daily-fork sync machinery:** `feat(sync): continuous forge → helioslite
  upstream sync` plus a refreshed `forge.schema.json` (compression-strategy
  description wrapped to 80 columns).
- **Release-please config, dependabot auto-merge, auto-assign reviewers,
  CODEOWNERS, and `pytest.ini`** to close the last "missing gates" PR.
- **User-oriented 32-pillar scorecard** (utility/usability/expandability).
- **Makefile, `pyproject.toml`, `conftest.py`, and pre-commit CI workflow**
  for a uniform Python-side developer experience.
- **Multi-lane Scorecard / coverage ratchet** workflows (#169):
  * `scorecard` on PR + main, `coverage` with PR comments and main-branch
    auto-update.
  * `branch-cleanup` workflow with `actions/checkout` step before `gh api`
    calls.
- **Helioslite accessibility module:** WCAG 2.1 AA helpers, screen-reader
  descriptions, contrast checking, and semantic validation
  (`feat(a11y)`, #7-1).
- **Internationalisation scaffolding** with English locale + accessibility
  CLI flag (`--a11y`).
- **Integration tests, release-please, SBOM workflow, and 3 new locales**
  (`feat(integration)` + i18n follow-ups).
- **CI fuzz tests, three new locales, and CODEOWNERS verification** job
  (`feat(testing)` #c72c64c).
- **SLA/SLO documentation + perf trend tracking** and a full **multi-region
  deployment guide** (`feat(docs)`, `feat(infra)`).
- **OpenTelemetry collector config + chaos-testing guide + perf dashboard**
  (`feat(infra)`, #eb13f30).
- **SRE pillar:** SLO burn-rate alerting, OTel deployment scripts, Terraform
  IaC validation CI, chaos CI gate, and OTel collector deployment workflow
  (`feat(sre)` #10e2c90 / #d39762f / #747fd4e).
- **OTel deployment workflows, terraform plan, and slo-monitor fix**
  (`chore(ci)` #bbaf054).

### Changed
- `release-drafter` pinned to the **fork version scheme
  v2.13.21-h.0.1.1**; the upstream-2.13.22 pin was retained as a fallback.
- Tooling rustfmt invocation moved to **nightly rustfmt** for import-grouping
  and comment-wrapping parity with upstream.
- Helioslite environment variables start migrating from `FORGE_*` to
  `HELIOSLITE_*`; legacy aliases remain active for the migration window.

### Deprecated
- The `--config-flag` aliases that duplicated the `FORGE_*` short forms
  (use the long form, or set `HELIOSLITE_*` directly).

### Fixed
- **Daemon write replay after ack loss** (#163) plus a follow-up to scope
  daemon deletes to the workspace and to add `workspace_id` to the
  `UpsertConversation` test initializers.
- **Anthropic provider:** refusal handling is now correctly scoped to
  Anthropic-specific responses (#166) instead of firing on every provider.
- **Workspace status test** gated on Unix-path only (#75fd926); equivalent
  Windows twin added for `forge_domain` snapshot absolute-path coverage.
- `TRACE` and shell-side path tests made **Windows-safe** in
  `forge_pheno_shell`, `forge_repo_map`, `forge_walker`, and `forge3d`.
- **zsh execution** bounded so `doctor` and `setup` can no longer hang the
  CLI indefinitely (was relying on a 0-second timeout that silently
  succeeded).
- **doctor/setup pwd tasks** routed via `cmd /c cd` on Windows so dual-harness
  runs find the project root.
- **Coverage and pre-commit prerequisites** restored on main (#169).
- **criterion / bench harness:** previously incompatible Trunk action was
  removed (#171); action pins restored to immutable SHAs (#173);
  chaos-workflow dispatch validated (#174).

## [v2.13.21] - 2026-08-16  (line release)

### Added
- **forge_dbd in the release matrix and CI** so the conversation-write daemon
  ships alongside `forge` and `helioslite` on every release (#176).
- **FTS5 unicode61 full-text search** with `remove_diacritics=2` migration for
  the conversation corpus (`feat(repo)` M3) plus a per-column `highlight()`
  helper for full-column markup rendering (#185).
- **M3 release deliverables:** GitHub webhooks, notifications, markdown
  rendering, custom conversation fields, and export.
- **M2 deliverables:** sprint management, burndown chart, velocity tracking,
  label management, and enhanced filters (`feat(sprint)`).
- **M1 deliverables:** Tracera desktop shell (Tauri 2), project board, and
  system tray (`feat(desktop)`).
- **Programmatic/semantic/AI-based compression, prune, and truncation hooks**
  layered on top of the upstream `forgecode` conversation engine.
- **`helioslite_helper::update`** Windows self-update binary in `forge_main`
  (#192).

### Changed
- **Authoritative version metadata** aligned to **v2.13.21** across the
  workspace (`fix(release)` #7469c73).
- **CI workflow reliability** hardened with `daemon-routed upserts` honouring
  the new `workspace_id` field, and the daemon's named-pipe transport now
  serves concurrent clients.

### Fixed
- `cargo-deny` failures surfaced by the v2.13.21 sync resolved (`fix(ci)`
  #cf3a3ef).
- Concurrent named-pipe clients + per-client `workspace_id` honoured by
  `forge_dbd` (#72793d8).
- `helioslite` snapshot import boundary hardened against path traversal
  (#160).

## [v2.10.8] - 2026-08-09

### Added
- **Platform-specific test matrix** (macOS + Windows) in CI (#198).
- **Criterion benchmarks** for `forge_domain`, `forge_config`, and
  `forge_syntax` covering similarity, syntax, and config load paths
  (`perf(bench)` #cda42d4).

### Changed
- **CI rustfmt contract** restored to `stable` (#199).

### Fixed
- `forge test` command compilation fixed end-to-end (#197) so the
  `forge --test` shorthand accepts portable test-command checks.
- Auto-repair hook for post-edit test/lint verification shipped clean on
  portable paths (#191).
- `BackgroundTasks::new_for_test` now exposed for downstream integration
  tests; collapsible-if and `match_single_binding` clippy lints cleaned up.

## [v2.10.7] - 2026-08-07

### Added
- **forge_dbd (opt-in) hot-path conversation writer:** real conversation
  writes, lib + bin targets, named-pipe transport, and an `Idle shutdown +
  spawn-on-first-write` daemon lifecycle.
- **forge_dbd CLI flags:** `--version` and `--help` now short-circuit without
  binding the daemon socket (previously `forge_dbd --version` hung on
  startup).
- **forge_dbd workspace forwarding:** client `spawn_daemon` forwards
  `FORGE_DBD_SOCKET` to the child so both ends bind the same pipe.
- **forge_dbd split-DB CLI integration test** seeds a legacy DB, lets the
  real binary create and migrate the write DB, then asserts
  `heliosdoctor --verbose` and `--integrity-only` porcelain output.
- **`helioslite forget` subcommand** with `--id`, `--source` (e.g.
  `imported:forge`), and `--age` selectors.
- **`helioslite export --format jsonl|csv`** for off-system consumption;
  exporter skips agent-launched rows by default (`--include-agent` to
  override).
- **`helioslite migrate`** atomic `~/.forge → ~/.helioslite` importer with
  `--dry-run`.
- **Shell completion** for `import`, `export`, `heliosdoctor`, and `migrate`
  via `clap_complete`.
- **Helioslite README** documenting the CLI surface, migration, and updater.
- **.github/workflows/release.yml** for the `heliosLite` Windows + Linux
  matrices.
- **`indexmap` serde feature** restored after the upstream sync (#144).
- **forge_main install/upgrade hardening** (`fix(updater)` #130) — installer
  verifies versions, checksums, and release metadata and fails closed when
  any of them cannot be verified.
- **doctor verification hardening** (#139) — propagates doctor failures,
  redacts API keys in info output, and fails closed on installer integrity
  errors.

### Changed
- **Infisical workflow** permissions and pins hardened (#142).
- **i18n locale files** added for English + 3 additional locales, with
  performance baselines captured alongside the a11y CLI flag.
- **Helioslite doctor and installer** no longer silently fall through when
  verification fails; errors propagate as non-zero exits.

### Fixed
- **Concurrent named-pipe clients** now served correctly by `forge_dbd`
  (multi-client regression resolved).
- **Split-DB read path** verified end-to-end through the real binary.
- **`-D warnings` clippy gate** cleared in `forge_app`, `forge_infra`,
  `forge_repo`, and `forge_main` (`ae4096a`).
- **CRLF in embedded tool descriptions** normalised so that `tool-macros`
  produces identical output on Windows and Unix line endings.

## [v2.10.6] - 2026-08-04

### Changed
- **ZSH plugin, setup, doctor, and standalone theme** now default to the
  canonical `helioslite` executable while continuing to honour the
  `FORGE_BIN` override (`fix(shell)`).
- **CycloneDX SBOM artefacts** refreshed using `cargo-cyclonedx 0.5.9` from
  the current lockfile (#134).
- Dependency bumps: `similar` → `3.1.2`, `nucleo-picker` → `0.11.2`,
  `ignore` → `0.4.33`, `base64` → `0.23.1` (`chore(deps)` #3810-#3816).

### Fixed
- 10/10 shell-dispatch regression coverage pinned for the new canonical
  `helioslite` default.

## [v2.10.5] - 2026-08-03

### Added
- **Release-asset attestation** for every published binary (#131) — every
  release asset is now signed by the CI attestation step before upload.
- **Updater release pinning** to matrix assets and install.sh hardening
  (#130).

### Fixed
- Indexing-slicing lint avoided in `release_asset_url` version check.

## [v2.10.4] - 2026-08-03

### Fixed
- Scheduled `Bounty Management` PR sync skipped on the upstream-sync branch
  to prevent `--pr` failure when the upstream merge is empty.
- Renovate dependency bumps (`tsx 4.23.4 → 4.23.5`, `csv-parse 7.0.2`)
  brought in from upstream.

## [Earlier]

For changes predating the v2.10.x fork cutover (helioslite rebrand,
SQLite split-DB session store, FTS/vector search, sub-agent breadcrumbs,
Phenotype overlays), please consult:

- `docs/RENAMES-STRATEGY.md` — fork rebrand and rename history.
- `docs/FORK.md` — fork attribution and divergence list.
- `docs/CHANGELOG-ARCHIVE.md` — pre-fork upstream history mirrored from
  `tailcallhq/forgecode`.

---

## Versioning notes

- `v2.10.x` — primary line release. `x` is incremented on every
  documented fork release that ships to crates.io / the GitHub release
  channel.
- `v2.10.x-h.0.1.y` — *upstream-sync line* versions. The `v2.10.x` portion
  tracks the most recent upstream `tailcallhq/forgecode` import, while
  `h.0.1.y` is the fork's Phenotype overlay counter.
- The 200-commit snapshot covers **2026-08-09 → 2026-08-31** and represents
  the bulk of the v2.10.4 → v2.13.21-h.0.1.4 cycle plus the start of the
  next fork release (`h.0.1.5`).

[Unreleased]: https://github.com/KooshaPari/forgecode/compare/v2.13.21-h.0.1.4...HEAD
[v2.13.21-h.0.1.4]: https://github.com/KooshaPari/forgecode/compare/v2.13.21-h.0.1.3...v2.13.21-h.0.1.4
[v2.13.21-h.0.1.3]: https://github.com/KooshaPari/forgecode/compare/v2.13.21-h.0.1.1...v2.13.21-h.0.1.3
[m4-20260821]: https://github.com/KooshaPari/forgecode/compare/v2.13.21-h.0.1.1...m4-20260821
[v2.13.21-h.0.1.1]: https://github.com/KooshaPari/forgecode/compare/v2.13.21...v2.13.21-h.0.1.1
[v2.13.21]: https://github.com/KooshaPari/forgecode/compare/v2.10.8...v2.13.21
[v2.10.8]: https://github.com/KooshaPari/forgecode/compare/v2.10.7...v2.10.8
[v2.10.7]: https://github.com/KooshaPari/forgecode/compare/v2.10.6...v2.10.7
[v2.10.6]: https://github.com/KooshaPari/forgecode/compare/v2.10.5...v2.10.6
[v2.10.5]: https://github.com/KooshaPari/forgecode/compare/v2.10.4...v2.10.5
[v2.10.4]: https://github.com/KooshaPari/forgecode/releases/tag/v2.10.4
