# Functional Requirements — Forgecode

## FR-WT: Worktree Management

| ID | Requirement |
|----|-------------|
| FR-WT-001 | System SHALL provide `wt new` command to create worktrees from a base branch |
| FR-WT-002 | System SHALL provide `wt list` to show all active worktrees |
| FR-WT-003 | System SHALL provide `wt prune` to remove stale worktrees |
| FR-WT-004 | System SHALL provide `wt checkout` to switch between worktrees |

## FR-GIT: Git Workflow Enforcement

| ID | Requirement |
|----|-------------|
| FR-GIT-001 | System SHALL configure FF-only pull policy via gitconfig |
| FR-GIT-002 | System SHALL configure GPG commit signing via gitconfig |
| FR-GIT-003 | System SHALL provide `sync-upstream` alias for upstream FF-merge |
| FR-GIT-004 | System SHALL provide `lhist` alias for linear history graph |

## FR-CFG: Tool Configuration

| ID | Requirement |
|----|-------------|
| FR-CFG-001 | System SHALL provide gitconfig template with merge policy and aliases |
| FR-CFG-002 | System SHALL provide worktrunk.yaml for worktree management config |
| FR-CFG-003 | System SHALL provide gix.toml for gitoxide settings |
| FR-CFG-004 | System SHALL provide setup-aliases.sh for one-command installation |

## FR-AGT: Agent Integration

| ID | Requirement |
|----|-------------|
| FR-AGT-001 | System SHALL provide CLAUDE.md with agent operating instructions |
| FR-AGT-002 | System SHALL provide skills directory for shared agent capabilities |
| FR-AGT-003 | System SHALL document worktree discipline for multi-agent workflows |

## FR-SCR: Scripts

| ID | Requirement |
|----|-------------|
| FR-SCR-001 | System SHALL provide sync-repos.sh for multi-repo synchronization |
| FR-SCR-002 | System SHALL provide create-worktree.sh for scripted worktree creation |
