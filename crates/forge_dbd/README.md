# forge_dbd

SQLite write daemon for persistent conversation storage, implementing the
[P3 single-writer design](../../docs/sessions/20260626-forge-sqlite-fix/P3_SINGLE_WRITER_DAEMON.md).

A background IPC daemon that owns the single writer connection to the split-DB
write database (`~/.forge/.forge.writes.db`, overridable with
`FORGE_WRITE_DB_PATH`) and serialises conversation-history writes with
transaction batching.

## Status

Wired into the client write path (opt-in, off by default):

- `forge_repo` routes the four hot-path conversation writes
  (`upsert_conversation`, `upsert_conversation_ref`, `update_parent_id`,
  `delete_conversation`) through the daemon when `FORGE_DBD_ENABLED=1`, with
  automatic fallback to the direct diesel path on any daemon failure. Reads
  always stay direct.
- Transports: Unix domain socket (`~/.forge/.forge.db.sock`, default; override
  with `FORGE_DBD_SOCKET`) on Unix, named pipe on Windows
  (`\\.\pipe\forge-dbd-...`). Windows serves one client at a time.
- Wire format: length-prefixed JSON frames (`protocol::read_frame` /
  `write_frame`).
- The daemon holds one SQLite connection for its lifetime and batches writes
  (15 ms window / 100-request threshold), executing the same operations
  forge_repo performs with diesel.

## Not yet done (follow-ups)

- Daemon lifecycle: clients do not spawn the daemon yet — it must be started
  separately (`cargo run -p forge_dbd`). Spawn-on-first-client, idle
  timeout, and stale-socket recreation are designed but not wired.
- The daemon is not part of the shipped installs (release builds cover
  `helioslite` / `forge`); `forge_dbd` is built standalone.
- `Context` is stored uncompressed (`context_zstd = NULL`,
  `is_compressed = 0`) in this wiring pass.
- Windows transport accepts connections sequentially (one at a time); a
  multi-instance named-pipe accept loop is a future refinement.

## Run

```sh
cargo run -p forge_dbd          # Unix: unix socket; Windows: named pipe
FORGE_DBD_ENABLED=1 forge ...   # client routes writes through the daemon
```
