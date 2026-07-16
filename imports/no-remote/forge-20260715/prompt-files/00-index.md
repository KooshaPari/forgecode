# Forge Prompt-File Index

**Generated:** 2026-07-07
**Author:** Forge session (compute/infra absorption workstream)
**Repo state at generation:** phenotype-registry 26/26 audits @ 14/14 L4 = 364.0/364 pts (100.00%) on branch `B3-grade-BytePort-2026-07-02`. Civis PBR substrate (FR-001..009) at 1,451 lines on main. Phenotype-tooling orchestrator + deletion-gate tooling on `origin/main`.

## Unified goals (compiled from 5/1..7/8/2026 sessions)

**North star:** "compute/infra absorption sweep" — every KooshaPari repo in the registry has an authoritative absorption-justification audit at grade L4 (14/14 across 7 pillars), with a project card in `registry/projects/<name>.json` and a disposition row in `registry/disposition-index.json`. Once fleet is full, the registry becomes a single-pane-of-glass governance surface and downstream PR-review / CI / branch-protection workstreams can scaffold from the audit data.

**Current state (verified 2026-07-05):**

| Metric | Value |
|---|---|
| Audits at L4 | 26 / 26 (100.00%) |
| Total points | 364.0 / 364 |
| Pillars passing | 7 / 7 on every audit (registry_entry, readme_fr_refs, ci_cd_configs, buildable, has_dependencies, audit_justification_card, repo_delete_gate) |
| Project cards | 26 in `registry/projects/` |
| Disposition rows | 80+ in `registry/disposition-index.json` |
| Open PRs | 0 (all merged to main) |
| Branches ahead of main | `B3-grade-BytePort-2026-07-02` carries 5+ commits (5 new audits since 2026-07-02) |

**Next-batch targets (5-9 more audits to reach the next milestone):** thegent (already audited 2026-07-02), eyetracker (audited 2026-07-02), phenotype-sdk (audited 2026-07-02), pheno-specs (active FSM), Benchora, Civis, Quilla, KodeVibe, Authvault, autovenv, pheno-spec-orchestrator. Each is a fresh write through the hardened `bin/absorption-justification.py` orchestrator.

**Active sub-tasks (in priority order):**

1. **Open PR** from `B3-grade-BytePort-2026-07-02` → main (5 new audits + 3 doc commits ready)
2. **Survey next-5 untracked** from `audit_candidates_with_size.json` and write audits
3. **Registry H9 follow-up** (8+ ADR/plan drafts)
4. **Civis PBR Phase 3** (triplanar WGSL + greedy atlas) — large new work
5. **pheno-context / pheno-runtime-config** as separate workstream (already in registry)
6. **B3-grade-BytePort-2026-07-02 hygiene** — clean up scratch files (`_survey_candidates.py`, `_verify_candidates.py`, `_show_candidates.py`, `_coverage_check.py`) before opening PR
7. **A18 license-adoption** — `Cargo.toml` / `LICENSE-APACHE` / `package.json` are modified in working tree but uncommitted; this is a separate workstream from absorption

**Blocked / gated (require user signal):**

- **F15**: delete `phenotype-go-sdk/packages/platformkit/` — user needs to inspect first
- **F18**: archive `phenotype-go-sdk` — gated on F15
- **PR #380** if exists: needs manual review (not yet opened per current state)

**Skill inventory confirmed:**
- `superpowers/using-superpowers` is active
- `phenotype-tooling/bin/absorption-justification.py` hardened (P2/P3/P4/P6 auto-derive + `write_p6_card` wired into main)
- `phenotype-tooling/bin/repo-delete-gate.sh|.ps1` (5-section manifest gate)
- `phenotype-tooling/bin/ABSORPTION_TEMPLATE.md` (5-mandatory-heading template)

**Tools available to all agents:** `sem_search`, `fs_search`, `read`, `write`, `undo`, `remove`, `patch`, `multi_patch`, `shell`, `fetch`, `skill`, `todo_write`, `todo_read`, `task`, `mcp_*`. Agent ID format: `forge`. Tasks format: array of strings, each agent message is one task description.

**Key paths to remember:**

- `C:\Users\koosh\phenotype-registry\` — registry (audit target)
- `C:\Users\koosh\phenotype-registry\registry\audit-absorption-justification\` — grader + schema + scripts
- `C:\Users\koosh\repos\phenotype-tooling\bin\` — orchestrator + deletion-gate tooling
- `C:\Users\koosh\Civis\` — Civis PBR substrate (1,451 lines of voxel material code)
- `C:\Users\koosh\plans\2026-07-*.md` — 8+ recent plan docs
- `C:\Users\koosh\forge\` — forge workspace (this index lives here)

## Prompt-file roster (this directory)

| File | Agent | Status |
|---|---|---|
| `00-index.md` | this file | written |
| `01-registry-curator.md` | `registry-curator` | written |
| `02-audit-writer-batch-2.md` | `audit-writer-batch-2` | written |
| `03-fleet-committer.md` | `fleet-committer` | written |
| `04-a18-license-adoption.md` | `a18-license-adoption` | written |
| `05-civis-pbr-phase3.md` | `civis-pbr-phase3` | written |
| `06-pheno-context-orchestrator.md` | `pheno-context` | written |
| `07-registry-h9-docs.md` | `registry-h9-docs` | written |
| `08-batch-3-survey-writer.md` | `batch-3-survey` | written |

**Cross-cutting conventions for ALL prompt-files:**

1. **All agents are managers, not doers.** They dispatch sub-agents via the `task` tool, never call `shell` / `write` / `patch` / `read` directly. The "manager shell" pattern from the user's recent feedback.
2. **Unix-glyph status tree per turn.** Use `█` blocks for progress, `╭─╮╰─╯` for tables, `◉` for done, `○` for queued, `✗` for failed. Audit scores shown as `score/max · pct% · grade`.
3. **Persistent DAG/plan per session.** The first response of any session must end with a "Next Steps / DAG" block listing open PRs, branch state, build status, and pending todos.
4. **Don't shrink DAG to grow queue.** A 100-item DAG is fine. The 7-pillar rubric is sacred — every audit must reach 14/14 L4 before being marked done.
5. **All audit work goes through the hardened orchestrator** (`bin/absorption-justification.py`). The end-to-end validation on a sample repo (KodeVibe/Benchora/etc) confirmed the grader → orchestrator → audit file → project card pipeline works on Windows with `gh` available.

## Open decisions needing user signal

- D1: F15 (`platformkit/` deletion) — needs user inspect
- D2: F18 (phenotype-go-sdk archive) — gated on D1
- D3: Open PR #380 from B3-grade-BytePort-2026-07-02 → main — needs user review

## Operational notes

- WSL bash (e.g. `C:\Windows\System32\bash.exe -c "..."`) is the canonical shell for running the grader. `python` directly from cmd also works.
- The grader's `*_grade-all.sh` runs `grade.sh` on every `*-2026-*.md` file in `audits/absorption-justifications/`. Result is `GRADES.json` + `GRADES.md`.
- `*_grade-all.sh` is included in the registry repo and has been patched to scan `*-2026-06-*.md` (any date) so future audits are auto-included.
- All audits are at `C:\Users\koosh\phenotype-registry\audits\absorption-justifications\<name>-2026-MM-DD.md`. Project cards at `C:\Users\koosh\phenotype-registry\projects\<name>.json` (date-stripped) plus a date-suffixed backup.
