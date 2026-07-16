#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

grep -Fq 'cargo:rerun-if-env-changed=FORGE_ZIG' crates/forge_daemon/build.rs
grep -Fq 'env::var_os("FORGE_ZIG")' crates/forge_daemon/build.rs
grep -Fq 'FORGE_ZIG points to a non-file' crates/forge_daemon/build.rs
grep -Fq 'mlugg/setup-zig@v2' .github/workflows/ci.yml
grep -Fq 'version: 0.16.0' .github/workflows/ci.yml
grep -Fq 'mlugg/setup-zig@v2' .github/workflows/release.yml
grep -Fq 'version: 0.16.0' .github/workflows/release.yml
