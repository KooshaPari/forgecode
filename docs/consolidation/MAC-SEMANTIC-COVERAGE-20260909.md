# Mac source coverage and first semantic bundles (2026-09-09)

Baseline captured before this ledger worktree was created. Main: `999b5c5046c047b7041fdfe0104e1e0135d3adb2`.
Historical PR257 ledger: 2652 source/ref records from three Mac/Windows checkouts, 1116 distinct tips. This lane refreshes Mac only, not Windows.

| Measure | Historical Mac | Current Mac |
|---|---:|---:|
| All refs |1195|1195|
| Local heads |31|44|
| Remote-tracking refs |753|739|
| Registered worktrees |16|29|
| Stashes |0|0|

Current Mac has 1093 unique tips; 31 refs added, 31 removed, 6 moved since historical snapshot. Counts include 395 tags,16 keep refs,1 tmp ref. Branch-only subset:783 records/704 tips (44 local,337 fork tracking,2 origin,400 upstream). Live fork had327 heads/283 tips before additive integration restoration; tracking is not live remote state.

## Squash-aware coverage

`git diff 06bf223d564cf30c3db3bfd4461bf798542b0a22 61ddccfd5530910e39a5cf7766be58708fb337af` is empty. The latter is main's PR254 squash. Main999 adds only the three h.0.1.7 version lines afterward. Therefore source ancestry maps through that verified squash, but does not assert every superseded historical behavior still exists.

Historical1116 tips:465 direct ancestors +12 verified squash-source ancestors =477 contained,625 pending,14 unknown. This precisely reconciles original477/625/14; no false regression from squash ancestry.
Current Mac1093 tips:457 direct +21 squash-source +615 requiring patch review.
All615 received `git cherry MAIN TIP`:71 have only patch-equivalent nonmerge commits;542 have unmatched patches;2 merge-only tips require aggregate-tree review. Patch equivalence does not prove merge-resolution equivalence or semantic uniqueness.

Dirty status only (no contents):4 worktrees/8 status entries. Root1; checksum worktree3; signing worktree2; historical coverage worktree2. All29 status commands succeeded.

## Preservation correction

Live integration source ref was absent. Verified commit locally, then restored additively via SSH without force:
`git push git@github.com:KooshaPari/forgecode.git 06bf223d564cf30c3db3bfd4461bf798542b0a22:refs/heads/integration/forgecode-h0.1.7`.
Live ls-remote confirmed exact SHA. This adds one remote ref, no new tip; not a merge or consolidation completion. No refs deleted/pruned/reset.

## First semantic bundles

1. Legacy schema visibility: `fix/forge-reasoning-retry-20260908` / `39c597afda0e43b90ebef6b2476320570eef67f9` (also remote repair/pr238-conversation-record-lints-20260904). Four unmatched commits in branch, four equivalent. Source tip adds missing-workspace sentinel and discovery read filters absent main. Must port semantically: source also widens DELETE/prune queries and reverts newer projection fixes if copied wholesale; retain strict mutation scope and current pragma/intent defaults. Regression-first port authorized.
2. Signing pipeline `fix/signing-pipeline` / `81cc94cb4a4bb62b61019df530de1ba508b126ba`: four unmatched patches, existing PR271 owner; do not duplicate.
3. Sandbox portability `fix/forge-sandbox-windows-config-20260901` / `ec13b571401e9617f5d7314bd5157b5d83ac96f4`: source tip changes /tmp to temp_dir, but current main uses TempDir fixture at affected test. Superseded semantic behavior; no mechanical reapplication.
4. `fix/stable-rustfmt-contract-20260829` / `dd4e1131f8cfe0e2a84402ddb58b7cce8069c0de`: one unmatched patch touching rustfmt and helper formatting; requires current policy comparison before bundling.
5. F3 `feat/forge-f3-semantic-memory-20260830` / `a9929f235d84e9f8ca26352908c213bbbfbb18c5`: two unmatched patches; existing reconciled F3 bundle exists, so semantic equivalence remains required.

## Method and limits

Independent 73-tip merge-effect audit completed: 67 have no uncovered merges, four have empty remerge diffs, and two stash resolution deltas are present in baseline. Evidence: `/private/tmp/forgecode-73-candidate-independent-audit-20260909.md`. Historical source coverage is therefore 551/1093 Mac tips; 542 remain unmatched. This is not a guarantee of current runtime semantics.

Exact remaining queue: `/tmp/forgecode-542-unmatched-baseline-20260909.json`, 586 source/ref aliases for542 baseline tips. Newly authored ledger and port branches are excluded from this fixed denominator. Frontier/shared-lineage audit delegated to independent reviewer.

First actual semantic port: PR272, commit `9f19f1b92`, source39c597af partial. New legacy-discovery regression failed main before port; full forge_repo414 passed/1ignored after port; strict clippy passed. Internal TEMP VIEW provenance flag distinguishes missing-schema workspace from real local/legacy workspace0. Source DELETE/prune/local-only FTS broadening deliberately excluded; source branch remains PARTIAL, other unmatched changes unclassified. See PR272's `LEGACY-DISCOVERY-PORT-20260909.md` for disposition and exact proof.

Read-only inventory script `/tmp/forgecode-mac-triage.py` uses for-each-ref, rev-list, commit peeling, full git cherry per pending tip, worktree porcelain/status and stash list. Raw historical snapshot remains under original coverage worktree. No private contents committed. No branch is approved for deletion or bulk merge. Windows live delta remains separate owner.
