# ForgeCode A+ Dogfood Recovery: DAG and WBS

## Objective and release rule

Produce one sponsor-evaluable ForgeCode build from the owned fork.  "Usable" means
that the exact evaluated commit is merged to `origin/main`, all required GitHub
checks for that commit are green, and a clean macOS worktree completes the documented
interactive dogfood path.  A branch-green result, a dirty worktree, or a local-only
binary does not satisfy this rule.

## Exact baseline (observed 2026-07-15)

| Item | Evidence | State |
| --- | --- | --- |
| Canonical remote main | `origin/main` = `3df15401267f9f31fbe85eedcb18e8190d4c19e2`, dated 2026-07-07, `feat(forge_infra): L133 cross-pollination (#93)` | Stale relative to active work |
| Repository root | `/Users/kooshapari/CodeProjects/Phenotype/repos/forgecode` is checked out at `e13e25c0` on `legacy/stash-0-AGENTS.md-2026-07-15` | Not an evaluator baseline |
| Existing evaluation worktree | `/Users/kooshapari/.config/superpowers/worktrees/forgecode/forge-eval-production` is `9d67435d` on `codex/forge-eval-production` with tracked and untracked changes | Evidence source only; not clean or canonical |
| Open recovery PR | [#94](https://github.com/KooshaPari/forgecode/pull/94), `fix/per-provider-error-resilience` (`69f83e9666bb5edbb455115a280fbb4528109dda`) to `main`, mergeable but failing required quality checks | Must be repaired or intentionally superseded |
| Worktree population | 18 registered worktrees: root, the evaluation worktree, and 16 `.claude/worktrees/*` feature/performance worktrees | Require individual disposition; no bulk cleanup |

### PR #94 verified failure classes

1. **Compile/test:** the provider-resilience change passes an `Option<EnvFilter>`
   where tracing subscriber configuration needs an `EnvFilter`.  Reproduce with the
   smallest affected crate test/check before changing code.
2. **cargo-deny:** CI invokes a cargo-deny command/argument shape incompatible with
   the installed CLI version.  Validate the workflow invocation against the pinned
   tool version, then add the narrowest workflow regression coverage available.
3. **TruffleHog:** the workflow installs/runs TruffleHog in a way that is not
   reproducible in its runner environment.  Pin the installation method/version and
   verify the scan command separately before relying on the aggregate job.
4. **Formatting:** `cargo fmt --all -- --check` fails.  Run rustfmt only after the
   semantic fix and include the resulting formatting diff in the PR.

The PR also has failures in Build and Test, nextest, clippy, and performance.  Those
are release gates and must be classified from their job logs; they must not be
silenced, marked advisory, or bypassed.

## Canonical recovery worktree

Create one new, clean worktree from the fetched owned-fork main, outside every
existing parked or feature worktree:

```text
git -C <repo> fetch origin --prune
git -C <repo> worktree add -b recovery/forgecode-a-plus-main \
  /Users/kooshapari/CodeProjects/Phenotype/worktrees/forgecode-a-plus-main \
  origin/main
```

This is the only recovery integration surface.  It starts with no copied edits,
no stash application, no generated artifacts, and no untracked product files.
All candidate work is compared to this SHA, integrated via a reviewed PR, and
evaluated again from a second fresh worktree after merge.

## Dependency graph

```text
F0 baseline + clean recovery worktree
 ├─ F1 reproduce and test EnvFilter root cause ─┐
 ├─ F2 repair cargo-deny invocation             ├─ F5 PR #94 quality gates green
 ├─ F3 make TruffleHog installation reproducible├─ F6 merge reviewed recovery PR to origin/main
 └─ F4 format + classify remaining job failures ┘       │
                                                       ├─ F7 fresh-main macOS binary/CLI smoke
Worktree triage (F8) ── individual PR/reject/archive ──┤
                                                       └─ F9 one-by-one product dogfood queue
```

## Work breakdown: root-cause-first and TDD

| ID | Work | Entry criterion | Exit evidence |
| --- | --- | --- | --- |
| F0 | Fetch remotes, record `origin/main` SHA, create canonical recovery worktree | No changes imported | Clean `git status --porcelain`; baseline SHA in recovery record |
| F1 | Write/reproduce the smallest failing test/check for `Option<EnvFilter>` | Exact compiler diagnostic captured | Test demonstrates bad input and corrected configuration; focused crate test passes |
| F2 | Reproduce cargo-deny workflow command with its pinned tool | CI log and workflow line identified | Exact command succeeds in a clean environment and workflow change is reviewed |
| F3 | Reproduce TruffleHog setup/scan in a runner-equivalent environment | Install/run failure classified | Pinned reproducible installation and scan command succeeds without disabling secret scanning |
| F4 | Apply rustfmt and classify every remaining failing #94 job | F1--F3 patches are present | `cargo fmt --all -- --check`, clippy, targeted test/nextest, build, and performance gate results recorded |
| F5 | Push the minimal repair branch and review PR #94 or its explicit successor | Local focused gates green | All required checks green at one head SHA; no advisory bypasses |
| F6 | Merge only the reviewed green SHA to `origin/main` | Branch protection/checks green | `origin/main` resolves to merged SHA and GitHub main checks green |
| F7 | Create a second clean macOS evaluator from merged `origin/main` | F6 complete | Build/install, `--help`, configured provider flow, normal request/stream, error path, and cleanup smoke evidence |
| F8 | Triage every registered worktree one-by-one | F0 complete; no bulk action | Each worktree has owner, purpose, base divergence, clean/dirty state, tests, PR URL, and disposition |
| F9 | Schedule dogfood consumption/compliance queue | F7 and relevant F8 PRs complete | Named workspace/branch runs with pass/fail evidence and regression backlog |

### 18-worktree triage protocol

For each registered worktree, in this order, record its path, branch, HEAD, merge
base against current `origin/main`, ahead/behind count, porcelain status, changed
files, intended outcome, test evidence, owner, and remote/PR linkage.  Select exactly
one disposition:

1. **Integrate:** rebase/cherry-pick only the coherent, tested delta into its own
   reviewed PR from the canonical recovery worktree.
2. **Preserve:** retain a clearly owned, active worktree unchanged while it is
   independently brought to a reviewable PR.
3. **Archive:** push any unique commits and document provenance; remove the worktree
   only after the owner/evidence confirms it is superseded.
4. **Reject:** leave data intact, document why it is out of scope or unsafe, and do
   not merge it.

Never reset, prune, relocate, delete, or combine worktrees in bulk.  A dirty
worktree is evidence to classify, not authority to absorb its contents.

## Acceptance checklist: merged main and macOS dogfood

### Merged-main quality scorecard (all required)

- [ ] `origin/main` points to the reviewed merge SHA; no local branch substitutes
      for it.
- [ ] GitHub main CI passes: build/test, nextest, rustfmt, clippy with warnings
      denied, cargo-deny, TruffleHog, and performance gates.
- [ ] `cargo fmt --all -- --check`, relevant `cargo check`/tests, and the release
      workflow commands are reproduced from a clean recovery/evaluator worktree.
- [ ] Security scans remain enabled; no token, secret, `--no-verify`, or advisory
      bypass is used to obtain green.
- [ ] Every intended feature-worktree contribution has a linked merged PR or a
      documented Preserve/Archive/Reject decision.

### macOS sponsor evaluation (all required)

- [ ] Fresh evaluator worktree is created from the merged `origin/main` SHA and is
      clean before build/install.
- [ ] The supported macOS build/install path completes with the pinned toolchain;
      executable version and SHA are recorded.
- [ ] CLI launch and `--help` work; first-run configuration is explicit and does not
      leak secrets.
- [ ] A real configured provider request exercises normal output/streaming, a
      provider failure shows graceful behavior, and a cancellation/error path leaves
      no stuck process or corrupt state.
- [ ] Logs, generated state, and network-facing behavior meet the project policy;
      uninstall/cleanup is documented and succeeds.
- [ ] Sponsor receives the exact SHA, commands, known limitations, and a concise
      pass/fail dogfood record.

## Definition of done

The recovery is done only when F0--F7 pass on the same merged-main lineage and F8
has a documented disposition for all 18 worktrees.  F9 then converts the stabilized
build into one-by-one workspace/branch/compliance dogfood, without allowing new
feature expansion to invalidate the release baseline.
