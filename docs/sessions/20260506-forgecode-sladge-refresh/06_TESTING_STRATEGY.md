# Testing Strategy

## Focused Checks

- `git diff --check`: passed.
- README badge presence with `rg`: passed.

## Lightweight Repo-Native Checks

- `cargo fmt --check`: blocked by broad pre-existing Rust formatting drift.
- `bun test:bounty`: skipped because `node_modules` is missing.
- Broad Cargo test/clippy: deferred because the current environment is low on
  disk after prior Cargo validation.
