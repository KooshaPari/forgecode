//! `cargo check` provider — runs `cargo check --message-format=json` on the
//! file's crate and parses the JSON messages into `Diagnostic` entries.
//!
//! `cargo check` is the cheapest diagnostic path: it type-checks
//! without producing codegen artifacts, so it completes in seconds
//! even on large crates. We invoke it on the closest enclosing
//! `Cargo.toml` (workspace-or-package).

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::anyhow;
use serde::Deserialize;

use crate::diagnostic::{Diagnostic, DiagnosticSeverity};
use crate::provider::{DiagnosticsProvider, DiagnosticsResult};

/// One entry from `cargo check --message-format=json` output.
///
/// `cargo` emits many event kinds (`compiler-artifact`, `build-script-executed`,
/// `compiler-message`, etc.). Only `compiler-message` carries the structured
/// diagnostic data the REPL surfaces to the user, and that event wraps the
/// actual diagnostic inside a nested `message` object — not the `String` we
/// previously deserialized (which always failed silently).
#[derive(Debug, Deserialize)]
struct CargoCompilerMessage {
    /// Top-level event discriminator (`"compiler-message"`, …).
    #[serde(default)]
    reason: Option<String>,
    /// Nested diagnostic object (only populated for compiler-message events).
    #[serde(default)]
    message: Option<CargoDiagnosticMessage>,
}

#[derive(Debug, Deserialize)]
struct CargoDiagnosticMessage {
    /// Human-readable diagnostic text.
    message: String,
    /// `error`, `warning`, `note`, `help`, or absent.
    #[serde(default)]
    level: Option<String>,
    /// `E0001`-style code, when cargo has one.
    #[serde(default)]
    code: Option<CargoCode>,
    /// Source spans attached to the diagnostic (primary + secondary).
    #[serde(default)]
    spans: Vec<CargoSpan>,
}

#[derive(Debug, Deserialize)]
struct CargoCode {
    code: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // Some span fields are kept for future filtering but unused today.
struct CargoSpan {
    file_name: Option<String>,
    line_start: Option<u32>,
    line_end: Option<u32>,
    #[serde(default)]
    column_start: Option<u32>,
    #[serde(default)]
    column_end: Option<u32>,
    is_primary: Option<bool>,
}

/// `cargo check --message-format=json` provider for the LSP layer.
///
/// The provider shells out to the host's `cargo` toolchain on each
/// request; it caches its own parsed [`Diagnostic`] vector in the
/// [`crate::service::DiagnosticsService`] so the REPL doesn't pay
/// compiler cost on every keystroke.
#[derive(Debug, Clone, Copy)]
pub struct RustcProvider;

impl RustcProvider {
    /// Construct a new `RustcProvider`. The provider is stateless and
    /// cheap to construct; prefer `RustcProvider::new()` for explicitness
    /// over `RustcProvider::default()` to satisfy clippy's
    /// `default_constructed_unit_structs` lint.
    pub const fn new() -> Self {
        Self
    }
}

impl Default for RustcProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl DiagnosticsProvider for RustcProvider {
    fn name(&self) -> &'static str {
        "rustc"
    }

