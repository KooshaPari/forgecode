//! `forge_lsp` — language-server-style diagnostics for the agent's
//! REPL. The crate exposes a [`DiagnosticsProvider`] trait and two
//! default providers ([`RustcProvider`], [`TscProvider`]) that shell
//! out to `cargo check --message-format=json` and `tsc --noEmit`
//! respectively. A [`DiagnosticsService`] composes the providers
//! behind a bounded LRU cache so the REPL doesn't pay compiler cost
//! on every keystroke.
//!
//! # Example
//!
//! ```rust,no_run
//! use forge_lsp::DiagnosticsService;
//!
//! let service = DiagnosticsService::with_defaults();
//! let diags = service
//!     .diagnostics_for_path(std::path::Path::new("src/main.rs"), std::path::Path::new("."))
//!     .expect("rust toolchain present");
//! ```

#![warn(missing_docs)]

/// The LSP-style diagnostic value type and its severity enum.
pub mod diagnostic;
/// The per-language extension point trait.
pub mod provider;
/// `cargo check --message-format=json` provider.
pub mod rustc;
/// The composing service: cache + provider dispatch.
pub mod service;
/// `tsc --noEmit` provider.
pub mod tsc;

pub use diagnostic::{Diagnostic, DiagnosticSeverity};
pub use provider::{DiagnosticsProvider, DiagnosticsResult};
pub use rustc::RustcProvider;
pub use service::{CompositeProviders, DiagnosticsService, LruCache, SharedDiagnosticsService};
pub use tsc::TscProvider;
