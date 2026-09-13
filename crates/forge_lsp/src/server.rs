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
use std::sync::Arc;

use crate::completion::{CompletionItem, CompletionProvider, CompletionResult};
use crate::definition::{DefinitionProvider, DefinitionResult, Location};
use crate::diagnostic as _diag; // alias to avoid name clash; not used directly
use crate::diagnostic::Diagnostic;
use crate::hover::{Hover, HoverProvider, HoverResult};
use crate::implementation::{ImplementationProvider, ImplementationResult};
use crate::lsp_client::Position;
use crate::references::{ReferencesProvider, ReferencesResult};
use crate::rename::{RenameProvider, RenameResult, WorkspaceEdit};
use crate::type_definition::{TypeDefinitionProvider, TypeDefinitionResult};

/// Alias preserved from prior P2.3 naming.
pub use crate::service::DiagnosticsService;

/// Re-export so callers can construct a `SharedServer` without importing
/// the inner service type.
pub use crate::service::SharedDiagnosticsService;

/// Cheap-clone handle for `Server`.
pub type SharedServer = Arc<Server>;

/// The central LSP facade. Holds:
///   * `diagnostics` — the P2.3 `DiagnosticsService` (rustc + tsc)
///   * `hover`       — `HoverProvider` over `rust-analyzer` / `tsserver`
///   * `definition`  — `DefinitionProvider` over the same
///   * `completion`  — `CompletionProvider` over the same
///   * `implementation` — `ImplementationProvider` (F1)
///   * `references`  — `ReferencesProvider` (F1)
///   * `rename`      — `RenameProvider` (F1)
///   * `type_definition` — `TypeDefinitionProvider` (F1)
pub struct Server {
    workspace_root: PathBuf,
    diagnostics: DiagnosticsService,
    hover: HoverProvider,
    definition: DefinitionProvider,
    completion: CompletionProvider,
    implementation: ImplementationProvider,
    references: ReferencesProvider,
    rename: RenameProvider,
    type_definition: TypeDefinitionProvider,
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
            .field("rename", &self.rename.name())
            .field("type_definition", &self.type_definition.name())
            .finish()
    }
}

impl Server {
    /// Build a server with the default providers (rustc + tsc for
    /// diagnostics, `rust-analyzer` / `tsserver` for hover/definition/completion).
    /// Returns an error if any subprocess fails to spawn — callers that
    /// want partial degradation should use [`Server::new`] with explicit
    /// providers.
    pub fn with_defaults(workspace_root: &Path) -> Result<Self, String> {
        let rust = crate::lsp_client::ProcessLspClient::rust_analyzer()?;
        let ts = crate::lsp_client::ProcessLspClient::tsserver()?;
        let rust2 = rust.clone();
        let rust3 = rust.clone();
        let rust4 = rust.clone();
        let rust5 = rust.clone();
        let rust_hover = rust.clone();
        Ok(Self::new(
            workspace_root,
            DiagnosticsService::with_defaults(),
            HoverProvider::new(Box::new(rust_hover)),
            DefinitionProvider::new(Box::new(ts)),
            CompletionProvider::new(Box::new(rust)),
            ImplementationProvider::new(Box::new(rust2)),
            ReferencesProvider::new(Box::new(rust3)),
            RenameProvider::new(Box::new(rust4)),
            TypeDefinitionProvider::new(Box::new(rust5)),
        ))
    }

    /// Build a server with explicit providers (used by tests and by
    /// callers that need finer-grained control).
    pub fn new(
        workspace_root: &Path,
        diagnostics: DiagnosticsService,
        hover: HoverProvider,
        definition: DefinitionProvider,
        completion: CompletionProvider,
        implementation: ImplementationProvider,
        references: ReferencesProvider,
        rename: RenameProvider,
        type_definition: TypeDefinitionProvider,
    ) -> Self {
        Self {
            workspace_root: workspace_root.to_path_buf(),
            diagnostics,
            hover,
            definition,
            completion,
            implementation,
            references,
            rename,
            type_definition,
        }
    }

