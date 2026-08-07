# HeliosLite namespace and read-only Forge session sync

**Status:** design approved for implementation planning; no implementation is
included in this document.

## Context and evidence

The fork already contains additive `helioslite` binary and packaging work on
the `fork/renames/helioslite` line. The actual GitHub remote remains
`KooshaPari/forgecode`; `KooshaPari/heliosLite` is referenced by some existing
documents but is not an existing canonical remote. The fork must therefore
keep `KooshaPari/forgecode` as its source/release remote until a real remote is
created and verified.

The standard Forge installation owns `~/.forge`, including `~/.forge/.forge.db`.
The live SQLite schema uses `conversations.conversation_id` as the primary key
and stores session content in either plain `context` or compressed
`context_zstd`, selected by `is_compressed`. Consumers that query
`conversations.id` are incorrect. Existing `workspace sync` concerns workspace
registration and is not a session-import mechanism; it must not be reused for
this boundary.

## Goals

1. Give the Phenotype product a distinct `helioslite` namespace and owned
   filesystem paths without changing standard Forge state.
2. Allow HeliosLite to consume Forge sessions through an explicit,
   read-only, one-way snapshot import.
3. Preserve provenance, schema identity, source hashes, and repeatable
   idempotent imports.
4. Make the release/repository naming policy truthful and collision-free.

## Non-goals

- No automatic migration, deletion, renaming, or in-place conversion of
  `~/.forge`.
- No shared Forge/HeliosLite SQLite WAL, journal, lock, config, cache, or
  executable path.
- No bidirectional sync, live replication, workspace registration, or
  conflict resolution.
- No new checkout, repository split, archive, tarball, or fabricated GitHub
  remote.
- No database schema rename or hard `forge_*` crate rename in this slice.
- No package publication or release claim without an independently verified
  authorized remote, artifact checksum, and release/attestation evidence.

## Architecture

HeliosLite owns all writable state:

```text
standard Forge                       Phenotype HeliosLite
~/.forge/.forge.db  --RO snapshot--> ~/.helioslite/sessions/forge-readonly.db
~/.forge/{WAL,locks,config}         ~/.helioslite/{config,cache,logs,locks}
        never written                 owned exclusively by HeliosLite
```

The default root is `~/.helioslite`. Its required subdirectories are:

```text
~/.helioslite/
  config/        # HeliosLite-only configuration
  sessions/      # imported snapshots and manifests
  cache/         # disposable HeliosLite cache
  logs/          # HeliosLite logs
  locks/         # HeliosLite locks only
```

The source path is explicit and may be overridden for diagnostics, but the
destination must remain inside the HeliosLite root unless an explicit future
expert-only override is designed and reviewed. The source database is opened
with SQLite read-only/immutable semantics. A source-side WAL checkpoint,
vacuum, lock acquisition, pragma that writes, or journal creation is forbidden.

## Snapshot contract

The importer is an explicit command with a stable contract:

```text
helioslite sessions import-forge \
  --source ~/.forge/.forge.db \
  --dest ~/.helioslite/sessions/forge-readonly.db
```

The command must:

1. Verify that source and destination are different canonical paths.
2. Open the source read-only and inspect schema before reading rows.
3. Require `conversations.conversation_id`; never fall back to
   `conversations.id`.
4. Read required metadata and content columns, including `context` and
   `context_zstd`, `is_compressed`, `parent_id`, `workspace_id`, timestamps,
   `source`, `cwd`, and message-count fields where present.
5. Preserve compressed bytes exactly when `is_compressed=1`; preserve plain
   context exactly otherwise. It may validate/decompress for diagnostics but
   must not rewrite the source or silently normalize content.
6. Write a destination SQLite snapshot plus a sidecar manifest in one
   destination-owned staging directory.
7. Atomically publish the completed snapshot and manifest with a same-filesystem
   rename. An interrupted import must leave the prior destination intact and
   no usable partial snapshot.

The destination snapshot is read-only to ordinary session consumers. Any
future writable HeliosLite session store must be a separate database under
`~/.helioslite/sessions`, never the imported file.

## Manifest and provenance

Each snapshot has a machine-readable manifest (JSON is preferred) containing:

- contract version;
- source canonical path (with home-directory redaction where appropriate);
- source file size and SHA-256 captured before and after the read;
- source schema fingerprint and required table/column list;
- import start/end timestamps and importer version;
- row count and deterministic content/ID digest;
- destination path and destination SHA-256 after atomic publication;
- source mode (`read-only`, `immutable`) and a boolean proving no source write;
- status (`complete` or `failed`) and diagnostic error, if failed.

