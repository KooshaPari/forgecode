# Testing Strategy

## Targeted Checks

- `git diff --check`
- README/session badge presence search
- Cargo formatter/check commands if they can run without pulling broad baseline
  formatting drift into this badge-only branch

## Expected Limits

This is a governance-only refresh. Existing Rust formatting or dependency-cache
blockers should be recorded and left outside the commit.

## Validation Result

- `git diff --check` passed.
- README/session badge presence passed.
- `cargo fmt --all --check` reports broad pre-existing Rust formatting drift.
- `cargo clippy --workspace --offline -- -D warnings` fails on a pre-existing
  `forge_domain` bool-literal warning promoted to an error.
- `cargo clippy -p forge_domain --offline` passes with that warning.
