//! # forge_tracera
//!
//! Outbound telemetry sink for the **Tracera** observability wire format.
//!
//! Provides:
//!
//! - [`TelemetrySink`] — async trait that any sink implements.
//! - [`TraceraEvent`] — the canonical wire-level event type (serde-tagged).
//! - [`TraceraSink`] — concrete HTTP sink with bearer / HMAC auth, batching,
//!   offline retry, and back-pressure.
//! - [`MemoryStore`] — pluggable in-memory batch queue with offline retry,
//!   used by default; suitable for tests and short-lived processes.
//!
//! ## Wire format
//!
//! Events are serialized as a JSON object with the following envelope:
//!
//! ```json
//! {
//!   "schema": "tracera.v1",
//!   "id": "01J...",
//!   "ts": "2026-09-12T00:00:00Z",
//!   "source": "forgecode",
//!   "session_id": "...",
//!   "kind": "tool_call",
//!   "payload": { ... }
//! }
//! ```
//!
//! Batches are sent as `{ "events": [...] }` to the configured endpoint.
//!
//! ## Auth
//!
//! Two strategies are supported via [`AuthMode`]:
//!
//! - `Bearer(token)` — `Authorization: Bearer <token>` header.
//! - `Hmac { secret, header }` — `X-Tracera-Signature: <hex hmac-sha256>`.
//!
//! ## Offline retry
//!
//! On transport failure (network error, 5xx, 429) events are kept in the
//! in-memory store. The sink retries with exponential back-off on
//! [`TraceraSink::flush`]. After the configured `max_retries` the queue is
//! dropped (caller decides whether to log / persist / page).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod config;
pub mod event;
pub mod sink;
pub mod store;

mod error;

pub use config::{AuthMode, SinkConfig};
pub use error::{SinkError, SinkResult};
pub use event::{EventKind, TraceraEvent};
pub use sink::TraceraSink;
pub use store::{MemoryStore, StoreHandle};

use async_trait::async_trait;
use std::sync::Arc;

/// Async trait implemented by any telemetry sink.
#[async_trait]
pub trait TelemetrySink: Send + Sync {
    /// Accept an event for delivery.
    ///
    /// Implementations should not block; they may buffer.
    async fn submit(&self, event: TraceraEvent) -> SinkResult<()>;

    /// Force a flush of any buffered events.
    ///
    /// Returns the number of events successfully delivered.
    async fn flush(&self) -> SinkResult<usize>;

    /// Drain pending events and shut down. After `return` the sink is dead.
    async fn shutdown(&self) -> SinkResult<usize>;
}

/// Convenience alias for a shared sink.
pub type SharedSink = Arc<dyn TelemetrySink>;
