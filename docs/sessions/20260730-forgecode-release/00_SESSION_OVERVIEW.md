# Forgecode release readiness

## Scope

Validate the fork's build, package, local install, runtime smoke, and resource scorecard on commit `1f039b801` (`preserve/workflow-schema-wave-20260729`). Preserve existing dirty migration work; no reset, clean, or force-push.

## Evidence

- `cargo check -p forge_main --bin forge`: passed.
- `cargo test -p forge_repo --lib migration_round_trip_all_migrations_apply_cleanly`: passed (1/1).
- `cargo build --release -p forge_main --bin forge`: passed.
- `target/release/forge --version`: `forge 2.10.0`; `--help`: 62 lines and command surface rendered.
- `cargo install --path crates/forge_main --root /tmp/forgecode-install-20260730 --locked --bin forge`: passed; installed binary runs.
- `perf_harness` warmup scorecard: macOS/aarch64, cold RSS 32 KiB, sampled RSS 224 KiB, idle CPU 0%, 1,135 ms.

## Release decision

Local usable build is available for sponsor evaluation. Remote merge/main and required CI remain separate release gates.
