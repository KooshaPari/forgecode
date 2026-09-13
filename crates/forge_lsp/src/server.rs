//! `Server` — the central LSP-facade type. Composes the diagnostics
//! service with the hover / definition / completion providers behind a
//! single per-workspace object.
//!
//! P2.3 added `DiagnosticsService` (rustc + tsc). P2.3.1 adds the
//! three new providers. This is the type downstream callers (the
//! agent REPL, the editor surface, the audit pipeline) should hold —
//! it exposes a small, language-routed API:
//!
//! ```ignore
//! let server = forge_lsp::Server::with_defaults(&workspace_root)?;
//! let diags = server.diagnostics_for_path(&Path::new("src/foo.rs"))?;
//! let hover = server.hover(&Path::new("src/foo.rs"), Position { line: 0, character: 0 })?;
//! let defs  = server.definition(&Path::new("src/foo.rs"), Position { line: 0, character: 0 })?;
//! let comps = server.complete(&Path::new("src/foo.ts"), Position { line: 0, character: 0 }, Some('.'))?;
//! ```
//!
//! `Server` is cheap-cloneable via `SharedServer`.

use std::path::{Path, PathBuf};

use crate::completion::{CompletionItem, CompletionProvider, CompletionResult};
use crate::definition::{DefinitionProvider, DefinitionResult, Location};
use crate::diagnostic::Diagnostic;
use crate::hover::{Hover, HoverProvider, HoverResult};
use crate::lsp_client::{LspClient, Position, ProcessLspClient};

/// Alias preserved from prior P2.3 naming.
pub use crate::service::DiagnosticsService;

/// Re-export so callers can construct a `SharedServer` without importing
/// the inner service type.
pub use crate::service::SharedDiagnosticsService;

/// Cheap-clone handle for `Server`.
pub type SharedServer = std::sync::Arc<Server>;

/// The central LSP facade. Holds:
///   * `diagnostics` — the P2.3 `DiagnosticsService` (rustc + tsc)
///   * `hover` — `HoverRouter` over `rust-analyzer` /
///     `typescript-language-server`
///   * `definition` — `DefinitionRouter` over the same
///   * `completion` — `CompletionRouter` over the same
pub struct Server {
    workspace_root: PathBuf,
    diagnostics: DiagnosticsService,
    hover: HoverRouter,
    definition: DefinitionRouter,
    completion: CompletionRouter,
}

impl std::fmt::Debug for Server {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Server")
            .field("workspace_root", &self.workspace_root)
            .field("hover", &self.hover.name())
            .field("definition", &self.definition.name())
            .field("completion", &self.completion.name())
            .finish()
    }
}

