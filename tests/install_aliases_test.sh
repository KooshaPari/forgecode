#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
fixture="$(mktemp -d)"
trap 'rm -rf "$fixture"' EXIT

cp "$repo_root/install.sh" "$fixture/install.sh"
chmod +x "$fixture/install.sh"
mkdir -p "$fixture/fake-bin" "$fixture/install"

cat > "$fixture/fake-bin/cargo" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
mkdir -p target/release
for binary in helioslite forge forge-dev; do
    printf '#!/usr/bin/env bash\necho "helioslite 2.13.21"\n' > "target/release/$binary"
    chmod +x "target/release/$binary"
done
EOF
chmod +x "$fixture/fake-bin/cargo"

printf '%s\n' 'old managed alias' > "$fixture/install/forge"
printf '%s\n' 'external target' > "$fixture/external-target"
ln -s "$fixture/external-target" "$fixture/install/forge-dev"

(
    cd "$fixture"
    PATH="$fixture/fake-bin:$PATH" HELIOSLITE_INSTALL_DIR="$fixture/install" ./install.sh --local
)

test "$("$fixture/install/forge" --version)" = "helioslite 2.13.21"
test "$(cat "$fixture/external-target")" = "external target"
test -L "$fixture/install/forge-dev"
