# Security Policy

## Reporting a Vulnerability

Do not open public issues for security vulnerabilities in `phenotype-infra`
(infrastructure configs, IaC, tunnel/DNS topology, or runbooks). Report them
privately to [kooshapari@gmail.com](mailto:kooshapari@gmail.com).

Please include:

- A description of the vulnerability (e.g. leaked credential pattern, exposed
  service, privilege escalation path, misconfigured firewall/ACL).
- Steps to reproduce.
- Potential blast radius (which node, which provider, which tenant data).
- Any suggested mitigations.

We will acknowledge reports within 48 hours.

## Scope

This repository holds **declarative infrastructure**: Terraform modules, Ansible
playbooks, Cloudflare/Tailscale topology, and runbooks. It does **not** hold
application secrets — those live in Vaultwarden and are injected at apply time.

See `docs/governance/security-policy.md` for the full token and SSH rotation
policy, and `docs/governance/incident-response.md` for outage playbooks.
