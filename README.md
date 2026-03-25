# Forgecode

AI-enabled pair programmer using gitoxide and modern Git tooling.

## Overview

Forgecode is a workflow framework for AI-augmented development using:
- **gitoxide** - Fast Git implementation in Rust
- **worktrunk** - Worktree management for parallel AI agent workflows
- **lazygit** - Terminal UI for Git
- **gh CLI** - GitHub integration

## Quick Start

```bash
# Clone
git clone git@github.com:KooshaPari/forgecode.git
cd forgecode

# Install tools (macOS)
brew install gitoxide worktrunk lazygit

# Set up aliases
source scripts/setup-aliases.sh
```

## Features

- Worktree management for parallel development
- Linear history enforcement
- FF-only merge policy
- Signed commits
- Claude Code integration

## Directory Structure

```
forgecode/
├── README.md
├── CLAUDE.md           # Agent instructions
├── scripts/
│   ├── setup-aliases.sh
│   ├── sync-repos.sh
│   └── create-worktree.sh
├── config/
│   ├── gitconfig
│   ├── worktrunk.yaml
│   └── gix.toml
└── skills/             # Shared skills
```

## Git Configuration

### Core Settings
```bash
git config --global pull.rebase false
git config --global pull.ff only
git config --global merge.ff false
git config --global commit.gpgsign true
```

### Aliases
```bash
git config --global alias.sync-upstream '!git fetch upstream && git merge --ff-only upstream/main'
git config --global alias.wt-new '!f() { name="${1}"; base="${2:-main}"; git worktree add "worktrees/$name" -b "$name" "$base"; }; f'
git config --global alias.lhist 'log --oneline --graph --all --decorate'
```

## Worktree Workflow

```bash
# Create feature worktree
wt new feat/my-feature

# List worktrees
wt list

# Prune stale worktrees
wt prune

# Switch worktrees
wt checkout worktree-name
```

## License

MIT
