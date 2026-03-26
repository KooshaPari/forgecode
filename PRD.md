# Product Requirements Document -- Forgecode

**Version:** 1.1.0
**Status:** Active
**Last Updated:** 2026-03-26

---

## Overview

Forgecode is an AI-enabled pair programming workflow framework built on gitoxide (gix), worktrunk, and modern Git tooling. It provides configuration templates, shell scripts, and conventions enabling parallel AI-agent development workflows using Git worktrees as the unit of isolation.

Forgecode is not an application binary. It is a composable framework of:
- Git configuration templates (`config/gix.toml`)
- Worktree management configuration (`config/worktrunk.yaml`)
- Shell scripts for one-command environment setup (`scripts/setup-aliases.sh`)
- CLAUDE.md agent operating instructions
- VitePress documentation site (`docs/`)

---

## E1: Worktree Management

### E1.1: Worktree Lifecycle

**As** a developer or AI agent,
**I want** to create, list, prune, and switch between Git worktrees,
**So that** multiple features and agents can work in parallel without file conflicts.

**Acceptance Criteria:**

- `wt new <branch> [base]` creates a worktree under `worktrees/{branch}` from the specified base (default: `main`)
- `wt list` shows all active worktrees with paths and HEAD commits
- `wt prune` removes stale worktrees (those with deleted branches)
- `wt checkout <name>` switches working directory to an existing worktree
- `auto_prune: true` in `config/worktrunk.yaml` enables prune-on-exit by default
- `auto_cleanup: true` deletes merged branches automatically after worktree removal
- `auto_mkdir: true` creates parent directories without error
- Worktree root is `worktrees/` relative to project root

### E1.2: Branch Naming Conventions

**As** a team using forgecode,
**I want** consistent branch naming enforced by worktrunk configuration,
**So that** branch history is predictable and auditable across agents.

**Acceptance Criteria:**

- Feature branches: `feat/{name}`
- Fix branches: `fix/{name}`
- Chore branches: `chore/{name}`
- Refactor branches: `refactor/{name}`
- Docs branches: `docs/{name}`
- Default base branch: `main`
- Worktree directory naming uses branch name only (not `{repo}-{branch}`)
- Notifications fire on create, delete, and prune events

---

## E2: Git Workflow Enforcement

### E2.1: Linear History Policy

**As** a team maintaining a multi-agent codebase,
**I want** enforced linear history via Git configuration installed by `scripts/setup-aliases.sh`,
**So that** the commit graph remains readable and bisectable when many branches merge.

**Acceptance Criteria:**

- `pull.rebase = false` prevents accidental rebase on pull
- `pull.ff = only` restricts pulls to fast-forward only
- `merge.ff = false` requires explicit merge commits for feature branches
- `git lhist` alias: `log --oneline --graph --all --decorate`
- `git l` alias: `log --oneline -10` for last 10 commits
- `git s` alias: `status --short`
- `git ff-check <ref1> <ref2>` reports whether ref1 is an ancestor of ref2 using `merge-base --is-ancestor`

### E2.2: Signed Commits

**As** a team,
**I want** GPG-signed commits configured by default in `setup-aliases.sh`,
**So that** authorship is cryptographically verifiable.

**Acceptance Criteria:**

- `commit.gpgsign = true` is set globally by `scripts/setup-aliases.sh`
- Signing key is configured per-developer (documented in CLAUDE.md)

### E2.3: Remote Sync Aliases

**As** a developer working on a fork,
**I want** one-command upstream and origin sync,
**So that** branches stay current without manual fetch-merge sequences.

**Acceptance Criteria:**

- `git sync-upstream` fetches from `upstream` remote and attempts `--ff-only` merge; falls back to `--no-ff` on failure
- `git sync-origin` fetches from `origin` remote and attempts `--ff-only` merge; falls back to `--no-ff` on failure
- Both aliases installed globally by `scripts/setup-aliases.sh`
- Alias bodies are shell functions (`!f() {...}; f`) for multi-step logic

### E2.4: PR Stack Introspection

**As** a developer managing stacked PRs,
**I want** a quick view of all open PRs,
**So that** I can track the current stack and merge order.

