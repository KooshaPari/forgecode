//! Mutation testing CLI stub.

use std::path::Path;
use std::process::Command;

use crate::cli::CliError;

/// Stub mutation runner. This only proves the executor path for now.
pub fn run(
    _package: Option<&str>,
    _file: Option<&Path>,
    _min_score: Option<f64>,
) -> Result<(), CliError> {
    let _status = Command::new("cargo")
        .args(["mutants", "--help"])
        .status()
        .map_err(|source| CliError::Io {
            path: "cargo".into(),
            source,
        })?;

    println!("TODO: parse cargo-mutants output and enforce --min-score");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::run;

    #[test]
    fn mutate_stub_runs_without_error() {
        run(Some("benchora"), None, Some(75.0)).expect("mutate stub should run");
    }
}
