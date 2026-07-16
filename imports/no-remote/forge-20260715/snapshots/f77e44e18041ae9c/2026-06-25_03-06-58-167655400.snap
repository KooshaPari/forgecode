# Httpora

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![CI](https://github.com/KooshaPari/Httpora/actions/workflows/ci.yml/badge.svg)](https://github.com/KooshaPari/Httpora/actions/workflows/ci.yml)
[![Python](https://img.shields.io/badge/python-3.11+-3776AB.svg?logo=python&logoColor=white)](pyproject.toml)

HTTP client/server toolkit with middleware support for the Phenotype ecosystem.

## Overview

`Httpora` (also published as `httpkit`) is a Rust HTTP utility library providing ergonomic middleware, request/response helpers, and common HTTP patterns (rate limiting, retries, circuit breaking) as composable building blocks for Phenotype HTTP services.

## Features

- **Composable Middleware Stack**: Tower-compatible async HTTP middleware
- **Rate Limiting**: Token bucket and fixed-window rate limiters
- **Retry Logic**: Exponential backoff with jitter for failed requests
- **Circuit Breaker**: Automatic failure detection and recovery
- **CORS Helpers**: Cross-origin resource sharing utilities
- **Request/Response Builders**: Ergonomic HTTP message construction

## Installation

```bash
# Add to Cargo.toml
[dependencies]
httpkit = { git = "https://github.com/KooshaPari/Httpora" }
```

Or with specific features:

```toml
[dependencies]
httpkit = { path = "../Httpora", features = ["full"] }
```

## Quick Start

```rust
use httpkit::{RateLimiter, RetryLayer, CircuitBreaker};
use std::time::Duration;

// Create a token bucket rate limiter
let limiter = RateLimiter::token_bucket(100, 10.0); // 100 capacity, 10/sec refill

// Configure retry with exponential backoff
let retry = RetryLayer::new(
    3, // max retries
    Duration::from_millis(100), // base delay
);

// Create a circuit breaker
let cb = CircuitBreaker::new(0.5, Duration::from_secs(30)); // 50% threshold, 30s timeout
```

## API Overview

### Middleware

| Component | Description |
|-----------|-------------|
| `RateLimiter` | Token bucket and fixed-window rate limiting |
| `RetryLayer` | Exponential backoff retry with jitter |
| `CircuitBreaker` | Three-state circuit breaker (closed/open/half-open) |
| `CorsLayer` | Cross-origin request handling |

### Error Types

- `HttpKitError::RateLimited` - Request exceeded rate limit
- `HttpKitError::CircuitOpen` - Circuit breaker is open
- `HttpKitError::RetryExhausted` - Max retries exceeded
- `HttpKitError::ParseError` - Response parsing failed

### Builders

```rust
use httpkit::{ResponseBuilder, RequestExtractor};

// Build JSON response
let response = ResponseBuilder::json(200, &my_data)?;

// Extract JSON body from request
let body: MyType = RequestExtractor::json_body(&request)?;
```

## Architecture

```
Httpora/
├── src/
│   ├── middleware/       # Tower middleware implementations
│   │   ├── rate_limit.rs
│   │   ├── retry.rs
│   │   └── circuit_breaker.rs
│   ├── builder/          # Request/response builders
│   ├── error.rs          # Error types
│   └── lib.rs            # Crate root
├── tests/                # Integration tests
└── Cargo.toml
```

## Development

```bash
# Build
cargo build

# Run tests
cargo test

# Check lints
cargo clippy -- -D warnings

# Format code
cargo fmt
```

## Testing

```bash
# Unit tests
cargo test --lib

# Integration tests
cargo test --test '*'

# With coverage
cargo llvm-cov --html
```

## Related Phenotype Projects

- **[cloud](../cloud/)** — Multi-tenant SaaS using Httpora middleware stack
- **[PhenoDevOps](../PhenoDevOps/)** — HTTP-based service orchestration via Httpora
- **[agentapi-plusplus](../agentapi-plusplus/)** — Multi-model gateway with Httpora client

## Links

- **Docs**: [https://docs.rs/httpkit](https://docs.rs/httpkit) (pending publication)
- **Repository**: https://github.com/KooshaPari/Httpora
- **Issues**: https://github.com/KooshaPari/Httpora/issues

## Contributing

1. Follow the [Phenotype contribution guidelines](https://github.com/KooshaPari/phenotype/blob/main/CONTRIBUTING.md)
2. Ensure all tests pass: `cargo test`
3. Ensure clippy is clean: `cargo clippy -- -D warnings`
4. Format code: `cargo fmt`
5. Submit PR with clear description

## License

MIT OR Apache-2.0
