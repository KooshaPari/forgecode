# Product Requirements Document — Forgecode

## E1: Worktree Management

### E1.1: Worktree Lifecycle
As a developer, I want to create, list, prune, and switch between Git worktrees so that I can run parallel agent workflows.

**Acceptance Criteria:**
- `wt new <branch> [base]` creates worktree from specified base
- `wt list` shows all active worktrees
- `wt prune` removes stale worktrees
- `wt checkout <name>` switches to existing worktree

---

## E2: Git Workflow Enforcement

### E2.1: Linear History
As a team, we want enforced linear history so that the commit graph remains readable.

**Acceptance Criteria:**
- FF-only merge policy configured via gitconfig
- No-ff for significant feature merges
- Aliases enforce correct merge strategy

### E2.2: Signed Commits
As a team, we want GPG-signed commits so that authorship is verifiable.

**Acceptance Criteria:**
- `commit.gpgsign = true` in gitconfig
- Signing key configured per-developer

### E2.3: Upstream Sync
As a developer, I want to sync with upstream remotes so that forks stay current.

**Acceptance Criteria:**
- `git sync-upstream` fetches and FF-merges from upstream/main
- Alias defined in gitconfig template

---

## E3: AI Agent Integration

### E3.1: Claude Code Integration
As an AI agent, I want standardized project instructions so that I can operate autonomously in worktrees.

**Acceptance Criteria:**
- CLAUDE.md provides agent instructions
- Skills directory contains shared agent skills
- Worktree discipline documented for agent workflows

---

## E4: Tool Configuration

### E4.1: Tool Config Templates
As a developer, I want pre-configured tool settings so that the workflow stack is ready out of the box.

**Acceptance Criteria:**
- gitconfig template with aliases and merge policy
- worktrunk.yaml for worktree management
- gix.toml for gitoxide configuration
- Setup script installs all aliases
