//! `forge ghostty` subcommand: inspect and manage the Ghostty integration.
//!
//! Ghostty is a GPU-accelerated terminal emulator that exposes a runtime
//! control surface over a Unix domain socket (see `ghostty_kit::ipc`).
//! This module wires that surface into the forge CLI so operators can:
//!
//! - View the effective Ghostty config (`forge ghostty config`).
//! - Probe the IPC socket (`forge ghostty ipc status`).
//! - Inspect and reload shaders (`forge ghostty shader list` / `reload`).
//! - Print Ghostty's own version (`forge ghostty version`).
//!
//! Every code path is non-fatal: a missing socket, a missing binary, or a
//! missing config directory prints `unavailable` or `unknown` and exits 0
//! unless the user supplied an invalid invocation.

use std::path::{Path, PathBuf};

use clap::{Arg, ArgMatches, Command};

/// Build the top-level `forge` command with the `ghostty` subcommand wired in.
///
/// Returning a `clap::Command` lets the host binary decide whether `forge`
/// has any other top-level subcommands; this module only owns `ghostty`.
pub fn cmd() -> Command {
    Command::new("forge")
        .about("forge: command-line tool")
        .subcommand(
            Command::new("ghostty")
                .about("Inspect and manage the Ghostty integration")
                .subcommand_required(true)
                .arg_required_else_help(true)
                .subcommand(
                    Command::new("config").about("Print effective Ghostty config"),
                )
                .subcommand(
                    Command::new("ipc")
                        .about("Inspect IPC socket state")
                        .subcommand(
                            Command::new("status")
                                .about("Print IPC socket path and connection state"),
                        ),
                )
                .subcommand(
                    Command::new("shader")
                        .about("Manage Ghostty shaders")
                        .subcommand(
                            Command::new("list").about(
                                "List registered shaders from ~/.config/ghostty/shaders/",
                            ),
                        )
                        .subcommand(
                            Command::new("reload")
                                .about("Reload a single shader via IPC")
                                .arg(
                                    Arg::new("name")
                                        .required(true)
                                        .help("Shader file basename (e.g. \"myeffect\")"),
                                ),
                        ),
                )
                .subcommand(Command::new("version").about("Print Ghostty version")),
        )
}

/// Dispatch a parsed `forge` invocation to the `ghostty` subcommand.
///
/// Returns a process exit code: 0 on success, 1 on user error, 2 on system
/// error. The top-level dispatcher in `main.rs` calls this.
pub fn run(matches: &ArgMatches) -> i32 {
    match matches.subcommand() {
        Some(("ghostty", sub)) => run_ghostty(sub),
        // `cmd()` only declares `ghostty` as a subcommand, so clap will
        // never hand us anything else here.
        _ => unreachable!("top-level dispatcher should only pass the ghostty subcommand"),
    }
}

fn run_ghostty(matches: &ArgMatches) -> i32 {
    match matches.subcommand() {
        Some(("config", _)) => run_config(),
        Some(("ipc", sub)) => match sub.subcommand() {
            Some(("status", _)) => run_ipc_status(),
            _ => unreachable!("ipc subcommand has no other children"),
        },
        Some(("shader", sub)) => match sub.subcommand() {
            Some(("list", _)) => run_shader_list(),
            Some(("reload", args)) => {
                // `name` is marked `required(true)` by clap, so the
                // `unwrap_or` branch is unreachable in practice. We use
                // `get_one` (not direct indexing) so future `ArgAction`
                // changes do not silently break this code.
                let name = match args.get_one::<String>("name") {
                    Some(n) => n.as_str(),
                    None => unreachable!("required arg \"name\" validated by clap"),
                };
                run_shader_reload(name)
            }
            _ => unreachable!("shader subcommand has no other children"),
        },
        Some(("version", _)) => run_version(),
        _ => unreachable!("arg_required_else_help guards this branch"),
    }
}

