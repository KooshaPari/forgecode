//! `Server` — the central LSP-facade type. Composes the diagnostics
//! service with the hover / definition / completion / implementation /
//! references / type-definition / rename providers behind a single
//! per-workspace object.
//!
//! P2.3 added `DiagnosticsService` (rustc + tsc). P2.3.1 added hover,
//! definition, completion. P2.3.2 adds implementation, references,
//! type-definition, and rename. This is the type downstream callers
//! (the agent REPL, the editor surface, the audit pipeline) should
//! hold — it exposes a small, language-routed API:
//!
//! ```ignore
//! let server = forge_lsp::Server::with_defaults(&workspace_root)?;
//! let diags  = server.diagnostics_for_path(&Path::new("src/foo.rs"))?;
//! let hover  = server.hover(&Path::new("src/foo.rs"), Position { line: 0, character: 0 })?;
//! let defs   = server.definition(&Path::new("src/foo.rs"), Position { line: 0, character: 0 })?;
//! let comps  = server.complete(&Path::new("src/foo.ts"), Position { line: 0, character: 0 }, Some('.'))?;
//! let impls  = server.implementation(&Path::new("src/trait.rs"), Position { line: 0, character: 0 })?;
//! let refs   = server.references(&Path::new("src/foo.rs"), Position { line: 0, character: 0 }, Default::default())?;
//! let tdef   = server.type_definition(&Path::new("src/foo.rs"), Position { line: 0, character: 0 })?;
//! let edit   = server.rename(&Path::new("src/foo.rs"), Position { line: 0, character: 0 }, "new_name")?;
//! ```
//!
//! `Server` is cheap-cloneable via `SharedServer`.

use std::path::{Path, PathBuf};

use crate::completion::{CompletionItem, CompletionProvider, CompletionResult};
use crate::definition::{DefinitionProvider, DefinitionResult, Location};
use crate::diagnostic::Diagnostic;
use crate::hover::{Hover, HoverProvider, HoverResult};
use crate::implementation::{ImplementationError, ImplementationProvider, ImplementationResult};
use crate::lsp_client::{Position, ProcessLspClient};
use crate::references::{
    ReferencesError, ReferencesOptions, ReferencesProvider, ReferencesResult,
    TypeDefinitionProvider,
};
use crate::rename::{RenameError, RenameOutcome, RenameProvider, WorkspaceEdit};

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
///   * `implementation` — `ImplementationRouter` over the same
///   * `references` — `ReferencesRouter` over the same
///   * `type_definition` — `TypeDefinitionRouter` over the same
///   * `rename` — `RenameRouter` over the same
pub struct Server {
    workspace_root: PathBuf,
    diagnostics: DiagnosticsService,
    hover: HoverRouter,
    definition: DefinitionRouter,
    completion: CompletionRouter,
    implementation: ImplementationRouter,
    references: ReferencesRouter,
    type_definition: TypeDefinitionRouter,
    rename: RenameRouter,
}

impl std::fmt::Debug for Server {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Server")
            .field("workspace_root", &self.workspace_root)
            .field("hover", &self.hover.name())
            .field("definition", &self.definition.name())
            .field("completion", &self.completion.name())
            .field("implementation", &self.implementation.name())
            .field("references", &self.references.name())
            .field("type_definition", &self.type_definition.name())
            .field("rename", &self.rename.name())
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
    ///   * `rust-analyzer` for Rust hover/definition/completion/
    ///     implementation/references/typeDefinition/rename
    ///   * `typescript-language-server` for TypeScript-family
    ///     hover/definition/completion/implementation/references/
    ///     typeDefinition/rename
    ///   * Plus rustc + tsc for diagnostics
    ///
    /// Both language servers are spawned, initialized against
    /// `workspace_root`, and the per-operation providers each get a
    /// `Box<dyn LspClient>` over the correct one. `ProcessLspClient`
    /// is `Clone` (it wraps its state in `Arc`), so each provider
    /// sees the same subprocess.
    ///
    /// Returns an error if either subprocess fails to spawn or to
    /// complete the LSP `initialize` handshake.
    pub fn with_defaults(workspace_root: &Path) -> Result<Self, String> {
        let rust = ProcessLspClient::rust_analyzer()?;
        let ts = ProcessLspClient::tsserver()?;

        // Each provider gets a `Box<dyn LspClient>`. Cloning the
        // `ProcessLspClient` shares the subprocess via `Arc`.
        let rust_box: Box<ProcessLspClient> = Box::new(rust);
        let ts_box: Box<ProcessLspClient> = Box::new(ts);

        Ok(Self::new(
            workspace_root,
            DiagnosticsService::with_defaults(),
            HoverProvider::new(rust_box.clone()),
            DefinitionProvider::new(rust_box.clone()),
            CompletionProvider::new(rust_box.clone()),
            ImplementationProvider::new(rust_box.clone()),
            ReferencesProvider::new(rust_box.clone()),
            TypeDefinitionProvider::new(rust_box.clone()),
            RenameProvider::new(rust_box.clone()),
            HoverProvider::new(ts_box.clone()),
            DefinitionProvider::new(ts_box.clone()),
            CompletionProvider::new(ts_box.clone()),
            ImplementationProvider::new(ts_box.clone()),
            ReferencesProvider::new(ts_box.clone()),
            TypeDefinitionProvider::new(ts_box.clone()),
            RenameProvider::new(ts_box),
        ))
    }

