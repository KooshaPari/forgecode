<!-- AI-DD-META:START -->
<!-- This repository is planned, maintained, and managed by AI Agents only. -->
<!-- Slop issues are expected and intentionally present as part of an HITL-less -->
<!-- /minimized AI-DD metaproject of learning, refining, and building brute-force -->
<!-- training for both agents and the human operator. -->
![Downloads](https://img.shields.io/github/downloads/KooshaPari/BytePort/total?style=flat-square&label=downloads&color=blue)
![GitHub release](https://img.shields.io/github/v/release/KooshaPari/BytePort?style=flat-square&label=release)
![License](https://img.shields.io/github/license/KooshaPari/BytePort?style=flat-square)
![AI-Slop](https://img.shields.io/badge/AI--DD-Slop%20Expected-orange?style=flat-square)
![AI-Only-Maintained](https://img.shields.io/badge/Planned%20%26%20Maintained%20by-AI%20Agents%20Only-red?style=flat-square)
![HITL-less](https://img.shields.io/badge/HITL--less%20AI--DD-metaproject-yellow?style=flat-square)

> ⚠️ **AI-Agent-Only Repository**
>
> This repo is **planned, maintained, and managed exclusively by AI Agents**.
> Slop issues, rough edges, and AI artifacts are **expected and intentionally
> present** as part of an **HITL-less / minimized AI-DD** metaproject focused
> on learning, refining, and brute-force training both the agents and the
> human operator. Bug reports and contributions are still welcome, but please
> expect AI-generated code, comments, and documentation throughout.
<!-- AI-DD-META:END -->
> **Pinned references (Phenotype-org)**
> - MSRV: see rust-toolchain.toml (Tauri 2 shell)
> - cargo-deny config: see deny.toml
> - cargo-audit: rustsec/audit-check@v2 weekly
> - Branch protection: 1 reviewer required, no force-push
> - Authority: phenotype-org-governance/SUPERSEDED.md

# BytePort

[![CI](https://github.com/KooshaPari/BytePort/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/KooshaPari/BytePort/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/byteport.svg)](https://crates.io/crates/byteport)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Phenotype](https://img.shields.io/badge/Phenotype-org-blueviolet)](https://github.com/KooshaPari)

## Badges

[![Build](https://img.shields.io/github/actions/workflow/status/KooshaPari/BytePort/ci.yml?branch=main&label=build)](https://github.com/KooshaPari/BytePort/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/KooshaPari/BytePort?include_prereleases&sort=semver)](https://github.com/KooshaPari/BytePort/releases)
[![License](https://img.shields.io/github/license/KooshaPari/BytePort)](LICENSE)
[![Phenotype](https://img.shields.io/badge/Phenotype-org-blueviolet)](https://github.com/KooshaPari)
[![AI Slop Inside](https://sladge.net/badge.svg)](https://sladge.net)

> **Architecture:** See [ARCHITECTURE.md](ARCHITECTURE.md) for component architecture.
> **Threat model:** See [docs/security/threat-model.md](docs/security/threat-model.md) for the per-component STRIDE analysis.

## What is this?

**BytePort is a self-hosted Infrastructure-as-Code deployment + portfolio platform
for developer projects.** Define one manifest (`odin.nvms`) at your repo root and
BytePort provisions a MicroVM-backed deployment, registers the resulting endpoints
with a portfolio site, and uses an LLM to generate showcase metadata for each
project.

### Canonical stack (last updated 2026-06-12)

| Layer | Technology | Where |
|---|---|---|
| Backend API | Go 1.25 + Gin + GORM + SQLite | `backend/byteport/` |
| NVMS runtime | Go 1.25 + Spin (Fermyon wasm) | `backend/nvms/` |
| Persistence | SQLite via GORM | `backend/database.db` |
| Auth | PASETO v2 + httpOnly cookies | `backend/byteport/lib/auth.go` |
| Crypto | AES-256-CFB + Argon2id | `backend/byteport/lib/crypto.go` |
| GitHub | OAuth 2.0 (auto-refresh) | `backend/byteport/lib/git.go` |
| AWS SDK | EC2 / S3 / IAM provisioning | `backend/byteport/lib/aws*.go` |
| LLM | OpenAI (pluggable: local, Gemini-stub) | `backend/nvms/lib/llm.go` |
| Frontend | SvelteKit 2 + Svelte 5 + Tailwind 4 | `frontend/web/` |
| Desktop | Tauri 2 (Windows code-signed, macOS notarized) | `frontend/web/src-tauri/` |
| Telemetry | OpenTelemetry (OTLP-ready, ConsoleSpanExporter default) | `backend/byteport/main.go` |

> **Retired narrative.** Earlier revisions of this README described BytePort as
> a Loco.rs / Rust / NanoVMS project with a custom hypervisor and OS. That
> product was never built. The repo root is Go/SvelteKit/Tauri; the `nvms`
> runtime is a Spin wasm module, not a custom kernel.

---

## Quickstart (60 seconds)

### Prerequisites

| Tool | Version | Why |
|---|---|---|
| `go` | 1.25+ | Backend, NVMS |
| `node` | 20+ | SvelteKit |
| `npm` | 10+ | SvelteKit deps |
| `tmux` | any | `./start dev` orchestration |
| `spin` | 1.20+ | `nvms` runtime (https://developer.fermyon.com/spin) |
| `air` | latest | Go hot-reload (`go install github.com/cosmtrek/air@latest`) |
| `git` | any | obvious |

### One command to start everything

```sh
./start dev
```

This opens a tmux session with three panes:

- SvelteKit dev server (port 5173)
- Go backend with `air` hot-reload (port 8081)
- (Spin is started manually for `nvms` — see `backend/nvms/README.md`)

### Sign up, link GitHub, deploy your first project

1. Open <http://localhost:5173/signup>
2. Create an account
3. Open <http://localhost:5173/link>, click "Link GitHub", authorize
4. Open <http://localhost:5173/deploy>, pick a repo, give it a name and description, click Deploy
5. Wait ~90s. Your project is live at the `AccessURL` shown on `/instances`

### A minimal `odin.nvms`

```yaml
NAME: my-app
DESCRIPTION: A task management web application

SERVICES:
  - NAME: "main"           # Required — public-facing service, exposed at "/"
    PATH: "./frontend"
    PORT: 8080
    RUNTIME: "nodejs"
    BUILD: ["npm install", "npm run build"]
    ENV:
      API_URL: "http://localhost:8081"

  - NAME: "backend"
    PATH: "./backend"
    PORT: 8081
    RUNTIME: "go"
    BUILD: ["go build -o server ./cmd/server"]
    ENV:
      DATABASE_URL: "postgres://localhost/myapp"

INFRASTRUCTURE:
  compute: ec2             # or ecs, lambda
  region: us-east-1
  instance_type: t3.micro

PORTFOLIO:
  generate_page: true
  description_source: llm  # or readme, manual
```

### Production

```sh
./start prod
```

Builds the SvelteKit frontend, runs `npm start`, then `go run main.go`.

---

## Project layout

```
BytePort/
├── backend/
│   ├── byteport/        # Core API: Go 1.25, Gin, GORM, SQLite
│   │   ├── main.go      # Entry: OTel init, auth init, Gin server
│   │   ├── lib/         # Auth, crypto, git, apilink (SSRF-safe)
│   │   ├── models/      # GORM data models
│   │   └── routes/      # Gin HTTP handlers
│   └── nvms/            # NVMS runtime: Go 1.25, Spin wasm, port 3000
│       ├── main.go      # Spin HTTP entry + router
│       ├── projectManager/  # deploy/terminate logic
│       ├── lib/         # LLM providers
│       ├── Provisioner/ # MicroVM lifecycle
│       └── Builder/     # Image building
├── frontend/
│   └── web/             # SvelteKit 2 admin UI
│       ├── src/         # Routes + components
│       └── src-tauri/   # Tauri 2 desktop shell
├── docs/                # Long-form documentation (auto-generated + hand-written)
├── .github/workflows/   # CI: go-ci, npm-ci, tauri-ci, nvms-ci, codeql, etc.
├── start                # tmux dev orchestration
├── start.bat            # Windows parity (Phase 9)
├── justfile             # just task runner
├── golangci.yml         # golangci-lint config
├── deny.toml            # cargo-deny config
├── AGENTS.md            # Forge agent instructions
├── CLAUDE.md            # Claude-specific orientation
├── CHARTER.md           # Mission, tenets, scope
├── PLAN.md              # v1.0 roadmap (Phase 0–11)
├── SPEC.md              # Canonical technical spec
├── SPECS_INDEX.md       # Auto-generated audit index
├── STATUS.md            # Current health + known gaps
├── PRD.md               # Epics + stories
├── FUNCTIONAL_REQUIREMENTS.md   # 20 FRs, traced to PRD epics
├── ARCHITECTURE.md      # Component boundaries
├── stub-inventory.md    # Open TODOs and stubs
└── worklog.md           # Development log
```

---

## Governance

- **`STATUS.md`** — current health, known gaps, what v1.0 means
- **`CHARTER.md`** — mission, tenets, scope, success criteria, authority levels
- **`PLAN.md`** — v1.0 roadmap (10 phases + governance pass)
- **`SPEC.md`** — canonical technical spec (stack, data models, API, security)
- **`PRD.md`** — epics + stories
- **`FUNCTIONAL_REQUIREMENTS.md`** — 20 FRs, grouped by capability, traced to PRD
- **`ARCHITECTURE.md`** — component boundaries
- **`SPECS_INDEX.md`** — auto-generated audit index
- **`stub-inventory.md`** — open TODOs and stubs
- **`worklog.md`** — development log

---

## Roadmap (v1.0)

See `PLAN.md` for the full 173-task DAG. Headline phases:

- **Phase 0** (PR #1) — Governance reset ← *we are here*
- **Phase 1** (PR #2) — Security & reliability floor (4 critical bugs)
- **Phase 2** (PR #3) — Manifest engine (NVMS YAML)
- **Phase 3** (PR #4) — Backend hardening (slog, OTLP, rate limit, healthz, shutdown)
- **Phase 4** (PR #5) — NVMS service completion (auth on, LLM providers, /metrics)
- **Phase 5** (PR #6) — SvelteKit frontend (11 routes, zod, superforms, runes, i18n, a11y)
- **Phase 6** (PR #7) — Tauri 2 desktop shell (signing, notarize, CSP, deep-link, updater)
- **Phase 7** (PR #8) — CI/CD (15+ workflows, dependabot, release-drafter, FR coverage)
- **Phase 8** (PR #9) — Dev orchestration & onboarding (parameterized `./start`, Dockerfiles)
- **Phase 9** (PR #10) — Long-form documentation
- **Phase 10** — Verification matrix (23 gates, fan-in at v1.0.0)

---

## API at a glance

| Method | Path | Auth | Purpose |
|---|---|---|---|
| `POST` | `/signup` | public | Create account |
| `POST` | `/login` | public | Authenticate, set PASETO cookie |
| `GET` | `/authenticate` | cookie | Validate token, return user object |
| `GET` | `/link` | cookie | Initiate GitHub OAuth |
| `POST` | `/link` | cookie | Save AWS + LLM + Portfolio credentials |
| `GET` | `/instances` | cookie | List user's instances |
| `GET` | `/projects` | cookie | List user's projects |
| `GET` | `/api/github/callback` | OAuth | GitHub OAuth callback |
| `GET` | `/api/github/repositories` | cookie | List user's GitHub repos |
| `POST` | `/deploy` | cookie | Trigger deployment |
| `POST` | `/terminate` | cookie | Terminate an instance |
| `GET` | `/user/:id/creds` | cookie | Get decrypted credentials |
| `PUT` | `/user/:id/creds` | cookie | Update profile (name, email, password) |

Full request/response shapes in `SPEC.md` §4.

---

## Security model (summary)

- **At rest** — all credentials (AWS, GitHub, LLM, Portfolio) AES-256-CFB encrypted with auto-generated master key
- **Passwords** — Argon2id (memory=64MiB, iterations=3, parallelism=2, salt=16B, key=32B)
- **Session** — PASETO v2 tokens in httpOnly cookies
- **GitHub tokens** — auto-refresh every 7h45m via background goroutine
- **SSRF protection** — `lib/apilink.go` rejects loopback / private / link-local / multicast; allowlist via env
- **AWS validation** — STS session + `s3.ListBuckets` smoke test
- **OpenAI validation** — single `GET /v1/models` call
- **OTel traces** — every protected handler wrapped in otelgin middleware

Full security model in `SPEC.md` §5 and `CHARTER.md` §2 (Tenets 7, 8).

---

## Quality gates (CI)

| Gate | Command | Required |
|---|---|---|
| `go vet ./backend/...` | 0 warnings | yes |
| `go build ./backend/...` | 0 errors | yes |
| `go test ./backend/...` | all pass | yes |
| `golangci-lint run` | 0 errors | yes |
| `cargo test` (src-tauri) | all pass | yes |
| `cargo clippy -- -D warnings` (src-tauri) | 0 errors | yes |
| `npm run check` (frontend) | 0 errors | yes |
| `osv-scanner --recursive .` | clean | yes |
| `trufflehog filesystem .` | 0 secrets | yes |
| `codeql analyze` | 0 alerts | yes |

Full verification matrix in `PLAN.md` Phase 10.

---

## Example projects

- [Fixit-Go](https://github.com/kooshapari/fixit-go) — Go + SvelteKit todo list, ready for BytePort deploy
- [Chatta](https://github.com/kooshapari/chatta) — Real-time chat, multi-service manifest
- [Slickport](https://github.com/kooshapari/slickport) — Portfolio integration example

---

## Related work

- **Phenotype-org governance** — `phenotype-org-governance/` defines the org-wide
  rules this repo follows (FR IDs, ADRs, cargo-deny baseline, branch protection).
- **Authvault** (formerly `authkit`) — Rust auth/secrets crate used by sibling repos.
- **phenotype-auth-ts** — TypeScript auth SDK.

---

## Contributing

- Read `PLAN.md` Phase 0–1 to understand current state and what's queued next.
- Read `AGENTS.md` for worktree + integration rules.
- Open a draft PR early; the org quality gate (`fr-coverage.yml`) requires
  PR-to-FR traceability before merge.

---

## License

MIT. See `LICENSE`.