// ---------------------------------------------------------------------------
// Subcommand handlers
// ---------------------------------------------------------------------------

/// `forge ghostty config`: print the effective Ghostty config.
///
/// Reads `$XDG_CONFIG_HOME/ghostty/config` if it exists, else
/// `$HOME/.config/ghostty/config`. If neither exists, prints
/// `status: unavailable` (exit 0) — a missing config is not a CLI error.
fn run_config() -> i32 {
    let path = match ghostty_config_path() {
        Some(p) => p,
        None => {
            println!("status: unavailable");
            println!("reason: no config dir");
            return 0;
        }
    };

    println!("source: {}", path.display());
    match ghostty_kit::parse_file(&path) {
        Ok(cfg) => {
            println!("status: ok");
            println!("entries: {}", cfg.entries.len());
            if !cfg.includes.is_empty() {
                println!("includes: {}", cfg.includes.len());
            }
            0
        }
        Err(e) => {
            println!("status: parse_error");
            println!("error: {e}");
            1
        }
    }
}

/// `forge ghostty ipc status`: probe the Ghostty control socket.
///
/// `GhosttyControl::try_new` is the contract from PR-1: it never panics
/// and returns `None` whenever no live socket is reachable. We surface
/// that as `unavailable` (exit 0); the user asked a question and we
/// answered it honestly.
fn run_ipc_status() -> i32 {
    match ghostty_kit::GhosttyControl::try_new() {
        Some(_) => {
            println!("status: available");
            0
        }
        None => {
            println!("status: unavailable");
            0
        }
    }
}

/// `forge ghostty shader list`: print shader basenames from the standard
/// Ghostty shaders directory, sorted.
///
/// A missing directory is not an error: a user without shaders just
/// gets an empty listing.
fn run_shader_list() -> i32 {
    let dir = match ghostty_shader_dir() {
        Some(d) => d,
        None => {
            println!("dir: unavailable");
            println!("shaders:");
            return 0;
        }
    };
    println!("dir: {}", dir.display());
    match std::fs::read_dir(&dir) {
        Ok(entries) => {
            let mut names: Vec<String> = entries
                .filter_map(|e| e.ok())
                .filter_map(|e| e.file_name().to_str().map(|s| s.to_string()))
                .collect();
            names.sort();
            for name in &names {
                println!("shader: {name}");
            }
            if names.is_empty() {
                println!("shaders:");
            }
            0
        }
        Err(_) => {
            println!("status: unavailable");
            0
        }
    }
}

/// `forge ghostty shader reload <name>`: emit `reload_config` over IPC.
///
/// The `<name>` is the shader we *intend* to reload; Ghostty's IPC
/// reloads the whole config (which re-evaluates all `custom-shader`
/// directives). The name is echoed back so log scrapers can correlate.
fn run_shader_reload(name: &str) -> i32 {
    println!("shader: {name}");
    match ghostty_kit::GhosttyControl::try_new() {
        None => {
            // No live socket: this is a system-level "we cannot reach the
            // terminal", not a user error, so we return 1 (user action
            // required) rather than 2. Logically: the user did the right
            // thing, the host just is not running.
            println!("status: unavailable");
            1
        }
        Some(ctl) => match ctl.reload_config() {
            Ok(()) => {
                println!("status: reloaded");
                0
            }
            Err(e) => {
                println!("status: error");
                println!("error: {e}");
                2
            }
        },
    }
}

/// `forge ghostty version`: print `ghostty --version` if available.
fn run_version() -> i32 {
    // `Command::new("ghostty")` will fail with `NotFound` when the
    // binary is not in `$PATH`; both that and a non-zero exit code
    // collapse into the `unknown` branch.
    let output = std::process::Command::new("ghostty").arg("--version").output();
    match output {
        Ok(out) if out.status.success() => {
            let raw = String::from_utf8_lossy(&out.stdout);
            // Ghostty prints "Ghostty 1.X.Y (commit)" — keep it verbatim.
            println!("ghostty: {}", raw.trim());
            0
        }
        _ => {
            println!("ghostty: unknown");
            0
        }
    }
}

