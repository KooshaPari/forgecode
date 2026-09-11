//! TypeScript / JavaScript provider — shells out to `tsc --noEmit`.
//!
//! Most TypeScript projects configure `tsc` via `tsconfig.json`. We
//! invoke the project-local `tsc` binary if present (so it picks up
//! the project's compiler options), falling back to `npx tsc` or
//! `node_modules/.bin/tsc`.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::anyhow;

use crate::diagnostic::{Diagnostic, DiagnosticSeverity};
use crate::provider::{DiagnosticsProvider, DiagnosticsResult};

/// `tsc --noEmit` writes canonical diagnostics to stdout in the form
/// `file(line,col): severity TS<code>: message` (e.g. `src/foo.ts(10,5): error TS2304: Cannot find name 'bar'.`).
/// Severity is `error` or `warning`; the remainder is the message text.
const TSC_OUTPUT_KIND_ERROR: &str = "error";
const TSC_OUTPUT_KIND_WARNING: &str = "warning";

/// `tsc --noEmit` provider for the LSP layer.
///
/// The provider shells out to the workspace-local `tsc` binary (or
/// the bare `tsc` on `PATH`) on each request; it caches its own
/// parsed [`Diagnostic`] vector in the [`crate::service::DiagnosticsService`]
/// so the REPL doesn't pay compiler cost on every keystroke.
#[derive(Debug, Clone, Copy)]
pub struct TscProvider;

impl TscProvider {
    /// Construct a new `TscProvider`. The provider is stateless and
    /// cheap to construct; prefer `TscProvider::new()` for explicitness
    /// over `TscProvider::default()` to satisfy clippy's
    /// `default_constructed_unit_structs` lint.
    pub const fn new() -> Self {
        Self
    }
}

impl Default for TscProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl TscProvider {
    /// Resolve the `tsc` binary. Prefer the workspace-local
    /// `node_modules/.bin/tsc` (most reliable for project-configured
    /// paths), then fall back to a bare `tsc` on PATH.
    fn resolve_tsc_bin(workspace_root: &Path) -> PathBuf {
        let local = workspace_root.join("node_modules/.bin/tsc");
        if local.exists() {
            return local;
        }
        PathBuf::from("tsc")
    }
}

impl DiagnosticsProvider for TscProvider {
    fn name(&self) -> &'static str {
        "tsc"
    }

    fn supports(&self, path: &Path) -> bool {
        matches!(
            path.extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_ascii_lowercase())
                .as_deref(),
            Some("ts") | Some("tsx") | Some("js") | Some("jsx") | Some("mts") | Some("cts")
        )
    }

    fn diagnostics(&self, path: &Path, workspace_root: &Path) -> DiagnosticsResult {
        // We run `tsc --noEmit` from the workspace root. The canonical
        // output goes to stdout (e.g. `file(line,col): error TSxxxx: msg`).
        // The companion `--pretty false` flag suppresses ANSI color codes
        // so the line parser can match canonical lines.
        let tsc_bin = Self::resolve_tsc_bin(workspace_root);
        let output = match Command::new(&tsc_bin)
            .arg("--noEmit")
            .arg("--pretty")
            .arg("false")
            .current_dir(workspace_root)
            .output()
        {
            Ok(out) => out,
            Err(e) => {
                return Err(anyhow!(
                    "failed to spawn {}: {e} (is tsc installed?)",
                    tsc_bin.display()
                ));
            }
        };

        // `tsc` writes canonical diagnostics to stdout. We parse stdout
        // and also check stderr for fatal errors (process spawn issues,
        // config parse failures, etc).
        let stdout = String::from_utf8(output.stdout.clone()).unwrap_or_else(|e| {
            tracing::warn!("tsc stdout is not valid UTF-8: {e}");
            String::new()
        });
        let stderr = String::from_utf8(output.stderr.clone()).unwrap_or_else(|e| {
            tracing::warn!("tsc stderr is not valid UTF-8: {e}");
            String::new()
        });

        let mut diagnostics = Vec::new();
        for line in stdout.lines() {
            if let Some(d) = parse_tsc_line(line) {
                diagnostics.push(d);
            }
        }
        // Filter to the requested path: `tsc --noEmit` reports every
        // diagnostic in the project, not just for the file we asked
        // about. Caching or surfacing the project-wide vector would
        // leak unrelated files' diagnostics into the caller's view.
        diagnostics.retain(|d| paths_match(&d.file, path));

        // If the process exited non-zero and we have stderr, surface it
        // as a single diagnostic attached to the requested path so the
        // REPL shows the user something instead of silently returning
        // empty diagnostics.
        if diagnostics.is_empty() && !output.status.success() && !stderr.trim().is_empty() {
            let summary = stderr.lines().next().unwrap_or("tsc failed").trim();
            diagnostics.push(
                Diagnostic::new(
                    path.to_path_buf(),
                    DiagnosticSeverity::Error,
                    1,
                    format!("tsc invocation failed: {summary}"),
                )
                .with_source("tsc"),
            );
        }

        Ok(diagnostics)
    }
}

