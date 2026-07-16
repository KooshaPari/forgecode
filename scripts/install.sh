#!/usr/bin/env bash
# ForgeCode installer for GitHub Release archives.
set -euo pipefail

APP="forge-dev"
REPO="KooshaPari/forgecode"
VERSION="${FORGE_DEV_VERSION:-latest}"
INSTALL_DIR="${FORGE_DEV_INSTALL_DIR:-$HOME/.local/bin}"

usage() {
  cat <<'EOF'
ForgeCode installer

Usage: curl -fsSL https://github.com/KooshaPari/forgecode/releases/latest/download/install.sh | bash

Environment:
  FORGE_DEV_VERSION       Release tag to install (default: latest)
  FORGE_DEV_INSTALL_DIR   Destination directory (default: ~/.local/bin)
EOF
}

if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  usage
  exit 0
fi

case "$(uname -m)" in
  x86_64|amd64) arch="x86_64" ;;
  aarch64|arm64) arch="aarch64" ;;
  *) printf 'unsupported architecture: %s\n' "$(uname -m)" >&2; exit 1 ;;
esac

case "$(uname -s)" in
  Darwin) target="${arch}-apple-darwin" ;;
  Linux) target="${arch}-unknown-linux-gnu" ;;
  *) printf 'unsupported operating system: %s\n' "$(uname -s)" >&2; exit 1 ;;
esac

if [[ "$VERSION" == "latest" ]]; then
  base_url="https://github.com/${REPO}/releases/latest/download"
else
  base_url="https://github.com/${REPO}/releases/download/${VERSION}"
fi

archive="${APP}-${target}.tar.gz"
temporary_dir="$(mktemp -d)"
trap 'rm -rf "$temporary_dir"' EXIT

curl --fail --location --silent --show-error "$base_url/$archive" --output "$temporary_dir/$archive"
curl --fail --location --silent --show-error "$base_url/SHA256SUMS" --output "$temporary_dir/SHA256SUMS"

if command -v shasum >/dev/null 2>&1; then
  (cd "$temporary_dir" && grep "  ${archive}$" SHA256SUMS | shasum -a 256 -c -)
elif command -v sha256sum >/dev/null 2>&1; then
  (cd "$temporary_dir" && sha256sum --ignore-missing --check SHA256SUMS)
else
  printf 'need shasum or sha256sum to verify the release archive\n' >&2
  exit 1
fi

tar -xzf "$temporary_dir/$archive" -C "$temporary_dir"
install -d "$INSTALL_DIR"
install -m 0755 "$temporary_dir/$APP" "$INSTALL_DIR/$APP"
"$INSTALL_DIR/$APP" --version
printf 'installed %s to %s\n' "$APP" "$INSTALL_DIR/$APP"