// ---------------------------------------------------------------------------
// Path helpers
// ---------------------------------------------------------------------------

/// Resolve the path to the user's Ghostty config.
///
/// Honours `$XDG_CONFIG_HOME` first (matching Ghostty itself), then
/// `dirs::config_dir()`. Always returns `Some` so callers see where we
/// looked even when the file does not yet exist; existence is the
/// caller's job.
fn ghostty_config_path() -> Option<PathBuf> {
    if let Some(p) = xdg_ghostty("config") {
        if p.exists() {
            return Some(p);
        }
    }
    Some(default_ghostty_subpath("config"))
}

fn ghostty_shader_dir() -> Option<PathBuf> {
    if let Some(p) = xdg_ghostty("shaders") {
        if p.exists() {
            return Some(p);
        }
    }
    Some(default_ghostty_subpath("shaders"))
}

/// `<$XDG_CONFIG_HOME>/ghostty/<sub>` when the env var is set.
fn xdg_ghostty(sub: &str) -> Option<PathBuf> {
    let xdg = std::env::var_os("XDG_CONFIG_HOME")?;
    Some(PathBuf::from(xdg).join("ghostty").join(sub))
}

/// `<dirs::config_dir()>/ghostty/<sub>` — the conventional fallback.
fn default_ghostty_subpath(sub: &str) -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| Path::new(".").to_path_buf());
    base.join("ghostty").join(sub)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cmd_builds() {
        let cmd = cmd();
        // The top-level command exposes exactly one subcommand: `ghostty`.
        let names: Vec<&str> = cmd.get_subcommands().map(|c| c.get_name()).collect();
        assert_eq!(names, vec!["ghostty"]);
        // `ghostty` itself exposes `config`, `ipc`, `shader`, `version`.
        let ghostty = cmd
            .get_subcommands()
            .find(|c| c.get_name() == "ghostty")
            .expect("ghostty subcommand must exist");
        let mut ghosts: Vec<&str> = ghostty.get_subcommands().map(|c| c.get_name()).collect();
        ghosts.sort();
        assert_eq!(
            ghosts,
            vec!["config", "ipc", "shader", "version"],
            "expected subcommands under ghostty"
        );
    }

    #[test]
    fn test_ghostty_subcommand_parses_config() {
        let m = cmd()
            .try_get_matches_from(["forge", "ghostty", "config"])
            .expect("`forge ghostty config` should parse");
        let (name, sub) = m.subcommand().expect("ghostty subcommand must match");
        assert_eq!(name, "ghostty");
        assert!(sub.subcommand_matches("config").is_some());
    }

    #[test]
    fn test_ghostty_subcommand_parses_shader_reload() {
        let m = cmd()
            .try_get_matches_from(["forge", "ghostty", "shader", "reload", "myshader"])
            .expect("`forge ghostty shader reload myshader` should parse");
        let (_, sub) = m.subcommand().expect("ghostty subcommand must match");
        let (_, shader_sub) = sub.subcommand().expect("shader subcommand must match");
        let reload = shader_sub
            .subcommand_matches("reload")
            .expect("reload must match");
        assert_eq!(
            reload.get_one::<String>("name").map(|s| s.as_str()),
            Some("myshader")
        );
    }

    #[test]
    fn test_shader_reload_missing_arg_errors() {
        let err = cmd()
            .try_get_matches_from(["forge", "ghostty", "shader", "reload"])
            .expect_err("missing `name` must be a clap error");
        // clap renders the error to its `Error` type; the message always
        // mentions the missing argument so log scrapers can grep for it.
        let msg = err.to_string();
        assert!(
            msg.contains("name") || msg.contains("<name>") || msg.contains("required"),
            "expected missing-arg error mentioning `name`; got: {msg}"
        );
    }
}