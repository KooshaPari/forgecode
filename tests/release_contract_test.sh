#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

grep -Fq 'version = "2.10.0"' Cargo.toml
grep -Fq 'repository = "https://github.com/KooshaPari/forgecode"' Cargo.toml
grep -Fq 'name: Release ForgeCode' .github/workflows/release.yml
grep -Fq 'scripts/install.sh' .github/workflows/release.yml
grep -Fq 'SHA256SUMS' .github/workflows/release.yml
grep -Fq 'taiki-e/setup-cross-toolchain-action@v1' .github/workflows/release.yml
grep -Fq "matrix.target == 'aarch64-unknown-linux-gnu'" .github/workflows/release.yml
grep -Fq 'ForgeCode / KooshaPari/forgecode / forge-dev /' README.md
grep -Fq '> v2.10.0.' README.md

# Cargo requires a bare SemVer package version, but every user-visible release
# boundary must identify the sole production executable and its release tag.
test "$(rg -c '^name = "forge-dev"$' crates/forge_main/Cargo.toml)" -eq 1
! rg -q '^name = "forge"$' crates/forge_main/Cargo.toml
rg -Fq '#[command(name = "forge-dev", version = "v2.10.0")]' crates/forge_main/src/cli.rs

bash scripts/install.sh --help | grep -Fq 'ForgeCode installer'
bash scripts/release-scorecard.sh --help | grep -Fq 'ForgeCode release scorecard'

fixture="$(mktemp -d)"
trap 'rm -rf "$fixture"' EXIT
case "$(uname -m)" in
  x86_64|amd64) arch="x86_64" ;;
  aarch64|arm64) arch="aarch64" ;;
  *) exit 1 ;;
esac
case "$(uname -s)" in
  Darwin) target="${arch}-apple-darwin" ;;
  Linux) target="${arch}-unknown-linux-gnu" ;;
  *) exit 1 ;;
esac

mkdir -p "$fixture/archive" "$fixture/bin" "$fixture/install"
printf '#!/usr/bin/env bash\necho forge-dev 2.10.0\n' > "$fixture/archive/forge-dev"
chmod +x "$fixture/archive/forge-dev"
tar -czf "$fixture/forge-dev-${target}.tar.gz" -C "$fixture/archive" forge-dev
(cd "$fixture" && shasum -a 256 "forge-dev-${target}.tar.gz") > "$fixture/SHA256SUMS"
cat > "$fixture/bin/curl" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
output=""
for ((index = 1; index <= $#; index++)); do
  if [[ "${!index}" == "--output" ]]; then
    next=$((index + 1))
    output="${!next}"
  fi
done
cp "$FIXTURE_DIR/$(basename "${!#}")" "$output"
EOF
chmod +x "$fixture/bin/curl"
PATH="$fixture/bin:$PATH" FIXTURE_DIR="$fixture" FORGE_DEV_INSTALL_DIR="$fixture/install" \
  bash scripts/install.sh
"$fixture/install/forge-dev" --version | grep -Fq 'forge-dev 2.10.0'
