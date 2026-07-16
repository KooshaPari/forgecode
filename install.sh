#!/usr/bin/env bash
# Kept as the stable GitHub Release asset path. The implementation lives in
# scripts/install.sh so CI and local validation exercise the exact same file.
set -euo pipefail
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/scripts/install.sh" "$@"
