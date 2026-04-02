# Forgecode — Technical Specification

## Architecture

```
┌─────────────────────────────────────────────┐
│          AI Agent (Claude Code)              │
│       /codex, /copilot, /composio           │
├─────────────────────────────────────────────┤
│            Forgecode CLI                     │
│    worktree create | list | prune | sync     │
├──────────┬──────────────────────────────────┤
│ gitoxide │  Worktrunk                      │
│ fast git │  worktree lifecycle             │
│ ops      │  management                     │
├──────────┴──────────────────────────────────┤
│              Git Configuration               │
│  FF-only merge | linear history | signed    │
├─────────────────────────────────────────────┤
│            Shell Aliases                     │
│     wt, lhist, sync-upstream                │
└─────────────────────────────────────────────┘
```

## Components

| Component | Location | Responsibility |
|-----------|----------|---------------|
| Setup | `scripts/setup-aliases.sh` | Shell alias installation |
| Sync | `scripts/sync-repos.sh` | Upstream synchronization |
| Worktree | `scripts/create-worktree.sh` | Worktree creation |
| Config | `config/gitconfig` | Git configuration |
| Config | `config/worktrunk.yaml` | Worktrunk settings |
| Config | `config/gix.toml` | gitoxide configuration |
| Skills | `skills/` | Shared agent skills |

## Git Policies

| Policy | Enforcement |
|--------|-------------|
| Linear history | `pull.rebase false`, `pull.ff only` |
| No fast-forward merge | `merge.ff false` |
| Signed commits | `commit.gpgsign true` |
| Worktree isolation | Feature branches in `worktrees/` |

## Data Models

```yaml
# worktrunk.yaml
worktrees:
  base_dir: "worktrees"
  naming: "<type>/<description>"
  auto_prune: true

upstream:
  remote: "upstream"
  branch: "main"
  sync_strategy: "ff-only"
```

## Workflow

```
main branch (canonical)
  └── worktrees/
       ├── feat/my-feature    (AI agent workspace)
       ├── fix/bug-fix        (AI agent workspace)
       └── chore/cleanup      (AI agent workspace)
```

| Step | Command | Action |
|------|---------|--------|
| Create | `wt new feat/my-feature` | Branch + worktree from main |
| Develop | AI agent works in worktree | Isolated changes |
| Sync | `wt sync` | Rebase on upstream/main |
| Merge | PR via gh CLI | Squash merge to main |
| Prune | `wt prune` | Remove merged worktrees |

## Performance Targets

| Metric | Target |
|--------|--------|
| Worktree create | <1s |
| Status across all worktrees | <2s |
| Upstream sync | <5s |
| Alias setup | <500ms |
| Worktree list (10+) | <1s |

## Extension Points

- Composio pattern: swappable AI backend (currently Claude Code only)
- Skills directory for shared agent capabilities
- Config files support custom git settings per project
