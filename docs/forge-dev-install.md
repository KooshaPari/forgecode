# forge-dev install guide

`forge-dev` is the sole public executable for the KooshaPari ForgeCode fork.
The `forge-dev` `[[bin]]` target in `crates/forge_main/Cargo.toml` is always
available, so source and release installs resolve to the same executable.
After install, `forge-dev --version` reports the fork version.

Install the binary directly from the git fork. This `cargo install` invocation
places the canonical executable at `~/.cargo/bin/forge-dev`:

```bash
cargo install --git https://github.com/KooshaPari/forgecode \
  --bin forge-dev
```
