//! `forge` binary entry point (PR-5).
//!
//! The full forge CLI lives in `forge_main`; this binary is the minimal
//! surface for the Ghostty integration that PR-5 introduces. It owns one
//! top-level subcommand: `ghostty`, dispatched through [`ghostty::cmd`].

mod ghostty;

fn main() {
    // `cmd()` declares `ghostty` as a subcommand of `forge` with
    // `subcommand_required(true)`, so `get_matches` will render help and
    // exit successfully when no subcommand is supplied.
    let matches = ghostty::cmd().get_matches();
    let code = ghostty::run(&matches);
    std::process::exit(code);
}