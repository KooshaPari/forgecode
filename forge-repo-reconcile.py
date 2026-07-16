#!/usr/bin/env python3
"""
forge-repo-reconcile.py

Reconcile remote repos against the policy allowlist.

For every non-archived repo created in the last 14 days whose name is
NOT in the allowlist table from forge-repo-create-policy.md §3, this
script archives it. It never deletes. Hard deletion requires a token
with delete_repo scope, which only the human can grant.

Usage:
    python forge-repo-reconcile.py [--dry-run] [--days N] [--org ORG]

Exit codes:
    0  success (or nothing to do)
    1  one or more gh repo archive calls failed
    2  bad invocation
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

# ---------------------------------------------------------------------------
# Allowlist — must mirror forge-repo-create-policy.md §3 verbatim.
# Keep this list short. Every entry is a privilege to defend.
# ---------------------------------------------------------------------------
ALLOWLIST: set[str] = {
    # Polyglot SDK targets
    "phenotype-python-sdk",
    "phenotype-rust-sdk",
    "phenotype-sdk",
    # Bot framework + adapters
    "phenotype-discord-adapter",
    "phenotype-bot-framework",
    # Cross-cutting libs
    "pheno-security-baselines",
    "pheno-errors",
    "phenotype-gateway",
    "phenotype-ops",
    "phenotype-config",
    "PhenoMCPServers",
    # Planning / governance
    "AgilePlus",
    # Game / sim
    "Civis",
    "DINOForge-UnityDoorstop",
    # Cross-cutting infra
    "AuthKit",
    "OmniRoute",
    "Tracera",
    "HeliosCLI",
}

# Repos that should NEVER be recreated (archived ancestors / forbidden)
FORBIDDEN: set[str] = {
    "Dino",                # archived 2026-06-18, content in DINOForge-UnityDoorstop
    "Dino-fork",           # archived 2026-06-19, was misnamed AgilePlus branch
}


def gh_repo_list(org: str) -> list[dict]:
    """Call `gh repo list` and return parsed JSON."""
    env = os.environ.copy()
    env["NO_COLOR"] = "1"
    env["GH_FORCE_TTY"] = "0"
    env["TERM"] = "dumb"
    out = subprocess.run(
        ["gh", "repo", "list", org, "--limit", "300",
         "--json", "name,createdAt,isArchived"],
        capture_output=True, text=True, env=env, check=False,
    )
    if out.returncode != 0:
        sys.stderr.write(f"gh repo list failed: {out.stderr}\n")
        sys.exit(1)
    return json.loads(out.stdout)


def gh_archive(org: str, name: str, dry_run: bool) -> tuple[bool, str]:
    """Archive one repo. Returns (ok, message)."""
    if dry_run:
        return True, "DRY-RUN"
    env = os.environ.copy()
    env["NO_COLOR"] = "1"
    env["GH_FORCE_TTY"] = "0"
    env["TERM"] = "dumb"
    out = subprocess.run(
        ["gh", "repo", "archive", f"{org}/{name}", "--yes"],
        capture_output=True, text=True, env=env, check=False,
        timeout=30,
    )
    if out.returncode == 0:
        return True, "ARCHIVED"
    return False, f"FAIL rc={out.returncode} stderr={out.stderr.strip()[:120]}"


def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--dry-run", action="store_true",
                   help="list candidates but do not archive")
    p.add_argument("--days", type=int, default=14,
                   help="age window in days (default 14)")
    p.add_argument("--org", default="KooshaPari",
                   help="GitHub org to scan (default KooshaPari)")
    p.add_argument("--audit-log",
                   default=str(Path.home() / "forge" / "forge-repo-create-audit.jsonl"),
                   help="path to append audit entries")
    return p.parse_args()


def append_audit(path: str, entry: dict) -> None:
    """Append one JSON line to the audit log."""
    p = Path(path)
    p.parent.mkdir(parents=True, exist_ok=True)
    with p.open("a", encoding="utf-8") as f:
        f.write(json.dumps(entry) + "\n")


def main() -> int:
    args = parse_args()
    repos = gh_repo_list(args.org)

    now = datetime.now(timezone.utc)
    archived_count = 0
    skipped_count = 0
    failed: list[tuple[str, str]] = []

    # Filter: active + created within window
    candidates: list[dict] = []
    for r in repos:
        if r.get("isArchived"):
            skipped_count += 1
            continue
        created = datetime.fromisoformat(r["createdAt"].replace("Z", "+00:00"))
        age_days = (now - created).days
        if age_days > args.days:
            skipped_count += 1
            continue
        candidates.append(r)

    # Decide: archive candidates NOT in allowlist, AND any in FORBIDDEN
    actions: list[tuple[str, str]] = []
    for r in candidates:
        name = r["name"]
        if name in ALLOWLIST:
            skipped_count += 1
            continue
        if name in FORBIDDEN:
            actions.append((name, "FORBIDDEN-RECURSION"))
        else:
            actions.append((name, "OUT-OF-ALLOWLIST"))

    # Print plan
    sys.stdout.write(f"CANDIDATES (created <= {args.days}d, not in allowlist): "
                     f"{len(actions)}\n")
    for name, reason in actions:
        sys.stdout.write(f"  {reason:22} {args.org}/{name}\n")

    # Execute
    for name, reason in actions:
        ok, msg = gh_archive(args.org, name, args.dry_run)
        line = f"{msg:10} {args.org}/{name}  ({reason})"
        sys.stdout.write(line + "\n")
        append_audit(args.audit_log, {
            "ts": now.isoformat(),
            "action": "archive",
            "org": args.org,
            "repo": name,
            "reason": reason,
            "dry_run": args.dry_run,
            "result": msg,
        })
        if ok:
            archived_count += 1
        else:
            failed.append((name, msg))

    sys.stdout.write(f"\nARCHIVED: {archived_count}  "
                     f"SKIPPED: {skipped_count}  "
                     f"FAILED: {len(failed)}\n")
    if failed:
        for name, msg in failed:
            sys.stderr.write(f"FAIL {name}: {msg}\n")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
