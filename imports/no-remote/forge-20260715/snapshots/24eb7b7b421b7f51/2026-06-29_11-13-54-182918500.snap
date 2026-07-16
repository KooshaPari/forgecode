# phenotype-infra

[![Build](https://img.shields.io/github/actions/workflow/status/KooshaPari/phenotype-infra/quality-gate.yml?branch=main&label=build)](https://github.com/KooshaPari/phenotype-infra/actions)
[![Release](https://img.shields.io/github/v/release/KooshaPari/phenotype-infra?include_prereleases&sort=semver)](https://github.com/KooshaPari/phenotype-infra/releases)
[![License](https://img.shields.io/github/license/KooshaPari/phenotype-infra)](LICENSE)
[![Phenotype](https://img.shields.io/badge/Phenotype-org-blueviolet)](https://github.com/KooshaPari)
[![AI Slop Inside](https://sladge.net/badge.svg)](https://sladge.net)


Canonical home for Phenotype-org infrastructure-as-code, architectural decision records (ADRs), specifications, and operational runbooks. Supports the **7-node hybrid compute mesh** spanning Oracle Cloud, GCP, AWS, Cloudflare, and a Tailscale-attached home desktop.

## Overview

`phenotype-infra` is the single source of truth for:

- **Network topology** — Tailscale-based control plane across 7 nodes (OCI primary/secondary, GCP e2-micro, AWS Lambda webhooks, Cloudflare Workers/Tunnel, home Mac runner, and a Hetzner spillover reserved for Phase 2).
- **Runner routing** — Forgejo + Woodpecker CI with label-based dispatch (`[self-hosted, heavy, home]` vs `[self-hosted, oci]`).
- **Credential management** — Vaultwarden as canonical credential store, rotation policy documented in `docs/governance/security-policy.md`.
- **Rollback kill-switch** — Every node has a documented path back to GitHub Actions / disable procedure.

## Quick-start

```bash
# 1. Clone + read the topology
git clone git@github.com:KooshaPari/phenotype-infra.git
cd phenotype-infra
cat docs/specs/compute-mesh-spec.md

# 2. Read the top ADRs in order
ls docs/adr/

# 3. Bring up OCI primary (Day-1)
cat docs/runbooks/day1-oci-first-light.md

# 4. Register home-desktop runner (Day-1)
cat docs/runbooks/day1-home-runner-setup.md
```

## Operational status (2026-04-24)

- **Windows desktop heavy runner** — operational. Service `actions.runner.KooshaPari-phenotype-tooling.desktop-kooshapari-desk` registered and idle on the home Mac. Install procedure (with the gotchas that surfaced live: em-dash → ASCII, alphanumeric password, 48-char Description cap, unquoted `-OrgUrl`) is captured in `docs/runbooks/windows-desktop-runner.md`. Parsec coexistence verified: runner service stays in `Manual` start, only triggered on dispatch.
- Credential for the local `runneruser` account is stored in Vaultwarden under `windows-runner/desktop-kooshapari-desk/runneruser`.
- See `docs/runbooks/windows-desktop-runner.md` for verification, tear-down, and replacement steps.

## Top ADRs

| ADR | Title |
|-----|-------|
| [0001](docs/adr/0001-hybrid-compute-mesh.md) | Hybrid Compute Mesh (7 nodes) |
| [0002](docs/adr/0002-oci-primary-backbone.md) | OCI Ampere as primary backbone |
| [0003](docs/adr/0003-home-desktop-as-heavy-runner.md) | Home desktop as heavy runner |
| [0004](docs/adr/0004-tailscale-as-control-plane.md) | Tailscale as control plane |
| [0005](docs/adr/0005-forgejo-woodpecker-vs-gitea-vs-gh-actions.md) | Forgejo + Woodpecker vs alternatives |
| [0006](docs/adr/0006-vaultwarden-as-canonical-cred-store.md) | Vaultwarden canonical credential store |
| [0007](docs/adr/0007-runner-label-routing.md) | Runner label routing taxonomy |
| [0008](docs/adr/0008-parsec-gaming-mode-pause.md) | Parsec gaming mode pause |
| [0009](docs/adr/0009-hw-mesh-agent-bus.md) | HW mesh agent bus (Phase 2) |

See also: the parent compute-mesh playbook lives at `../docs/governance/compute_mesh.md` (sibling `repos/docs/governance/` directory).

## Contribution rules

- **No secrets.** Every credential is a placeholder (`<OCI_TENANCY_OCID>`, `<TAILSCALE_AUTHKEY>`, etc.). Real values live in Vaultwarden and are injected at runtime.
- **Terraform apply is human-only.** Agents may `terraform plan` and open PRs; `apply` requires explicit user approval.
- **ADR-first.** Any topology change needs an ADR before the IaC change.
- **Runbook-first.** Any node addition needs a runbook before the IaC scaffold.
- **Scripting hierarchy** (per `~/.claude/CLAUDE.md`): Rust default; Zig/Mojo/Go with one-line justification; Bash only as ≤5-line glue with justification comment. Terraform/Ansible/YAML are exempt as domain tools.

## Repository layout

```
docs/adr/             Architectural decisions (immutable once accepted)
docs/specs/           Topology, routing, credential inventory, rollback specs
docs/runbooks/        Step-by-step operational procedures
docs/governance/      Security, cost, incident-response policies
iac/                  Operational crates index — see iac/README.md
iac/oci-lottery/      A1.Flex capacity-lottery daemon (Rust)
iac/oci-post-acquire/ Post-acquire hook orchestrator
iac/tailscale/        Tailscale ACL + ephemeral keygen (Rust)
iac/landing-bootstrap/ Per-node landing-page generator (Rust)
iac/terraform/        Per-provider Terraform modules (stubs)
iac/ansible/          Configuration management playbooks
iac/scripts/          Bootstrap helpers (bash ≤5-line or Rust)
configs/              Per-service .example config files
.github/workflows/    CI (terraform plan, ansible-lint, docs check)
```

For the operational-crates entry index (oci-lottery, oci-post-acquire,
tailscale-keygen, landing-bootstrap), see [`iac/README.md`](iac/README.md).

## License

Dual-licensed under MIT **OR** Apache-2.0 at your option. See `LICENSE-MIT` and `LICENSE-APACHE`.
