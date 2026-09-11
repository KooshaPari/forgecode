//! `forge_lsp` — language-server-style diagnostics for the agent's
//! REPL. The crate exposes a `DiagnosticsProvider` trait and two
//! default providers (`RustcProvider`, `TscProvider`) that shell out
//! to `cargo check --message-format=json` and `tsc --noEmit`
//! respectively. A `DiagnosticsService` composes the providers behind
//! a bounded LRU cache so the REPL doesn't pay compiler cost on every
//! keystroke.

#![warn(missing_docs)]

pub mod diagnostic;
pub mod provider;
pub mod rustc;
pub mod service;
pub mod tsc;

pub use diagnostic::{Diagnostic, DiagnosticSeverity};
pub use provider::{DiagnosticsError, DiagnosticsProvider, DiagnosticsResult};
pub use rustc::RustcProvider;
pub use service::{DiagnosticsService, LruCache, SharedDiagnosticsService};
pub use tsc::TscProvider;