    /// Build a server with explicit providers (used by tests and by
    /// callers that need finer-grained control).
    ///
    /// `Server::hover` / `definition` / `complete` / `implementation`
    /// / `references` / `type_definition` / `rename` route by file
    /// extension internally, so each operation needs both a Rust and
    /// a TypeScript family provider.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        workspace_root: &Path,
        diagnostics: DiagnosticsService,
        hover_rust: HoverProvider,
        definition_rust: DefinitionProvider,
        completion_rust: CompletionProvider,
        implementation_rust: ImplementationProvider,
        references_rust: ReferencesProvider,
        type_definition_rust: TypeDefinitionProvider,
        rename_rust: RenameProvider,
        hover_ts: HoverProvider,
        definition_ts: DefinitionProvider,
        completion_ts: CompletionProvider,
        implementation_ts: ImplementationProvider,
        references_ts: ReferencesProvider,
        type_definition_ts: TypeDefinitionProvider,
        rename_ts: RenameProvider,
    ) -> Self {
        Self {
            workspace_root: workspace_root.to_path_buf(),
            diagnostics,
            hover: HoverRouter::new(hover_rust, hover_ts),
            definition: DefinitionRouter::new(definition_rust, definition_ts),
            completion: CompletionRouter::new(completion_rust, completion_ts),
            implementation: ImplementationRouter::new(implementation_rust, implementation_ts),
            references: ReferencesRouter::new(references_rust, references_ts),
            type_definition: TypeDefinitionRouter::new(type_definition_rust, type_definition_ts),
            rename: RenameRouter::new(rename_rust, rename_ts),
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

    /// Implementation at `position` in `path` (P2.3.2).
    pub fn implementation(&self, path: &Path, position: Position) -> ImplementationResult {
        match classify_language(path) {
            Language::Unsupported => Ok(Vec::new()),
            Language::Rust | Language::TypeScript => {
                let uri = path_to_uri(path, &self.workspace_root);
                self.implementation.implementation(&uri, position)
            }
        }
    }

    /// References at `position` in `path` (P2.3.2).
    pub fn references(
        &self,
        path: &Path,
        position: Position,
        options: ReferencesOptions,
    ) -> ReferencesResult {
        match classify_language(path) {
            Language::Unsupported => Ok(Vec::new()),
            Language::Rust | Language::TypeScript => {
                let uri = path_to_uri(path, &self.workspace_root);
                self.references.references(&uri, position, options)
            }
        }
    }

    /// Type definition at `position` in `path` (P2.3.2).
    pub fn type_definition(&self, path: &Path, position: Position) -> ReferencesResult {
        match classify_language(path) {
            Language::Unsupported => Ok(Vec::new()),
            Language::Rust | Language::TypeScript => {
                let uri = path_to_uri(path, &self.workspace_root);
                self.type_definition.type_definition(&uri, position)
            }
        }
    }

    /// Rename the symbol at `position` in `path` to `new_name`
    /// (P2.3.2). Returns the workspace edit set on success.
    pub fn rename(
        &self,
        path: &Path,
        position: Position,
        new_name: &str,
    ) -> Result<Option<WorkspaceEdit>, RenameError> {
        match classify_language(path) {
            Language::Unsupported => Ok(None),
            Language::Rust | Language::TypeScript => {
                let uri = path_to_uri(path, &self.workspace_root);
                self.rename.rename(&uri, position, new_name)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Routing wrappers — pick the right provider for each language family.
// Each router wraps a pair of concrete providers — one for Rust, one
// for the TS family.
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
    rust: HoverProvider,
    ts: HoverProvider,
}

impl HoverRouter {
    pub fn new(rust: HoverProvider, ts: HoverProvider) -> Self {
        Self { rust, ts }
    }

    pub fn name(&self) -> &'static str {
        self.rust.name()
    }

    pub fn hover(
        &self,
        uri: &str,
        position: Position,
    ) -> Result<Option<Hover>, crate::hover::HoverError> {
        if pick_ts_family(uri) {
            self.ts.hover(uri, position)
        } else {
            self.rust.hover(uri, position)
        }
    }
}

pub struct DefinitionRouter {
    rust: DefinitionProvider,
    ts: DefinitionProvider,
}

impl DefinitionRouter {
    pub fn new(rust: DefinitionProvider, ts: DefinitionProvider) -> Self {
        Self { rust, ts }
    }

    pub fn name(&self) -> &'static str {
        self.rust.name()
    }

    pub fn definition(
        &self,
        uri: &str,
        position: Position,
    ) -> Result<Vec<Location>, crate::definition::DefinitionError> {
        if pick_ts_family(uri) {
            self.ts.definition(uri, position)
        } else {
            self.rust.definition(uri, position)
        }
    }
}

pub struct CompletionRouter {
    rust: CompletionProvider,
    ts: CompletionProvider,
}

impl CompletionRouter {
    pub fn new(rust: CompletionProvider, ts: CompletionProvider) -> Self {
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

pub struct ImplementationRouter {
    rust: ImplementationProvider,
    ts: ImplementationProvider,
}

impl ImplementationRouter {
    pub fn new(rust: ImplementationProvider, ts: ImplementationProvider) -> Self {
        Self { rust, ts }
    }

    pub fn name(&self) -> &'static str {
        self.rust.name()
    }

    pub fn implementation(
        &self,
        uri: &str,
        position: Position,
    ) -> Result<Vec<Location>, ImplementationError> {
        if pick_ts_family(uri) {
            self.ts.implementation(uri, position)
        } else {
            self.rust.implementation(uri, position)
        }
    }
}

pub struct ReferencesRouter {
    rust: ReferencesProvider,
    ts: ReferencesProvider,
}

impl ReferencesRouter {
    pub fn new(rust: ReferencesProvider, ts: ReferencesProvider) -> Self {
        Self { rust, ts }
    }

    pub fn name(&self) -> &'static str {
        self.rust.name()
    }

    pub fn references(
        &self,
        uri: &str,
        position: Position,
        options: ReferencesOptions,
    ) -> Result<Vec<Location>, ReferencesError> {
        if pick_ts_family(uri) {
            self.ts.references(uri, position, options)
        } else {
            self.rust.references(uri, position, options)
        }
    }
}

pub struct TypeDefinitionRouter {
    rust: TypeDefinitionProvider,
    ts: TypeDefinitionProvider,
}

impl TypeDefinitionRouter {
    pub fn new(rust: TypeDefinitionProvider, ts: TypeDefinitionProvider) -> Self {
        Self { rust, ts }
    }

    pub fn name(&self) -> &'static str {
        self.rust.name()
    }

    pub fn type_definition(
        &self,
        uri: &str,
        position: Position,
    ) -> Result<Vec<Location>, ReferencesError> {
        if pick_ts_family(uri) {
            self.ts.type_definition(uri, position)
        } else {
            self.rust.type_definition(uri, position)
        }
    }
}

pub struct RenameRouter {
    rust: RenameProvider,
    ts: RenameProvider,
}

impl RenameRouter {
    pub fn new(rust: RenameProvider, ts: RenameProvider) -> Self {
        Self { rust, ts }
    }

    pub fn name(&self) -> &'static str {
        self.rust.name()
    }

    pub fn rename(&self, uri: &str, position: Position, new_name: &str) -> RenameOutcome {
        if pick_ts_family(uri) {
            self.ts.rename(uri, position, new_name)
        } else {
            self.rust.rename(uri, position, new_name)
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
    use crate::lsp_client::{LspClient, LspRequest, LspResponse};
    use std::sync::Mutex;

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
    fn _type_aliases(
        _h: Hover,
        _l: Location,
        _c: CompletionItem,
        _impl_err: ImplementationError,
        _refs_err: ReferencesError,
        _rename_err: RenameError,
    ) {
    }

    // -----------------------------------------------------------------------
    // Mock-driven Server integration tests — exercise the
    // language-routing wrappers using fake LSP clients so we don't
    // need rust-analyzer / typescript-language-server installed.
    // -----------------------------------------------------------------------
    mod server_routing {
        use super::*;
        use crate::completion::CompletionItem;
        use crate::hover::Hover;
        use crate::lsp_client::LspError;
        use std::path::PathBuf;

        /// Multi-response mock: returns successive scripted results so a
        /// single `Server` test can hit multiple endpoints. We derive
        /// `Clone` so multiple `Box<ScriptedClient>` can share the
        /// scripted queue — matching the way `ProcessLspClient` clones
        /// share their subprocess.
        #[derive(Clone)]
        struct ScriptedClient {
            name: &'static str,
            scripted: std::sync::Arc<Mutex<Vec<Result<LspResponse, String>>>>,
        }

        impl ScriptedClient {
            fn new(name: &'static str, responses: Vec<Result<LspResponse, String>>) -> Self {
                Self { name, scripted: std::sync::Arc::new(Mutex::new(responses)) }
            }
        }

        impl crate::lsp_client::LspClient for ScriptedClient {
            fn server_name(&self) -> &'static str {
                self.name
            }
            fn send(&self, _request: LspRequest) -> Result<LspResponse, String> {
                let next = self.scripted.lock().unwrap().remove(0);
                match next {
                    Ok(r) => Ok(r),
                    Err(e) => Err(e),
                }
            }
        }

        fn ok(value: serde_json::Value) -> LspResponse {
            LspResponse {
                jsonrpc: Some("2.0".into()),
                id: Some(1),
                result: Some(value),
                error: None,
            }
        }

        fn err(code: i64, message: &str) -> LspResponse {
            LspResponse {
                jsonrpc: Some("2.0".into()),
                id: Some(1),
                result: None,
                error: Some(LspError { code, message: message.to_string(), data: None }),
            }
        }

        fn build_server(
            rust_resp: Vec<Result<LspResponse, String>>,
            ts_resp: Vec<Result<LspResponse, String>>,
        ) -> Server {
            let rust = Box::new(ScriptedClient::new("rust-analyzer", rust_resp));
            let ts = Box::new(ScriptedClient::new("typescript-language-server", ts_resp));
            Server::new(
                &workspace_dir(),
                DiagnosticsService::with_defaults(),
                HoverProvider::new(rust.clone()),
                DefinitionProvider::new(rust.clone()),
                CompletionProvider::new(rust.clone()),
                ImplementationProvider::new(rust.clone()),
                ReferencesProvider::new(rust.clone()),
                TypeDefinitionProvider::new(rust.clone()),
                RenameProvider::new(rust.clone()),
                HoverProvider::new(ts.clone()),
                DefinitionProvider::new(ts.clone()),
                CompletionProvider::new(ts.clone()),
                ImplementationProvider::new(ts.clone()),
                ReferencesProvider::new(ts.clone()),
                TypeDefinitionProvider::new(ts.clone()),
                RenameProvider::new(ts),
            )
        }

        #[test]
        fn server_implementation_returns_empty_for_unsupported() {
            let server = build_server(vec![], vec![]);
            let r = server.implementation(Path::new("foo.py"), Position { line: 0, character: 0 });
            assert!(r.is_ok());
            assert!(r.unwrap().is_empty());
        }

        #[test]
        fn server_references_returns_empty_for_unsupported() {
            let server = build_server(vec![], vec![]);
            let r = server.references(
                Path::new("foo.py"),
                Position { line: 0, character: 0 },
                ReferencesOptions::default(),
            );
            assert!(r.is_ok());
            assert!(r.unwrap().is_empty());
        }

        #[test]
        fn server_type_definition_returns_empty_for_unsupported() {
            let server = build_server(vec![], vec![]);
            let r = server.type_definition(Path::new("foo.py"), Position { line: 0, character: 0 });
            assert!(r.is_ok());
            assert!(r.unwrap().is_empty());
        }

        #[test]
        fn server_rename_returns_none_for_unsupported() {
            let server = build_server(vec![], vec![]);
            let r = server.rename(
                Path::new("foo.py"),
                Position { line: 0, character: 0 },
                "new",
            );
            assert!(r.is_ok());
            assert!(r.unwrap().is_none());
        }

        #[test]
        fn server_implementation_routes_rust_file_to_rust_analyzer() {
            let server = build_server(
                vec![Ok(ok(serde_json::json!([
                    {"uri":"file:///impl.rs","range":{"start":{"line":1,"character":0},"end":{"line":1,"character":3}}}
                ])))],
                vec![Ok(err(-1, "ts should not be called"))],
            );
            let r = server
                .implementation(Path::new("foo.rs"), Position { line: 0, character: 0 })
                .unwrap();
            assert_eq!(r.len(), 1);
            assert_eq!(r.first().map(|l| l.uri.as_str()), Some("file:///impl.rs"));
        }

        #[test]
        fn server_implementation_routes_typescript_to_ts_server() {
            let server = build_server(
                vec![Ok(err(-1, "rust should not be called"))],
                vec![Ok(ok(serde_json::json!([
                    {"uri":"file:///impl.ts","range":{"start":{"line":2,"character":0},"end":{"line":2,"character":3}}}
                ])))],
            );
            let r = server
                .implementation(Path::new("foo.ts"), Position { line: 0, character: 0 })
                .unwrap();
            assert_eq!(r.len(), 1);
            assert_eq!(r.first().map(|l| l.uri.as_str()), Some("file:///impl.ts"));
        }

        #[test]
        fn server_references_routes_rust_file_to_rust_analyzer() {
            let server = build_server(
                vec![Ok(ok(serde_json::json!([
                    {"uri":"file:///use.rs","range":{"start":{"line":1,"character":0},"end":{"line":1,"character":3}}}
                ])))],
                vec![],
            );
            let r = server
                .references(
                    Path::new("foo.rs"),
                    Position { line: 0, character: 0 },
                    ReferencesOptions::default(),
                )
                .unwrap();
            assert_eq!(r.len(), 1);
            assert_eq!(r.first().map(|l| l.uri.as_str()), Some("file:///use.rs"));
        }

        #[test]
        fn server_references_routes_tsx_file_to_ts_server() {
            let server = build_server(
                vec![],
                vec![Ok(ok(serde_json::json!([
                    {"uri":"file:///use.tsx","range":{"start":{"line":4,"character":0},"end":{"line":4,"character":3}}}
                ])))],
            );
            let r = server
                .references(
                    Path::new("foo.tsx"),
                    Position { line: 0, character: 0 },
                    ReferencesOptions::default(),
                )
                .unwrap();
            assert_eq!(r.len(), 1);
            assert_eq!(r.first().map(|l| l.uri.as_str()), Some("file:///use.tsx"));
        }

        #[test]
        fn server_type_definition_routes_rust_file() {
            let server = build_server(
                vec![Ok(ok(serde_json::json!({
                    "uri": "file:///types.rs",
                    "range": {"start":{"line":5,"character":0},"end":{"line":5,"character":4}}
                })))],
                vec![],
            );
            let r = server
                .type_definition(Path::new("foo.rs"), Position { line: 0, character: 0 })
                .unwrap();
            assert_eq!(r.len(), 1);
            assert_eq!(r.first().map(|l| l.uri.as_str()), Some("file:///types.rs"));
        }

        #[test]
        fn server_type_definition_routes_mjs_file() {
            let server = build_server(
                vec![],
                vec![Ok(ok(serde_json::json!({
                    "uri": "file:///types.mjs",
                    "range": {"start":{"line":7,"character":0},"end":{"line":7,"character":4}}
                })))],
            );
            let r = server
                .type_definition(Path::new("foo.mjs"), Position { line: 0, character: 0 })
                .unwrap();
            assert_eq!(r.len(), 1);
            assert_eq!(r.first().map(|l| l.uri.as_str()), Some("file:///types.mjs"));
        }

        #[test]
        fn server_rename_routes_rust_file_to_rust_analyzer() {
            let server = build_server(
                vec![Ok(ok(serde_json::json!({
                    "documentChanges": [{
                        "textDocument": {"uri": "file:///a.rs"},
                        "edits": [{"range":{"start":{"line":1,"character":0},"end":{"line":1,"character":3}},"newText":"x"}]
                    }]
                })))],
                vec![],
            );
            let r = server
                .rename(Path::new("foo.rs"), Position { line: 0, character: 0 }, "x")
                .unwrap()
                .expect("rename should return WorkspaceEdit");
            assert_eq!(r.document_changes.len(), 1);
            assert_eq!(r.document_changes[0].text_document.uri, "file:///a.rs");
        }

        #[test]
        fn server_rename_routes_ts_file_to_ts_server() {
            let server = build_server(
                vec![],
                vec![Ok(ok(serde_json::json!({
                    "documentChanges": [{
                        "textDocument": {"uri": "file:///a.ts"},
                        "edits": [{"range":{"start":{"line":1,"character":0},"end":{"line":1,"character":3}},"newText":"y"}]
                    }]
                })))],
            );
            let r = server
                .rename(Path::new("foo.ts"), Position { line: 0, character: 0 }, "y")
                .unwrap()
                .expect("rename should return WorkspaceEdit");
            assert_eq!(r.document_changes.len(), 1);
            assert_eq!(r.document_changes[0].text_document.uri, "file:///a.ts");
        }

        #[test]
        fn server_hover_routes_rust_to_rust_analyzer() {
            let server = build_server(
                vec![Ok(ok(serde_json::json!({"contents":"fn foo()"})))],
                vec![Ok(err(-1, "ts should not be called"))],
            );
            let r = server
                .hover(Path::new("foo.rs"), Position { line: 0, character: 0 })
                .unwrap();
            assert!(r.is_some());
            assert_eq!(r.unwrap().contents, "fn foo()");
        }

        #[test]
        fn server_definition_routes_ts_to_ts_server() {
            let server = build_server(
                vec![Ok(err(-1, "rust should not be called"))],
                vec![Ok(ok(serde_json::json!([
                    {"uri":"file:///def.ts","range":{"start":{"line":3,"character":0},"end":{"line":3,"character":3}}}
                ])))],
            );
            let r = server
                .definition(Path::new("foo.ts"), Position { line: 0, character: 0 })
                .unwrap();
            assert_eq!(r.len(), 1);
            assert_eq!(r.first().map(|l| l.uri.as_str()), Some("file:///def.ts"));
        }

        #[test]
        fn server_complete_routes_rust_to_rust_analyzer() {
            let server = build_server(
                vec![Ok(ok(serde_json::json!([{"label":"foo","kind":3}])))],
                vec![],
            );
            let r = server
                .complete(
                    Path::new("foo.rs"),
                    Position { line: 0, character: 0 },
                    None,
                )
                .unwrap();
            assert_eq!(r.len(), 1);
            assert_eq!(r.first().map(|c| c.label.as_str()), Some("foo"));
        }

        // Compile-time alias check.
        #[allow(dead_code)]
        fn _compile_aliases(_h: Hover, _c: CompletionItem, _loc: Location, _p: PathBuf) {}
    }

    // -----------------------------------------------------------------------
    // Standalone compile-time checks that ensure the type-aliases
    // exposed by `lib.rs` for the new handlers are usable.
    // -----------------------------------------------------------------------
    #[allow(clippy::too_many_arguments, dead_code)]
    fn _exposed_aliases_compile(
        _h: crate::hover::Hover,
        _loc: crate::definition::Location,
        _c: crate::completion::CompletionItem,
        _ip: crate::implementation::ImplementationProvider,
        _rp: crate::references::ReferencesProvider,
        _tp: crate::references::TypeDefinitionProvider,
        _rn: crate::rename::RenameProvider,
        _we: crate::rename::WorkspaceEdit,
        _ro: crate::references::ReferencesOptions,
    ) {
    }
}
