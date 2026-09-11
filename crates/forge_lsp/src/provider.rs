//! `DiagnosticsProvider` trait — the per-language extension point.
//!
//! Each backend (`rustc`, `tsc`, …) implements this trait to declare
//! which file extensions it supports and how to produce diagnostics.
//! The service (`DiagnosticsService`) is the only place that knows
//! about providers; downstream callers (the agent's REPL, the audit
//! pipeline) use the service's `diagnostics(&Path)` entry point.

use std::path::Path;

use crate::diagnostic::Diagnostic;

/// Result of a provider's diagnostic pass. Per repo `AGENTS.md`, services
/// surface errors through `anyhow::Result` so structured causes and
/// context chains survive across the provider / service boundary.
pub type DiagnosticsResult = anyhow::Result<Vec<Diagnostic>>;

/// A language backend.
pub trait DiagnosticsProvider: Send + Sync {
    /// Stable identifier (`"rustc"`, `"tsc"`, …) used in audit
    /// records and tool call metadata.
    fn name(&self) -> &'static str;

    /// Whether this provider can handle `path` (typically a file
    /// extension check).
    fn supports(&self, path: &Path) -> bool;

    /// Run the diagnostic pass for `path` (relative to
    /// `workspace_root`). Implementations typically shell out to a
    /// compiler/linter and parse its output.
    fn diagnostics(&self, path: &Path, workspace_root: &Path) -> DiagnosticsResult;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::{Diagnostic, DiagnosticSeverity};
    use std::path::PathBuf;

    struct StubProvider;
    impl DiagnosticsProvider for StubProvider {
        fn name(&self) -> &'static str {
            "stub"
        }
        fn supports(&self, path: &Path) -> bool {
            path.extension().and_then(|e| e.to_str()) == Some("xyz")
        }
        fn diagnostics(&self, _path: &Path, _workspace_root: &Path) -> DiagnosticsResult {
            Ok(vec![Diagnostic::new(
                PathBuf::from("test.xyz"),
                DiagnosticSeverity::Warning,
                1,
                "stub",
            )])
        }
    }

    #[test]
    fn supports_returns_true_for_matching_extension() {
        let p = StubProvider;
        assert!(p.supports(Path::new("foo.xyz")));
        assert!(!p.supports(Path::new("foo.rs")));
        assert!(!p.supports(Path::new("foo")));
    }

    #[test]
    fn name_returns_stable_id() {
        assert_eq!(StubProvider.name(), "stub");
    }

    #[test]
    fn diagnostics_returns_vec() {
        let p = StubProvider;
        let d = p
            .diagnostics(Path::new("/tmp/foo.xyz"), Path::new("/tmp"))
            .unwrap();
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].message, "stub");
    }

    #[test]
    fn diagnostics_result_err_propagates_anyhow() {
        // The trait uses `anyhow::Result` so an implementation can
        // surface whatever it wants. Verify that an `Err(anyhow!(...))`
        // round-trips cleanly and renders its original message.
        let res: DiagnosticsResult = Err(anyhow::anyhow!("tool not found"));
        assert!(res.is_err());
        let err = match res {
            Ok(_) => panic!("expected Err"),
            Err(e) => e,
        };
        assert_eq!(err.to_string(), "tool not found");
    }
}
