# Functional Requirements -- Forgecode

**Version:** 1.1.0
**Status:** Active
**Last Updated:** 2026-03-26

All requirements trace to epics in PRD.md using the format `PRD: E{n}.{m}`.

---

## FR-WT: Worktree Management

| ID | Requirement | PRD Trace | Source |
|----|-------------|-----------|--------|
| FR-WT-001 | System SHALL create a new Git worktree at `worktrees/{branch}` when `wt new <branch>` is invoked | E1.1 | `config/worktrunk.yaml` `worktree_root: worktrees` |
| FR-WT-002 | System SHALL accept an optional `[base]` argument to `wt new`; default base SHALL be `main` | E1.1 | `config/worktrunk.yaml` `default_base: main` |
| FR-WT-003 | System SHALL auto-create parent directories without error (`auto_mkdir: true`) | E1.1 | `config/worktrunk.yaml` |
| FR-WT-004 | System SHALL list all active worktrees with paths and HEAD commits via `wt list` | E1.1 | `config/worktrunk.yaml` |
| FR-WT-005 | System SHALL remove stale worktrees (deleted-branch worktrees) via `wt prune` | E1.1 | `config/worktrunk.yaml` `auto_prune: true` |
| FR-WT-006 | System SHALL auto-prune worktrees on worktrunk exit when `auto_prune: true` | E1.1 | `config/worktrunk.yaml` |
| FR-WT-007 | System SHALL auto-delete merged branches after worktree removal when `auto_cleanup: true` | E1.1 | `config/worktrunk.yaml` |
| FR-WT-008 | System SHALL switch working directory to a named worktree via `wt checkout <name>` | E1.1 | worktrunk |
| FR-WT-009 | System SHALL name worktrees using branch name only (not `{repo}-{branch}`) | E1.2 | `config/worktrunk.yaml` `naming: "{branch}"` |
| FR-WT-010 | System SHALL enforce branch naming patterns: `feat/`, `fix/`, `chore/`, `refactor/`, `docs/` | E1.2 | `config/worktrunk.yaml` `branch_patterns` |
| FR-WT-011 | System SHALL emit notifications on worktree create, delete, and prune events | E1.2 | `config/worktrunk.yaml` `notifications` |

---

## FR-GIT: Git Workflow Enforcement

| ID | Requirement | PRD Trace | Source |
|----|-------------|-----------|--------|
| FR-GIT-001 | System SHALL set `pull.rebase = false` globally via `setup-aliases.sh` | E2.1 | `scripts/setup-aliases.sh` line 25 |
| FR-GIT-002 | System SHALL set `pull.ff = only` globally via `setup-aliases.sh` | E2.1 | `scripts/setup-aliases.sh` line 26 |
| FR-GIT-003 | System SHALL set `merge.ff = false` globally via `setup-aliases.sh` | E2.1 | `scripts/setup-aliases.sh` line 27 |
| FR-GIT-004 | System SHALL install `git lhist` alias as `log --oneline --graph --all --decorate` | E2.1 | `scripts/setup-aliases.sh` line 11 |
| FR-GIT-005 | System SHALL install `git l` alias as `log --oneline -10` | E2.1 | `scripts/setup-aliases.sh` line 12 |
| FR-GIT-006 | System SHALL install `git s` alias as `status --short` | E2.1 | `scripts/setup-aliases.sh` line 13 |
| FR-GIT-007 | System SHALL install `git ff-check <ref1> <ref2>` alias using `merge-base --is-ancestor` | E2.1 | `scripts/setup-aliases.sh` line 14 |
| FR-GIT-008 | System SHALL set `commit.gpgsign = true` globally via `setup-aliases.sh` | E2.2 | `scripts/setup-aliases.sh` |
| FR-GIT-009 | System SHALL install `git sync-upstream` alias: fetch upstream, attempt `--ff-only` merge, fall back to `--no-ff` | E2.3 | `scripts/setup-aliases.sh` line 9 |
| FR-GIT-010 | System SHALL install `git sync-origin` alias: fetch origin, attempt `--ff-only` merge, fall back to `--no-ff` | E2.3 | `scripts/setup-aliases.sh` line 10 |
| FR-GIT-011 | System SHALL install `git pr-stack` alias listing up to 20 open PRs with number and title via `gh pr list` | E2.4 | `scripts/setup-aliases.sh` line 22 |
| FR-GIT-012 | System SHALL install `git wt-new`, `git wt-list`, `git wt-prune` as Git-native worktree shortcuts | E1.1 | `scripts/setup-aliases.sh` lines 17-19 |

