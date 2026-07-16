> **Work-state:** beta — `[#######---] 70%`
>
> Observability primitives unified (tracing+metrics+logging). Tests + benchmarks wired. Exporters (OTLP, Prometheus, Jaeger, Zipkin) ported. Pending: edge-case stress tests, load-balancer integration tests, perf parity verification.

> **Pinned references (Phenotype-org)**
> - MSRV: see rust-toolchain.toml
> - cargo-deny config: see deny.toml
> - cargo-audit: rustsec/audit-check@v2 weekly
> - Branch protection: 1 reviewer required, no force-push
> - Authority: phenotype-org-governance/SUPERSEDED.md

# Tracely — Observability Primitives

Unified observability library for Rust: distributed tracing, metrics, and structured logging. Wraps OpenTelemetry, metrics, and tracing crates for ergonomic all-in-one observability. Supports OTLP, Prometheus, Jaeger, and Zipkin exports.

## Overview

**Tracely** is the canonical observability layer for the Phenotype ecosystem. It provides a unified, zero-allocation API for distributed tracing, metrics collection, and structured logging—all built on battle-tested crates (tracing, metrics, opentelemetry) without reimplementation.

**Core Mission**: Eliminate observability friction by providing one ergonomic API for spans, metrics, and logs, with seamless export to any backend.

## Technology Stack

- **Language**: Rust (Edition 2024)
- **Core Crates**:
  - `tracing` (0.1) — Span and event collection
  - `metrics` (0.21) — Counters, gauges, histograms
  - `opentelemetry` (0.21) — OTLP export and context propagation
- **Exporters**: OTLP, Prometheus, Jaeger, Zipkin
- **Serialization**: serde_json
- **Performance**: Criterion benchmarks; zero-allocation log path

## Key Features

- **Distributed Tracing**: OpenTelemetry-compatible spans with automatic context propagation
- **Metrics**: Ergonomic macros for counters, gauges, histograms with optional dimensions
- **Structured Logging**: Zero-allocation JSON logging macros
- **Multiple Exporters**: OTLP, Prometheus, Jaeger, Zipkin out-of-the-box
- **Unified API**: Single init call sets up all three observability pillars
- **Zero-Cost Abstractions**: Compiles down to efficient instrumentation

## Quick Start

```bash
# Clone and explore
git clone <repo-url>
cd Tracely

# Build and test
cargo build --release
cargo test --workspace
cargo clippy -- -D warnings    # Zero warnings enforced

# View documentation
cargo doc --open

# Run performance benchmarks
cargo bench
```

## Project Structure

```
Tracely/
├── crates/
│   ├── tracely-core/     # Core observability primitives & macros
│   └── tracely-sentinel/ # Monitoring, alerting, phenoSentinel integration
├── src/
│   ├── tracing/          # Span lifecycle, context propagation
│   ├── metrics/          # Counter, gauge, histogram macros
│   ├── logging/          # Structured JSON log macros
│   ├── export/           # OTLP, Prometheus, Jaeger, Zipkin exporters
│   └── config.rs         # TracingConfig builder
└── CLAUDE.md, AGENTS.md  # Governance & agent contract
```

## Usage Example

```rust
use tracely::{tracer, metrics, log};

// Initialize observability
tracely::init_with_defaults()?;

// Tracing
let span = tracer::start("process_request");
defer { span.end(); }

// Metrics
metrics::counter!("requests_total").inc();
metrics::histogram!("request_duration_ms").observe(duration_ms);

// Logging
log::info!("Request processed", { "duration_ms": 42, "user_id": user_id });
```

## Quality Requirements

- `cargo clippy -- -D warnings` — **zero warnings** required
- `cargo test --workspace` — all tests pass
- `cargo doc` — zero missing doc warnings on public API
- Benchmarks: verify zero-allocation log path via criterion

## Related Phenotype Projects

- **[PhenoObservability](../PhenoObservability)** — Platform-wide observability infrastructure
- **[Tracera](../Tracera)** — Distributed tracing backend and storage
- **[cheap-llm-mcp](../cheap-llm-mcp)** — Uses Tracely for LLM model routing metrics