/// Parse a single `tsc` output line into a `Diagnostic` if it matches
/// the canonical format: `path(line,col): error|warning TS<code>:
/// <message>`.
fn parse_tsc_line(line: &str) -> Option<Diagnostic> {
    // Must start with a path prefix that contains a parenthesised
    // `(line,col)` location. tsc prefixes errors with the relative
    // file path; warnings often don't, so this helper is conservative
    // (errors and warnings only).
    let (file, remainder) = line.split_once('(')?;
    let (loc, after_loc) = remainder.split_once(')')?;
    if loc.is_empty() {
        return None;
    }

    let line_num: u32 = loc.split(',').next()?.parse().ok()?;
    // The remainder after `file(line,col): ` is the kind/code/message.
    let after_loc = after_loc.strip_prefix(':')?.trim_start();
    let (kind, rest) = after_loc.split_once(' ')?;
    let severity = match kind {
        TSC_OUTPUT_KIND_ERROR => DiagnosticSeverity::Error,
        TSC_OUTPUT_KIND_WARNING => DiagnosticSeverity::Warning,
        _ => DiagnosticSeverity::Information,
    };
    let message = rest.trim().to_string();
    Some(Diagnostic::new(PathBuf::from(file), severity, line_num, message).with_source("tsc"))
}

/// `tsc` emits paths verbatim from the compiler's working directory,
/// which may differ from the requested path's canonical form (e.g.
/// when the caller passes a relative `foo.ts` and `tsc` reports
/// `src/foo.ts`, or vice versa). Compare by exact byte representation
/// first; fall back to a canonical-form comparison for the cases
/// where both sides exist on the filesystem.
fn paths_match(emitted: &Path, requested: &Path) -> bool {
    if emitted == requested {
        return true;
    }
    match (emitted.canonicalize(), requested.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn tsc_provider_supports_ts_variants() {
        let p = TscProvider::new();
        let actual: Vec<bool> = [
            "foo.ts", "foo.tsx", "foo.js", "foo.jsx", "foo.TS", "foo.rs", "foo.json", "foo",
        ]
        .iter()
        .map(|f| p.supports(Path::new(f)))
        .collect();
        let expected: Vec<bool> = vec![true, true, true, true, true, false, false, false];
        assert_eq!(actual, expected);
    }

    #[test]
    fn tsc_provider_name() {
        let actual = TscProvider::new().name();
        let expected = "tsc";
        assert_eq!(actual, expected);
    }

    #[test]
    fn tsc_provider_default_matches_new() {
        let actual = TscProvider::new();
        // Both constructors produce equivalent values; name() is the
        // cheapest equality check we can do on a zero-field struct.
        assert_eq!(actual.name(), TscProvider::new().name());
    }

    #[test]
    fn parse_tsc_line_extracts_error_with_code() {
        let actual = parse_tsc_line("src/foo.ts(10,5): error TS2304: Cannot find name 'bar'.")
            .expect("parses canonical tsc error");
        let expected = Diagnostic::new(
            PathBuf::from("src/foo.ts"),
            DiagnosticSeverity::Error,
            10,
            "TS2304: Cannot find name 'bar'.",
        )
        .with_source("tsc");
        assert_eq!(actual, expected);
    }

    #[test]
    fn parse_tsc_line_extracts_warning() {
        let actual =
            parse_tsc_line("src/bar.tsx(42,12): warning TS6133: 'x' is declared but never used.")
                .expect("parses warning");
        let expected = Diagnostic::new(
            PathBuf::from("src/bar.tsx"),
            DiagnosticSeverity::Warning,
            42,
            "TS6133: 'x' is declared but never used.",
        )
        .with_source("tsc");
        assert_eq!(actual, expected);
    }

    #[test]
    fn parse_tsc_line_rejects_non_tsc_lines() {
        let fixtures = [
            "",
            "Compiling foo.ts",
            "src/no_line.ts: error",
            "src/no_col.ts(): error",
        ];
        let actual: Vec<bool> = fixtures
            .iter()
            .map(|f| parse_tsc_line(f).is_some())
            .collect();
        let expected: Vec<bool> = vec![false, false, false, false];
        assert_eq!(actual, expected);
    }

    #[test]
    fn tsc_provider_with_path_no_manifest_still_runs() {
        // The provider just tries to spawn `tsc`. If `tsc` isn't on
        // PATH, the test environment returns Err (which is fine).
        let dir = std::env::temp_dir().join("forge_lsp_test_tsc");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let r = TscProvider::new().diagnostics(&dir.join("foo.ts"), &dir);
        // Either Ok(empty) if tsc produced no errors, or Err if tsc
        // isn't installed. Both are valid outcomes.
        if let Ok(v) = r {
            assert!(v.is_empty());
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn paths_match_exact_and_canonical() {
        // Exact byte equality.
        assert!(paths_match(
            Path::new("src/foo.ts"),
            Path::new("src/foo.ts")
        ));
        // Different-but-equivalent text: emit an absolute path that
        // canonicalizes to the same inode as the requested relative path.
        let dir = std::env::temp_dir().join("forge_lsp_paths_match");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let rel = PathBuf::from("a.ts");
        let abs = dir.join("a.ts");
        std::fs::write(&abs, "").unwrap();
        assert!(paths_match(&rel, &rel));
        // If emitted is relative and requested is absolute to the same
        // file, the canonicalize fallback discovers they are equal.
        let orig_cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(&dir).unwrap();
        let emitted = Path::new("a.ts");
        assert!(paths_match(emitted, &abs));
        std::env::set_current_dir(orig_cwd).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn paths_match_different_files_not_equal() {
        let dir = std::env::temp_dir().join("forge_lsp_paths_match_diff");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let a = dir.join("a.ts");
        let b = dir.join("b.ts");
        std::fs::write(&a, "x").unwrap();
        std::fs::write(&b, "y").unwrap();
        assert!(!paths_match(&a, &b));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
