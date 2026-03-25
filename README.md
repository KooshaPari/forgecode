# forge

Modern CLI task runner and build orchestrator. Define tasks in Rust, execute in parallel.

## Features

- **Parallel Execution**: Run tasks concurrently
- **Dependency Graph**: Automatic topological sort
- **Hot Reload**: Watch files and restart
- **Plugin System**: Extend with custom tasks

## Installation

```bash
cargo install forge-cli
```

## Usage

```rust
use forge::{task, deps};

#[task]
fn build() {
    println!("Building...");
}

#[task]
#[deps(build)]
fn test() {
    println!("Testing...");
}

#[task]
fn serve() {
    println!("Serving...");
}
```

Run:
```bash
forge test    # Builds first, then tests
forge --watch # Hot reload
```

## License

MIT
