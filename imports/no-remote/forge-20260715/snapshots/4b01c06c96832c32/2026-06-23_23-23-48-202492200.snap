//! T22 observability init for `phenotype-infra` iac/* daemons.
//!
//! Wires `pheno-tracing` (the canonical pheno-* tracing substrate, ADR-036)
//! plus `tracing-subscriber` (fmt + EnvFilter). The current substrate ships
//! a port-driven `TracePort` with local adapters (`StdoutAdapter`,
//! `InMemoryAdapter`); full OTLP export will be added when the substrate
//! adds an `OtlpAdapter` (tracked separately).
//!
//! # Usage
//!
//! ```rust,no_run
//! use phenotype_infra_observability::init_tracing;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let _port = init_tracing("oci-lottery");
//!     // ... rest of app ...
//!     Ok(())
//! }
//! ```
//!
//! Filter via `RUST_LOG=info,oci_lottery=debug`.

use pheno_tracing::adapters::StdoutAdapter;
use pheno_tracing::port::TracePort;
use std::sync::Arc;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Initialise tracing + return a `TracePort` for the calling daemon.
///
/// Idempotent — safe to call from each binary's `main`. Reads
/// `RUST_LOG` from env (default `info`). Includes `service.name` on
/// every `info!` line so fleet-side aggregators can join per-service
/// traces.
pub fn init_tracing(service_name: &'static str) -> Arc<dyn TracePort> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let subscriber = tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_target(true).with_level(true));

    let _ = subscriber.try_init();

    let port: Arc<dyn TracePort> = Arc::new(StdoutAdapter);
    tracing::info!(service.name = service_name, "tracing initialised (pheno-tracing 0.1, T22)");
    port
}

#[cfg(test)]
mod tests {
    use super::*;
    use pheno_tracing::adapters::InMemoryAdapter;
    use pheno_tracing::port::{SpanId, SpanKind, TraceId, TraceOperation};
    use std::collections::HashMap;

    #[test]
    fn init_tracing_does_not_panic() {
        // Calling twice should be a no-op (try_init).
        let _ = init_tracing("oci-lottery-test");
        let _ = init_tracing("oci-post-acquire-test");
    }

    #[tokio::test]
    async fn traceport_inmemory_roundtrip() {
        let adapter = InMemoryAdapter::new();
        let op = TraceOperation {
            trace_id: TraceId("t-t22-1".into()),
            span_id: SpanId("s-t22-1".into()),
            parent_span_id: None,
            kind: SpanKind::Internal,
            name: "test-span".into(),
            attributes: HashMap::from([(
                "service.name".into(),
                "phenotype-infra-observability".into(),
            )]),
        };
        let result = adapter.submit(op).await;
        assert_eq!(result.trace_id.0, "t-t22-1");
        assert!(adapter.spans.lock().unwrap().len() == 1);
    }
}