The importer must fail if the source hash changes during the read. A repeated
import with the same source hash, schema fingerprint, and contract version is a
no-op with a successful, idempotent result. A changed source creates a new
staged snapshot or an explicit versioned destination; it must not overwrite a
consumer's open database in place.

## Namespace, environment, and remote policy

- `helioslite` is the canonical executable and user-facing product name.
- Existing `forge`/`forge-dev` names remain compatibility aliases only while
  their deprecation policy is active; they must not write HeliosLite state.
- New environment variables use `HELIOSLITE_*` for new behavior. Existing
  `FORGE_*` variables may be read only where compatibility is explicitly
  documented; no implicit cross-product state sharing is allowed.
- The repository/release remote is `KooshaPari/forgecode` until a verified
  `KooshaPari/heliosLite` repository exists. Documentation and installers must
  not advertise a nonexistent remote as canonical.
- Package names, formula names, artifact names, and install directories must
  use `helioslite`; Forge artifact names are accepted only as explicitly
  labeled upstream inputs.

## Error handling and safety invariants

- Missing source, unreadable source, schema mismatch, hash drift, malformed
  compressed content, destination escape, and interrupted writes fail closed.
- Source errors never trigger fallback to a writable Forge path.
- The importer emits a clear error and nonzero exit status; it does not delete
  staging files needed for forensic diagnosis until they are safely isolated.
- No secrets, API keys, session content, or raw prompts are logged.
- A source database may be copied by the OS only through read-only APIs; the
  implementation must not invoke `cp`, `sqlite3 .backup`, WAL checkpoint, or
  other write-capable operation against the source.

## Testing and acceptance gates

Tests must be fixture-only and use temporary destination directories:

1. **Schema contract:** fixture has `conversation_id`, `context`,
   `context_zstd`, and `is_compressed`; the importer reads the correct key and
   never generates an `id` query.
2. **Content fidelity:** plain and compressed contexts, null contexts,
   parent/child IDs, and metadata round-trip with byte/content equality.
3. **Read-only source:** source file bytes, mtime, journal/WAL set, and hash
   remain unchanged after import; no source write-capable pragma is issued.
4. **Manifest:** required fields, schema fingerprint, source/destination
   hashes, row count, and complete status are deterministic and validated.
5. **Idempotency:** identical source hash is a no-op; changed source produces a
   distinct staged result and never corrupts an existing destination.
6. **Atomicity:** injected read, decompression, hash, and rename failures leave
   no partially published destination.
7. **Isolation:** destination defaults under `~/.helioslite/sessions`, has no
   shared lock/WAL path with Forge, and rejects destination traversal.
8. **Namespace:** new config/cache/lock resolution uses HeliosLite paths while
   standard Forge resolution remains unchanged.
9. **Consumer regression:** session listing/resume fixtures use
   `conversation_id`; a fixture with only `id` is rejected with a schema error.

Acceptance requires focused unit/integration tests, `git diff --check`,
formatting for touched files, and a repository-preserved commit/branch. It
does not authorize a release, package publication, or deletion of old state.

## Alternatives considered

### A. Live read-only view of Forge (rejected)

Opening `~/.forge/.forge.db` directly reduces storage but retains availability,
WAL, locking, schema-change, and accidental-write risks. It also makes
HeliosLite behavior dependent on the standard Forge process lifecycle.

### B. Explicit snapshot import (recommended)

An immutable, manifested snapshot gives deterministic provenance, strong
filesystem isolation, reproducible tests, and a clear recovery boundary. The
cost is an extra copy and an explicit refresh operation.

### C. Full migration/rename of Forge state (deferred/rejected)

Moving or rewriting `~/.forge` would risk data loss, break the standard Forge
CLI, and violate preserve-first operation. It may be revisited only as a
separately approved, reversible migration with independent backups.

## Rollout gates

1. Verify the real remote and branch provenance; correct stale
   `KooshaPari/heliosLite` references.
2. Land namespace/path resolution without changing Forge defaults.
3. Land the fixture-tested read-only importer and manifest contract.
4. Dogfood import against a copied fixture, not the live Forge database.
5. Review release/package/attestation evidence separately before publishing.

This design is intentionally additive and preserves the standard Forge CLI,
its sessions, and its state as the source of truth.