/// Classify a path's language for routing decisions. Centralised here so
/// the per-provider `supports()` checks and the language-aware routing
/// in [`Server::with_defaults`] agree on what counts as rust vs.
/// typescript-family.
pub(crate) fn classify_language(path: &Path) -> Language {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase());
    match ext.as_deref() {
        Some("rs") => Language::Rust,
        Some("ts") | Some("tsx") | Some("mts") | Some("cts") | Some("js") | Some("jsx")
        | Some("mjs") | Some("cjs") => Language::TypeScript,
        _ => Language::Unsupported,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Language {
    Rust,
    TypeScript,
    Unsupported,
}

impl Server {
    /// Build a server with the default providers:
    ///   * `rust-analyzer` for Rust hover/definition/completion
    ///   * `typescript-language-server` for TypeScript-family
    ///     hover/definition/completion
    ///   * Plus rustc + tsc for diagnostics
    ///
    /// Both language servers are spawned, initialized against
    /// `workspace_root`, and the per-operation providers each get a
    /// reference to the correct one (so we never route a Rust file
    /// through `typescript-language-server` or vice-versa).
    ///
    /// Returns an error if either subprocess fails to spawn or to
    /// complete the LSP `initialize` handshake.
    pub fn with_defaults(workspace_root: &Path) -> Result<Self, String> {
        let rust = ProcessLspClient::rust_analyzer()?;
        rust.initialize(workspace_root)?;
        let ts = ProcessLspClient::typescript_language_server()?;
        ts.initialize(workspace_root)?;

        // We hand the same Arc<ProcessLspClient> to all three providers
        // for each language family. `ProcessLspClient` clones share
        // the subprocess via Arc.
        let rust_arc = std::sync::Arc::new(rust);
        let ts_arc = std::sync::Arc::new(ts);

        Ok(Self::new(
            workspace_root,
            DiagnosticsService::with_defaults(),
            HoverProvider::new(rust_arc.clone()),
            DefinitionProvider::new(rust_arc.clone()),
            CompletionProvider::new(rust_arc),
            HoverProvider::new(ts_arc.clone()),
            DefinitionProvider::new(ts_arc.clone()),
            CompletionProvider::new(ts_arc),
        ))
    }

    /// Build a server with explicit providers (used by tests and by
    /// callers that need finer-grained control).
    ///
    /// `Server::hover` / `definition` / `complete` route by file
    /// extension internally, so each operation pair (hover,
    /// definition, completion) needs both a Rust and a TypeScript
    /// family provider.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        workspace_root: &Path,
        diagnostics: DiagnosticsService,
        hover_rust: HoverProvider<ProcessLspClient>,
        definition_rust: DefinitionProvider<ProcessLspClient>,
        completion_rust: CompletionProvider<ProcessLspClient>,
        hover_ts: HoverProvider<ProcessLspClient>,
        definition_ts: DefinitionProvider<ProcessLspClient>,
        completion_ts: CompletionProvider<ProcessLspClient>,
    ) -> Self {
        Self {
            workspace_root: workspace_root.to_path_buf(),
            diagnostics,
            hover: HoverRouter::new(hover_rust, hover_ts),
            definition: DefinitionRouter::new(definition_rust, definition_ts),
            completion: CompletionRouter::new(completion_rust, completion_ts),
        }
    }

    /// Workspace root this server was bound to.
    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    /// Diagnostics pass (P2.3).
    pub fn diagnostics_for_path(&self, path: &Path) -> Vec<Diagnostic> {
        self.diagnostics
            .diagnostics_for_path(path, &self.workspace_root)
            .unwrap_or_default()
    }

    /// Invalidate cached diagnostics for `path`.
    pub fn invalidate(&self, path: &Path) {
        self.diagnostics.invalidate(path);
    }

    /// Hover at `position` in `path`. `path` is converted to a `file://`
    /// URI, resolved against `workspace_root` when relative.
    pub fn hover(&self, path: &Path, position: Position) -> HoverResult {
        match classify_language(path) {
            Language::Unsupported => Ok(None),
            Language::Rust | Language::TypeScript => {
                let uri = path_to_uri(path, &self.workspace_root);
                self.hover.hover(&uri, position)
            }
        }
    }

    /// Definition at `position` in `path`.
    pub fn definition(&self, path: &Path, position: Position) -> DefinitionResult {
        match classify_language(path) {
            Language::Unsupported => Ok(Vec::new()),
            Language::Rust | Language::TypeScript => {
                let uri = path_to_uri(path, &self.workspace_root);
                self.definition.definition(&uri, position)
            }
        }
    }

    /// Completion at `position` in `path`.
    pub fn complete(
        &self,
        path: &Path,
        position: Position,
        trigger: Option<char>,
    ) -> CompletionResult {
        match classify_language(path) {
            Language::Unsupported => Ok(Vec::new()),
            Language::Rust | Language::TypeScript => {
                let uri = path_to_uri(path, &self.workspace_root);
                self.completion.complete(&uri, position, trigger)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Routing wrappers — pick the right provider for each language family.
// Each router wraps a pair of concrete `HoverProvider<ProcessLspClient>` (or
// definition/completion variant) — one for Rust, one for the TS family.
// ---------------------------------------------------------------------------

fn pick_ts_family(uri: &str) -> bool {
    let path = uri_to_path(uri);
    matches!(
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .as_deref(),
        Some("ts")
            | Some("tsx")
            | Some("mts")
            | Some("cts")
            | Some("js")
            | Some("jsx")
            | Some("mjs")
            | Some("cjs")
    )
}

pub struct HoverRouter {
    rust: HoverProvider<ProcessLspClient>,
    ts: HoverProvider<ProcessLspClient>,
}

impl HoverRouter {
    pub fn new(rust: HoverProvider<ProcessLspClient>, ts: HoverProvider<ProcessLspClient>) -> Self {
        Self { rust, ts }
    }

    pub fn name(&self) -> &'static str {
        self.rust.name()
    }

    pub fn hover(
        &self,
        uri: &str,
        position: Position,
    ) -> Result<Option<crate::hover::Hover>, crate::hover::HoverError> {
        if pick_ts_family(uri) {
            self.ts.hover(uri, position)
        } else {
            self.rust.hover(uri, position)
        }
    }
}

pub struct DefinitionRouter {
    rust: DefinitionProvider<ProcessLspClient>,
    ts: DefinitionProvider<ProcessLspClient>,
}

impl DefinitionRouter {
    pub fn new(
        rust: DefinitionProvider<ProcessLspClient>,
        ts: DefinitionProvider<ProcessLspClient>,
    ) -> Self {
        Self { rust, ts }
    }

    pub fn name(&self) -> &'static str {
        self.rust.name()
    }

    pub fn definition(
        &self,
        uri: &str,
        position: Position,
    ) -> Result<Vec<crate::definition::Location>, crate::definition::DefinitionError> {
        if pick_ts_family(uri) {
            self.ts.definition(uri, position)
        } else {
            self.rust.definition(uri, position)
        }
    }
}

pub struct CompletionRouter {
    rust: CompletionProvider<ProcessLspClient>,
    ts: CompletionProvider<ProcessLspClient>,
}

impl CompletionRouter {
    pub fn new(
        rust: CompletionProvider<ProcessLspClient>,
        ts: CompletionProvider<ProcessLspClient>,
    ) -> Self {
        Self { rust, ts }
    }

    pub fn name(&self) -> &'static str {
        self.rust.name()
    }

    pub fn complete(
        &self,
        uri: &str,
        position: Position,
        trigger: Option<char>,
    ) -> Result<Vec<crate::completion::CompletionItem>, crate::completion::CompletionError> {
        if pick_ts_family(uri) {
            self.ts.complete(uri, position, trigger)
        } else {
            self.rust.complete(uri, position, trigger)
        }
    }
}

// ---------------------------------------------------------------------------
// Path <-> URI conversion (percent-encoded file:// URIs)
// ---------------------------------------------------------------------------

/// Convert an absolute-or-relative path to a `file://` URI. Relative
/// paths are resolved against `workspace_root`. The output is
/// percent-encoded per RFC 3986 via the `url` crate so paths with
/// spaces or non-ASCII characters round-trip safely.
pub(crate) fn path_to_uri(path: &Path, workspace_root: &Path) -> String {
    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace_root.join(path)
    };
    path_to_uri_for(&abs)
}

