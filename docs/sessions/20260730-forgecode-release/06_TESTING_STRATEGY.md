# Testing and acceptance evidence

## Executed

1. `cargo check -p forge_main --bin forge` — passed.
2. `cargo test -p forge_repo --lib migration_round_trip_all_migrations_apply_cleanly` — passed, 1 test.
3. `cargo build --release -p forge_main --bin forge` — passed.
4. `target/release/forge --version` and `--help` — passed.
5. Isolated `cargo install ... --locked --bin forge` — passed; installed binary reports `forge 2.10.0`.
6. `perf_harness` warmup — completed; no crash, 0% idle CPU.

## Required before release claim

- Full `just test` and `just lint` on the final PR head.
- Perf harness warmup+sustained+burst on the final PR head.
- GitHub required checks green, review complete, conflict resolved, PR merged to fork `main`.
- Download/install the published artifact and rerun `--version`, `--help`, and a non-network smoke.