    /// Workspace root this server was bound to.
    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    /// Diagnostics pass (P2.3).
    pub fn diagnostics_for_path(&self, path: &Path) -> Vec<Diagnostic> {
        // Surface the error as an empty vector — callers that want the
        // error can use `DiagnosticsService::diagnostics_for_path` directly.
        self.diagnostics
            .diagnostics_for_path(path, &self.workspace_root)
            .unwrap_or_default()
    }

    /// Invalidate cached diagnostics for `path`.
    pub fn invalidate(&self, path: &Path) {
        self.diagnostics.invalidate(path);
    }

    /// Hover at `position` in `path`. `path` is converted to a `file://` URI.
    pub fn hover(&self, path: &Path, position: Position) -> HoverResult {
        if !self.hover.supports(path) {
            // For unsupported files, treat as "no hover available" rather
            // than an error — the REPL doesn't care, and forcing the
            // caller to special-case errors is annoying.
            return Ok(None);
        }
        let uri = path_to_uri(path);
        self.hover.hover(&uri, position)
    }

    /// Definition at `position` in `path`.
    pub fn definition(&self, path: &Path, position: Position) -> DefinitionResult {
        if !self.definition.supports(path) {
            return Ok(Vec::new());
        }
        let uri = path_to_uri(path);
        self.definition.definition(&uri, position)
    }

    /// Completion at `position` in `path`.
    pub fn complete(
        &self,
        path: &Path,
        position: Position,
        trigger: Option<char>,
    ) -> CompletionResult {
        if !self.completion.supports(path) {
            return Ok(Vec::new());
        }
        let uri = path_to_uri(path);
        self.completion.complete(&uri, position, trigger)
    }

    pub fn implementation(&self, path: &Path, position: Position) -> ImplementationResult {
        if !self.implementation.supports(path) { return Ok(Vec::new()); }
        self.implementation.implementation(&path_to_uri(path), position)
    }
    pub fn references(&self, path: &Path, position: Position, include_declaration: bool) -> ReferencesResult {
        if !self.references.supports(path) { return Ok(Vec::new()); }
        self.references.references(&path_to_uri(path), position, include_declaration)
    }
    pub fn rename(&self, path: &Path, position: Position, new_name: &str) -> RenameResult {
        if !self.rename.supports(path) { return Ok(None); }
        self.rename.rename(&path_to_uri(path), position, new_name)
    }
    pub fn type_definition(&self, path: &Path, position: Position) -> TypeDefinitionResult {
        if !self.type_definition.supports(path) { return Ok(Vec::new()); }
        self.type_definition.type_definition(&path_to_uri(path), position)
    }
}

/// Convert an absolute path to a `file://` URI.
fn path_to_uri(path: &Path) -> String {
    // Naive implementation — no percent-encoding. Sufficient for our
    // test paths; production code would use the `url` crate.
    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::path::PathBuf::from("/").join(path)
    };
    format!("file://{}", abs.display().to_string().replace('\\', "/"))
}

#[allow(dead_code)]
fn _force_link(_: &_diag::Diagnostic) {} // keep diagnostics re-export alive

#[cfg(test)]
mod tests {
    use super::*;
    use crate::completion::CompletionKind;
    use crate::definition::Location;
    use crate::hover::Hover;
    use crate::lsp_client::{LspClient, LspRequest, LspResponse};
    use std::sync::Mutex;

    /// Shared mock used by all three provider modules — keeps the
    /// per-module tests consistent.
    pub struct RoutingMockClient {
        name: &'static str,
        scripted: Mutex<Vec<Result<LspResponse, String>>>,
        captured: Mutex<Vec<LspRequest>>,
    }

