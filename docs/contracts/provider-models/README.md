# Provider-Model Contract

**Version:** 1.0.0
**Status:** Reference artifact — not yet a published package.

## Purpose

This directory contains the language-agnostic contract for the provider/model registry
surface shared across three KooshaPari repos that independently implement the same domain:

| Repo | Language | Role |
|------|----------|------|
| forgecode | Rust | CLI coding agent — reference implementation |
| OmniRoute | TypeScript | LLM router / proxy |
| cliproxy | Go | CLI auth proxy |

All three implement `Provider → Models → Capabilities` and the same SSE/OAuth stop rules.
Rather than a single shared binary (impossible across Rust/TS/Go without FFI/WASM overhead),
the contract is a **JSON Schema** that each repo aligns its native types against.

## Files

| File | Description |
|------|-------------|
| `provider-model.schema.json` | JSON Schema 2020-12 for `Model`, `ProviderConfig`, `SseStopRule`, `OAuthRefreshPolicy` |
| `oauth-refresh-policy.schema.json` | P5.2 — full OAuth token-field model + parameterized refresh-lead / expiry policy (`OAuthTokens`, `OAuthRefreshPolicy`, per-provider `RefreshLeadOverride`) |
| `resilience-policy.schema.json` | P5.3 — request retry/backoff params, retryable-error classification, SSE reconnect backoff, and the SSE terminal-marker rule set |
| `README.md` | This file |

> The `oauth-refresh-policy` and `resilience-policy` schemas are the P5.2/P5.3 *expansions* of the
> inline `OAuthRefreshPolicy` and `SseStopRule` `$defs` already embedded in
> `provider-model.schema.json`. The inline versions remain the minimal cross-reference; the
> dedicated files add the full token model, per-provider lead overrides, retry parameters, and the
> retryable-error taxonomy. Field names and defaults are kept identical between inline and expanded forms.

## How to use this contract

### forgecode (Rust)

`forge_domain::Model` and `forge_domain::Provider` are the reference implementation.
`forge_eventsource::is_sse_terminal` is the reference implementation of `SseStopRule`.
When the domain types change, update the schema to stay in sync.

### OmniRoute (TypeScript)

Optionally codegen TypeScript types via:

```bash
npx json-schema-to-typescript provider-model.schema.json -o src/types/provider-model.d.ts
```

Align `src/lib/modelCapabilities.ts`, `src/lib/sseTextTransform.ts`, and
`src/lib/tokenHealthCheck.ts` against the schema semantics (field names, enum values,
`is_sse_terminal` logic, and `TOKEN_EXPIRY_BUFFER`).

### cliproxy (Go)

Optionally codegen Go structs via:

```bash
go-jsonschema -p registry provider-model.schema.json -o pkg/llmproxy/registry/provider_model_gen.go
```

Align `pkg/llmproxy/registry/model_registry.go` field names and capability enums to the schema.
Per-provider `RefreshLead()` overrides (e.g. codebuddy 24 h) are valid because the
`oauth_refresh_policy.default_refresh_lead_seconds` field is explicitly *parameterized*.

## SSE terminal-marker rules (normative)

Implementations MUST treat the following SSE event data values as end-of-stream:

- `[DONE]` — the canonical OpenAI/Anthropic sentinel
- `""` (empty string) — keepalive / implicit close

Additionally:
- OpenAI: `choices[0].finish_reason` in `{stop, length, content_filter, tool_calls}` signals model completion.
- Anthropic: `stop_reason` / `message_delta.stop_reason` fields signal model completion.
- **Synthetic `[DONE]` on silent close:** when the upstream connection closes without an
  explicit terminal event, implementations MUST emit a synthetic terminal signal rather
  than propagating an unexpected EOF to callers.

See `forge_eventsource::is_sse_terminal` for the canonical Rust implementation.

## OAuth refresh policy (normative)

A token needs refresh when:

```
now + refresh_lead >= token.expires_at
```

Default `refresh_lead` is **300 seconds (5 minutes)**, matching:
- forgecode: `chrono::Duration::minutes(5)`
- OmniRoute: `TOKEN_EXPIRY_BUFFER = 5 * 60 * 1000`
- cliproxy: `5 * time.Minute` (most providers)

Per-provider overrides are valid (e.g. cliproxy codebuddy uses 86400 s).
The contract requires the lead to be *parameterized*, not hardcoded.

## OAuth token model + refresh policy (P5.2, normative)

`oauth-refresh-policy.schema.json` defines the persisted token set and the refresh decision.

**Token fields** (`OAuthTokens`): `access_token` (required), `refresh_token`, `id_token`,
`expires_at` (required, absolute RFC 3339 UTC), `token_type` (default `Bearer`), `scope`.
A relative `expires_in` from the token endpoint MUST be converted to an absolute `expires_at`
at acquisition time. forgecode reference: `forge_domain::auth::credentials::OAuthTokens`.

**Refresh predicate** (normative, identical across repos):

```
needs_refresh  =  now + lead >= expires_at
is_expired     =  now >= expires_at            // lead = 0
```

**Parameterized lead.** Default `300 s` (forgecode `Duration::minutes(5)`, OmniRoute
`5*60*1000`, cliproxy `5 * time.Minute`). Per-provider overrides via `refresh_lead_overrides`
are conformant, not violations — e.g. cliproxy codebuddy uses `86400 s`.

**Credential-kind short-circuits:** `api_key` / `aws_profile` never need refresh;
`google_adc` (≈1h, re-minted) always does. Reference: `AuthCredential::needs_refresh`.

## Resilience policy: retry + SSE (P5.3, normative)

`resilience-policy.schema.json` covers concerns 3 (retry/backoff) and 4 (SSE stop-signal).
Only parameters and rules are contractual; the backoff loop stays language-native
(Rust `backon`, TS/Go bespoke).

**Request retry** (`RetryPolicy`): `max_attempts`, `initial_backoff_ms`, `min_delay_ms`,
`max_delay_secs`, `backoff_factor` (default 2), `jitter` (default true), `suppress_errors`.
Reference: forgecode `forge_config::RetryConfig`.

**Retryable classification:** retry when HTTP status ∈ retryable set
`{408, 429, 500, 502, 503, 504, 520, 522, 524, 529}` (forgecode `RetryConfig.status_codes`)
OR error kind ∈ transport set (`timeout`, `connection_reset`, …). Non-retryable client
statuses `{400, 401, 403, 404, 422}` always win. forgecode reference:
`forge_app::retry::should_retry` (gates on `Error::Retryable`).

**SSE reconnect** (`SseReconnectPolicy`): `start_ms` 300, `factor` 2, `max_delay_ms` 5000,
`max_retries` null (unbounded), honor server `retry:` field. Reference:
`forge_eventsource::retry::DEFAULT_RETRY`.

**SSE stop rule** (`SseStopRule`): terminal `data:` values `["[DONE]", ""]`
(forgecode `is_sse_terminal`), OpenAI `finish_reason`, Anthropic `stop_reason` /
`message_delta.stop_reason`, and the synthetic-`[DONE]`-on-silent-close rule.

## Versioning

Contract changes follow semver:
- **Patch** — clarifications, description-only updates, no field changes.
- **Minor** — new optional fields; existing fields unchanged.
- **Major** — field renames, type changes, or removal of fields.

Each consuming repo should record the contract version it was aligned against in its
own changelog or a `docs/contracts/VERSION` pin file.