**Acceptance Criteria:**

- `git pr-stack` alias lists up to 20 open PRs with number and title
- Uses `gh pr list --state open --json number,title` with jq formatting
- Requires only `gh` CLI (already a minimum-version dependency)

---

## E3: AI Agent Integration

### E3.1: Agent Operating Instructions

**As** an AI agent (Claude Code or compatible),
**I want** structured operating instructions in CLAUDE.md,
**So that** I can autonomously create worktrees, follow branch discipline, and operate within project conventions.

**Acceptance Criteria:**

- `CLAUDE.md` at project root specifies:
  - Worktree-first development mandate (features in worktrees, not main)
  - Linear history and merge strategy (FF-only pull, no-FF feature merge)
  - Signed commit requirement
  - PR-based workflow requirement (branch protection on main, squash merges)
  - Minimum tool versions: gix >= 0.50.0, worktrunk >= 0.30.0, lazygit >= 0.40.0, git >= 2.40.0, gh >= 2.40.0
  - Absolute path conventions (never relative path references)
  - UTF-8 encoding requirement (no smart quotes, no em-dashes, no Windows-1252 characters)
- `AGENTS.md` provides supplemental multi-agent coordination guidance

### E3.2: Multi-Agent Worktree Isolation

**As** a system running 10-50 parallel AI agents,
**I want** clear worktree isolation rules documented and enforced by convention,
**So that** agents do not conflict on the same files or branches.

**Acceptance Criteria:**

- Each agent operates in its own named worktree (`worktrees/{branch}`)
- Feature worktrees are created from `main` before any agent edits
- Canonical `main` worktree is treated as read-only during active agent workflows
- Merge/integration into `main` happens only after agent work is complete in the worktree
- AgilePlus work tracking is mandatory: each agent task has a corresponding AgilePlus spec

---

## E4: Tool Configuration

### E4.1: Gitoxide (gix) Configuration

**As** a developer using gitoxide,
**I want** a ready-to-use `config/gix.toml`,
**So that** gitoxide behavior matches team conventions immediately after clone.

**Acceptance Criteria:**

- `config/gix.toml` provides:
  - Commit message template with `{{title}}`, `{{body}}`, `{{footer}}` slots
  - Aliases: `co` (checkout), `br` (branch), `st` (status)
  - Delta pager for `diff`, `log`, and `reflog` outputs
  - `gh auth git-credential` as the credential helper
- File is documented for installation at `~/.config/gitoxide/config.toml`

### E4.2: Worktrunk Configuration

**As** a developer using worktrunk,
**I want** a project-level `config/worktrunk.yaml`,
**So that** worktree behavior is consistent across all developers and agents on this project.

**Acceptance Criteria:**

- `config/worktrunk.yaml` specifies (see actual file for canonical values):
  - `version: "1"`
  - `worktree_root: worktrees`
  - `auto_mkdir: true`, `auto_prune: true`, `auto_cleanup: true`
  - All five branch naming patterns
  - `default_base: main`
  - `naming: "{branch}"`
  - Notifications enabled for create, delete, and prune

### E4.3: One-Command Setup

**As** a new developer or agent bootstrapping an environment,
**I want** a single idempotent script that installs all required Git aliases and settings,
**So that** setup is repeatable and produces no drift between machines.

**Acceptance Criteria:**

- `scripts/setup-aliases.sh` uses `set -e` (fail-fast)
- Installs all aliases listed in E2.1-E2.4 acceptance criteria
- Prints confirmation message and full command list on success
- Is idempotent (safe to re-run; `git config --global` overwrites are safe)

---

## E5: Documentation Site

### E5.1: VitePress Docsite

**As** a developer onboarding to forgecode,
**I want** a browsable documentation site,
**So that** I can understand the tool stack, workflows, and conventions without reading raw config files.

**Acceptance Criteria:**

- `docs/` directory contains a VitePress site (`docs/package.json` with vitepress >= 1.6.4)
- `pnpm docs:dev` runs local development server
- `pnpm docs:build` produces static output
- `pnpm docs:preview` serves the built site
- `docs/index.md` serves as the home page
- `docs/guide/` and `docs/reference/` subdirectories provide structured content
