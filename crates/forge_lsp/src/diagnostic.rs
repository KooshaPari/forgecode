//! `Diagnostic` value type — the unified LSP-style diagnostic shape
//! every provider produces and every consumer (agent REPL, audit
//! pipeline, future editor surfaces) consumes.

use std::path::PathBuf;

/// Severity classification for a [`Diagnostic`].
///
/// Follows the LSP `DiagnosticSeverity` ordering (`Error` is the most
/// severe, `Hint` the least).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum DiagnosticSeverity {
    /// Compilation/parse error — the code will not produce the
    /// expected output without intervention.
    Error,
    /// Suspicious construct that is still legal. Worth fixing but not
    /// strictly required.
    #[default]
    Warning,
    /// Informational note from the compiler (e.g. an `unused variable`).
    Information,
    /// Subtle hint, often pointing at a possible improvement rather
    /// than a problem.
    Hint,
}

impl DiagnosticSeverity {
    /// String form for audit/tooling (e.g. `"error"`).
    pub fn as_str(&self) -> &'static str {
        match self {
            DiagnosticSeverity::Error => "error",
            DiagnosticSeverity::Warning => "warning",
            DiagnosticSeverity::Information => "information",
            DiagnosticSeverity::Hint => "hint",
        }
    }
}

/// A single LSP-style diagnostic produced by a [`crate::provider::DiagnosticsProvider`].
#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    /// File the diagnostic refers to. The path may be absolute or
    /// workspace-relative depending on what the underlying provider
    /// emits (e.g. `cargo check` reports absolute paths).
    pub file: PathBuf,
    /// Severity bucket. See [`DiagnosticSeverity`].
    pub severity: DiagnosticSeverity,
    /// 1-based line number.
    pub line: u32,
    /// Human-readable message.
    pub message: String,
    /// Source identifier (provider name) — e.g. `"rustc"`, `"tsc"`.
    pub source: Option<String>,
}

impl Diagnostic {
    /// Construct a new `Diagnostic`. The `source` field defaults to
    /// `None`; use [`Self::with_source`] to attach a provider label.
    pub fn new(
        file: PathBuf,
        severity: DiagnosticSeverity,
        line: u32,
        message: impl Into<String>,
    ) -> Self {
        Self { file, severity, line, message: message.into(), source: None }
    }

    /// Attach a provider/source label to this diagnostic.
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn diagnostic_severity_as_str() {
        let fixture = [
            (DiagnosticSeverity::Error, "error"),
            (DiagnosticSeverity::Warning, "warning"),
            (DiagnosticSeverity::Information, "information"),
            (DiagnosticSeverity::Hint, "hint"),
        ];
        let actual: Vec<(&'static str, &'static str)> = fixture
            .iter()
            .map(|(sev, label)| (sev.as_str(), *label))
            .collect();
        let expected: Vec<(&'static str, &'static str)> = vec![
            ("error", "error"),
            ("warning", "warning"),
            ("information", "information"),
            ("hint", "hint"),
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn diagnostic_severity_default_is_warning() {
        let actual: DiagnosticSeverity = Default::default();
        let expected = DiagnosticSeverity::Warning;
        assert_eq!(actual, expected);
    }

    #[test]
    fn diagnostic_new_sets_fields() {
        let actual = Diagnostic::new(
            PathBuf::from("foo.rs"),
            DiagnosticSeverity::Error,
            42,
            "bad",
        );
        let expected = Diagnostic {
            file: PathBuf::from("foo.rs"),
            severity: DiagnosticSeverity::Error,
            line: 42,
            message: "bad".to_string(),
            source: None,
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn diagnostic_with_source_overrides_default() {
        let actual = Diagnostic::new(PathBuf::from("x.ts"), DiagnosticSeverity::Warning, 1, "msg")
            .with_source("tsc");
        assert_eq!(actual.source.as_deref(), Some("tsc"));
    }

    #[test]
    fn diagnostic_message_accepts_string_and_str() {
        let from_str = Diagnostic::new(
            PathBuf::from("a.rs"),
            DiagnosticSeverity::Hint,
            1,
            "from &str",
        );
        let from_string = Diagnostic::new(
            PathBuf::from("a.rs"),
            DiagnosticSeverity::Hint,
            1,
            String::from("from String"),
        );
        assert_eq!(from_str.message, "from &str");
        assert_eq!(from_string.message, "from String");
    }
}
