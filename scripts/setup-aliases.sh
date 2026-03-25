#!/bin/bash
# Setup aliases for forgecode workflow

set -e

echo "Setting up forgecode aliases..."

# Core Git aliases
git config --global alias.sync-upstream '!f() { git fetch upstream && git merge --ff-only upstream/main || git merge --no-ff upstream/main; }; f'
git config --global alias.sync-origin '!f() { git fetch origin && git merge --ff-only origin/main || git merge --no-ff origin/main; }; f'
git config --global alias.lhist 'log --oneline --graph --all --decorate'
git config --global alias.l 'log --oneline -10'
git config --global alias.s 'status --short'
git config --global alias.ff-check '!f() { git merge-base --is-ancestor "$1" "$2" && echo "YES" || echo "NO"; }; f'

# Worktree aliases
git config --global alias.wt-new '!f() { name="${1}"; base="${2:-main}"; dir="worktrees/$(basename $(pwd))/$name"; git worktree add "$dir" -b "$name" "$base"; }; f'
git config --global alias.wt-list 'worktree list'
git config --global alias.wt-prune 'worktree prune'

# PR aliases
git config --global alias.pr-stack '!f() { gh pr list --state open --json number,title --jq ".[] | \"#\(.number) \(.title)\"" | head -20; }; f'

# Git settings
git config --global pull.rebase false
git config --global pull.ff only
git config --global merge.ff false

echo "Aliases installed!"
echo ""
echo "Available commands:"
echo "  git sync-upstream  - Fetch + merge from upstream"
echo "  git sync-origin    - Fetch + merge from origin"
echo "  git lhist          - Linear history graph"
echo "  git l              - Last 10 commits"
echo "  git s              - Short status"
echo "  git wt-new <name>  - Create feature worktree"
echo "  git wt-list        - List worktrees"
echo "  git pr-stack        - List open PRs"
