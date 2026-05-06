# Known Issues

- Canonical `forgecode` has unrelated local `Cargo.toml` and `deny.toml`
  changes.
- Broad Cargo validation is deferred because this environment is low on disk
  after prior Cargo validation.
- `cargo fmt --check` reports broad pre-existing Rust formatting drift outside
  the README/session-doc change.
- `bun test:bounty` is not runnable in this isolated worktree until
  `node_modules` is installed.