/// Convert an absolute path to a `file://` URI. Exposed (crate-private)
/// so `ProcessLspClient::initialize` can reuse the encoding logic
/// without taking a circular dependency on the rest of `Server`.
pub(crate) fn path_to_uri_for(abs_path: &Path) -> String {
    match url::Url::from_file_path(abs_path) {
        Ok(u) => u.to_string(),
        Err(_) => {
            // Fallback: best-effort manual conversion. Should never
            // happen on real absolute paths, but a malformed Path
            // shouldn't bring the whole LSP layer down.
            let s = abs_path.display().to_string().replace('\\', "/");
            format!("file://{s}")
        }
    }
}

fn uri_to_path(uri: &str) -> PathBuf {
    match url::Url::parse(uri) {
        Ok(u) if u.scheme() == "file" => {
            u.to_file_path().unwrap_or_else(|_| PathBuf::from(u.path()))
        }
        _ => PathBuf::from(uri),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace_dir() -> PathBuf {
        std::env::temp_dir().join("forge_lsp_test_server")
    }

    #[test]
    fn classify_language_routes_rust_extensions() {
        assert_eq!(classify_language(Path::new("foo.rs")), Language::Rust);
        assert_eq!(classify_language(Path::new("foo.RS")), Language::Rust);
    }

    #[test]
    fn classify_language_routes_typescript_family() {
        for ext in ["ts", "tsx", "mts", "cts", "js", "jsx", "mjs", "cjs"] {
            let p = PathBuf::from(format!("foo.{ext}"));
            assert_eq!(
                classify_language(&p),
                Language::TypeScript,
                "{ext} should be TS family"
            );
        }
    }

    #[test]
    fn classify_language_rejects_unsupported() {
        assert_eq!(
            classify_language(Path::new("foo.py")),
            Language::Unsupported
        );
        assert_eq!(classify_language(Path::new("foo")), Language::Unsupported);
    }

    #[test]
    fn path_to_uri_uses_url_crate_for_percent_encoding() {
        // A path with a space — the `url` crate percent-encodes it.
        let p = PathBuf::from("/tmp/has space/file.rs");
        let uri = path_to_uri(&p, &workspace_dir());
        assert!(uri.starts_with("file:///"), "expected file:///, got {uri}");
        assert!(
            uri.contains("has%20space") || uri.contains("has%20Space"),
            "expected percent-encoded space in {uri}"
        );
    }

    #[test]
    fn path_to_uri_resolves_relative_paths_against_workspace_root() {
        let root = PathBuf::from("/repo");
        let uri = path_to_uri(Path::new("src/foo.rs"), &root);
        assert!(uri.starts_with("file:///repo/"), "got {uri}");
        assert!(uri.ends_with("/repo/src/foo.rs"), "got {uri}");
    }

    // Compile-time sanity: re-exported types are usable.
    #[allow(dead_code)]
    fn _type_aliases(_h: Hover, _l: Location, _c: CompletionItem) {}
}
