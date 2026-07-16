//! BytePort-specific tracing helpers and span wrapping.
//!
//! Provides convenience functions for creating spans with consistent
//! attributes aligned with ADR-008.

use opentelemetry::{
    KeyValue,
    trace::{Span, SpanKind, Tracer, TracerProvider},
};
use opentelemetry::trace::TraceContextExt;

/// The tracer name used by BytePort.
const TRACER_NAME: &str = "byteport";

/// Start a root span for a BytePort request.
///
/// Adds common attributes (`service.name`, `byteport.version`).
pub fn start_request_span(
    tracer_provider: &impl TracerProvider,
    schema_id: i64,
    encoder_id: i64,
) -> Span {
    let tracer = tracer_provider.tracer(TRACER_NAME);
    tracer
        .span_builder("byteport.request")
        .with_kind(SpanKind::Server)
        .with_attributes(vec![
            KeyValue::new("byteport.schema_id", schema_id),
            KeyValue::new("byteport.encoder_id", encoder_id),
        ])
        .start(&tracer)
}

/// Start an encode span as a child of the current context.
pub fn start_encode_span(encoder_id: i64) -> Span {
    let tracer = opentelemetry::global::tracer(TRACER_NAME);
    let cx = opentelemetry::Context::current();
    tracer
        .span_builder("byteport.encode")
        .with_kind(SpanKind::Internal)
        .with_attributes(vec![KeyValue::new("byteport.encoder_id", encoder_id)])
        .start_with_context(&tracer, &cx)
}

/// Start a decode span as a child of the current context.
pub fn start_decode_span(encoder_id: i64) -> Span {
    let tracer = opentelemetry::global::tracer(TRACER_NAME);
    let cx = opentelemetry::Context::current();
    tracer
        .span_builder("byteport.decode")
        .with_kind(SpanKind::Internal)
        .with_attributes(vec![KeyValue::new("byteport.encoder_id", encoder_id)])
        .start_with_context(&tracer, &cx)
}

/// Start a transport span.
pub fn start_transport_span(transport_id: i64, operation: &str) -> Span {
    let tracer = opentelemetry::global::tracer(TRACER_NAME);
    let cx = opentelemetry::Context::current();
    tracer
        .span_builder(format!("byteport.transport.{operation}"))
        .with_kind(SpanKind::Client)
        .with_attributes(vec![KeyValue::new("byteport.transport_id", transport_id)])
        .start_with_context(&tracer, &cx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use opentelemetry_sdk::trace::TracerProvider;

    /// Verify that span creation does not panic in a no-op environment.
    #[test]
    fn span_creation_no_panic() {
        let provider = TracerProvider::default();
        let _span = start_request_span(&provider, 1, 2);
        // If we get here without panicking, the test passes.
    }

    #[test]
    fn global_span_no_panic() {
        // With no global provider set, these should use the no-op tracer.
        let _span = start_encode_span(42);
        let _span = start_decode_span(99);
        let _span = start_transport_span(7, "ping");
    }
}
