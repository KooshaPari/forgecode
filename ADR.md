# Architecture Decision Records -- Forgecode

**Version:** 1.1.0
**Status:** Active
**Last Updated:** 2026-03-26

---

## ADR-001 | Gitoxide (gix) as Primary Git Implementation | Adopted

**Status:** Adopted
**Date:** 2026-03-26
**Traces to:** FR-CFG-002, FR-CFG-003, FR-CFG-004, FR-CFG-005

**Context:**

Forgecode requires a Git implementation for worktree management and repository operations. Options: system `git` CLI, libgit2 (via bindings), or gitoxide (gix). Libgit2 is the historical default for embedded Git but is written in C and lacks first-class Rust integration. System `git` CLI is universally available but is slow for scripted operations and has no typed API. Gitoxide is a pure Rust reimplementation with significantly faster cold-start and object-reading performance.

**Decision:**

Use gitoxide (gix >= 0.50.0) as the primary Git implementation, with standard `git` CLI as fallback for operations not yet covered by gix (e.g., some remote operations). The `config/gix.toml` file standardizes gix behavior: commit template with `{{title}}`/`{{body}}`/`{{footer}}` slots, aliases (`co`, `br`, `st`), delta pager for diff/log/reflog, and `gh auth git-credential` as the credential helper.

**Alternatives Considered:**

- libgit2: rejected -- C dependency, slower, no first-class Rust integration
- System `git` CLI only: rejected -- no typed API, slower for scripted multi-repo workflows

**Consequences:**

- Faster object-read and worktree operations vs libgit2 baseline
- Pure Rust implementation aligns with Phenotype tooling direction
- Some advanced remote operations require `git` CLI fallback
- Minimum version constraint: gix >= 0.50.0

---

## ADR-002 | Worktrunk for Worktree Orchestration | Adopted

**Status:** Adopted
**Date:** 2026-03-26
**Traces to:** FR-WT-001 through FR-WT-011, FR-CFG-001

**Context:**

`git worktree add/list/prune` are low-level primitives. For a multi-agent workflow where 10-50 agents may create and destroy worktrees concurrently, a higher-level orchestration layer is needed to enforce naming conventions, handle auto-cleanup, auto-prune, and emit lifecycle notifications.

**Decision:**

Use worktrunk (>= 0.30.0) as the worktree management layer. `config/worktrunk.yaml` configures it with: `worktree_root: worktrees`, `auto_mkdir`/`auto_prune`/`auto_cleanup` all true, five branch naming patterns (`feat/`, `fix/`, `chore/`, `refactor/`, `docs/`), `naming: "{branch}"` (branch name only), and lifecycle notifications for create/delete/prune.

**Alternatives Considered:**

- Raw `git worktree` subcommands: rejected -- no naming enforcement, no auto-cleanup, no notification hooks
- Custom shell wrapper: rejected -- maintenance burden; worktrunk covers all required cases

**Consequences:**

- Simplified worktree lifecycle for developers and agents
- Consistent naming and directory layout across all users
- External binary dependency (worktrunk >= 0.30.0)
- Branch patterns enforced at config level; deviations require config change

---

## ADR-003 | Config-and-Scripts Architecture (No Compiled Binary) | Adopted

**Status:** Adopted
**Date:** 2026-03-26
**Traces to:** FR-CFG-006, FR-CFG-007, FR-CFG-008

**Context:**

Forgecode is a workflow framework, not an application. It needs to be lightweight, composable, and immediately usable after `git clone` without a build step.

**Decision:**

Structure forgecode as configuration files (`config/gix.toml`, `config/worktrunk.yaml`), shell scripts (`scripts/setup-aliases.sh`), and documentation (`docs/`, `CLAUDE.md`, `AGENTS.md`). No compiled binary, no build step, no language runtime requirement beyond `bash`. `scripts/setup-aliases.sh` is the sole installer: it sets global Git config and aliases via `git config --global`, uses `set -e` for fail-fast, and is idempotent.

**Alternatives Considered:**

- CLI binary (Go/Rust): rejected -- adds build toolchain requirement, reduces portability
- Makefile-based install: rejected -- less transparent than explicit shell; harder for agents to reason about

**Consequences:**

- Zero compilation step; works immediately after clone
- Easy per-developer customization via dotfile overlays
- Limited to shell script expressiveness (no typed logic, no error recovery beyond `set -e`)
- All behavior is inspectable as plain text (shell, YAML, TOML)

---

## ADR-004 | FF-Only Pull + Explicit Feature Merge Commits | Adopted

**Status:** Adopted
**Date:** 2026-03-26
**Traces to:** FR-GIT-001, FR-GIT-002, FR-GIT-003, FR-GIT-004

**Context:**

In multi-agent workflows, many branches are created and merged in parallel. Without enforced merge policy, history becomes a DAG that is difficult to read, bisect, or audit. Two conflicting goals: (1) pulls should never create merge commits on main, (2) feature merges should be explicit and traceable.

**Decision:**

Enforce via `scripts/setup-aliases.sh`: `pull.rebase = false` (no rebase on pull), `pull.ff = only` (pulls fail unless fast-forward -- prevents silent merge commits on main), `merge.ff = false` (feature merges always create a merge commit, never silently fast-forward). The `sync-upstream` and `sync-origin` aliases attempt `--ff-only` and fall back to `--no-ff` explicitly.

**Alternatives Considered:**

- `pull.rebase = true`: rejected -- rebasing pushed branches rewrites history visible to other agents
- Always FF: rejected -- loses feature-merge traceability
- No enforcement: rejected -- agents and developers will drift

**Consequences:**

- History stays linear on main (fast-forward only pulls)
- Feature merges are explicit and traceable
- Developers and agents must fast-forward or rebase before merging if behind main
- `git ff-check <ref1> <ref2>` alias available to verify ancestry before merge

---

## ADR-005 | Delta as Default Pager | Adopted

**Status:** Adopted
**Date:** 2026-03-26
**Traces to:** FR-CFG-004

**Context:**

Default Git diff and log output is uncolored and hard to read, especially for AI agents parsing diff output. `delta` is a syntax-highlighting pager for Git that significantly improves diff readability and is already widely used in the Phenotype tooling ecosystem.

**Decision:**

Set `delta` as the pager for `diff`, `log`, and `reflog` in `config/gix.toml` under `[pager]`. Delta is a recommended tool; the config degrades gracefully if delta is not installed (gix falls back to raw output).

**Alternatives Considered:**

- `bat`: rejected -- good for file viewing but less optimized for Git diff output
- Default pager (`less`): rejected -- no syntax highlighting, harder to read

**Consequences:**

- Requires `delta` to be installed (recommended; not hard-required)
- Significantly improved diff readability in terminal and agent log capture

---

## ADR-006 | VitePress for Documentation Site | Adopted

**Status:** Adopted
**Date:** 2026-03-26
**Traces to:** FR-DOCS-001 through FR-DOCS-005

**Context:**

Forgecode needs human-readable documentation beyond root-level markdown files. The Phenotype ecosystem standard for docsite tooling is VitePress (Vue-based static site generator), used in phenodocs, heliosApp, and thegent.

**Decision:**

Use VitePress >= 1.6.4 in `devDependencies` (`package.json`). The `docs/` directory contains the VitePress site. Three scripts exposed: `docs:dev`, `docs:build`, `docs:preview`. Package manager: bun (detected from `bun.lock`).

**Consequences:**

- Consistent with Phenotype ecosystem tooling
- Static output can be served from any CDN or opened as `file://`
- Documentation changes go through the same PR workflow as code changes