    impl RoutingMockClient {
        pub fn new(name: &'static str, responses: Vec<LspResponse>) -> Self {
            Self {
                name,
                scripted: Mutex::new(responses.into_iter().map(Ok).collect()),
                captured: Mutex::new(Vec::new()),
            }
        }
    }

    impl LspClient for RoutingMockClient {
        fn send(&self, request: LspRequest) -> Result<LspResponse, String> {
            self.captured.lock().unwrap().push(request);
            let mut q = self.scripted.lock().unwrap();
            if q.is_empty() {
                Ok(LspResponse {
                    jsonrpc: Some("2.0".into()),
                    id: Some(0),
                    result: Some(serde_json::Value::Null),
                    error: None,
                })
            } else {
                q.remove(0)
            }
        }
        fn server_name(&self) -> &'static str {
            self.name
        }
    }

    fn workspace_dir() -> PathBuf {
        std::env::temp_dir().join("forge_lsp_test_server")
    }

    #[test]
    fn server_routes_unsupported_paths_to_empty() {
        let server = Server::new(
            &workspace_dir(),
            DiagnosticsService::with_defaults(),
            HoverProvider::new(boxed(client_name("rust-analyzer"))),
            DefinitionProvider::new(boxed(client_name("rust-analyzer"))),
            CompletionProvider::new(boxed(client_name("rust-analyzer"))),
            crate::implementation::ImplementationProvider::new(boxed(client_name("rust-analyzer"))),
            crate::references::ReferencesProvider::new(boxed(client_name("rust-analyzer"))),
            crate::rename::RenameProvider::new(boxed(client_name("rust-analyzer"))),
            crate::type_definition::TypeDefinitionProvider::new(boxed(client_name("rust-analyzer"))),
        );
        let h = server
            .hover(&PathBuf::from("foo.py"), Position { line: 0, character: 0 })
            .unwrap();
        assert!(h.is_none());
        let defs = server
            .definition(&PathBuf::from("foo.py"), Position { line: 0, character: 0 })
            .unwrap();
        assert!(defs.is_empty());
        let comps = server
            .complete(
                &PathBuf::from("foo.py"),
                Position { line: 0, character: 0 },
                None,
            )
            .unwrap();
        assert!(comps.is_empty());
    }

    #[test]
    fn server_holds_workspace_root_and_provider_names() {
        let server = Server::new(
            &workspace_dir(),
            DiagnosticsService::with_defaults(),
            HoverProvider::new(boxed(client_name("rust-analyzer"))),
            DefinitionProvider::new(boxed(client_name("tsserver"))),
            CompletionProvider::new(boxed(client_name("rust-analyzer"))),
            crate::implementation::ImplementationProvider::new(boxed(client_name("rust-analyzer"))),
            crate::references::ReferencesProvider::new(boxed(client_name("rust-analyzer"))),
            crate::rename::RenameProvider::new(boxed(client_name("rust-analyzer"))),
            crate::type_definition::TypeDefinitionProvider::new(boxed(client_name("rust-analyzer"))),
        );
        assert_eq!(server.workspace_root(), workspace_dir());
        let dbg = format!("{server:?}");
        assert!(dbg.contains("rust-analyzer"));
        assert!(dbg.contains("tsserver"));
    }

    #[test]
    fn server_definition_routes_through_provider() {
        // script a successful single-location response for the
        // underlying definition provider
        let client = RoutingMockClient::new(
            "rust-analyzer",
            vec![LspResponse {
                jsonrpc: Some("2.0".into()),
                id: Some(1),
                result: Some(serde_json::json!({
                    "uri":"file:///lib.rs",
                    "range":{"start":{"line":0,"character":0},"end":{"line":0,"character":3}}
                })),
                error: None,
            }],
        );
        let server = Server::new(
            &workspace_dir(),
            DiagnosticsService::with_defaults(),
            HoverProvider::new(boxed(client_name("rust-analyzer"))),
            DefinitionProvider::new(boxed(client)),
            CompletionProvider::new(boxed(client_name("rust-analyzer"))),
            crate::implementation::ImplementationProvider::new(boxed(client_name("rust-analyzer"))),
            crate::references::ReferencesProvider::new(boxed(client_name("rust-analyzer"))),
            crate::rename::RenameProvider::new(boxed(client_name("rust-analyzer"))),
            crate::type_definition::TypeDefinitionProvider::new(boxed(client_name("rust-analyzer"))),
        );
        let locs = server
            .definition(
                &PathBuf::from("src/foo.rs"),
                Position { line: 0, character: 0 },
            )
            .unwrap();
        assert_eq!(locs.len(), 1);
        assert_eq!(locs[0].uri, "file:///lib.rs");
    }

    #[test]
    fn server_completion_routes_through_provider() {
        let client = RoutingMockClient::new(
            "tsserver",
            vec![LspResponse {
                jsonrpc: Some("2.0".into()),
                id: Some(1),
                result: Some(serde_json::json!([
                    {"label":"hello","kind":1}
                ])),
                error: None,
            }],
        );
        let server = Server::new(
            &workspace_dir(),
            DiagnosticsService::with_defaults(),
            HoverProvider::new(boxed(client_name("tsserver"))),
            DefinitionProvider::new(boxed(client_name("tsserver"))),
            CompletionProvider::new(boxed(client)),
            crate::implementation::ImplementationProvider::new(boxed(client_name("rust-analyzer"))),
            crate::references::ReferencesProvider::new(boxed(client_name("rust-analyzer"))),
            crate::rename::RenameProvider::new(boxed(client_name("rust-analyzer"))),
            crate::type_definition::TypeDefinitionProvider::new(boxed(client_name("rust-analyzer"))),
        );
        let comps = server
            .complete(
                &PathBuf::from("src/foo.ts"),
                Position { line: 1, character: 4 },
                Some('.'),
            )
            .unwrap();
        assert_eq!(comps.len(), 1);
        assert_eq!(comps[0].label, "hello");
        assert_eq!(comps[0].kind, Some(CompletionKind::Text));
    }

    #[test]
    fn server_hover_routes_through_provider() {
        let client = RoutingMockClient::new(
            "rust-analyzer",
            vec![LspResponse {
                jsonrpc: Some("2.0".into()),
                id: Some(1),
                result: Some(serde_json::json!({"contents":"fn main()"})),
                error: None,
            }],
        );
        let server = Server::new(
            &workspace_dir(),
            DiagnosticsService::with_defaults(),
            HoverProvider::new(boxed(client)),
            DefinitionProvider::new(boxed(client_name("rust-analyzer"))),
            CompletionProvider::new(boxed(client_name("rust-analyzer"))),
            crate::implementation::ImplementationProvider::new(boxed(client_name("rust-analyzer"))),
            crate::references::ReferencesProvider::new(boxed(client_name("rust-analyzer"))),
            crate::rename::RenameProvider::new(boxed(client_name("rust-analyzer"))),
            crate::type_definition::TypeDefinitionProvider::new(boxed(client_name("rust-analyzer"))),
        );
        let h = server
            .hover(
                &PathBuf::from("src/main.rs"),
                Position { line: 0, character: 0 },
            )
            .unwrap();
        let h = h.expect("hover present");
        assert_eq!(h.contents, "fn main()");
    }

    fn client_name(name: &'static str) -> RoutingMockClient {
        RoutingMockClient::new(name, vec![])
    }

    /// Wrap a concrete client in a `Box<dyn LspClient>`.
    fn boxed<T: LspClient + 'static>(c: T) -> Box<dyn LspClient> {
        Box::new(c)
    }

    // Compile-time sanity: re-exported types are usable.
    #[allow(dead_code)]
    fn _type_aliases(_h: Hover, _l: Location, _c: CompletionItem) {}
}
