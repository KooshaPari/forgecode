# OmniRoute Benchmark Plan

**Created:** 2026-05-05
**Status:** Complete
**Session:** 9d873d05

## Overview

Benchmark plan for comparing OmniRoute implementations: TypeScript vs Rust/Go performance.

## Implementation Status

### Completed
- [x] Created `src/lib/benchmarks.ts` with benchmark framework
- [x] Created `tests/unit/benchmarks.test.ts` with routing/mode selection benchmarks
- [x] Created `scripts/run-benchmarks.mjs` standalone runner
- [x] Added `npm run test:benchmarks` script
- [x] Created `benches/results/` directory for historical results
- [x] **Ran initial baseline benchmarks** (2026-05-07)

### Baseline Results (2026-05-07)

| Benchmark | Ops/sec | p50 (ms) | p95 (ms) | p99 (ms) | Status |
|-----------|--------|----------|----------|----------|--------|
| PolicyEngine.evaluate (no policies) | 292K | 0.000 | 0.001 | 0.001 | ✅ |
| PolicyEngine.evaluate (4 policies) | 96K | 0.002 | 0.004 | 0.050 | ✅ |
| PolicyEngine.evaluate (claude pattern) | 62K | 0.002 | 0.003 | 0.037 | ✅ |
| Fallback chain resolution | 272K | 0.000 | 0.001 | 0.001 | ✅ |
| Get next fallback | 498K | 0.000 | 0.000 | 0.002 | ✅ |
| Has fallback check | 986K | 0.000 | 0.000 | 0.001 | ✅ |
| String comparison (exact) | 6.28M | 0.000 | 0.000 | 0.000 | ✅ |
| String comparison (prefix) | 251K | 0.000 | 0.000 | 0.000 | ✅ |
| Token estimation | 371K | 0.000 | 0.000 | 0.000 | ✅ |
| Cost calculation | 382K | 0.000 | 0.000 | 0.001 | ✅ |
| Single fallback | 461K | 0.000 | 0.001 | 0.001 | ✅ |
| Multi-fallback | 434K | 0.001 | 0.001 | 0.001 | ✅ |
| Map.get (10 elements) | 181K | 0.000 | 0.000 | 0.000 | ✅ |
| Array.filter (10 elements) | 1.32M | 0.000 | 0.001 | 0.001 | ✅ |
| Set.has (10 elements) | 384K | 0.001 | 0.001 | 0.002 | ✅ |
| JSON.parse | 206K | 0.001 | 0.001 | 0.002 | ✅ |
| JSON.stringify | 326K | 0.000 | 0.000 | 0.001 | ✅ |

**Results file:** `OmniRoute/benches/2026-05-07-routing-benchmarks.json`

### Performance Assessment

All routing operations complete in **<1ms** at p99, well under the 5ms target.

## Test Scenarios

### 1. Request Routing Performance
- [x] Single route resolution (no model selection)
- [x] Multi-route resolution with fallback
- [x] Policy engine evaluation
- [ ] Concurrent request handling (100/500/1000 RPS) ← requires k6

### 2. Model Selection Latency
- [x] Token counting overhead
- [x] Cost calculation per provider
- [x] Model alias resolution
- [ ] Response time comparison (OpenAI vs Anthropic) ← requires live API

### 3. Provider Fallback Chains
- [x] Single fallback (1 primary, 1 backup)
- [x] Multi-fallback (1 primary, 2+ backups)
- [ ] Rate limit handling ← requires load testing

### 4. Throughput Benchmarks

| Scenario | TS Target | Status |
|----------|-----------|--------|
| Route Only | <5ms | ✅ Implemented |
| With Model Select | <50ms | ✅ Implemented |
| 100 RPS | <200ms p99 | ⏳ k6 load test |
| 500 RPS | <500ms p99 | ⏳ k6 load test |

## Benchmark Infrastructure

```
OmniRoute/
├── benches/                  # Benchmark suite
│   └── results/             # Historical results (YYYY-MM-DD/*.json)
├── src/lib/
│   └── benchmarks.ts        # Benchmark framework
└── tests/unit/
    └── benchmarks.test.ts    # Benchmark tests
```

## Execution Commands

```bash
# Run unit benchmarks
npm run test:benchmarks

# Run all benchmarks including latency percentiles
npx vitest run tests/unit/benchmarks.test.ts

# Run load tests (requires k6)
k6 run tests/load/proxy-load.js
k6 run tests/load/proxy-load.js --env BASE_URL=https://llms.omniroute.online
```

## Baseline Metrics Location

- TS baseline: `baseline_metrics.json` (from session 48462b3f)
- Results: `benches/results/YYYY-MM-DD/*.json`

## Next Steps

1. Run initial baseline: `npm run test:benchmarks`
2. Save results to `benches/results/2026-05-06/`
3. Compare with baseline from session 48462b3f
4. Document p50/p95/p99 latency targets

## Dependencies

- ✅ Node.js built-in `performance.now()` for timing
- ✅ Node.js built-in `--test` runner
- ⏳ k6 for load testing (`tests/load/proxy-load.ts`)
- ⏳ Go `benchstat` for comparison (future)
- ⏳ Rust Criterion (future, for Rust port)

## Running the Benchmarks

```bash
cd OmniRoute
npm run test:benchmarks
```

This will output:
- Operations per second for each benchmark
- p50/p95/p99 latency for policy evaluation
- Markdown-formatted results table
