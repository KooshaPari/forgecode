# Repository Creation Policy (forge-repo-create-policy)

**Status:** ACTIVE — effective 2026-06-19
**Owner:** Koosha Pari (sole human approver)
**Enforced by:** every Forge session, every sub-agent, every CI workflow

---

## 1. Purpose

On 2026-06-15 through 2026-06-19, an uncontrolled proliferation of
GitHub repositories occurred under the `KooshaPari` organization. In
five days the remote repository count grew from **160 → 209** (+49),
driven almost entirely by sub-agents creating `pheno*` / `phenotype*`
scaffold repos. Many of these were created and archived within the same
calendar day, leaving the archive list polluted with placeholder
shells.

This policy exists to **stop the bleeding**. It applies to every agent,
skill, and automation run from this point forward.

---

## 2. Hard rules

### 2.1 Default action is NO

A sub-agent **must not** create a new GitHub repository unless ALL of
the following are true:

1. The user has explicitly named the new repository in the current
   prompt, **OR** the new repository name appears verbatim in an
   approved `RATIONALIZATION_PLAN.md` entry that the user has signed
   off on.
2. An existing repository cannot host the work. "Existing" means:
   - A repo in the **[Allowlist — Target Monorepos]** table below
     (Section 3), or
   - A repo the user has explicitly named in the current prompt.
3. The agent logs the action to `forge-repo-create-audit.jsonl` with:
   `{ ts, agent_id, session_id, repo_name, justification, target_alternative_rejected, user_approval_evidence }`
   and prints the entry to stdout so the human sees it before the
   `gh repo create` invocation.

If any of those conditions is false, the agent **stops** and asks.

### 2.2 Forbidden substitutions

These phrasings in an agent's reasoning **do not** justify a new repo:

- "It's cleaner as a separate repo"
- "Fork X into Y"
- "Create a new sibling for X"
- "Absorb X by forking"
- "Move Y into a fresh repo"
- "It's only a placeholder, we'll merge it later"

All of these must be redirected into the **[Allowlist]** table —
typically by adding a `crates/<name>` or `services/<name>` path inside
an existing target monorepo.

### 2.3 Placeholder repos are forbidden

A repo may not be created in an empty, near-empty, or "scaffold-only"
state. If a repo is going to be empty for more than 24 hours, it must
not exist.

### 2.4 Bulk archive on session end

At the end of any session that touched the repo surface, the agent
**must** run:

```
python forge-repo-reconcile.py
```

This script compares the remote list against the allowlist and
auto-archives anything not on the allowlist that was created in the
last 14 days. The script **only archives** — it never deletes.

---

## 3. Allowlist — Target Monorepos

These repositories are the canonical homes for the listed scopes. New
work in any of these scopes **must land inside the listed repo** as a
path, branch, crate, or service.

| Target repo | Scope | Notes |
|---|---|---|
| `KooshaPari/phenotype-python-sdk` | Python SDK, parsers, llms-txt, prompt-test, lexer | All Python-side phenotype code |
| `KooshaPari/phenotype-rust-sdk` | Rust SDK, error types, tracing, capacity | All Rust-side phenotype code |
| `KooshaPari/phenotype-sdk` | Polyglot SDK facade (re-exports) | Thin re-export layer only |
| `KooshaPari/phenotype-discord-adapter` | Discord-specific bot glue | Must not absorb other channels |
| `KooshaPari/phenotype-bot-framework` | Cross-channel bot framework | Parent of adapters |
| `KooshaPari/pheno-security-baselines` | Security baselines, threat models | Read-only consumers should symlink |
| `KooshaPari/pheno-errors` | Error taxonomy, codes, mapping | Pure library, no I/O |
| `KooshaPari/phenotype-gateway` | HTTP gateway, routing | Includes `crates/gateway/` |
| `KooshaPari/phenotype-ops` | Ops tooling, dashboards | |
| `KooshaPari/phenotype-config` | Shared config schema | |
| `KooshaPari/PhenoMCPServers` | MCP server registry | Subdirs per server |
| `KooshaPari/AgilePlus` | Planning, DAGs, worklogs, schemas | Branch naming: `wsm/<topic>-<yyyymmdd>` |
| `KooshaPari/Civis` | Civis simulation, manifests, traits | |
| `KooshaPari/DINOForge-UnityDoorstop` | Dino / Unity engine port | `Dino` repo itself is archived; do not recreate |
| `KooshaPari/AuthKit` | Auth library | |
| `KooshaPari/OmniRoute` | Routing infra | |
| `KooshaPari/Tracera` | Tracing + governance | |
| `KooshaPari/HeliosCLI` | CLI surface | |

Anything outside this table is a **violation candidate**.

---

## 4. Incident log

| Date | Event | Action taken |
|---|---|---|
| 2026-06-09 | `phenoShared-niche` scaffolded+archived same day | Archived |
| 2026-06-10 | `services` scaffolded+archived same day | Archived |
| 2026-06-15–19 | **54 new repos created**, 47 are `pheno*`/`phenotype*` siblings | 39 archived 2026-06-19, 7 archive-fallback (PAT lacks `delete_repo`) |
| 2026-06-18 | `KooshaPari/Dino` archived | Done by agent using user's PAT |
| 2026-06-19 | `KooshaPari/Dino-fork` created with branch `wsm/agileplus-dag-20260610` (AgilePlus branch, not Dino content) | Archived 2026-06-19 |

---

## 5. Reconciliation script contract

`forge-repo-reconcile.py` (placed at `C:\Users\koosh\forge\forge-repo-reconcile.py`)
must:

1. `gh repo list KooshaPari --limit 300 --json name,createdAt,isArchived > /tmp/repos.json`
2. Parse the JSON. For each repo:
   - If `isArchived == true` → skip
   - If `createdAt` is older than 14 days → skip (out of policy scope)
   - If `name` is in the Section 3 allowlist → skip
   - Otherwise → `gh repo archive KooshaPari/<name> --yes`
3. Print a one-line summary: `ARCHIVED: <count> SKIPPED: <count>`.
4. Exit 0 if the count delta is positive or zero. Exit 1 if any
   `gh repo archive` returned non-zero.

The script must never invoke `gh repo delete`. Hard deletion requires
a token with the `delete_repo` scope, which only the human can grant.

---

## 6. Scope and exceptions

This policy applies to the `KooshaPari` GitHub organization. It does
not cover:

- Forks of upstream open-source repos (those create via the GitHub
  UI's "Fork" button, not via `gh repo create`)
- GitHub Packages / container registries
- Wikis, discussions, projects (those live inside existing repos)

If an agent believes an exception is warranted, it must surface the
exception in the chat **before** invoking `gh repo create` and wait
for human approval.

---

## 7. Violation handling

If an agent is observed creating a repo in violation of this policy:

1. The agent must immediately archive the violating repo.
2. The agent must append an entry to `forge-repo-create-audit.jsonl`
   with `action: "rollback"` and the repo name.
3. The agent must report the violation to the user in chat with the
   line: `POLICY VIOLATION: archived <repo_name> — see
   forge-repo-create-policy.md §2.1`.

The human reserves the right to revoke agent privileges for repeated
violations.

---

*Approved by human operator: 2026-06-19. Effective immediately.*
