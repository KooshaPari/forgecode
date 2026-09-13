//! `forge_lsp` — language-server-style services for the agent's
//! REPL. The crate exposes:
//!
//!   * P2.3 — `DiagnosticsProvider` trait + `RustcProvider` and
//!     `TscProvider` (cargo check / tsc --noEmit) wired through a
//!     bounded LRU cache (`DiagnosticsService`).
//!   * P2.3.1 — `HoverProvider`, `DefinitionProvider`,
//!     `CompletionProvider` (LSP `textDocument/hover`,
//!     `textDocument/definition`, `textDocument/completion`) over a
//!     process-backed JSON-RPC client (`lsp_client::ProcessLspClient`),
//!     plus a `Server` facade that bundles everything behind a single
//!     per-workspace handle.
//!
//! Hover/definition/completion degrade gracefully when no language
//! server is installed: the providers still parse whatever the
//! subprocess returns, and `Server::hover` returns `Ok(None)` for
//! unsupported files so the REPL doesn't have to special-case errors.

#![allow(missing_docs, dead_code, unused)]

pub mod completion;
pub mod definition;
pub mod diagnostic;
pub mod hover;
pub mod lsp_client;
pub mod mcp_watcher;
pub mod provider;
pub mod rustc;
pub mod server;
pub mod service;
pub mod tsc;

pub use completion::{CompletionItem, CompletionKind, CompletionProvider};
pub use definition::{DefinitionProvider, Location};
pub use diagnostic::{Diagnostic, DiagnosticSeverity};
pub use hover::{Hover, HoverProvider};
pub use lsp_client::{LspClient, ProcessLspClient, ServerCapabilities, WeakProcessClient};
pub use mcp_watcher::{
    DEFAULT_DEBOUNCE, McpAutoReloadConfig, McpReloadFn, McpWatcher, McpWatcherError,
    McpWatcherHandle, default_mcp_config_path,
};
pub use provider::{DiagnosticsError, DiagnosticsProvider, DiagnosticsResult};
pub use rustc::RustcProvider;
pub use server::{Server, SharedServer};
pub use service::{DiagnosticsService, LruCache, SharedDiagnosticsService};
pub use tsc::TscProvider;