    fn supports(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e == "rs")
            .unwrap_or(false)
    }

    fn diagnostics(&self, path: &Path, workspace_root: &Path) -> DiagnosticsResult {
        // Resolve the path against `workspace_root` first so that
        // relative paths (e.g. unsaved files reported by an LSP
        // client) walk up from inside the supplied workspace rather
        // than the host process's CWD. Absolute paths are kept as-is.
        let resolved_path = resolve_request_path(path, workspace_root);
        // Find the closest enclosing Cargo.toml.
        let cargo_root = match find_cargo_root(&resolved_path, workspace_root) {
            Some(r) => r,
            None => {
                return Err(anyhow!(
                    "no Cargo.toml found for {}",
                    resolved_path.display()
                ));
            }
        };

        let output = match Command::new("cargo")
            .arg("check")
            .arg("--message-format=json")
            .arg("--quiet")
            .arg("--manifest-path")
            .arg(cargo_root.join("Cargo.toml"))
            .arg("--color=never")
            .current_dir(&cargo_root)
            .output()
        {
            Ok(out) => out,
            Err(e) => {
                return Err(anyhow!("failed to spawn cargo: {e} (is Rust installed?)"));
            }
        };

        if !output.status.success() {
            let stderr = String::from_utf8(output.stderr.clone()).unwrap_or_default();
            let stderr_trimmed = stderr.trim();
            if stderr_trimmed.is_empty() {
                return Err(anyhow!(
                    "cargo check failed for {} with status {:?} and no stderr",
                    cargo_root.display(),
                    output.status.code()
                ));
            }
            return Err(anyhow!(
                "cargo check failed for {} (status {:?}): {}",
                cargo_root.display(),
                output.status.code(),
                stderr_trimmed
            ));
        }

        let stdout = String::from_utf8(output.stdout.clone()).unwrap_or_else(|e| {
            tracing::warn!("cargo check stdout is not valid UTF-8: {e}");
            String::new()
        });
        let mut diagnostics = Vec::new();
        for line in stdout.lines() {
            if line.is_empty() {
                continue;
            }
            let parsed: CargoCompilerMessage = match serde_json::from_str(line) {
                Ok(m) => m,
                Err(_) => continue,
            };
            // `cargo check --message-format=json` emits a top-level
            // `reason` field; we only care about compiler-message events.
            if parsed.reason.as_deref() != Some("compiler-message") {
                continue;
            }
            let Some(msg) = parsed.message else {
                continue;
            };
            if msg.message.is_empty() {
                continue;
            }
            // Pick the primary span (is_primary=true) for the line/file.
            let span = msg
                .spans
                .iter()
                .find(|s| s.is_primary.unwrap_or(false))
                .or_else(|| msg.spans.first());
            let (file, line_num) = match span {
                Some(s) => (
                    PathBuf::from(s.file_name.clone().unwrap_or_default()),
                    s.line_start.unwrap_or(1),
                ),
                None => continue,
            };
            // `cargo check` reports project-wide diagnostics. Filter to
            // those that touch the requested file (or its primary span
            // file if the user asked for a directory), so the REPL
            // doesn't surface unrelated diagnostics on every keystroke.
            if !diagnostic_matches_path(&file, path) {
                continue;
            }
            let severity = match msg.level.as_deref() {
                Some("error") => DiagnosticSeverity::Error,
                Some("warning") => DiagnosticSeverity::Warning,
                Some("note") => DiagnosticSeverity::Information,
                Some("help") => DiagnosticSeverity::Hint,
                _ => DiagnosticSeverity::Warning,
            };
            let mut message = msg.message.clone();
            if let Some(code) = msg.code.as_ref().and_then(|c| c.code.clone()) {
                message = format!("{code}: {message}");
            }
            diagnostics.push(Diagnostic::new(
                file.to_path_buf(),
                severity,
                line_num,
                message,
            ));
        }
        Ok(diagnostics)
    }
}

/// A span's file matches the user's requested path when either is a
/// path-suffix match of the other. This handles the common cases:
///   - user passes `src/main.rs`, span reports `src/main.rs` or `/abs/src/main.rs`
///   - user passes `src/main.rs`, span reports `src/main.rs` relative to cargo_root
fn diagnostic_matches_path(span_file: &Path, requested_path: &Path) -> bool {
    use std::path::Component;
    let span_components: Vec<_> = span_file
        .components()
        .filter(|c| !matches!(c, Component::RootDir | Component::Prefix(..)))
        .collect();
    let requested_components: Vec<_> = requested_path
        .components()
        .filter(|c| !matches!(c, Component::RootDir | Component::Prefix(..)))
        .collect();
    if span_components.is_empty() || requested_components.is_empty() {
        return false;
    }
    let span_len = span_components.len();
    let req_len = requested_components.len();
    let min_len = span_len.min(req_len);
    let span_tail = span_components.get(span_len - min_len..).unwrap_or(&[]);
    let req_tail = requested_components.get(req_len - min_len..).unwrap_or(&[]);
    span_tail == req_tail
}

/// Resolve a requested diagnostics path against the workspace root.
/// Absolute paths are returned unchanged; relative paths are joined
/// onto `workspace_root` so manifest discovery starts inside the
/// workspace the caller supplied (not the host process CWD).
fn resolve_request_path(path: &Path, workspace_root: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace_root.join(path)
    }
}

