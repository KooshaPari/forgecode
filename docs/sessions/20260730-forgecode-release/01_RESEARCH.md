# Research and command map

## Canonical commands

| Purpose | Command |
|---|---|
| Debug build | `cargo build` or `just build` |
| Release build | `cargo build --release -p forge_main --bin forge` or `just release` |
| Tests | `just test` (nextest if installed, else cargo test) |
| Lint | `just lint` (clippy deny-warnings + fmt check) |
| Local package/install | `cargo install --path crates/forge_main --root <isolated-root> --locked --bin forge` |
| Resource scorecard | `cargo run --release -p perf_harness -- run --project . --regimes warmup,sustained,burst --out <path>` |

The release binary is `target/release/forge`; `forge-dev` is an opt-in feature binary and `helioslite` is an additive alias. The README's network installer targets GitHub releases and was not invoked locally.

## Findings

The tracked `audit_scorecard.json` is stale (D+, reports zero Rust source files) and is not a valid current quality claim. The authoritative local evidence for this lane is the Cargo gates plus the perf harness output.
