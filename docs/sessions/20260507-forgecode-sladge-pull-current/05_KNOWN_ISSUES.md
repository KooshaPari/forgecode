# Known Issues

## Canonical Dirty State

Canonical `forgecode` has unrelated local edits in `Cargo.toml`, `deny.toml`,
Rust source files, and plan docs. This refresh preserves those changes by
remaining isolated.

## Stale Prior Evidence

`forgecode-wtrees/sladge-current` is behind the active branch and should be
treated as stale evidence.

## Rust Validation Baseline

`cargo fmt --all --check` reports broad pre-existing Rust formatting drift
outside the README/session-doc change. `cargo clippy --workspace --offline --
-D warnings` fails on an existing `forge_domain` bool-literal lint; the same
package passes when clippy is run without `-D warnings`.