/// Walk up from `path` looking for the closest `Cargo.toml`. Returns
/// `None` when no manifest exists (no rust diagnostic pass will run).
fn find_cargo_root(path: &Path, workspace_root: &Path) -> Option<PathBuf> {
    let mut start = if path.is_file() {
        path.parent().map(Path::to_path_buf)?
    } else {
        path.to_path_buf()
    };
    loop {
        if start.join("Cargo.toml").is_file() {
            return Some(start);
        }
        if start == workspace_root || start.parent().is_none() {
            return None;
        }
        start = start.parent()?.to_path_buf();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn rustc_provider_supports_rs_only() {
        let p = RustcProvider::new();
        let actual: Vec<bool> = ["foo.rs", "foo.RS", "foo.toml", "foo"]
            .iter()
            .map(|f| p.supports(Path::new(f)))
            .collect();
        let expected: Vec<bool> = vec![true, false, false, false];
        assert_eq!(actual, expected);
    }

    #[test]
    fn rustc_provider_name() {
        let actual = RustcProvider::new().name();
        let expected = "rustc";
        assert_eq!(actual, expected);
    }

    #[test]
    fn resolve_request_path_keeps_absolute() {
        let abs = std::env::temp_dir().join("abs_probe.rs");
        let ws = std::env::temp_dir().join("ws_probe");
        let actual = resolve_request_path(&abs, &ws);
        assert_eq!(actual, abs);
    }

    #[test]
    fn resolve_request_path_joins_relative_to_workspace() {
        let ws = std::env::temp_dir();
        let rel = Path::new("crates/forge_app/src/lib.rs");
        let actual = resolve_request_path(rel, &ws);
        assert_eq!(actual, ws.join(rel));
    }

    #[test]
    fn rustc_provider_resolves_relative_path_within_workspace() {
        // Relative `foo.rs` inside a manifest-less workspace must
        // still produce a `no Cargo.toml` error rather than a CWD
        // walk that never reaches the workspace root.
        let dir = std::env::temp_dir().join("forge_lsp_test_rel_ws");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let r = RustcProvider::new().diagnostics(Path::new("foo.rs"), &dir);
        assert!(r.is_err(), "relative path must resolve into workspace");
        let msg = format!("{}", r.unwrap_err());
        assert!(
            msg.contains("no Cargo.toml"),
            "error should mention missing manifest, got: {msg}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rustc_provider_default_matches_new() {
        let actual = RustcProvider::new();
        assert_eq!(actual.name(), RustcProvider::new().name());
    }

    #[test]
    fn rustc_provider_errors_without_cargo_manifest() {
        let dir = std::env::temp_dir().join("forge_lsp_test_no_cargo");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let r = RustcProvider::new().diagnostics(&dir.join("foo.rs"), &dir);
        assert!(r.is_err(), "no manifest should produce Err");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rustc_provider_returns_empty_for_unsupported() {
        let actual: Vec<bool> = ["foo.txt", "foo.py"]
            .iter()
            .map(|f| RustcProvider::new().supports(Path::new(f)))
            .collect();
        let expected: Vec<bool> = vec![false, false];
        assert_eq!(actual, expected);
    }

    #[test]
    fn find_cargo_root_returns_workspace_when_no_manifest() {
        let dir = std::env::temp_dir().join("forge_lsp_test_nomf");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let nested = dir.join("src");
        std::fs::create_dir_all(&nested).unwrap();
        let r = find_cargo_root(&nested.join("foo.rs"), &dir);
        assert_eq!(r, None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn find_cargo_root_returns_manifest_when_present() {
        let dir = std::env::temp_dir().join("forge_lsp_test_mf");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("Cargo.toml"),
            "[package]\nname=\"x\"\nversion=\"0.0.0\"\nedition=\"2021\"\n",
        )
        .unwrap();
        let nested = dir.join("src");
        std::fs::create_dir_all(&nested).unwrap();
        let r = find_cargo_root(&nested.join("foo.rs"), &dir).unwrap();
        assert_eq!(r, dir);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn diagnostic_matches_path_handles_absolute_and_relative() {
        // Relative-to-relative match.
        assert!(diagnostic_matches_path(
            Path::new("src/main.rs"),
            Path::new("src/main.rs"),
        ));
        // Span is absolute, requested is relative.
        assert!(diagnostic_matches_path(
            Path::new("/workspace/src/main.rs"),
            Path::new("src/main.rs"),
        ));
        // Different files should not match.
        assert!(!diagnostic_matches_path(
            Path::new("src/main.rs"),
            Path::new("src/lib.rs"),
        ));
    }
}
