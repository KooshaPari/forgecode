# Forgecode Agent Instructions

## Overview

Forgecode is an AI-enabled pair programmer using gitoxide, worktrunk, and modern Git tooling.

## Core Principles

1. **Worktree-First Development**
   - Features go in worktrees, not main
   - Use `wt new <branch>` to create feature worktrees
   - Keep main worktree clean

2. **Linear History**
   - Use `--ff-only` when possible
   - Use `--no-ff` for significant merges
   - Never rebase pushed branches

3. **Signed Commits**
   - GPG signed commits required
   - Configure signing key in gitconfig

4. **PR-Based Workflow**
   - All changes via pull requests
   - Branch protection on main
   - Squash merges for feature branches

## Commands

### Worktree Management
```bash
wt new <branch> [base]   # Create new worktree
wt list                   # List all worktrees
wt prune                  # Remove stale worktrees
wt checkout <name>         # Switch to worktree
```

### Git Operations
```bash
git sync-upstream          # Fetch + FF-merge from upstream
git sync-origin            # Fetch + FF-merge from origin
git lhist                  # Linear history graph
git pr-stack               # List stacked PRs
```

### Quick Status
```bash
git s                      # Short status
git l                      # Last 10 commits
```

## File Paths

Always use absolute paths or paths relative to project root:
- ✅ `forgecode/scripts/setup-aliases.sh`
- ❌ `the scripts folder`

## Encoding

Use UTF-8 only. Avoid:
- Smart quotes (" ")
- Em dashes (---)
- Windows-1252 characters

## Tool Versions

| Tool | Minimum Version |
|------|----------------|
| gix | 0.50.0 |
| worktrunk (wt) | 0.30.0 |
| lazygit | 0.40.0 |
| git | 2.40.0 |
| gh | 2.40.0 |
