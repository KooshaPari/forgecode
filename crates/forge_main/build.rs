fn clean_version(version: &str) -> String {
    // Remove 'v' prefix if present using strip_prefix
    version.strip_prefix('v').unwrap_or(version).to_string()
}

fn main() {
    // Priority order:
    // 1. APP_VERSION environment variable (for CI/CD builds)
    // 2. Cargo package metadata, which is the local and release build source
    //    of truth for the production executable.

    let version = std::env::var("APP_VERSION")
        .map(|v| clean_version(&v))
        .unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string());

    // Make version available to the application
    println!("cargo:rustc-env=CARGO_PKG_VERSION={version}");

    // Make version available to the application
    println!("cargo:rustc-env=CARGO_PKG_NAME=forge-dev");

    // Ensure rebuild when environment changes
    println!("cargo:rerun-if-env-changed=APP_VERSION");
}
