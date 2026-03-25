# Code Entity Map — Forgecode

## Forward Map (Code -> Requirements)

| Entity | Type | FRs |
|--------|------|-----|
| `config/gitconfig` | Git configuration | FR-GIT-001..004, FR-CFG-001 |
| `config/worktrunk.yaml` | Worktree manager config | FR-WT-001..004, FR-CFG-002 |
| `config/gix.toml` | Gitoxide config | FR-CFG-003 |
| `scripts/setup-aliases.sh` | Setup script | FR-CFG-004 |
| `scripts/sync-repos.sh` | Repo sync | FR-SCR-001 |
| `scripts/create-worktree.sh` | Worktree creation | FR-SCR-002, FR-WT-001 |
| `CLAUDE.md` | Agent instructions | FR-AGT-001, FR-AGT-003 |
| `skills/` | Agent skills | FR-AGT-002 |

## Reverse Map (Requirements -> Code)

| FR ID | Code Entities |
|-------|---------------|
| FR-WT-001..004 | `config/worktrunk.yaml`, `scripts/create-worktree.sh` |
| FR-GIT-001..004 | `config/gitconfig` |
| FR-CFG-001..004 | `config/gitconfig`, `config/worktrunk.yaml`, `config/gix.toml`, `scripts/setup-aliases.sh` |
| FR-AGT-001..003 | `CLAUDE.md`, `skills/` |
| FR-SCR-001..002 | `scripts/sync-repos.sh`, `scripts/create-worktree.sh` |
