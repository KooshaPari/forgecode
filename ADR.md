# Architecture Decision Records — Forgecode

## ADR-001 | Gitoxide over Libgit2 | Adopted

**Status:** Adopted

**Context:** Need a fast, modern Git implementation for worktree management and repository operations.

**Decision:** Use gitoxide (gix) as the primary Git implementation, with standard git CLI as fallback.

**Consequences:**
- Faster operations than libgit2 for common workflows
- Pure Rust implementation aligns with Phenotype tooling direction
- Some advanced features may require git CLI fallback

---

## ADR-002 | Worktrunk for Worktree Orchestration | Adopted

**Status:** Adopted

**Context:** Managing multiple worktrees across repos requires tooling beyond `git worktree` subcommands.

**Decision:** Use worktrunk as the worktree management layer, providing higher-level operations (new, list, prune, checkout).

**Consequences:**
- Simplified worktree lifecycle for developers and agents
- Consistent naming and directory conventions
- Dependency on external tool (worktrunk)

---

## ADR-003 | Config-and-Scripts Architecture | Adopted

**Status:** Adopted

**Context:** Forgecode is a workflow framework, not an application. It needs to be lightweight and composable.

**Decision:** Structure as configuration files (gitconfig, worktrunk.yaml, gix.toml) plus shell scripts, with no compiled binary.

**Consequences:**
- Zero compilation step; works immediately after clone
- Easy to customize per-developer via dotfile overrides
- Limited to what shell scripts and config can express

---

## ADR-004 | FF-Only Merge Policy | Adopted

**Status:** Adopted

**Context:** Maintaining linear, readable Git history is critical for multi-agent workflows where many branches merge frequently.

**Decision:** Enforce `pull.ff = only` and `merge.ff = false` (explicit merge commits for features) via gitconfig template.

**Consequences:**
- History stays linear and readable
- Feature merges are explicit and traceable
- Developers must rebase before merging if behind
