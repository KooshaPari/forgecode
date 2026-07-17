#!/usr/bin/env bash
# Validate the ForgeCode GitHub Release distribution contract.
set -euo pipefail

readonly app="forge-dev"
readonly targets=(
  aarch64-apple-darwin
  x86_64-apple-darwin
  aarch64-unknown-linux-gnu
  x86_64-unknown-linux-gnu
  x86_64-pc-windows-msvc
)

usage() {
  cat <<'EOF'
ForgeCode release scorecard

Usage: scripts/release-scorecard.sh [--release-dir DIR] [--version vX.Y.Z]

Checks the GitHub Release asset contract: every target archive, SHA256SUMS,
installers, a release evidence manifest, and a tag consistent with Cargo.toml.
EOF
}

release_dir=""
version=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --release-dir) release_dir="$2"; shift 2 ;;
    --version) version="$2"; shift 2 ;;
    --help|-h) usage; exit 0 ;;
    *) printf 'unknown argument: %s\n' "$1" >&2; exit 2 ;;
  esac
done

grep -Fq 'version = "2.10.0"' Cargo.toml
grep -Fq 'repository = "https://github.com/KooshaPari/forgecode"' Cargo.toml
grep -Fq 'APP="forge-dev"' scripts/install.sh
grep -Fq 'REPO="KooshaPari/forgecode"' scripts/install.sh
bash -n scripts/install.sh

if [[ -n "$version" && "$version" != "v2.10.0" ]]; then
  printf 'release tag must be v2.10.0, got %s\n' "$version" >&2
  exit 1
fi

if [[ -z "$release_dir" ]]; then
  printf 'source release contract validated\n'
  exit 0
fi

test -f "$release_dir/install.sh"
test -f "$release_dir/install.ps1"
test -f "$release_dir/SHA256SUMS"
test -f "$release_dir/RELEASE_EVIDENCE.json"
grep -Fq '"schema_version": 1' "$release_dir/RELEASE_EVIDENCE.json"
grep -Fq "\"release\": \"$version\"" "$release_dir/RELEASE_EVIDENCE.json"
grep -Fq '"required_checks": "passed"' "$release_dir/RELEASE_EVIDENCE.json"
grep -Fq '"asset_contract": "passed"' "$release_dir/RELEASE_EVIDENCE.json"
grep -Fq '"macos_or_linux_installer": "passed"' "$release_dir/RELEASE_EVIDENCE.json"
grep -Fq '"windows_installer": "passed"' "$release_dir/RELEASE_EVIDENCE.json"
for target in "${targets[@]}"; do
  if [[ "$target" == *windows* ]]; then
    asset="${app}-${target}.zip"
  else
    asset="${app}-${target}.tar.gz"
  fi
  test -f "$release_dir/$asset"
  grep -Fq "  $asset" "$release_dir/SHA256SUMS"
done
(cd "$release_dir" && sha256sum --check SHA256SUMS)
printf 'A+ release asset contract validated\n'
