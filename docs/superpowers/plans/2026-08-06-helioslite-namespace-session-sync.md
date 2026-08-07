# HeliosLite namespace and read-only Forge session sync

## Goal

Bring the fork to a semantically distinct **HeliosLite** product surface
without renaming upstream internals or colliding with a standard Forge
installation.  HeliosLite owns its executable/config/cache/session paths and
may consume Forge sessions only through an explicit, read-only, one-way
snapshot import.  The Forge database, WAL, locks, configuration, and binary
must never become HeliosLite write targets.

## Evidence and constraints

- `docs/RENAMES-STRATEGY.md` defines the additive rename policy: keep
  `forge_*` crate names and legacy `forge`/`forge-dev` aliases while making
  `helioslite` the canonical binary and `~/.helioslite/` the target data root.
- `docs/FORK.md` defines upstream provenance and the remaining publish-surface
  work; repository identity must not be claimed as canonical until the remote
  actually exists.
- Live Diesel schema (`crates/forge_repo/src/database/schema.rs`) uses
  `conversations.conversation_id` as the primary key.  Session rows include
  metadata plus either plain `context` or compressed `context_zstd`, selected
  by `is_compressed`; `hidden` is an integer flag.  There is no authoritative
  `conversations.id` column.
- All work is additive and preserve-first: no reset, clean, deletion, forced
  push, in-place Forge migration, shared SQLite locking, or unverified package
  publication.

## Five implementation tasks

### 1. Establish an independent HeliosLite namespace

**Files:** `crates/forge_main/src/` path/config helpers, binary metadata in
`crates/forge_main/Cargo.toml`, relevant config/help tests, and the canonical
rename/publishing docs.

Make the runtime resolve HeliosLite-owned roots (`~/.helioslite/config`,
`cache`, `logs`, `locks`, and `sessions`) by default.  Keep Forge paths as
explicit read-only compatibility inputs only; do not silently migrate or
write through them.  Keep internal crate names and legacy aliases intact.
Correct repository/domain/package references only where they are verified;
do not invent a `KooshaPari/heliosLite` remote.

**Tests:** path-resolution unit tests for default, explicit, and legacy input
paths; binary/help smoke tests; assertions that runtime writes are confined to
the HeliosLite root.

**Invariants:** standard Forge continues to use `~/.forge`; HeliosLite never
shares Forge lock/WAL files; no hard `forge_*` crate rename is introduced.

### 2. Define a versioned, read-only session snapshot contract

**Files:** new session-sync module under `crates/forge_main/src/` (or the
existing persistence boundary), `crates/forge_repo/src/database/schema.rs`
adjacent adapters, and a manifest/format document under `docs/`.

Specify a snapshot manifest containing source path, source SHA-256, schema
version, export timestamp, row count, and importer version.  Read Forge SQLite
with SQLite read-only/immutable semantics and select by `conversation_id`.
Preserve metadata and context payloads without interpreting or mutating Forge
rows; support both `context` and `context_zstd` according to `is_compressed`.
Reject unknown/incompatible schemas rather than guessing columns.

**Tests:** fixture databases for plain and compressed contexts, hidden rows,
null metadata, duplicate IDs, malformed compression, and schema mismatch;
manifest hash/row-count verification tests.

**Invariants:** source database mtime, bytes, WAL, and locks are unchanged;
IDs remain stable; no `conversations.id` query exists; failed reads produce no
partial destination.

### 3. Implement an explicit one-way import into HeliosLite storage

**Files:** session CLI command wiring (for example
`crates/forge_main/src/cli.rs`), importer/storage implementation, and command
docs.

Add an explicit command such as
`helioslite sessions import-forge --source <path> --dest <path>`.
Validate source and destination ownership, create only HeliosLite directories,
write a temporary destination plus manifest, fsync/atomically rename within the
HeliosLite root, and make repeated imports idempotent.  Default destination
must be under `~/.helioslite/sessions`; an explicit destination outside that
root requires a safe, documented refusal rather than a bypass.

**Tests:** CLI parsing, read-only source open, atomic failure cleanup,
idempotent re-import, source mutation checks (hash/mtime), destination path
rejection, and interruption/restart behavior.

**Invariants:** import is opt-in and one-way; no live view, symlink, bind mount,
shared WAL, or write-back path; source remains usable by the standard Forge CLI.

### 4. Complete the remaining rename and release surfaces

**Files:** `docs/RENAMES-STRATEGY.md`, `docs/FORK.md`, release workflow and
installer manifests, package metadata, update/doctor output, and any existing
HeliosLite tests.

Audit every remaining publish surface (GitHub release identity, crates.io,
Homebrew, Chocolatey, winget, domains, installers, update URLs, and legacy
tombstones).  Implement only surfaces backed by an existing authorized remote
and signing/checksum evidence.  Keep legacy Forge aliases during the stated
deprecation window and document each unresolved gate instead of publishing or
redirecting speculatively.

**Tests:** static namespace scans, installer checksum tests, update-source
selection tests, doctor/banner tests, workflow YAML validation, and package
metadata checks.

**Invariants:** no package is called deployed without receipt plus provenance;
upstream sync remains mergeable; legacy aliases cannot write HeliosLite state.

### 5. Verify, preserve, and hand off the migration

**Files:** session documentation under `docs/superpowers/` and the existing
session/known-issues records; no source deletion.

Run focused and full applicable tests, format/lint checks, schema probes, and
an end-to-end import against a disposable copied fixture (never the live Forge
database).  Record exact commands, SHAs, package receipts, unresolved gates,
and preservation refs.  Push an additive preservation branch/PR when changes
are ready; do not merge, archive, or delete any checkout without independent
remote and restoration evidence.

**Tests/checks:** `cargo test` for affected crates, `cargo fmt --check`,
`git diff --check`, source/destination hash comparison, `git fsck --full`, and
manual invocation of both `forge` and `helioslite` against isolated roots.

**Invariants:** every artifact remains recoverable; verification evidence is
reproducible; release readiness is reported separately from local green tests;
no destructive cleanup is part of this plan.

## Completion gate

The work is complete only when HeliosLite can run with an independent namespace,
an explicit import produces a verified local snapshot without changing Forge,
all intended rename/release surfaces have authoritative evidence, and every
remaining gap is named with an owner and next action.  A passing unit test or a
preservation snapshot alone is not release or deployment proof.
