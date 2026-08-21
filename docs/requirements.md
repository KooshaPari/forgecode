# Requirements — KooshaPari/forgecode (fork of tailcallhq/forgecode)

> Captures intent that was weak/under-specified in early fork iterations. This doc is the source of truth for “what must be true” before code is written — fixes the 5/10 completeness gap flagged in the last depth scan.

## 1. Fork identity
- **Upstream:** `tailcallhq/forgecode` `v2.13.21` (tracked in `Cargo.toml` workspace.version, `crates/forge_main/Cargo.toml`).
- **Fork versioning:** `vUPSTREAM-h.FORK` where `FORK = 0.1.N` (3-part, `h` = helioslite). Single source `crates/forge_ci/src/jobs/release_draft.rs::FORK_RELEASE_VERSION` → wired to `release-drafter` `version:` input (action input honoured, config `version:` ignored). Bump `h` on each fork release.
- **Release:** `Draft Release` (ci.yml) + `release-drafter.yml` both use `version: FORK_RELEASE_VERSION` → 36 assets, `prerelease: false`, `latest`.

## 2. Home isolation
- **Official `forge` writes:** `~/.forge/.forge.db` (`FORGE_WRITE_DB_PATH` when set).
- **`helioslite` writes:** `~/.helioslite/.forge.writes.db` (`helioslite_home` when binary is `helioslite`).
- **Sync invariant:** `~/.forge` owns upstream writes. `helioslite` watches `~/.forge/.forge.db` (poll 5s, `FORGE_SYNC_INTERVAL_SECS`, `FORGE_SYNC_DISABLED=1` to disable) and idempotently `import_forge_db` new rows only. `helioslite` also writes locally to its own home; union = `conversations_all` view.
- **Binary identity:** `ConfigReader::is_helioslite_binary()`, `forge_base_path()` vs `base_path()`.

## 3. Daemon (`forge_dbd`) — P3 single-writer
- **Transport:** Unix socket `~/.forge/.forge.db.sock` (Windows named pipe `\\.\pipe\forge-dbd-*`), `DbClient::connect` → `DbClient::send` per request.
- **Lifecycle:** first routed write spawns `forge_dbd` (from `FORGE_DBD_BIN` or `forge_dbd` on PATH), `SPAWN_ATTEMPTED` guard once-per-process, 2s poll.
- **Safety:** `DaemonWriteOutcome::{Ack,Unavailable,Indeterminate}` — only `Unavailable` (no bytes sent) may fallback to direct `inner`; `Indeterminate` (transport/error after send) is surfaced, never replayed. `write_or_fallback` enforces.
- **Protocol:** `Request::MutationV2{workspace_id: i64, mutation: ConversationMutation}` (v2, `MUTATION_PROTOCOL_VERSION=2`). Legacy `UpsertConversation/Ref/UpdateParentId/Delete` rejected with `legacy unscoped mutation` error. Health probe `Ping → Health{protocol_version, uptime, queue_depth, db_reachable}` negotiates before mutation. Inner `ConversationMutation::Upsert* {workspace_id: Option<i64>}` carried but outer `workspace_id` is authoritative; server ignores inner via `..`.
- **Storage:** `conversation_storage::persist_context` (zstd legacy envelope) → `context_zstd`/`is_compressed`/`context`/`message_count` atomically on conflict (`ON CONFLICT(conversation_id)`), `workspace_id` = `self.inner.workspace_id()` (hash of client cwd, `WorkspaceHash::new` zero-seed DefaultHasher).

## 4. Audit / Scorecard
- **Scorecard:** all workflows `permissions: contents: read` least-privilege, `PinnedDependencies` via SHA pins (ratchet). No `TokenPermissions` over-broad.
- **Supply chain:** `cargo-deny` (`advisories, bans, licenses`), `Socket Security`, `Trufflehog`, `CodeQL` all `success`. `h2 0.3.27` ignore scoped to `>=0.3.0 <0.4.0` dev-only.
- **Fmt:** `cargo fmt --all -- --check` must pass on stable (nightly `.rustfmt.toml` `unstable_features` not enforced on stable).

## 5. Non-functional / SLO
- **Perf-dashboard / otel-health / chaos-testing** are `schedule`/`workflow_dispatch` gated, not required for `main` green, but must be pinned and `contents: read`.

## 6. Acceptance
- `heliosdoctor` after install shows `forge 2.13.21-h.0.1.x` + base `~/.forge` (or `~/.helioslite` for helioslite) + FTS ok.
- `gh api repos/KooshaPari/forgecode/releases/latest` → fork assets, `forge.exe 49.1MB` verified `heliosdoctor`.
- `cargo test --workspace` 3047+ tests, `forge_dbd` daemon tests `spawn_is_attempted_once_then_falls_back`, `does_not_fallback_after_daemon_records_request_then_loses_ack` pass.
