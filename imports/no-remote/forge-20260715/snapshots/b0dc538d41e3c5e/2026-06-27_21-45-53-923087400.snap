//! `pt` binary entry-point.
//!
//! Parses CLI args, dispatches to the subcommand router in
//! `phenotype_cli::run`, and exits with the returned code. Never panics.

use clap::Parser;
use phenotype_cli::{exit_code, run, Cli};

fn main() {
    let cli = Cli::parse();
    let code = run(cli);
    if code != exit_code::OK {
        std::process::exit(code);
    }
}