---

## FR-CFG: Tool Configuration

| ID | Requirement | PRD Trace | Source |
|----|-------------|-----------|--------|
| FR-CFG-001 | System SHALL provide `config/worktrunk.yaml` with version, worktree root, auto flags, branch patterns, naming, default base, and notifications | E4.2 | `config/worktrunk.yaml` |
| FR-CFG-002 | System SHALL provide `config/gix.toml` with commit template, aliases, delta pager, and gh credential helper | E4.1 | `config/gix.toml` |
| FR-CFG-003 | `config/gix.toml` commit template SHALL use `{{title}}`, `{{body}}`, `{{footer}}` placeholder slots | E4.1 | `config/gix.toml` `[defaults]` |
| FR-CFG-004 | `config/gix.toml` pager settings SHALL reference `delta` for `diff`, `log`, and `reflog` | E4.1 | `config/gix.toml` `[pager]` |
| FR-CFG-005 | `config/gix.toml` credential helper SHALL be set to `gh auth git-credential` | E4.1 | `config/gix.toml` `[credential]` |
| FR-CFG-006 | System SHALL provide `scripts/setup-aliases.sh` as an idempotent one-command installer | E4.3 | `scripts/setup-aliases.sh` |
| FR-CFG-007 | `setup-aliases.sh` SHALL use `set -e` for fail-fast behavior | E4.3 | `scripts/setup-aliases.sh` line 3 |
| FR-CFG-008 | `setup-aliases.sh` SHALL print a confirmation message and full command reference on success | E4.3 | `scripts/setup-aliases.sh` lines 29-38 |

---

## FR-AGT: Agent Integration

| ID | Requirement | PRD Trace | Source |
|----|-------------|-----------|--------|
| FR-AGT-001 | System SHALL provide `CLAUDE.md` at project root with agent operating instructions | E3.1 | `CLAUDE.md` |
| FR-AGT-002 | `CLAUDE.md` SHALL specify minimum tool versions: gix >= 0.50.0, worktrunk >= 0.30.0, lazygit >= 0.40.0, git >= 2.40.0, gh >= 2.40.0 | E3.1 | `CLAUDE.md` Tool Versions table |
| FR-AGT-003 | `CLAUDE.md` SHALL mandate worktree-first development: features in worktrees, not main | E3.1, E3.2 | `CLAUDE.md` Core Principles |
| FR-AGT-004 | `CLAUDE.md` SHALL mandate linear history: `--ff-only` when possible, `--no-ff` for significant merges, never rebase pushed branches | E3.1 | `CLAUDE.md` Core Principles |
| FR-AGT-005 | `CLAUDE.md` SHALL mandate GPG-signed commits | E3.1 | `CLAUDE.md` Core Principles |
| FR-AGT-006 | `CLAUDE.md` SHALL mandate PR-based workflow with branch protection on main and squash merges | E3.1 | `CLAUDE.md` Core Principles |
| FR-AGT-007 | `CLAUDE.md` SHALL mandate absolute path references; relative path references are forbidden | E3.1 | `CLAUDE.md` File Paths section |
| FR-AGT-008 | `CLAUDE.md` SHALL mandate UTF-8 encoding and explicitly prohibit smart quotes, em-dashes, and Windows-1252 characters | E3.1 | `CLAUDE.md` Encoding section |
| FR-AGT-009 | System SHALL provide `AGENTS.md` for supplemental multi-agent coordination guidance | E3.2 | `AGENTS.md` |
| FR-AGT-010 | All agent work SHALL be tracked in AgilePlus before implementation begins | E3.2 | `CLAUDE.md` AgilePlus Mandate |

---

## FR-DOCS: Documentation Site

| ID | Requirement | PRD Trace | Source |
|----|-------------|-----------|--------|
| FR-DOCS-001 | System SHALL provide a VitePress documentation site under `docs/` | E5.1 | `docs/package.json`, `package.json` scripts |
| FR-DOCS-002 | `pnpm docs:dev` SHALL start a local VitePress development server | E5.1 | `package.json` `scripts.docs:dev` |
| FR-DOCS-003 | `pnpm docs:build` SHALL produce a static build of the documentation | E5.1 | `package.json` `scripts.docs:build` |
| FR-DOCS-004 | `pnpm docs:preview` SHALL serve the static documentation build locally | E5.1 | `package.json` `scripts.docs:preview` |
| FR-DOCS-005 | VitePress dependency SHALL be >= 1.6.4 | E5.1 | `package.json` devDependencies |
