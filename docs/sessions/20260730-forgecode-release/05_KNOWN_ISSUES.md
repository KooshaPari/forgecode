# Known issues and blockers

- Worktree is intentionally dirty with the schema migration wave; no cleanup was performed.
- `forge_main/Cargo.toml` emits a non-fatal warning because the same `src/main.rs` is used by `forge`, `forge-dev`, and `helioslite` targets.
- Remote release is not claimed: PR #3781 was previously observed as conflicting/dirty with required CI not yet green. Re-check GitHub before merge or publish.
- `audit_scorecard.json` is stale and under-reports the Rust workspace; do not use its D+ as current quality evidence.
- Perf harness currently reports epoch-style timestamp (`epoch+...`) and warmup only in this run; sustained/burst should be run before an A+ performance claim.
