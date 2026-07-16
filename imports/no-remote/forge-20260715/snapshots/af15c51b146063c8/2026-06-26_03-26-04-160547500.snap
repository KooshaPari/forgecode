//! # byteport-otel
//!
//! BytePort OpenTelemetry instrumentation: metrics, tracing, and OTLP export.
//!
//! ## Modules
//!
//! | Module             | Description                                          |
//! |--------------------|------------------------------------------------------|
//! | [`init`]           | Global `TracerProvider` / `MeterProvider` bootstrap   |
//! | [`metrics`]        | BytePort-specific metric instruments                 |
//! | [`tracing`]        | BytePort-specific span helpers and propagation       |
//! | [`config`]         | Configuration types for the observability stack      |
//!
//! ## Usage
//!
//! ```rust,no_run
//! use byteport_otel::init;
//!
//! let guard = init::init_telemetry(init::TelemetryConfig::default());
//! // application runs here
//! drop(guard); // flushes all spans and metrics on shutdown
//! ```
//!
//! ## Feature flags
//!
//! - `semconv` (default): enables semantic-convention attribute helpers.

pub mod config;
pub mod init;
pub mod metrics;
pub mod tracing;

// Re-export commonly used OTel types for convenience.
pub use opentelemetry::{
    Context,
    KeyValue,
    metrics::{Counter, Histogram, UpDownCounter},
    trace::{Span, SpanKind, Status, Tracer},
};
